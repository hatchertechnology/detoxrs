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
    /// maximum matters. Fixed width so a lexical sort is a numeric sort.
    fn next_seq(dir: &Path) -> io::Result<u64> {
        let mut max = 0;
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(1),
            Err(e) => return Err(e),
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            if let Some(seq) = name
                .to_str()
                .and_then(|n| n.split('-').next())
                .and_then(|p| p.parse::<u64>().ok())
            {
                max = max.max(seq);
            }
        }
        Ok(max + 1)
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
                "separator": policy.separator.to_string(),
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
}

/// Read a batch journal.
///
/// A truncated final line is expected rather than exceptional -- this file is
/// append-only and a crash can cut it mid-write -- so it is ignored. Anything else
/// that does not add up goes in [`Replay::anomalies`] and is reported to the user.
///
/// # Errors
///
/// Any failure to open or read the file.
pub fn replay(path: &Path) -> io::Result<Replay> {
    let mut out = Replay::default();
    let mut pending: Option<(u64, UndoItem)> = None;
    let text = fs::read_to_string(path)?;
    let lines: Vec<&str> = text.lines().collect();

    for (n, line) in lines.iter().enumerate() {
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
                if let Some((ino, item)) = pending.take() {
                    out.anomalies.push(format!(
                        "line {}: a new intent starts while {} (inode {ino}) has no recorded \
                         outcome; that item cannot be undone and must be checked by hand",
                        n + 1,
                        item.original.to_string_lossy()
                    ));
                }
                match parse_intent(&rec) {
                    Some(item) => pending = Some((item.ident.ino, item)),
                    None => out.anomalies.push(format!(
                        "line {} is an intent record missing a field it needs; the item it \
                         describes cannot be undone",
                        n + 1
                    )),
                }
            }
            Some(op @ ("done" | "failed")) => {
                let claimed = rec.get("ino").and_then(Value::as_u64);
                match pending.take() {
                    // The outcome must name the intent it closes. Positional
                    // trust is what let an item disappear silently.
                    Some((ino, item)) if claimed == Some(ino) => {
                        if op == "done" {
                            out.items.push(item);
                        }
                    }
                    Some((ino, item)) => {
                        out.anomalies.push(format!(
                            "line {}: a '{op}' for inode {} closes an intent for inode {ino} \
                             ({}); neither is trusted and neither will be undone",
                            n + 1,
                            claimed.map_or_else(|| "?".to_owned(), |i| i.to_string()),
                            item.original.to_string_lossy()
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
    out.interrupted = pending.map(|(_, item)| item);
    out.items.reverse();
    Ok(out)
}

/// One `intent` record as an undoable item, or `None` if it is missing a field it
/// needs. The caller records that as an anomaly rather than skipping it quietly.
fn parse_intent(rec: &Value) -> Option<UndoItem> {
    Some(UndoItem {
        dir: PathBuf::from(get_os(rec, "dir")?),
        current: get_os(rec, "to")?,
        original: get_os(rec, "from")?,
        ident: Ident {
            dev: rec.get("dev")?.as_u64()?,
            ino: rec.get("ino")?.as_u64()?,
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

/// Every recorded batch, oldest first. The timestamp prefix is fixed-width, so
/// sorting the names sorts the batches.
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
    let mut out: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("jsonl")))
        .collect();
    out.sort();
    Ok(out)
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
            format!("{id:?} is not a batch id; ids look like 20260801T142233Z-a91c"),
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
    use super::{civil_from_days, get_os, put_os, utc_stamp};
    use serde_json::json;
    use std::ffi::OsStr;
    use std::time::{Duration, UNIX_EPOCH};

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
}
