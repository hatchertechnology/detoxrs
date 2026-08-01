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
//! * **A symlinked directory is never descended, and there is no flag for it.**
//!   `follow_links(false)` is that guarantee; the link's own name is still
//!   cleaned, as any other directory entry would be.
//! * **`.git`, `.hg`, `.svn` are skipped unconditionally**, and there will be no
//!   option to include them.
//! * **Dotfiles are skipped while recursing, processed when named explicitly.**
//! * **`symlink_metadata` only.** Never `stat`: what gets renamed is a directory
//!   entry, so the entry is what gets inspected.

use detoxrs_core::decode::{Decoded, decode};
use detoxrs_core::plan::{Entry, EntryKind, Ident, VolumeCase};
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, Metadata};
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
    let mut seen: HashSet<(PathBuf, OsString)> = HashSet::new();

    for path in paths {
        let md = fs::symlink_metadata(path).map_err(|e| WalkError::Unreadable(path.clone(), e))?;
        push(&mut out, &mut seen, path, &md, 0);

        if recursive && md.is_dir() {
            walk_into(&mut out, &mut seen, path)?;
        }
    }
    Ok(out)
}

/// Recurse below a named directory.
fn walk_into(
    out: &mut Vec<Entry>,
    seen: &mut HashSet<(PathBuf, OsString)>,
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
            Ok(md) => push(out, seen, path, &md, entry.depth()),
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
    seen: &mut HashSet<(PathBuf, OsString)>,
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
    if !seen.insert((dir.to_path_buf(), name.to_os_string())) {
        return;
    }
    out.push(Entry {
        dir: dir.to_path_buf(),
        name: name.to_os_string(),
        kind: kind_of(md),
        ident: ident_of(md),
        depth: u32::try_from(depth).unwrap_or(u32::MAX),
    });
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
