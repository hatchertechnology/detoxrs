//! The snapshot walk (proposal §5.1, §5.6; plan §7.3).
//!
//! Phase one of two, and it runs to completion before anything else happens:
//! `snapshot` returns an owned `Vec<Entry>` and nothing downstream re-reads the
//! directory. detox renames a directory and then recurses into its new path,
//! which is the hazard its own maintainer named; here the list is frozen first.
//!
//! Rules, each one a decision recorded in §5.6 rather than an implementation
//! detail:
//!
//! * **`-r` is the only thing that descends.** Without it, a directory argument
//!   has **only its own basename cleaned** and nothing inside it is touched.
//!   detox differs: its `-r` gates descent only *past* the first level, so a
//!   named directory's immediate children are processed either way. That quirk
//!   is deliberately not copied — a flag whose scope is one level deeper than it
//!   reads produces a preview the user will misjudge. **Settled by owner ruling on
//!   2026-08-01** (`docs/owner-decisions.md`), which closed the contradiction
//!   between §5.6/§2.4/§9.2 and §2.2's worked example in favour of the three
//!   sections; §2.2 now carries a warning block rather than a rewritten example.
//! * **A symlinked directory is never descended, and there is no flag for it,
//!   no matter how the argument is spelled.** `follow_links(false)` guards
//!   descent into entries the walk *discovers*, but it says nothing about the
//!   walk's own root: POSIX resolves a trailing slash by dereferencing, so
//!   `lstat("link/")` returns the *target's* metadata and a naive check would
//!   see a directory and descend. Every argument is normalized (trailing
//!   separators stripped) before its own `lstat` for exactly this reason —
//!   shell tab-completion appends that slash by default, so `link` and
//!   `link/` must answer identically. The link's own name is still cleaned, as
//!   any other directory entry would be.
//! * **`.git`, `.hg`, `.svn` are skipped unconditionally**, and there will be no
//!   option to include them.
//! * **Dotfiles are skipped while recursing, processed when named explicitly.**
//! * **`symlink_metadata` only.** Never `stat`: what gets renamed is a directory
//!   entry, so the entry is what gets inspected.

use detoxrs_core::decode::{Decoded, decode};
use detoxrs_core::pipeline::{TransformResult, transform};
use detoxrs_core::plan::{DirIdent, Entry, EntryKind, Ident, VolumeCase};
use detoxrs_core::policy::Policy;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, Metadata};
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

/// Why there is no snapshot.
///
/// Both variants abort the run before any output: an incomplete snapshot is the
/// one thing the two-phase design in §5.1 cannot tolerate, so a walk that could
/// not see what it was asked to see does not get to print a partial preview.
#[derive(Debug)]
pub enum WalkError {
    /// A path named on the command line could not be inspected at all.
    Unreadable(PathBuf, io::Error),
    /// Descriptor exhaustion. Continuing would produce a silently short list.
    OutOfDescriptors(io::Error),
}

impl fmt::Display for WalkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable(p, e) => write!(f, "cannot read {}: {e}", p.display()),
            Self::OutOfDescriptors(e) => write!(
                f,
                "out of file descriptors ({e}); refusing to plan from an incomplete walk"
            ),
        }
    }
}

