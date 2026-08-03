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
//! Plan B's accepted debt rather than an oversight. The same limitation is why the
//! warning no longer claims to name a mount: see [`demote`].

use super::{Dir, RenameErr, RenameOps};
#[cfg(not(unix))]
use detoxrs_core::plan::Ident;
use std::ffi::OsStr;
#[cfg(not(unix))]
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
///
/// The warning does not name the directory, and that is a correction rather than
/// an omission: §5.4 asks for a warning "naming the mount", and a `PlanItem`'s
/// `dir` is not a mount point -- printing it would have implied detoxrs knew which
/// filesystem was involved when all it knows is which directory failed. Naming the
/// real mount needs `statfs`, which arrives with M5's per-directory limits.
pub fn demote() {
    DEMOTED.store(true, Ordering::Relaxed);
    if !WARNED_DEMOTED.swap(true, Ordering::Relaxed) {
        eprintln!(
            "detoxrs: warning: this filesystem does not support atomic no-clobber rename; \
             falling back to check-then-rename for the rest of this run. Renames still never \
             overwrite an existing file."
        );
    }
}

/// Warn once about rung 1, which measurement says should never happen.
pub fn warn_same_inode_once() {
    if !WARNED_SAME_INODE.swap(true, Ordering::Relaxed) {
        eprintln!(
            "detoxrs: warning: this filesystem reported an existing destination for a rename of \
             a name onto itself; using a plain rename for that item. Please report this: it is a \
             filesystem behaviour detoxrs has not measured."
        );
    }
}

/// Are `from` and `to` the very same directory entry target, as seen through the
/// pinned directory?
///
/// This is the same-inode test detox has had for twenty years (`st_dev`/`st_ino`
/// match), minus its `nlink == 1` condition, which upstream needs only because it
/// has no batch-level plan.
///
/// Two hardlinks to one inode also answer `true`, which is a superset of the
/// respell case this is for. That is safe rather than merely tolerable: POSIX
/// requires `rename` over two names for the same file to succeed and *perform no
/// other action*, so neither name is destroyed. The planner will not normally
/// produce such an item anyway -- a hardlink present in the snapshot occupies its
/// name at collision layer 2 and gets renumbered instead.
#[must_use]
pub fn same_inode(ops: &dyn RenameOps, dir: &Dir, from: &OsStr, to: &OsStr) -> bool {
    let (Ok(a), Ok(b)) = (ops.ident_at(dir, from), ops.ident_at(dir, to)) else {
        return false;
    };
    a.dev == b.dev && a.ino == b.ino
}

/// `lstat` a name under a path rather than a descriptor. Non-Unix only; see
/// [`Dir`]'s docs for what that costs.
#[cfg(not(unix))]
pub fn ident_at_path(dir: &Path, name: &OsStr) -> Result<Ident, RenameErr> {
    let md = std::fs::symlink_metadata(dir.join(name)).map_err(|e| from_io(&e))?;
    Ok(Ident {
        dev: 0,
        ino: 0,
        nlink: 1,
        mtime: md.modified().unwrap_or(std::time::UNIX_EPOCH),
    })
}

/// Check the destination, then rename, with the window in between.
///
/// Both the check and the rename go through the pinned [`Dir`], so this path
/// inherits the directory pin even though it cannot inherit the atomicity.
///
/// # Errors
///
/// [`RenameErr::AlreadyExists`] when the destination is occupied by anything at
/// all, including a broken symlink -- an `lstat` is what makes a dangling link
/// count as occupied, which following it would not.
pub fn check_then_rename(
    ops: &dyn RenameOps,
    dir: &Dir,
    from: &OsStr,
    to: &OsStr,
) -> Result<(), RenameErr> {
    if ops.ident_at(dir, to).is_ok() && !same_inode(ops, dir, from, to) {
        return Err(RenameErr::AlreadyExists);
    }
    rename_plain(dir, from, to)
}

#[cfg(unix)]
fn rename_plain(dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
    super::unix::renameat_plain(&dir.fd, from, to)
}

#[cfg(not(unix))]
fn rename_plain(dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
    let d = &dir.path;
    std::fs::rename(d.join(from), d.join(to)).map_err(|e| from_io(&e))
}

/// `io::Error` to §5.8's taxonomy, for the paths that go through `std`.
#[cfg_attr(
    unix,
    allow(dead_code, reason = "only the non-Unix rename path uses it")
)]
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
    use crate::fsops::{Dir, PlatformRenameOps, RenameErr, RenameOps as _};
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;

    fn pin(p: &Path) -> Dir {
        PlatformRenameOps.open(p).expect("open the directory")
    }

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
            check_then_rename(
                &PlatformRenameOps,
                &pin(t.path()),
                OsStr::new("a"),
                OsStr::new("b")
            ),
            Err(RenameErr::AlreadyExists)
        );
        assert_eq!(fs::read(t.path().join("b")).expect("read"), b"bbb");
    }

    /// A dangling symlink is an occupied name. Following the link would report the
    /// destination free, and then the rename would delete the link.
    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_counts_as_occupied() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        std::os::unix::fs::symlink("nowhere", t.path().join("b")).expect("symlink");

        assert_eq!(
            check_then_rename(
                &PlatformRenameOps,
                &pin(t.path()),
                OsStr::new("a"),
                OsStr::new("b")
            ),
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
        check_then_rename(
            &PlatformRenameOps,
            &pin(t.path()),
            OsStr::new("a"),
            OsStr::new("c"),
        )
        .expect("free");
        assert_eq!(fs::read(t.path().join("c")).expect("read"), b"aaa");
    }

    #[cfg(unix)]
    #[test]
    fn a_hardlink_to_the_same_inode_is_recognised() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::hard_link(t.path().join("a"), t.path().join("b")).expect("link");
        let d = pin(t.path());
        assert!(same_inode(
            &PlatformRenameOps,
            &d,
            OsStr::new("a"),
            OsStr::new("b")
        ));
        assert!(!same_inode(
            &PlatformRenameOps,
            &d,
            OsStr::new("a"),
            OsStr::new("nope")
        ));
    }
}
