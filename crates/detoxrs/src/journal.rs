//! The undo journal (proposal §5.5, §5.8; plan §7.3).
//!
//! Append-only JSONL, one file per batch, at
//! `$XDG_STATE_HOME/detoxrs/journal/<seq>-<UTC-timestamp>.jsonl`, falling back to
//! `$HOME/.local/state/...`. XDG names state as the home for "actions history".
//! Not `os.TempDir()` keyed by a hash of the working directory, which is what f2
//! does and which does not survive a reboot.
//!
//! **The protocol, and the reason the whole design is staked on it:** per item,
//! write an `intent` record, fsync it, *then* rename, then write `done` or
//! `failed`. A crash at any point therefore leaves a journal from which the exact
//! interrupted item is knowable: it is the one `intent` with no outcome. If the
//! `intent` cannot be written or fsynced the rename does **not** happen and the
//! batch aborts, because an unjournaled rename is the one thing `undo` cannot
//! reverse.
//!
//! ponytail: the fsync is on the journal *file*, not on the directory holding it,
//! so the guarantee is "survives `kill -9`" and not "survives power loss" — after
//! a hard power cut the file's own directory entry may not be there to find. That
//! is exactly the threat model §5.5 and §8.4 specify, and the tested one. The
//! upgrade is one `File::open(dir)?.sync_all()` at create time, plus `F_FULLFSYNC`
//! on Apple where a plain `fsync` on a directory promises less than it looks like
//! it does; it is not written here because it would be an untested syscall
//! defending against a case nobody has asked about.
//!
//! Records are built with `serde_json`, never hand-escaped. This artifact is the
//! safety net and it holds path-derived data; a byte-escaping bug here is exactly
//! what that dependency slot was spent to buy off.
//!
//! Two deliberate departures from §5.5's example line:
//!
//! * **No `policy_digest`.** There is no hash function in the dependency budget,
//!   and a digest nobody can recompute documents nothing. The policy's fields are
//!   written out verbatim instead, which is strictly more useful and costs one
//!   more line of JSON.
//! * **A non-UTF-8 directory path is written as `dir_bytes`,** an array of byte
//!   values, rather than as a lossily converted string. Names cannot need this —
//!   an undecodable name is `Skipped` and never reaches a rename — but the
//!   directory holding them can be undecodable on Linux, and a journal that
//!   records an approximation of a path is a journal that cannot undo.

use crate::fsops::RenameErr;
use detoxrs_core::plan::{EntryKind, Ident, PlanItem, Resolution};
use detoxrs_core::policy::Policy;
use serde_json::{Value, json};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// What `apply` needs from a journal, so a test can supply something else.
pub trait JournalWrite {
    /// Record the intention to rename, durably, before the rename happens.
    ///
    /// # Errors
    ///
    /// Any write or fsync failure. The caller must treat this as fatal to the
    /// whole batch and must not perform the rename.
    fn intent(&mut self, item: &PlanItem) -> io::Result<()>;

    /// Record that the rename happened.
    ///
    /// # Errors
    ///
    /// Any write failure.
    fn done(&mut self, item: &PlanItem) -> io::Result<()>;

    /// Record that the rename was attempted and failed.
    ///
    /// # Errors
    ///
    /// Any write failure.
    fn failed(&mut self, item: &PlanItem, why: RenameErr) -> io::Result<()>;
}

/// An open batch journal.
pub struct Journal {
    file: File,
    path: PathBuf,
    id: String,
}