/// Freeze the entry list.
///
/// Warnings about individual unreadable directories encountered *during*
/// recursion go to stderr and the walk continues (§5.8, matching detox). Only
/// the two [`WalkError`] cases stop it.
///
/// # Errors
///
/// [`WalkError::Unreadable`] if a named path cannot be `lstat`ed;
/// [`WalkError::OutOfDescriptors`] on `EMFILE`/`ENFILE`.
pub fn snapshot(paths: &[PathBuf], recursive: bool) -> Result<Vec<Entry>, WalkError> {
    let mut out = Vec::new();
    // Overlapping arguments (`detoxrs a a`, or `a` and `a/b` under `-r`) would
    // otherwise put one directory entry in the snapshot twice, and the planner
    // would treat the second copy as a pre-existing occupant of the first one's
    // name. Deduplicating here is cheaper than teaching the planner about it.
    //
    // Keyed on the containing directory's *identity* plus the entry's own
    // name, not on the directory's textual spelling (C8): `.`, ``, and `./x`
    // are three different strings for one directory, and a text-keyed set
    // would let the same directory entry in twice under two arguments that
    // name it differently. Keying on the entry's own `(dev, ino)` instead (as
    // first proposed) is the wrong identity to dedupe on: two hardlinks in two
    // different directories share an inode on purpose and must stay two
    // entries, so what has to match is "this directory, this name", not "this
    // inode".
    let mut seen: HashSet<(DirIdent, OsString)> = HashSet::new();
    // The containing directory's own identity, cached by its path spelling so
    // a directory reached under two different spellings is `lstat`ed once per
    // spelling but resolves to one identity either way. `plan()`'s collision
    // engine keys on this same identity (`Entry::dir_ident`) for the same
    // reason.
    let mut dir_idents: HashMap<PathBuf, DirIdent> = HashMap::new();
    // C9: `plan()` stays I/O-free (its own module doc), so a destination that
    // already exists outside the walked set is checked here, once, before
    // `plan()` ever runs. M1 has no transform flags yet (main.rs's own comment
    // at its `Policy::default()` call), so this is the only policy there is to
    // check against.
    //
    // ponytail: a threaded `&Policy` parameter is dead flexibility until M3's
    // config file makes more than one policy reachable from here.
    let policy = Policy::default();

    for path in paths {
        // C3: strip a trailing separator before the *first* `lstat`, not after.
        // POSIX dereferences a trailing slash, so `lstat("link/")` would
        // otherwise return the target's metadata and `md.is_dir()` would lie.
        let lstat_path = trim_trailing_slash(path);
        let md = fs::symlink_metadata(&lstat_path)
            .map_err(|e| WalkError::Unreadable(path.clone(), e))?;
        // #3: use the name `readdir` actually stores for this argument, not
        // the bytes typed on the command line -- see `corrected_top_level_path`.
        let real_path = corrected_top_level_path(&lstat_path, &md);
        push(&mut out, &mut seen, &mut dir_idents, &real_path, &md, 0);
        // #1: a top-level argument's own basename can collide with a sibling
        // whether or not `-r` is present -- `walk_into` below only ever
        // checks *inside* a directory argument, never beside it, so this
        // must run unconditionally rather than only when recursion is skipped.
        seed_pre_existing_destination(
            &mut out,
            &mut seen,
            &mut dir_idents,
            &real_path,
            &md,
            &policy,
        );

        if recursive && md.is_dir() {
            walk_into(&mut out, &mut seen, &mut dir_idents, &lstat_path)?;
        }
    }
    Ok(out)
}

/// The name `readdir` actually stores for `path`, not the bytes the argument
/// happened to spell (#3).
///
/// A normalization-insensitive lookup filesystem (APFS) can resolve an
/// argument successfully even when the bytes typed are not the bytes stored:
/// an NFC-typed argument can find an NFD-stored file, and vice versa.
/// Recursive discovery never has this ambiguity -- `WalkDir` hands back
/// whatever `readdir` returned for each entry it finds -- so a top-level
/// argument is normalized to match: list its directory once and keep
/// whichever entry shares the argument's identity. Falls back to the
/// argument's own bytes when identity is unavailable (non-unix) or the
/// directory cannot be listed, which is no worse than before this existed.
fn corrected_top_level_path(path: &Path, md: &Metadata) -> PathBuf {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return path.to_path_buf();
    };
    dev_ino_of(md).map_or_else(
        || path.to_path_buf(),
        |own| dir.join(real_entry_name(dir, own, name)),
    )
}

#[cfg(unix)]
fn real_entry_name(dir: &Path, own: (u64, u64), fallback: &OsStr) -> OsString {
    let Ok(read) = fs::read_dir(syscall_path(dir)) else {
        return fallback.to_os_string();
    };
    for entry in read.flatten() {
        if entry.metadata().is_ok_and(|md| unix_dev_ino(&md) == own) {
            return entry.file_name();
        }
    }
    fallback.to_os_string()
}

#[cfg(not(unix))]
fn real_entry_name(_dir: &Path, _own: (u64, u64), fallback: &OsStr) -> OsString {
    fallback.to_os_string()
}

/// Drop a trailing separator (or several) so a symlink argument's own `lstat`
/// cannot be tricked into dereferencing it (C3).
///
/// `Components::as_path` already normalizes this away when it rebuilds a path
/// -- a trailing slash is not its own component -- so this is exactly the
/// stdlib's own notion of "the same path" and nothing hand-rolled. `.`, `..`,
/// and `/` alone are untouched: none of them have a trailing separator to
/// drop.
fn trim_trailing_slash(path: &Path) -> PathBuf {
    path.components().as_path().to_path_buf()
}

