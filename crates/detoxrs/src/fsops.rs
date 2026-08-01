//! The one and only rename entry point (proposal §5.4, plan §7.3).
//!
//! Everything that ever changes a name on disk goes through
//! [`RenameOps::rename_noreplace`]. There is no second method, no clobbering
//! variant, and nothing in this crate calls `std::fs::rename` outside
//! [`fallback`]'s demoted path.
//!
//! **There is no `rename_case_only`.** An earlier draft had one, justified by the
//! claim that `RENAME_NOREPLACE`/`RENAME_EXCL` would report `EEXIST` for a
//! same-inode respell (`Case.txt` -> `case.txt`). That was measured on APFS and
//! is false: the call returns `Ok(())`, with a control in the same run proving
//! the flag was honored. So the method is deleted and what survives is a
//! narrow *observed-error* fallback, in [`fallback`], for a filesystem nobody
//! has measured yet. See §5.4 and doc 06 row 4f.
//!
//! The trait exists for one reason beyond documentation: it is the fault
//! injection point. §8.4's "`RENAME_NOREPLACE` unsupported" row needs a rename
//! that returns [`RenameErr::Unsupported`] on demand, and no filesystem
//! available to this project produces one.

pub mod fallback;

use std::ffi::OsStr;
use std::fmt;
use std::path::Path;

/// Why a rename did not happen.
///
/// §5.8's taxonomy. Every variant is a per-item error line naming the path;
/// `ReadOnlyFilesystem` and `NoSpace` additionally abort the rest of the batch,
/// because they will fail every remaining item and 200k identical lines is not a
/// report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenameErr {
    /// `EEXIST`. The destination was taken between the walk and this call.
    AlreadyExists,
    /// `EACCES`/`EPERM`.
    PermissionDenied,
    /// `EROFS`. Aborts the batch.
    ReadOnlyFilesystem,
    /// `ENOSPC`/`EDQUOT`. Aborts the batch.
    NoSpace,
    /// `ENAMETOOLONG`. After §3.10 this means the detected limit was wrong, so
    /// it is worth a loud report rather than a shrug.
    NameTooLong,
    /// `ENOENT`. Raced away since the walk.
    NotFound,
    /// The no-replace flag is not supported here. Triggers the demotion in
    /// [`fallback`]; a caller never sees this from `PlatformRenameOps`, only
    /// from a test double.
    Unsupported,
    /// Any other errno, carried verbatim. Plan §7.3 listed seven variants and no
    /// catch-all; errno space is larger than seven, and mapping an unlisted
    /// error onto the nearest listed one would put a wrong cause in the one
    /// output a user reads before deciding what to do next.
    Other(i32),
}

impl fmt::Display for RenameErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists => f.write_str("destination already exists (EEXIST)"),
            Self::PermissionDenied => f.write_str("permission denied (EACCES/EPERM)"),
            Self::ReadOnlyFilesystem => f.write_str("read-only filesystem (EROFS)"),
            Self::NoSpace => f.write_str("no space or quota exceeded (ENOSPC/EDQUOT)"),
            Self::NameTooLong => {
                f.write_str("name too long (ENAMETOOLONG); the detected length limit was wrong")
            }
            Self::NotFound => f.write_str("no longer there (ENOENT)"),
            Self::Unsupported => f.write_str("atomic no-clobber rename unsupported here"),
            Self::Other(n) => write!(f, "rename failed (errno {n})"),
        }
    }
}

impl RenameErr {
    /// Does this error mean every remaining item will fail too (§5.8)?
    #[must_use]
    pub const fn aborts_batch(self) -> bool {
        matches!(self, Self::ReadOnlyFilesystem | Self::NoSpace)
    }
}

/// Rename within one directory, never across directories (§5.2), never
/// clobbering (§5.4).
pub trait RenameOps {
    /// Rename `from` to `to` inside `dir`.
    ///
    /// # Errors
    ///
    /// Any [`RenameErr`]. Notably [`RenameErr::AlreadyExists`] rather than a
    /// silent overwrite: this call fails instead of destroying a file, on every
    /// platform and every filesystem, including the demoted path.
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}

/// The real thing.
pub struct PlatformRenameOps;

impl RenameOps for PlatformRenameOps {
    #[cfg(unix)]
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        // Already demoted by an earlier item on this run, so do not pay for a
        // call that is known to fail here.
        if fallback::is_demoted() {
            return fallback::check_then_rename(dir, from, to);
        }
        match unix::renameat_noreplace(dir, from, to) {
            Ok(()) => Ok(()),
            // §5.4's two observed-error rungs, in order of specificity.
            Err(RenameErr::AlreadyExists) if fallback::same_inode(dir, from, to) => {
                fallback::warn_same_inode_once(dir);
                unix::renameat_plain(dir, from, to)
            }
            Err(RenameErr::Unsupported) => {
                fallback::demote(dir);
                fallback::check_then_rename(dir, from, to)
            }
            Err(e) => Err(e),
        }
    }

    /// Windows is a best-effort tier (owner decision, 2026-07-31): it compiles
    /// and unit-tests, and no filesystem behavior is asserted. `MoveFileExW`
    /// without `MOVEFILE_REPLACE_EXISTING` is the right call there and would
    /// cost a `windows-sys` budget slot to reach, so the honest M1 answer is the
    /// documented-TOCTOU path that already exists, reported as
    /// `"atomicity": "check-then-rename"` rather than claimed as atomic.
    #[cfg(not(unix))]
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        fallback::check_then_rename(dir, from, to)
    }
}