impl Journal {
    /// Create this run's journal file and write its header.
    ///
    /// # Errors
    ///
    /// Any failure to locate, create, or write the state directory or the file.
    /// The caller must abort rather than proceed unjournaled.
    pub fn create(policy: &Policy, cwd: &Path) -> io::Result<Self> {
        let dir = journal_dir()?;
        fs::create_dir_all(&dir)?;
        let stamp = utc_stamp(SystemTime::now());

        // `create_new` so two runs starting at once cannot share a file, and the
        // sequence number so the retry lands on a *later*-sorting name.
        let mut seq = Self::next_seq(&dir)?;
        let mut last = None;
        for _ in 0..64_u32 {
            let id = format!("{seq:06}-{stamp}");
            let path = dir.join(format!("{id}.jsonl"));
            match OpenOptions::new().create_new(true).append(true).open(&path) {
                Ok(file) => {
                    let mut j = Self { file, path, id };
                    j.header(policy, cwd)?;
                    return Ok(j);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                    last = Some(e);
                    seq += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Err(last.unwrap_or_else(|| io::Error::other("cannot name a journal file")))
    }

    /// One past the highest sequence number already in the journal directory.
    ///
    /// **This is what makes `undo --last` correct**, and it is not a cosmetic
    /// choice. An earlier version named batches `<UTC-stamp>-<subsecond-hex>` and
    /// relied on that sorting in creation order; an adversarial review pointed out
    /// that `SystemTime::now()` is not monotonic, so a backward NTP step between two
    /// runs makes the later batch sort *first* and `undo --last` revert the wrong
    /// one. A counter read from the directory cannot do that, whatever the clock
    /// does. The timestamp stays in the name because it is what makes the directory
    /// readable by a human, but nothing depends on it for ordering any more.
    ///
    /// A gap in the sequence (someone deleted an old journal) is harmless: only the
    /// maximum matters. Fixed width so a lexical sort is a numeric sort -- for the
    /// first million batches; past that, ordering falls back to the parsed number
    /// rather than the padded text (see [`list`]).
    ///
    /// **C-10 / O3-3.** The directory's filenames are attacker- or corruption-
    /// reachable (a shared `XDG_STATE_HOME`, a hand-planted file), so `max` is
    /// untrusted input the same way a journal record's `from`/`to` are. A filename
    /// whose leading token parses as `u64::MAX` used to make `max + 1` overflow:
    /// a panic (exit 101) in a debug build, a silent wraparound to `0` in a
    /// release one. Refusing outright -- rather than saturating at `u64::MAX` and
    /// quietly reusing sequence numbers -- keeps the "only the maximum matters"
    /// invariant this function is named for from ever going backwards.
    fn next_seq(dir: &Path) -> io::Result<u64> {
        let mut max: u64 = 0;
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(1),
            Err(e) => return Err(e),
        };
        for entry in entries.filter_map(Result::ok) {
            if let Some(seq) = parse_seq(&entry.file_name()) {
                max = max.max(seq);
            }
        }
        max.checked_add(1).ok_or_else(|| {
            io::Error::other(
                "the journal directory already contains a filename at the highest possible \
                 sequence number; it is corrupt or hostile. Move it aside before running \
                 detoxrs again.",
            )
        })
    }

    /// This batch's id, which is also its file's stem: what `undo <BATCH-ID>` takes.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Where the journal was written, for the closing report.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Close the batch: a terminal record saying it ran to completion.
    ///
    /// Its *absence* is the information that matters. A journal with no `end`
    /// either belongs to a run that died or to one that is still going, and
    /// `undo` must say so instead of silently reverting a prefix of a batch that
    /// is still being written -- an adversarial review reproduced exactly that,
    /// leaving a permanently half-cleaned tree and exit 0.
    ///
    /// # Errors
    ///
    /// Any write or fsync failure. A batch that renamed successfully but could
    /// not be closed is reported, not silently downgraded.
    pub fn finish(&mut self) -> io::Result<()> {
        self.write_line(&json!({ "op": "end" }))?;
        self.file.sync_data()
    }

    fn header(&mut self, policy: &Policy, cwd: &Path) -> io::Result<()> {
        let mut rec = json!({
            "v": 1,
            "batch": self.id,
            // Verbatim rather than digested: see the module docs.
            "policy": {
                "separator": policy.separator().to_string(),
                "max_len_bytes": policy.max_len_bytes,
                "max_len_utf16": policy.max_len_utf16,
            },
        });
        put_os(&mut rec, "cwd", cwd.as_os_str());
        self.write_line(&rec)?;
        // The header is fsynced too. A journal whose header never reached disk is
        // a journal `undo` cannot identify.
        self.file.sync_data()
    }

    fn write_line(&mut self, rec: &Value) -> io::Result<()> {
        let mut line = serde_json::to_vec(rec).map_err(io::Error::other)?;
        line.push(b'\n');
        self.file.write_all(&line)
    }
}

impl JournalWrite for Journal {
    fn intent(&mut self, item: &PlanItem) -> io::Result<()> {
        let mut rec = json!({
            "op": "intent",
            "dev": item.ident.dev,
            "ino": item.ident.ino,
            "kind": kind_str(item.kind),
            "mtime": secs(item.ident.mtime),
        });
        // **Absolute, not as the user spelled it.** The plan carries `.` or
        // `nested dir`, which resolve against the cwd of the run that wrote them;
        // a journal is supposed to still mean something tomorrow from a different
        // directory, and a relative `dir` in it silently does not. `absolute` is
        // purely lexical and does not resolve symlinks, which matters here: what
        // gets renamed is a directory entry, and resolving the path would be
        // recording a different one.
        put_os(&mut rec, "dir", absolute(&item.dir).as_os_str());
        put_os(&mut rec, "from", &item.from);
        put_os(&mut rec, "to", &item.to);
        self.write_line(&rec)?;
        // The one fsync the design depends on. `sync_data` rather than
        // `sync_all`: the file's length and contents must be durable, its
        // mtime need not be.
        self.file.sync_data()
    }

    fn done(&mut self, item: &PlanItem) -> io::Result<()> {
        self.write_line(&json!({ "op": "done", "ino": item.ident.ino }))
    }

    fn failed(&mut self, item: &PlanItem, why: RenameErr) -> io::Result<()> {
        self.write_line(&json!({
            "op": "failed",
            "ino": item.ident.ino,
            "why": why.to_string(),
        }))
    }
}

/// One rename to put back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoItem {
    /// The directory it lives in. Renames never cross directories (§5.2), so
    /// this is the same for both names.
    pub dir: PathBuf,
    /// The name it has now: the forward run's destination.
    pub current: OsString,
    /// The name to put back: the forward run's source.
    pub original: OsString,
    /// Identity as the forward run recorded it. `undo` refuses an item whose
    /// current name no longer resolves to this.
    pub ident: Ident,
    /// What kind of entry it was, for the report only.
    pub kind: EntryKind,
}

impl UndoItem {
    /// The inverse rename, shaped as a plan item so `undo` can go through the
    /// same apply loop, the same identity recheck, the same no-clobber rename and
    /// the same journal as a forward run — which is what makes an undo itself
    /// undoable, at no extra code.
    #[must_use]
    pub fn as_plan_item(&self) -> PlanItem {
        PlanItem {
            dir: self.dir.clone(),
            from: self.current.clone(),
            to: self.original.clone(),
            kind: self.kind,
            ident: self.ident,
            depth: 0,
            resolution: Resolution::Rename,
            // An undo replays a completed rename verbatim; it never re-runs
            // `transform`, so there is nothing here for stage 12 to shorten.
            truncated: false,
        }
    }
}

/// What a journal file says happened.
#[derive(Debug, Default)]
pub struct Replay {
    /// The completed renames, **in the order `undo` must apply them**: reverse of
    /// the forward run. The forward run goes deepest-first, so reversing puts a
    /// directory back *before* the entries inside it, which is what makes each
    /// item's recorded `dir` resolve again by the time its turn comes.
    pub items: Vec<UndoItem>,
    /// The item whose outcome is unknown: an `intent` with neither `done` nor
    /// `failed` after it. At most one *can* exist in a journal a clean run wrote,
    /// because the forward loop records the outcome of item N before the intent of
    /// item N+1. That is now checked rather than trusted -- see `anomalies`.
    pub interrupted: Option<UndoItem>,
    /// Did the batch record that it finished? `false` means it crashed or is still
    /// running, and the caller must say so.
    pub complete: bool,
    /// Everything about this journal that does not add up.
    ///
    /// This field exists because an adversarial review fed `replay` a journal with
    /// two intents and one `done`, and one item vanished from the undo set with no
    /// error at all: outcomes used to close whatever intent happened to be pending
    /// without checking that they named the same inode. For the one file the whole
    /// safety story rests on, silence was the wrong failure mode.
    pub anomalies: Vec<String>,
    /// Renames the forward run completed and journalled (a `done` record exists),
    /// but whose own `intent` record failed to parse -- a hand-edited or corrupt
    /// record, or a name this build's platform now refuses (C1's `\` case, before
    /// that fix) -- so no [`UndoItem`] could be built for it.
    ///
    /// **C-11 / O3-2.** These items reach neither `items` nor a `Failed` outcome
    /// in `apply::Summary`, because they never become a `PlanItem` at all -- they
    /// are dropped before `undo`'s per-item loop even starts. Before this field
    /// existed, `main.rs`'s closing tally was built only from `renamed`/`failed`,
    /// so a batch that renamed 2 items and could only undo 1 printed
    /// `1 reverted, 0 refused` -- correct-looking arithmetic over the wrong total.
    /// The anomaly lines say the same thing on stderr, but the summary line is
    /// the one a user actually reads, and it must not contradict them.
    pub lost: usize,
}

/// Read a batch journal.
///
/// A truncated final line is expected rather than exceptional -- this file is
/// append-only and a crash can cut it mid-write -- so it is ignored. Anything else
/// that does not add up goes in [`Replay::anomalies`] and is reported to the user.
///
/// **C2: read as bytes, decode per line.** `fs::read_to_string` fails the
/// *whole* read on one invalid UTF-8 byte anywhere in the file, which turns
/// one flipped bit into an unrecoverable batch even though every other line
/// is perfectly good JSON. This file also legitimately carries non-UTF-8
/// bytes -- an undecodable directory path is written as `dir_bytes` (see the
/// module docs) -- so the file was never guaranteed to be valid UTF-8 as a
/// whole in the first place; treating it as one big `str` was always the odd
/// choice, not the safe one. Splitting on `\n` at the byte level first and
/// decoding one line at a time gives a bad byte the same per-line fault
/// tolerance a JSON syntax error already gets, below.
///
/// # Errors
///
/// Any failure to open or read the file.
pub fn replay(path: &Path) -> io::Result<Replay> {
    let mut out = Replay::default();
    // `Err(())` is a pending intent whose own record failed to parse -- its
    // inode (when the record had one) is kept anyway, purely so a `done`/`failed`
    // a few lines later can still be matched to it and the two anomalies don't
    // get reported as unrelated (C-11).
    let mut pending: Option<(u64, Result<UndoItem, ()>)> = None;
    let bytes = fs::read(path)?;
    let lines = split_lines(&bytes);

    for (n, line) in lines.iter().enumerate() {
        let Ok(line) = std::str::from_utf8(line) else {
            // Same tolerance as a JSON syntax error, and for the same reason:
            // only the last line can legitimately be half-written (a crash
            // mid-write can cut a multi-byte UTF-8 sequence in half, not just
            // a JSON token), so only report it when it is *not* the last line.
            if n + 1 < lines.len() {
                out.anomalies
                    .push(format!("line {} is not valid UTF-8 and was ignored", n + 1));
            }
            continue;
        };
        let Ok(rec) = serde_json::from_str::<Value>(line) else {
            // Only the last line can legitimately be half-written.
            if n + 1 < lines.len() {
                out.anomalies
                    .push(format!("line {} is not valid JSON and was ignored", n + 1));
            }
            continue;
        };
        match rec.get("op").and_then(Value::as_str) {
            Some("intent") => {
                if let Some((ino, prev)) = pending.take() {
                    let name = prev.as_ref().map_or_else(
                        |()| "?".to_owned(),
                        |item| item.original.to_string_lossy().into_owned(),
                    );
                    out.anomalies.push(format!(
                        "line {}: a new intent starts while {name} (inode {ino}) has no \
                         recorded outcome; that item cannot be undone and must be checked by \
                         hand",
                        n + 1,
                    ));
                }
                match parse_intent(&rec) {
                    Ok(item) => pending = Some((item.ident.ino, Ok(item))),
                    Err(reason) => {
                        out.anomalies.push(format!("line {} {reason}", n + 1));
                        // Keep the inode, if the record had one, purely so a
                        // later `done` can still be matched to this intent
                        // instead of reading as an unrelated, unexplained line.
                        if let Some(ino) = rec.get("ino").and_then(Value::as_u64) {
                            pending = Some((ino, Err(())));
                        }
                    }
                }
            }
            Some(op @ ("done" | "failed")) => {
                let claimed = rec.get("ino").and_then(Value::as_u64);
                match pending.take() {
                    // The outcome must name the intent it closes. Positional
                    // trust is what let an item disappear silently.
                    Some((ino, Ok(item))) if claimed == Some(ino) => {
                        if op == "done" {
                            out.items.push(item);
                        }
                    }
                    // C-11: the intent this closes never became an `UndoItem` --
                    // its own record was rejected -- so a `done` here is a
                    // completed rename this batch can never undo, not a
                    // no-op. `failed` needs no such counting: nothing was
                    // renamed, so there is nothing to lose.
                    Some((ino, Err(()))) if claimed == Some(ino) => {
                        if op == "done" {
                            out.lost += 1;
                            out.anomalies.push(format!(
                                "line {n_1}: inode {ino} was renamed by this batch, but its \
                                 intent record could not be read, so it cannot be undone and \
                                 must be checked by hand",
                                n_1 = n + 1
                            ));
                        }
                    }
                    Some((ino, Ok(item))) => {
                        out.anomalies.push(format!(
                            "line {}: a '{op}' for inode {} closes an intent for inode {ino} \
                             ({}); neither is trusted and neither will be undone",
                            n + 1,
                            claimed.map_or_else(|| "?".to_owned(), |i| i.to_string()),
                            item.original.to_string_lossy()
                        ));
                    }
                    Some((ino, Err(()))) => {
                        out.anomalies.push(format!(
                            "line {}: a '{op}' for inode {} closes an intent for inode {ino} \
                             that could not be read; neither is trusted and neither will be \
                             undone",
                            n + 1,
                            claimed.map_or_else(|| "?".to_owned(), |i| i.to_string()),
                        ));
                    }
                    None => out
                        .anomalies
                        .push(format!("line {}: a '{op}' with no intent before it", n + 1)),
                }
            }
            Some("end") => out.complete = true,
            _ => {}
        }
    }
    // A dangling `Err` (an unreadable intent with nothing after it) has already
    // put its own explanation in `anomalies` above; there is no `UndoItem` to
    // build for it, so it cannot become `interrupted` too, only a well-formed
    // pending intent can.
    out.interrupted = pending.and_then(|(_, item)| item.ok());
    out.items.reverse();
    Ok(out)
}

/// Split raw journal bytes into lines the way [`str::lines`] would, minus the
/// UTF-8 requirement: split on `\n`, and drop the one phantom empty element a
/// byte-level split leaves behind when the file ends with a newline (every
/// well-formed line does). An empty file has no lines, matching `"".lines()`.
///
/// This exists so [`replay`] can decode each line's UTF-8 independently
/// instead of requiring the whole file to be valid UTF-8 at once (C2) --
/// `\r\n` is not special-cased because this file is never written with `\r`.
fn split_lines(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&[u8]> = bytes.split(|&b| b == b'\n').collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines
}

/// True when `name` is usable as a single, ordinary path component: not empty,
/// not `.` or `..`, and free of whatever this platform's `rename`/`lstat`
/// calls treat as a path separator.
///
/// `\` is checked only under `cfg!(windows)`, not unconditionally. A journal
/// is portable text that *can* be replayed on a different platform than the
/// one that wrote it (C1), but the escape this guards against is real only on
/// the platform that will actually pass `from`/`to` to a rename syscall: on
/// Windows `MoveFileExW` and friends treat `\` as a separator, so a `\`-laden
/// name is a multi-component path in disguise there and must be refused. On
/// Unix `renameat` never treats `\` specially -- `Path::components()` even
/// parses it as one `Normal` component -- so a name containing it is exactly
/// as ordinary as any other basename, and rejecting it on this platform
/// serves no protective purpose while permanently orphaning any such file
/// from `undo` (it was rename-able by `-x` in the first place; `classes.rs`
/// only rewrites `\` in *sanitized* output names, never in the original
/// `from`). Checking `cfg!(windows)` here, rather than the platform that
/// wrote the record, is what keeps this in sync with the actual replay
/// syscall: it is *this* process's rename call the escape would have to go
/// through.
fn is_plain_basename(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    !bytes.is_empty()
        && bytes != b"."
        && bytes != b".."
        && !bytes.contains(&b'/')
        && (!cfg!(windows) || !bytes.contains(&b'\\'))
}

/// One `intent` record as an undoable item, or the reason it must be treated
/// as an anomaly instead. The caller records that reason rather than skipping
/// the line quietly.
///
/// **C1: this is where a journal record stops being trusted.** `from`/`to`
/// become the arguments to `statat`/`renameat` at replay time
/// (`apply.rs::attempt`), relative to the `dir` handle pinned in step 0. Those
/// syscalls take a *path*, not a name: a `from`/`to` containing `..` walks back
/// out of that handle, and an absolute one makes the kernel ignore the handle
/// entirely — either way `undo` reports a normal-looking revert while moving a
/// file outside the directory the dirfd pin exists to confine it to.
/// `journal::path_of` already treats the batch **id** as untrusted input for
/// exactly this reason ("a trust boundary is a trust boundary"); a record's
/// *contents* are equally untrusted — a shared or attacker-influenced
/// `XDG_STATE_HOME`, or a hand-edited journal, can put anything in this JSON —
/// so `from`/`to` are rejected outright unless each is a single plain
/// basename. This is refuse-not-sanitize on purpose: silently stripping `..`
/// would still let a crafted record rename to an attacker-chosen name, just a
/// differently-shaped one.
///
/// `dir` gets a different check. `Journal::intent` always writes it through
/// `absolute()` (above), so a well-formed record's `dir` is already absolute;
/// nothing downstream joins `dir` onto anything else before opening it
/// (`apply.rs::attempt` step 0 is `ops.open(&item.dir)` directly) — it *is* the
/// anchor, not a path appended to one — so there is no further component to
/// walk out of and no basename restriction to apply. Requiring "absolute" is
/// therefore both consistent with what this file always writes and sufficient
/// to catch a hand-edited or corrupted record: a relative `dir` would open
/// relative to whatever directory the `undo` process happens to be run from,
/// which is never what a journal line means.
fn parse_intent(rec: &Value) -> Result<UndoItem, &'static str> {
    const MISSING: &str = "is an intent record missing a field it needs; the item it describes \
                            cannot be undone";