/// Recurse below a named directory.
fn walk_into(
    out: &mut Vec<Entry>,
    seen: &mut HashSet<(DirIdent, OsString)>,
    dir_idents: &mut HashMap<PathBuf, DirIdent>,
    root: &Path,
) -> Result<(), WalkError> {
    let walker = WalkDir::new(root)
        .follow_links(false)
        .sort_by_file_name()
        .min_depth(1)
        .into_iter()
        // The root is exempt: `detoxrs -r .git` is a user pointing at a
        // repository on purpose, and `detoxrs -r .config` a user pointing at a
        // dot-directory on purpose. Only what recursion *discovers* is filtered.
        .filter_entry(|e| e.depth() == 0 || !is_vcs_dir(e) && !is_dotfile(e.file_name()));

    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                if let Some(io) = e.io_error()
                    && is_descriptor_exhaustion(io)
                {
                    // `e.into_io_error()` is the only way to own the error, and
                    // it cannot fail here because `io_error()` just returned Some.
                    return Err(WalkError::OutOfDescriptors(
                        e.into_io_error()
                            .unwrap_or_else(|| io::Error::other("file descriptor exhaustion")),
                    ));
                }
                eprintln!("detoxrs: warning: {e}");
                continue;
            }
        };
        // `follow_links(false)` makes walkdir's own metadata call an `lstat`, but
        // this module's contract is "never `stat`", and a contract that depends
        // on a flag set three lines up is one refactor from being false.
        let path = entry.path();
        match fs::symlink_metadata(path) {
            Ok(md) => push(out, seen, dir_idents, path, &md, entry.depth()),
            Err(e) => eprintln!("detoxrs: warning: cannot read {}: {e}", path.display()),
        }
    }
    Ok(())
}

/// Add one entry, unless its name is not a name we could ever rewrite.
///
/// `.`, `..` and `/` have no basename to clean. They are still perfectly good
/// walk roots, so `detoxrs -r .` works; they are just never candidates
/// themselves.
fn push(
    out: &mut Vec<Entry>,
    seen: &mut HashSet<(DirIdent, OsString)>,
    dir_idents: &mut HashMap<PathBuf, DirIdent>,
    path: &Path,
    md: &Metadata,
    depth: usize,
) {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return;
    };
    if name == OsStr::new(".") || name == OsStr::new("..") {
        return;
    }
    let dir_ident = dir_ident_of(dir_idents, dir);
    if !seen.insert((dir_ident, name.to_os_string())) {
        return;
    }
    out.push(Entry {
        dir: dir.to_path_buf(),
        name: name.to_os_string(),
        kind: kind_of(md),
        ident: ident_of(md),
        dir_ident,
        depth: u32::try_from(depth).unwrap_or(u32::MAX),
    });
}

/// C9: a directory argument's own basename, or a plain file argument, is
/// never otherwise checked against its siblings -- `walk_into` only looks
/// *inside* a directory, never beside it -- so `plan()`'s layer-2 occupancy
/// check, built only from what is in the snapshot (`plan.rs`'s own module
/// doc), has no way to see a destination that already exists there. `plan()`
/// staying I/O-free is deliberate, so the one `lstat` this needs happens
/// here, before `plan()` ever runs, and what is found is frozen as an
/// ordinary snapshot entry: `plan()` needs no change at all to occupy a name
/// it can already see.
fn seed_pre_existing_destination(
    out: &mut Vec<Entry>,
    seen: &mut HashSet<(DirIdent, OsString)>,
    dir_idents: &mut HashMap<PathBuf, DirIdent>,
    path: &Path,
    own_md: &Metadata,
    policy: &Policy,
) {
    let (Some(dir), Some(name)) = (path.parent(), path.file_name()) else {
        return;
    };
    let Decoded::Utf8(text) = decode(name) else {
        return; // never renamed (§6.1); nothing to check a destination for
    };
    let TransformResult::Name(wanted) = transform(&text, policy) else {
        return; // no representable destination; nothing can collide with it
    };
    if wanted.text == text {
        return; // already clean; `plan()` calls this `Unchanged` on its own
    }
    let candidate = dir.join(&wanted.text);
    let Ok(cand_md) = fs::symlink_metadata(&candidate) else {
        return;
    };
    // #2: a normalization-insensitive lookup filesystem (APFS) can resolve
    // the *transformed* name straight back to the very entry being renamed
    // (it folds NFD and NFC together for lookup, even though it stores only
    // one of them). That is this entry seen a second time under its own
    // destination's spelling, not a second occupant -- pushing it would make
    // the planner number a rename that has nothing left to collide with.
    if let (Some(own), Some(cand)) = (dev_ino_of(own_md), dev_ino_of(&cand_md))
        && own == cand
    {
        return;
    }
    push(out, seen, dir_idents, &candidate, &cand_md, 0);
}

