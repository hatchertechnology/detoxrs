//! The two demotion rungs below [`super::PlatformRenameOps`] (proposal §5.4).
//!
//! Both are entered on an **observed error**, never on a prediction. There is no
//! capability probe: `rustix` does not wrap `getattrlist`, and a probe's only job
//! would be to guess at open time what the rename call reports anyway.
//!
//! 1. `EEXIST` where the destination is the *same inode* as the source. A
//!    same-inode respell (case-only, or NFD -> NFC) measured `Ok(())` on APFS, so
//!    this rung is defensive-only and is not expected to fire on either tier-1
//!    platform. It never unlinks anything: it renames one name onto itself.
//! 2. `EINVAL`/`ENOSYS`/`EOPNOTSUPP`, meaning the no-replace flag is not
//!    supported here. The run demotes to check-then-rename, warns once, and
//!    reports `"atomicity": "check-then-rename"` in `--json`. The TOCTOU window
//!    is real, documented, and still never clobbers on the losing side of the
//!    race — the `symlink_metadata` check is what closes the common case and
//!    `rename(2)` is what stays correct when it does not.
//!
//! ponytail: the demotion and both warnings are process-global, not per-mount. A
//! batch spanning a supporting and a non-supporting filesystem demotes the whole
//! run after the first failure rather than just that mount. The upgrade is a
//! `Mutex<HashSet<PathBuf>>` keyed on `dir` at these same call sites, and it is
//! Plan B's accepted debt rather than an oversight.

use super::RenameErr;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

static DEMOTED: AtomicBool = AtomicBool::new(false);
static WARNED_DEMOTED: AtomicBool = AtomicBool::new(false);
static WARNED_SAME_INODE: AtomicBool = AtomicBool::new(false);

/// Has this run already given up on the atomic path?
#[must_use]
pub fn is_demoted() -> bool {
    DEMOTED.load(Ordering::Relaxed)
}

/// Give up on the atomic path for the rest of the run, warning once.
pub fn demote(dir: &Path) {
    DEMOTED.store(true, Ordering::Relaxed);
    if !WARNED_DEMOTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "detoxrs: warning: {} does not support atomic no-clobber rename; \
             falling back to check-then-rename for the rest of this run. \
             Renames still never overwrite an existing file.",
            dir.display()
        );
    }
}

/// Warn once about rung 1, which measurement says should never happen.
pub fn warn_same_inode_once(dir: &Path) {
    if !WARNED_SAME_INODE.swap(true, Ordering::Relaxed) {
        eprintln!(
            "detoxrs: warning: {} reported an existing destination for a rename of a name \
             onto itself; using a plain rename for that item. Please report this: it is a \
             filesystem behaviour detoxrs has not measured.",
            dir.display()
        );
    }
}

/// Is `to` the very same directory entry target as `from`?
///
/// This is the same-inode test detox has had for twenty years (`st_dev`/`st_ino`
/// match), minus its `nlink == 1` condition, which upstream needs only because it
/// has no batch-level plan.
///
/// Two hardlinks to one inode also answer `true`, which is a superset of the
/// respell case this is for. That is safe rather than merely tolerable: POSIX
/// requires `rename` over two names for the same file to succeed and *perform no
/// other action*, so neither name is destroyed. The planner will not normally
/// produce such an item anyway — a hardlink present in the snapshot occupies its
/// name at collision layer 2 and gets renumbered instead.
#[must_use]
pub fn same_inode(dir: &Path, from: &OsStr, to: &OsStr) -> bool {
    let Ok(a) = std::fs::symlink_metadata(dir.join(from)) else {
        return false;
    };
    let Ok(b) = std::fs::symlink_metadata(dir.join(to)) else {
        return false;
    };
    is_same(&a, &b)
}

#[cfg(unix)]
fn is_same(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    a.dev() == b.dev() && a.ino() == b.ino()
}

/// Windows is best-effort: with no `dev`/`ino`, this rung cannot be decided, and
/// answering "yes" would authorise the one call in this crate that can clobber.
#[cfg(not(unix))]
fn is_same(_a: &std::fs::Metadata, _b: &std::fs::Metadata) -> bool {
    false
}

/// `symlink_metadata(to)` and then `rename`, with the window in between.
///
/// # Errors
///
/// [`RenameErr::AlreadyExists`] when the destination is occupied by anything at
/// all, including a broken symlink -- `symlink_metadata` is what makes a dangling
/// link count as occupied, which `Path::exists` would not.
pub fn check_then_rename(dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
    let src = dir.join(from);
    let dst = dir.join(to);
    if std::fs::symlink_metadata(&dst).is_ok() && !same_inode(dir, from, to) {
        return Err(RenameErr::AlreadyExists);
    }
    std::fs::rename(&src, &dst).map_err(|e| from_io(&e))
}

/// `io::Error` to §5.8's taxonomy, for the paths that go through `std`.
fn from_io(e: &std::io::Error) -> RenameErr {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::AlreadyExists => RenameErr::AlreadyExists,
        ErrorKind::PermissionDenied => RenameErr::PermissionDenied,
        ErrorKind::ReadOnlyFilesystem => RenameErr::ReadOnlyFilesystem,
        ErrorKind::StorageFull | ErrorKind::QuotaExceeded => RenameErr::NoSpace,
        ErrorKind::InvalidFilename => RenameErr::NameTooLong,
        ErrorKind::NotFound => RenameErr::NotFound,
        _ => RenameErr::Other(e.raw_os_error().unwrap_or(0)),
    }
}

#[cfg(test)]
mod tests {
    use super::{check_then_rename, same_inode};
    use crate::fsops::RenameErr;
    use std::ffi::OsStr;
    use std::fs;

    /// The demoted path must keep the guarantee the atomic path makes. It is
    /// slower and racier; it is not permitted to be less safe about the common
    /// case, which is what §8.4's "falls back, warns once, still never clobbers"
    /// row asks for.
    #[test]
    fn the_demoted_path_still_refuses_an_occupied_destination() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::write(t.path().join("b"), b"bbb").expect("write");

        assert_eq!(
            check_then_rename(t.path(), OsStr::new("a"), OsStr::new("b")),
            Err(RenameErr::AlreadyExists)
        );
        assert_eq!(fs::read(t.path().join("b")).expect("read"), b"bbb");
    }

    /// A dangling symlink is an occupied name. `Path::exists` follows the link
    /// and would report the destination free, then `rename` would delete the
    /// link.
    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_counts_as_occupied() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        std::os::unix::fs::symlink("nowhere", t.path().join("b")).expect("symlink");

        assert_eq!(
            check_then_rename(t.path(), OsStr::new("a"), OsStr::new("b")),
            Err(RenameErr::AlreadyExists)
        );
        assert!(
            fs::symlink_metadata(t.path().join("b"))
                .expect("lstat")
                .is_symlink()
        );
    }

    #[test]
    fn the_demoted_path_renames_into_a_free_name() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        check_then_rename(t.path(), OsStr::new("a"), OsStr::new("c")).expect("free");
        assert_eq!(fs::read(t.path().join("c")).expect("read"), b"aaa");
    }

    #[cfg(unix)]
    #[test]
    fn a_hardlink_to_the_same_inode_is_recognised() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::hard_link(t.path().join("a"), t.path().join("b")).expect("link");
        assert!(same_inode(t.path(), OsStr::new("a"), OsStr::new("b")));
        assert!(!same_inode(t.path(), OsStr::new("a"), OsStr::new("nope")));
    }
}