    let dir = get_os(rec, "dir").ok_or(MISSING)?;
    let from = get_os(rec, "from").ok_or(MISSING)?;
    let to = get_os(rec, "to").ok_or(MISSING)?;
    let dev = rec.get("dev").and_then(Value::as_u64).ok_or(MISSING)?;
    let ino = rec.get("ino").and_then(Value::as_u64).ok_or(MISSING)?;

    if !is_plain_basename(&from) {
        return Err(
            "has a 'from' that is not a plain filename (a path separator, `.`, `..`, or empty); \
             refusing rather than risking a rename outside the pinned directory",
        );
    }
    if !is_plain_basename(&to) {
        return Err(
            "has a 'to' that is not a plain filename (a path separator, `.`, `..`, or empty); \
             refusing rather than risking a rename outside the pinned directory",
        );
    }
    if !Path::new(&dir).is_absolute() {
        return Err(
            "has a 'dir' that is not an absolute path; refusing rather than opening an \
             unknown location",
        );
    }

    Ok(UndoItem {
        dir: PathBuf::from(dir),
        current: to,
        original: from,
        ident: Ident {
            dev,
            ino,
            // Neither is used by the identity recheck, which compares
            // `(dev, ino)` only: a rename leaves mtime alone, so requiring it to
            // match would add nothing, and requiring nlink to match would refuse
            // an undo because someone made a hardlink in between.
            nlink: 1,
            mtime: UNIX_EPOCH,
        },
        kind: match rec.get("kind").and_then(Value::as_str) {
            Some("dir") => EntryKind::Dir,
            Some("symlink") => EntryKind::Symlink,
            Some("other") => EntryKind::Other,
            _ => EntryKind::File,
        },
    })
}