/// `dir` itself, unless it is `""`, POSIX's spelling of "the current
/// directory" when it comes from `Path::parent()` on a single-component
/// relative name (`"sub".parent() == Some("")`). No syscall accepts an empty
/// path, so every call in this module that hands a directory to one
/// substitutes `.` via this function first, which resolves to the same
/// directory the syscall would reach if it could take the empty string at
/// all. This is what lets `detoxrs -r . sub` see `.`'s and `sub`'s shared
/// parent as one identity instead of two -- and what makes a single-file
/// argument's own directory listable in [`real_entry_name`].
fn syscall_path(dir: &Path) -> &Path {
    if dir.as_os_str().is_empty() {
        Path::new(".")
    } else {
        dir
    }
}

/// The identity of the directory at `dir`, cached by its literal path so the
/// same spelling is `lstat`ed only once.
fn dir_ident_of(cache: &mut HashMap<PathBuf, DirIdent>, dir: &Path) -> DirIdent {
    if let Some(&id) = cache.get(dir) {
        return id;
    }
    let id = real_dir_ident(syscall_path(dir)).unwrap_or_else(|| path_hash_ident(dir));
    cache.insert(dir.to_path_buf(), id);
    id
}

#[cfg(unix)]
fn real_dir_ident(dir: &Path) -> Option<DirIdent> {
    fs::symlink_metadata(dir).ok().map(|md| unix_dev_ino(&md))
}

/// Never faked on non-unix (`ident_of`'s own doc comment gives the reason):
/// every directory here falls back to [`path_hash_ident`], which is no worse
/// than the textual keying this replaces, and never merges two directories
/// that are not, in fact, the same one.
#[cfg(not(unix))]
fn real_dir_ident(_dir: &Path) -> Option<DirIdent> {
    None
}

#[cfg(unix)]
fn unix_dev_ino(md: &Metadata) -> (u64, u64) {
    use std::os::unix::fs::MetadataExt as _;
    (md.dev(), md.ino())
}

/// An already-inspected file's identity, or `None` where identity is never
/// faked (non-unix; see `ident_of`'s own doc comment). Distinct from
/// [`real_dir_ident`]/[`dir_ident_of`], which name *directories* and fall back
/// to a path hash when identity is unavailable -- there is no path to hash
/// here, and a caller comparing two entries' identity must be able to tell
/// "unavailable" apart from "available and different".
#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the `#[cfg(not(unix))]` sibling below returns `None`; clippy \
              only ever sees one cfg arm at a time and reads this one as \
              trivially always-`Some`"
)]
fn dev_ino_of(md: &Metadata) -> Option<(u64, u64)> {
    Some(unix_dev_ino(md))
}

#[cfg(not(unix))]
fn dev_ino_of(_md: &Metadata) -> Option<(u64, u64)> {
    None
}

/// A directory that could not be `lstat`ed (a race after `push` already saw
/// one of its entries), or any directory at all on a platform where identity
/// is never faked. Never `(0, 0)`: that sentinel is what non-unix's
/// `ident_of` uses for every real entry, and colliding with it would fold
/// every one of these into every one of those. A path hash keeps two
/// different directories apart; it cannot unify two spellings of the same
/// one, which is exactly the textual keying this function's caller exists to
/// improve on -- so the degraded case is never worse than before this fix.
fn path_hash_ident(dir: &Path) -> DirIdent {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    dir.hash(&mut h);
    (u64::MAX, h.finish())
}

fn is_dotfile(name: &OsStr) -> bool {
    name.as_encoded_bytes().first() == Some(&b'.')
}

fn is_vcs_dir(e: &DirEntry) -> bool {
    e.file_type().is_dir() && matches!(e.file_name().to_str(), Some(".git" | ".hg" | ".svn"))
}