/// What `--json` should report about the guarantee this run actually had.
#[must_use]
pub fn atomicity() -> &'static str {
    if cfg!(unix) && !fallback::is_demoted() {
        "renameat-noreplace"
    } else {
        "check-then-rename"
    }
}

#[cfg(unix)]
mod unix {
    use super::RenameErr;
    use rustix::fs::{Mode, OFlags, RenameFlags};
    use rustix::io::Errno;
    use std::ffi::OsStr;
    use std::path::Path;

    /// `renameat2(RENAME_NOREPLACE)` on Linux, `renameatx_np(RENAME_EXCL)` on
    /// Apple. One safe call, no `#[cfg]` split here, no `unsafe`.
    pub fn renameat_noreplace(dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        let fd = open_dir(dir)?;
        rustix::fs::renameat_with(&fd, from, &fd, to, RenameFlags::NOREPLACE).map_err(map_errno)
    }

    /// Plain `renameat(2)`, reached only from the same-inode rung in §5.4. It
    /// can clobber in general, which is why nothing else calls it: the caller
    /// has already established that `to` *is* `from`.
    pub fn renameat_plain(dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        let fd = open_dir(dir)?;
        rustix::fs::renameat(&fd, from, &fd, to).map_err(map_errno)
    }

    /// A directory handle for the rename.
    ///
    /// ponytail: one `open` per rename. Items arrive grouped by directory, so
    /// caching the last `(dir, fd)` pair would remove most of these calls; the
    /// upgrade is a two-field struct at this call site and is worth doing when a
    /// 200k-entry batch measures it, not before.
    fn open_dir(dir: &Path) -> Result<rustix::fd::OwnedFd, RenameErr> {
        // An empty parent means the current directory: `detoxrs file.txt` puts
        // `""` in `PlanItem::dir`, and `open("")` is `ENOENT`.
        let path = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(map_errno)
    }

    /// errno to §5.8's taxonomy.
    const fn map_errno(e: Errno) -> RenameErr {
        match e {
            Errno::EXIST => RenameErr::AlreadyExists,
            Errno::ACCESS | Errno::PERM => RenameErr::PermissionDenied,
            Errno::ROFS => RenameErr::ReadOnlyFilesystem,
            Errno::NOSPC | Errno::DQUOT => RenameErr::NoSpace,
            Errno::NAMETOOLONG => RenameErr::NameTooLong,
            Errno::NOENT => RenameErr::NotFound,
            // The demotion set from §5.4. `EINVAL` is what a Linux filesystem
            // without `renameat2` flag support returns; `ENOSYS` is a kernel
            // older than 3.15; `EOPNOTSUPP`/`ENOTSUP` is the assumed macOS
            // answer for a volume lacking `VOL_CAP_INT_RENAME_EXCL` and is
            // [UNVERIFIED] (§11 spike 13) -- which is exactly why it demotes on
            // the observed error instead of being predicted by a probe.
            Errno::INVAL | Errno::NOSYS | Errno::OPNOTSUPP => RenameErr::Unsupported,
            other => RenameErr::Other(other.raw_os_error()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlatformRenameOps, RenameErr, RenameOps as _};
    use std::ffi::OsStr;
    use std::fs;

    /// The whole point of the module, on whatever filesystem the tests run on:
    /// an occupied destination is refused, and the occupant is left alone.
    #[test]
    fn an_occupied_destination_is_refused() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::write(t.path().join("b"), b"bbb").expect("write");

        let err = PlatformRenameOps
            .rename_noreplace(t.path(), OsStr::new("a"), OsStr::new("b"))
            .expect_err("must not clobber");
        assert_eq!(err, RenameErr::AlreadyExists);
        assert_eq!(fs::read(t.path().join("b")).expect("read"), b"bbb");
        assert_eq!(fs::read(t.path().join("a")).expect("read"), b"aaa");
    }

    #[test]
    fn a_free_destination_is_taken() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");

        PlatformRenameOps
            .rename_noreplace(t.path(), OsStr::new("a"), OsStr::new("c"))
            .expect("free destination");
        assert_eq!(fs::read(t.path().join("c")).expect("read"), b"aaa");
        assert!(!t.path().join("a").exists());
    }

    /// §8.4's case-only row, as far as it can be asserted without knowing which
    /// filesystem the tests are on: the *syscall's return value* is `Ok`, not a
    /// misreported collision, on a case-insensitive volume and a case-sensitive
    /// one alike. This is doc 06 row 4f made permanent.
    #[test]
    fn a_case_only_respell_succeeds() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("Case.txt"), b"x").expect("write");
        PlatformRenameOps
            .rename_noreplace(t.path(), OsStr::new("Case.txt"), OsStr::new("case.txt"))
            .expect("a case-only respell is one syscall, not a collision");
        assert_eq!(fs::read(t.path().join("case.txt")).expect("read"), b"x");
    }

    #[test]
    fn a_vanished_source_is_not_found() {
        let t = tempfile::tempdir().expect("tempdir");
        let err = PlatformRenameOps
            .rename_noreplace(t.path(), OsStr::new("gone"), OsStr::new("also-gone"))
            .expect_err("nothing to rename");
        assert_eq!(err, RenameErr::NotFound);
    }
}