/// The leading `<seq>` token of a journal filename, as the total order batches
/// are actually created in.
///
/// **C-10 / O3-4.** `{seq:06}` is fixed-width only below 1 000 000: past it,
/// `"1000000-…"` sorts lexically *before* `"999999-…"`, so a byte sort of the
/// filenames stops being a sort of the batches. Parsing the number and ordering
/// on that instead of on the padded text is what keeps `--last` correct on both
/// sides of that boundary, and it is also what makes a name this program did
/// not write -- one with no numeric prefix at all -- a filename [`list`]
/// declines to treat as a batch, rather than one that quietly sorts wherever a
/// byte comparison happens to put it.
fn parse_seq(file_name: &OsStr) -> Option<u64> {
    file_name.to_str()?.split('-').next()?.parse::<u64>().ok()
}

/// Every recorded batch, oldest first, ordered by the sequence number actually
/// parsed out of each filename -- not by a lexical sort of the filenames
/// themselves, which stops agreeing with creation order past six digits (see
/// [`parse_seq`]). A `.jsonl` file whose name does not start with a plain
/// integer is not a batch this program could have written and is left out
/// entirely, rather than sorted in some undefined position and later handed to
/// `--last`.
///
/// # Errors
///
/// Any failure to locate or read the journal directory. A directory that does not
/// exist yet is not an error: it is an empty list.
pub fn list() -> io::Result<Vec<PathBuf>> {
    let dir = journal_dir()?;
    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out: Vec<(u64, PathBuf)> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("jsonl")))
        .filter_map(|p| {
            let seq = parse_seq(p.file_stem()?)?;
            Some((seq, p))
        })
        .collect();
    out.sort_by_key(|(seq, _)| *seq);
    Ok(out.into_iter().map(|(_, p)| p).collect())
}