/// `EMFILE` (24) or `ENFILE` (23).
///
/// Raw errno rather than an `ErrorKind`: `std` has no stable variant for
/// descriptor exhaustion on this MSRV. Both numbers are the same on Linux and
/// macOS, which are the two tier-1 platforms; elsewhere this is simply `false`
/// and the walk falls back to warning and continuing.
#[cfg(unix)]
fn is_descriptor_exhaustion(e: &io::Error) -> bool {
    matches!(e.raw_os_error(), Some(23 | 24))
}

#[cfg(not(unix))]
fn is_descriptor_exhaustion(_e: &io::Error) -> bool {
    false
}

fn kind_of(md: &Metadata) -> EntryKind {
    let ft = md.file_type();
    if ft.is_symlink() {
        EntryKind::Symlink
    } else if ft.is_dir() {
        EntryKind::Dir
    } else if ft.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

#[cfg(unix)]
fn ident_of(md: &Metadata) -> Ident {
    use std::os::unix::fs::MetadataExt as _;
    Ident {
        dev: md.dev(),
        ino: md.ino(),
        nlink: md.nlink(),
        mtime: md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
    }
}

/// Windows is a best-effort tier (owner decision, 2026-07-31): it must compile
/// and unit-test, and no filesystem behaviour is asserted on it. `dev`/`ino` are
/// zeroed rather than faked from `file_index()`, because the only consumer is
/// `apply`'s identity recheck, which does not exist yet and must not be handed
/// numbers that look real and are not.
#[cfg(not(unix))]
fn ident_of(md: &Metadata) -> Ident {
    Ident {
        dev: 0,
        ino: 0,
        nlink: 1,
        mtime: md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
    }
}

/// Does this volume fold case when comparing names (§6.2)?
///
/// Read-only by necessity: `WP5a` has no write path, so the probe cannot create a
/// file. Instead it takes a name that is already there, flips its ASCII case,
/// and `lstat`s the result in the same directory. Same `(dev, ino)` means the
/// volume folded the two spellings together; `ENOENT` means it did not.
///
/// Case-insensitivity stays an input to `plan()` rather than a `cfg!` on the
/// platform, because case-sensitive APFS exists and case-insensitive ext4 does
/// not follow from either OS name.
///
/// When no entry has an ASCII letter the answer is [`VolumeCase::Sensitive`],
/// and that fallback is *moot* rather than merely convenient: with no cased
/// character anywhere in the batch, case folding is the identity function and no
/// two names can differ only by case.
///
/// ponytail: one probe for the whole run, and ASCII-only. A batch spanning two
/// mounts with different case behaviour, or colliding only on non-ASCII case
/// (`Ä` vs `ä`), gets the first mount's answer. The upgrade is a probe per
/// `dev`, which is where it belongs once `apply` needs per-directory limits
/// anyway (M5's `statfs` work).
#[must_use]
pub fn volume_case(entries: &[Entry]) -> VolumeCase {
    for e in entries {
        let Decoded::Utf8(name) = decode(&e.name) else {
            continue;
        };
        let Some(flipped) = ascii_case_flipped(&name) else {
            continue;
        };
        match fs::symlink_metadata(e.dir.join(flipped)) {
            Ok(md) => {
                let other = ident_of(&md);
                return if other.dev == e.ident.dev && other.ino == e.ident.ino {
                    VolumeCase::Insensitive
                } else {
                    // The flipped spelling resolved to a *different* file, so
                    // both spellings coexist in this directory.
                    VolumeCase::Sensitive
                };
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => return VolumeCase::Sensitive,
            // Anything else (a permission error, say) tells us nothing about
            // case folding. Fall through to the next entry rather than guess.
            Err(_) => {}
        }
    }
    VolumeCase::Sensitive
}

/// `Some` with every ASCII letter's case flipped, or `None` if there are none.
///
/// ASCII only, on purpose: `ß`'s uppercase is two characters, so a
/// Unicode-aware flip would change the name's length and probe a name that is
/// not the same name in a different case.
fn ascii_case_flipped(s: &str) -> Option<String> {
    if !s.bytes().any(|b| b.is_ascii_alphabetic()) {
        return None;
    }
    Some(
        s.chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    c.to_ascii_lowercase()
                } else if c.is_ascii_lowercase() {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect(),
    )
}