/// The path of a batch named by id.
///
/// The id comes from the command line, so it is validated rather than trusted: it
/// names one file inside the journal directory and nothing else. Nothing terrible
/// is reachable through it — a journal is only ever read, and every rename it
/// describes still goes through the identity recheck — but "the worst case looks
/// survivable" is not a reason to join unvalidated input onto a path.
///
/// # Errors
///
/// [`io::ErrorKind::InvalidInput`] if the id contains a path separator or `..`;
/// any failure to locate the journal directory.
pub fn path_of(id: &str) -> io::Result<PathBuf> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{id:?} is not a batch id; ids look like 000001-20260803T184819Z"),
        ));
    }
    Ok(journal_dir()?.join(format!("{id}.jsonl")))
}

/// `path` made absolute without resolving a single symlink.
///
/// Falls back to the path as given if the cwd cannot be read: a journal line with
/// a relative directory is worse than one with an absolute directory, and both are
/// better than no line at all, which is what returning an error here would cost.
fn absolute(path: &Path) -> PathBuf {
    // A bare relative argument (`detoxrs file.txt`) has an empty parent, and
    // `std::path::absolute("")` is an error rather than the cwd.
    let path = if path.as_os_str().is_empty() {
        Path::new(".")
    } else {
        path
    };
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `$XDG_STATE_HOME/detoxrs/journal`, or `$HOME/.local/state/detoxrs/journal`.
///
/// # Errors
///
/// [`io::ErrorKind::NotFound`] when neither variable is set, which is the one
/// case where there is nowhere durable to put a journal and guessing would be
/// worse than saying so.
pub fn journal_dir() -> io::Result<PathBuf> {
    let base = if let Some(v) = std::env::var_os("XDG_STATE_HOME").filter(|v| !v.is_empty()) {
        PathBuf::from(v)
    } else {
        let home = std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "neither XDG_STATE_HOME nor HOME is set, so there is nowhere to write an \
                     undo journal",
                )
            })?;
        PathBuf::from(home).join(".local").join("state")
    };
    Ok(base.join("detoxrs").join("journal"))
}

const fn kind_str(k: EntryKind) -> &'static str {
    match k {
        EntryKind::File => "file",
        EntryKind::Dir => "dir",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

fn secs(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Store an `OsStr` under `key` when it is text, under `<key>_bytes` when it is
/// not. See the module docs for why there is no third option.
fn put_os(rec: &mut Value, key: &str, value: &OsStr) {
    let Some(map) = rec.as_object_mut() else {
        return;
    };
    if let Some(text) = value.to_str() {
        map.insert(key.to_owned(), Value::from(text));
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        map.insert(format!("{key}_bytes"), Value::from(value.as_bytes()));
    }
    // Windows is best-effort (owner decision, 2026-07-31): `OsStr` there is WTF-8
    // with no stable byte accessor, so a name that is not `to_str`-able cannot be
    // recorded exactly. It is recorded as *unrecordable* rather than as a lossy
    // approximation, which makes that one item non-undoable instead of making
    // `undo` rename the wrong path.
    #[cfg(not(unix))]
    {
        map.insert(format!("{key}_unrepresentable"), Value::from(true));
    }
}

/// The inverse of [`put_os`]. `None` when the value is absent or was recorded as
/// unrepresentable, which the caller turns into "this item cannot be undone".
fn get_os(rec: &Value, key: &str) -> Option<OsString> {
    if let Some(text) = rec.get(key).and_then(Value::as_str) {
        return Some(OsString::from(text));
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt as _;
        let bytes: Vec<u8> = rec
            .get(format!("{key}_bytes"))?
            .as_array()?
            .iter()
            .map(|v| u8::try_from(v.as_u64()?).ok())
            .collect::<Option<_>>()?;
        Some(OsString::from_vec(bytes))
    }
    #[cfg(not(unix))]
    None
}

/// `YYYYMMDDTHHMMSSZ`, UTC.
///
/// Hand-rolled rather than a `chrono`/`time` dependency, and it is 15 lines
/// because the calendar is the only hard part. Days-since-epoch to a civil date
/// is Hinnant's algorithm, which is exact for every date this program can see.
fn utc_stamp(t: SystemTime) -> String {
    let secs = secs(t);
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let sod = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}{m:02}{d:02}T{:02}{:02}{:02}Z",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Days since 1970-01-01 to a proleptic Gregorian `(year, month, day)`.
/// The single-letter names are the published algorithm's own; renaming them to
/// `era_adjusted_year` and friends would make the correspondence to the reference
/// unreviewable, which is the only way anyone checks calendar code.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, get_os, parse_seq, put_os, replay, utc_stamp};
    use serde_json::json;
    use std::ffi::OsStr;
    use std::time::{Duration, UNIX_EPOCH};

    /// A [`super::JournalWrite`] that records nothing, for tests that only care
    /// about what ends up on disk, not about the undo-of-the-undo journal.
    struct NullJournal;
    impl super::JournalWrite for NullJournal {
        fn intent(&mut self, _item: &detoxrs_core::plan::PlanItem) -> std::io::Result<()> {
            Ok(())
        }
        fn done(&mut self, _item: &detoxrs_core::plan::PlanItem) -> std::io::Result<()> {
            Ok(())
        }
        fn failed(
            &mut self,
            _item: &detoxrs_core::plan::PlanItem,
            _why: super::RenameErr,
        ) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// C1. A forged journal with a relative-traversal record (`from`:
    /// `"../../ESCAPED.txt"`) and an absolute-path record, alongside one
    /// legitimate record, replayed and then actually run through `apply::undo`
    /// with the real platform rename ops -- on a real temporary tree, not a
    /// fake `RenameOps`, so nothing here can pass by construction.
    ///
    /// Before the fix, `parse_intent` copied `from`/`to` verbatim into the
    /// `UndoItem`, both escape records became ordinary items, and `apply::undo`
    /// (which only ever re-checks *identity*, never confinement) renamed the
    /// pinned directory's `victim2.txt`/`victim3.txt` straight to
    /// `../../ESCAPED.txt` and to an absolute path outside the tree entirely --
    /// exit 0, reported as a normal revert. The assertions below are on the
    /// filesystem after the real apply loop has run, not on `replay`'s own
    /// output, so a change that satisfies `replay` alone without actually
    /// stopping the rename would still be caught here.
    #[test]
    fn undo_refuses_a_traversal_record_instead_of_escaping_the_pinned_directory() {
        use crate::apply;
        use crate::fsops::PlatformRenameOps;
        use std::fs;
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt as _;

        let base = tempfile::tempdir().expect("tempdir");
        let inner = base.path().join("exp").join("inner");
        fs::create_dir_all(&inner).expect("mkdir -p exp/inner");

        fs::write(inner.join("victim1.txt"), b"legit").expect("write victim1");
        fs::write(inner.join("victim2.txt"), b"must not move").expect("write victim2");
        fs::write(inner.join("victim3.txt"), b"must not move either").expect("write victim3");
        let meta1 = fs::metadata(inner.join("victim1.txt")).expect("stat victim1");

        // ".." from `inner` is `exp`; ".." again is `base` -- two levels out.
        let rel_escape_target = base.path().join("ESCAPED.txt");
        let abs_escape_target = base.path().join("ABS_ESCAPED.txt");

        let records = [
            // The one legitimate record: undo should really revert this.
            json!({
                "op": "intent", "dev": meta1.dev(), "ino": meta1.ino(), "kind": "file",
                "mtime": 0, "dir": inner.to_str().unwrap(),
                "from": "original.txt", "to": "victim1.txt",
            }),
            json!({ "op": "done", "ino": meta1.ino() }),
            // Relative traversal.
            json!({
                "op": "intent", "dev": 999, "ino": 999, "kind": "file", "mtime": 0,
                "dir": inner.to_str().unwrap(),
                "from": "../../ESCAPED.txt", "to": "victim2.txt",
            }),
            json!({ "op": "done", "ino": 999 }),
            // Absolute path, ignoring the dirfd pin altogether.
            json!({
                "op": "intent", "dev": 998, "ino": 998, "kind": "file", "mtime": 0,
                "dir": inner.to_str().unwrap(),
                "from": abs_escape_target.to_str().unwrap(), "to": "victim3.txt",
            }),
            json!({ "op": "done", "ino": 998 }),
            json!({ "op": "end" }),
        ];
        let mut text = String::new();
        for r in &records {
            text.push_str(&r.to_string());
            text.push('\n');
        }
        let journal_path = base.path().join("forged.jsonl");
        fs::write(&journal_path, text).expect("write journal");

        let replayed = replay(&journal_path).expect("a well-formed journal always reads");
        // Both escape attempts must be refused before they ever become an
        // `UndoItem`, only the legitimate record survives to `items`.
        assert_eq!(
            replayed.items.len(),
            1,
            "escape records must not reach the apply loop: {:?}",
            replayed.anomalies
        );

        let mut journal = NullJournal;
        let mut out = Vec::new();
        let _ = apply::undo(&replayed.items, &PlatformRenameOps, &mut journal, &mut out);

        // The legitimate item really was reverted...
        assert!(!inner.join("victim1.txt").exists());
        assert!(inner.join("original.txt").exists());
        // ...and neither escape attempt touched the filesystem, inside the
        // pinned directory or out. This is the load-bearing assertion: it is
        // on disk, derived from nothing the code under test reports about
        // itself.
        assert!(
            !rel_escape_target.exists(),
            "a relative-traversal record must not create {rel_escape_target:?}"
        );
        assert!(
            !abs_escape_target.exists(),
            "an absolute-path record must not create {abs_escape_target:?}"
        );
        assert!(inner.join("victim2.txt").exists(), "victim2.txt untouched");
        assert!(inner.join("victim3.txt").exists(), "victim3.txt untouched");
    }

    /// A name containing `\` is an ordinary basename on Unix -- `-x` renames
    /// it like any other -- so the journal that recorded the rename must be
    /// able to record and replay it too, or `undo` silently and permanently
    /// loses the file. Before the fix, `is_plain_basename` rejected any `from`
    /// containing `\` unconditionally (mistaking a byte that is only a
    /// separator on Windows for one that is a separator everywhere), so
    /// `replay` turned this record into an anomaly instead of an [`UndoItem`]
    /// and the round trip below failed with 0 items replayed.
    #[cfg(unix)]
    #[test]
    fn a_backslash_named_file_round_trips_through_undo() {
        use crate::apply;
        use crate::fsops::PlatformRenameOps;
        use std::fs;
        use std::os::unix::fs::MetadataExt as _;

        let base = tempfile::tempdir().expect("tempdir");
        let original = "back\\slash.txt";
        let renamed = "back_slash.txt";
        fs::write(base.path().join(original), b"payload").expect("write original");

        // What `-x` itself would do: rename the backslash-named file, then
        // record the intent. Order matches the journal's own write-then-act
        // protocol closely enough for this test's purpose -- what matters
        // here is that the record survives `replay`.
        fs::rename(base.path().join(original), base.path().join(renamed)).expect("rename");
        let meta = fs::metadata(base.path().join(renamed)).expect("stat renamed");

        let records = [
            json!({
                "op": "intent", "dev": meta.dev(), "ino": meta.ino(), "kind": "file",
                "mtime": 0, "dir": base.path().to_str().unwrap(),
                "from": original, "to": renamed,
            }),
            json!({ "op": "done", "ino": meta.ino() }),
            json!({ "op": "end" }),
        ];
        let mut text = String::new();
        for r in &records {
            text.push_str(&r.to_string());
            text.push('\n');
        }
        let journal_path = base.path().join("j.jsonl");
        fs::write(&journal_path, text).expect("write journal");

        let replayed = replay(&journal_path).expect("a well-formed journal always reads");
        assert_eq!(
            replayed.items.len(),
            1,
            "a `\\` in `from` must not be treated as a path separator on Unix: {:?}",
            replayed.anomalies
        );

        let mut journal = NullJournal;
        let mut out = Vec::new();
        let u = apply::undo(&replayed.items, &PlatformRenameOps, &mut journal, &mut out);

        assert_eq!(u.renamed, 1, "{:?}", String::from_utf8_lossy(&out));
        assert!(
            base.path().join(original).exists(),
            "undo must restore the original backslash-named file"
        );
        assert!(!base.path().join(renamed).exists());
    }

    /// C2. One invalid UTF-8 byte in the middle of an otherwise-good journal
    /// must cost exactly that line, not the whole file. Before the fix,
    /// `replay` read the file with `fs::read_to_string`, which fails the whole
    /// read on this byte, so the two good renames either side of it became
    /// unrecoverable too (`exit 2`, nothing reported).
    #[test]
    fn replay_recovers_lines_around_one_invalid_utf8_byte() {
        use std::fs;

        let base = tempfile::tempdir().expect("tempdir");
        let path = base.path().join("j.jsonl");

        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            br#"{"op":"intent","dev":1,"ino":1,"kind":"file","mtime":0,"dir":"/tmp/x","from":"a.txt","to":"a2.txt"}"#,
        );
        bytes.push(b'\n');
        bytes.extend_from_slice(br#"{"op":"done","ino":1}"#);
        bytes.push(b'\n');
        // Not valid UTF-8 at all -- `fs::read_to_string` fails the whole file
        // on this line; `fs::read` plus a per-line decode must not.
        bytes.extend_from_slice(&[0xFF, 0xFE, b'g', b'a', b'r', b'b', b'a', b'g', b'e']);
        bytes.push(b'\n');
        bytes.extend_from_slice(
            br#"{"op":"intent","dev":2,"ino":2,"kind":"file","mtime":0,"dir":"/tmp/x","from":"b.txt","to":"b2.txt"}"#,
        );
        bytes.push(b'\n');
        bytes.extend_from_slice(br#"{"op":"done","ino":2}"#);
        bytes.push(b'\n');
        bytes.extend_from_slice(br#"{"op":"end"}"#);
        bytes.push(b'\n');
        fs::write(&path, &bytes).expect("write journal");

        let replayed =
            replay(&path).expect("one bad byte must not fail the whole read -- that is C2");
        assert_eq!(replayed.items.len(), 2, "{:?}", replayed.anomalies);
        assert!(
            replayed
                .anomalies
                .iter()
                .any(|a| a.contains("not valid UTF-8")),
            "{:?}",
            replayed.anomalies
        );
    }

    /// The one thing in this file that is arithmetic rather than I/O, so the one
    /// thing that can be wrong silently. Leap day, century non-leap, and the
    /// 400-year leap are all in here.
    #[test]
    fn the_calendar_is_right() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
        assert_eq!(civil_from_days(20_666), (2026, 8, 1));
    }

    #[test]
    fn the_stamp_is_sortable_utc() {
        let t = UNIX_EPOCH + Duration::from_secs(1_785_000_023);
        assert_eq!(utc_stamp(t), "20260725T172023Z");
        assert_eq!(utc_stamp(UNIX_EPOCH), "19700101T000000Z");
    }

    #[test]
    fn a_text_name_round_trips_as_a_string() {
        let mut rec = json!({});
        put_os(&mut rec, "from", OsStr::new("Björk – Volta.mp3"));
        assert!(rec.get("from").is_some(), "{rec}");
        assert_eq!(
            get_os(&rec, "from").as_deref(),
            Some(OsStr::new("Björk – Volta.mp3"))
        );
    }

    /// The case the module docs exist for: a path that is not text is recorded
    /// exactly, not approximately, so `undo` can still name it.
    #[cfg(unix)]
    #[test]
    fn an_undecodable_path_round_trips_as_bytes() {
        use std::os::unix::ffi::OsStrExt as _;
        let raw = OsStr::from_bytes(b"/tmp/bad\xffdir");
        let mut rec = json!({});
        put_os(&mut rec, "dir", raw);
        assert!(
            rec.get("dir").is_none(),
            "must not be a lossy string: {rec}"
        );
        assert_eq!(get_os(&rec, "dir").as_deref(), Some(raw));
    }

    /// C-10 / O3-3. A journal directory can contain a filename this program did
    /// not write -- planted, corrupted, or (worst case) an attacker sharing
    /// `XDG_STATE_HOME` -- and a sequence number of `u64::MAX` used to make
    /// `next_seq`'s `max + 1` overflow: a panic in a debug build, a silent
    /// wraparound in a release one. It must instead be a plain, reported `Err`.
    #[test]
    fn next_seq_refuses_rather_than_overflowing_on_a_crafted_filename() {
        use std::fs;

        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path()
                .join(format!("{}-20260101T000000Z.jsonl", u64::MAX)),
            b"",
        )
        .expect("write crafted filename");

        let result = super::Journal::next_seq(dir.path());
        assert!(
            result.is_err(),
            "a sequence number at u64::MAX must be refused, not overflowed: {result:?}"
        );
    }

    /// C-10 / O3-4. `{seq:06}` is fixed-width only below 1 000 000: past that,
    /// `"1000000-…"` sorts lexically *before* `"999999-…"`, so a byte sort of
    /// the filenames no longer agrees with creation order. `list` must order by
    /// the parsed sequence number instead, or `--last` silently reverts an
    /// older batch once any batch crosses six digits.
    #[test]
    fn list_orders_by_parsed_sequence_not_filename_bytes() {
        use std::fs;
        use std::path::PathBuf;

        let dir = tempfile::tempdir().expect("tempdir");
        // A byte sort would put "1000000-..." before "999999-...": '1' < '9'.
        fs::write(dir.path().join("999999-20260101T000000Z.jsonl"), b"").expect("write");
        fs::write(dir.path().join("1000000-20260102T000000Z.jsonl"), b"").expect("write");

        let mut names: Vec<u64> = fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension() == Some(OsStr::new("jsonl")))
            .filter_map(|p| parse_seq(p.file_stem()?))
            .collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec![999_999, 1_000_000],
            "the numeric order, not the lexical one"
        );

        // Same assertion `list()` itself would give, if it read this directory:
        // reimplemented against the private `journal_dir` is not possible from
        // here, so this pins the ordering primitive `list()` is built on.
        let mut seqs: Vec<(u64, PathBuf)> = fs::read_dir(dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension() == Some(OsStr::new("jsonl")))
            .filter_map(|p| Some((parse_seq(p.file_stem()?)?, p)))
            .collect();
        seqs.sort_by_key(|(seq, _)| *seq);
        assert_eq!(seqs[0].0, 999_999);
        assert_eq!(seqs[1].0, 1_000_000, "the newest batch must sort last");
    }

    /// C-11 / O3-2. A `done` record can close an `intent` that failed to parse
    /// (a corrupt or hand-edited record): a rename really happened and was
    /// journalled, but no [`UndoItem`] can be built for it. That item must be
    /// counted as lost, not silently absent from both `items` and the tally.
    #[test]
    fn a_done_for_an_unparseable_intent_is_counted_as_lost_not_dropped() {
        use std::fs;

        let base = tempfile::tempdir().expect("tempdir");
        let path = base.path().join("j.jsonl");

        let records = [
            // One legitimate item.
            json!({
                "op": "intent", "dev": 1, "ino": 1, "kind": "file", "mtime": 0,
                "dir": "/tmp/x", "from": "keep.txt", "to": "keep_ok.txt",
            }),
            json!({ "op": "done", "ino": 1 }),
            // An intent missing required fields (dev): parse_intent fails, but
            // the forward run still renamed and journalled it.
            json!({
                "op": "intent", "ino": 999_999, "kind": "file", "mtime": 0,
                "dir": "/tmp/x", "from": "lost.txt", "to": "lost_ok.txt",
            }),
            json!({ "op": "done", "ino": 999_999 }),
            json!({ "op": "end" }),
        ];
        let mut text = String::new();
        for r in &records {
            text.push_str(&r.to_string());
            text.push('\n');
        }
        fs::write(&path, text).expect("write journal");

        let replayed = replay(&path).expect("a well-formed journal always reads");
        assert_eq!(replayed.items.len(), 1, "{:?}", replayed.anomalies);
        assert_eq!(
            replayed.lost, 1,
            "the renamed-but-unparseable item must be counted, not vanish: {:?}",
            replayed.anomalies
        );
    }
}
