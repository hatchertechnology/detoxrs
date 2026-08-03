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
//! **The directory is pinned, and that is a safety property rather than an
//! optimisation.** [`RenameOps::open`] returns a [`Dir`] -- an open directory file
//! descriptor on Unix -- and the identity check, the occupancy check and the
//! rename all go through that one handle. An earlier version resolved `dir` by
//! path a second time, inside the rename, *after* the checks had already passed;
//! an adversarial review reproduced the consequence, which is that renaming a
//! directory out from under a run in that gap made the rename land on a file that
//! was never checked, while the journal recorded a false success against the
//! original inode. A path resolved twice is two different directories in the
//! general case. A descriptor resolved once is one directory, permanently.
//!
//! The trait exists for one reason beyond documentation: it is the fault
//! injection point. §8.4's "`RENAME_NOREPLACE` unsupported" row needs a rename
//! that returns [`RenameErr::Unsupported`] on demand, and no filesystem
//! available to this project produces one.

pub mod fallback;

use detoxrs_core::plan::Ident;
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

/// A directory, pinned for the duration of one item's checks and rename.
///
/// On Unix this is an open descriptor, which is what makes the pin real: the
/// kernel resolves it to the same inode no matter what happens to the path that
/// opened it. On other platforms it degrades to the path, and the identity
/// guarantee degrades with it -- stated rather than hidden, because Windows is a
/// best-effort tier (owner decision, 2026-07-31).
pub struct Dir {
    #[cfg(unix)]
    fd: rustix::fd::OwnedFd,
    #[cfg(not(unix))]
    path: std::path::PathBuf,
}

/// Rename within one directory, never across directories (§5.2), never
/// clobbering (§5.4).
///
/// Every method takes the same [`Dir`], so a caller cannot accidentally check one
/// directory and rename in another. That is the whole reason the handle is in the
/// signature instead of a `&Path`.
pub trait RenameOps {
    /// Pin `dir` so that everything else in this item happens inside it.
    ///
    /// # Errors
    ///
    /// Any [`RenameErr`] from opening the directory.
    fn open(&self, dir: &Path) -> Result<Dir, RenameErr>;

    /// `lstat` `name` relative to the pinned directory. Never follows a symlink:
    /// what gets renamed is a directory entry, so the entry is what is inspected.
    ///
    /// # Errors
    ///
    /// [`RenameErr::NotFound`] if there is no such entry, or any other
    /// [`RenameErr`].
    fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr>;

    /// Rename `from` to `to` inside the pinned directory.
    ///
    /// # Errors
    ///
    /// Any [`RenameErr`]. Notably [`RenameErr::AlreadyExists`] rather than a
    /// silent overwrite: this call fails instead of destroying a file, on every
    /// platform and every filesystem, including the demoted path.
    fn rename_noreplace(&self, dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}

/// The real thing.
pub struct PlatformRenameOps;

impl RenameOps for PlatformRenameOps {
    #[cfg(unix)]
    fn open(&self, dir: &Path) -> Result<Dir, RenameErr> {
        Ok(Dir {
            fd: unix::open_dir(dir)?,
        })
    }

    #[cfg(not(unix))]
    fn open(&self, dir: &Path) -> Result<Dir, RenameErr> {
        // Best-effort tier: no pin, so the checks and the rename can in principle
        // see different directories. Recorded in `Dir`'s own docs.
        Ok(Dir {
            path: if dir.as_os_str().is_empty() {
                std::path::PathBuf::from(".")
            } else {
                dir.to_path_buf()
            },
        })
    }

    #[cfg(unix)]
    fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr> {
        unix::ident_at(&dir.fd, name)
    }

    #[cfg(not(unix))]
    fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr> {
        fallback::ident_at_path(&dir.path, name)
    }

    #[cfg(unix)]
    fn rename_noreplace(&self, dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        // Already demoted by an earlier item on this run, so do not pay for a
        // call that is known to fail here.
        if fallback::is_demoted() {
            return fallback::check_then_rename(self, dir, from, to);
        }
        match unix::renameat_noreplace(&dir.fd, from, to) {
            Ok(()) => Ok(()),
            // §5.4's two observed-error rungs, in order of specificity.
            Err(RenameErr::AlreadyExists) if fallback::same_inode(self, dir, from, to) => {
                fallback::warn_same_inode_once();
                unix::renameat_plain(&dir.fd, from, to)
            }
            Err(RenameErr::Unsupported) => {
                fallback::demote();
                fallback::check_then_rename(self, dir, from, to)
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
    fn rename_noreplace(&self, dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        fallback::check_then_rename(self, dir, from, to)
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
    use detoxrs_core::plan::Ident;
    use rustix::fd::{AsFd, OwnedFd};
    use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags};
    use rustix::io::Errno;
    use std::ffi::OsStr;
    use std::path::Path;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// `renameat2(RENAME_NOREPLACE)` on Linux, `renameatx_np(RENAME_EXCL)` on
    /// Apple. One safe call, no `#[cfg]` split here, no `unsafe`.
    pub fn renameat_noreplace(fd: &OwnedFd, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        rustix::fs::renameat_with(fd.as_fd(), from, fd.as_fd(), to, RenameFlags::NOREPLACE)
            .map_err(map_errno)
    }

    /// Plain `renameat(2)`, reached only from the same-inode rung in §5.4. It
    /// can clobber in general, which is why nothing else calls it: the caller
    /// has already established that `to` *is* `from`.
    pub fn renameat_plain(fd: &OwnedFd, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        rustix::fs::renameat(fd.as_fd(), from, fd.as_fd(), to).map_err(map_errno)
    }

    /// `fstatat(AT_SYMLINK_NOFOLLOW)`: the identity of a name *inside the pinned
    /// directory*, immune to anything happening to the path that opened it.
    pub fn ident_at(fd: &OwnedFd, name: &OsStr) -> Result<Ident, RenameErr> {
        let st =
            rustix::fs::statat(fd.as_fd(), name, AtFlags::SYMLINK_NOFOLLOW).map_err(map_errno)?;
        Ok(Ident {
            // `as` rather than `try_from`: these fields are signed on some
            // targets and unsigned on others, and the comparison only has to
            // agree with `walk`'s own reading of the same two numbers, which
            // `ident_matches_std_metadata` pins.
            #[allow(clippy::cast_sign_loss, clippy::unnecessary_cast)]
            dev: st.st_dev as u64,
            #[allow(clippy::cast_sign_loss, clippy::unnecessary_cast)]
            ino: st.st_ino as u64,
            // `From` rather than `as`: this field is `u16` on Apple and `u64` on
            // Linux, and both convert infallibly.
            nlink: u64::from(st.st_nlink),
            mtime: mtime_of(&st),
        })
    }

    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::unnecessary_cast,
        reason = "the seconds field is signed on some targets; a pre-epoch mtime is \
                  clamped to the epoch rather than wrapping, and mtime is not used \
                  by any identity comparison anyway"
    )]
    fn mtime_of(st: &rustix::fs::Stat) -> SystemTime {
        let secs = st.st_mtime as i64;
        u64::try_from(secs).map_or(UNIX_EPOCH, |s| UNIX_EPOCH + Duration::from_secs(s))
    }

    /// A directory handle for the checks and the rename.
    ///
    /// `O_DIRECTORY` means a symlink-to-a-directory cannot be opened as one here,
    /// and `O_NOFOLLOW` is deliberately *not* set: the walk already refuses to
    /// descend through a symlinked directory, and a user who names one on the
    /// command line is pointing at its target on purpose.
    pub fn open_dir(dir: &Path) -> Result<OwnedFd, RenameErr> {
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
    use super::{Dir, PlatformRenameOps, RenameErr, RenameOps as _};
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;

    fn pin(p: &Path) -> Dir {
        PlatformRenameOps.open(p).expect("open the directory")
    }

    /// The whole point of the module, on whatever filesystem the tests run on:
    /// an occupied destination is refused, and the occupant is left alone.
    #[test]
    fn an_occupied_destination_is_refused() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::write(t.path().join("b"), b"bbb").expect("write");

        let err = PlatformRenameOps
            .rename_noreplace(&pin(t.path()), OsStr::new("a"), OsStr::new("b"))
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
            .rename_noreplace(&pin(t.path()), OsStr::new("a"), OsStr::new("c"))
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
            .rename_noreplace(
                &pin(t.path()),
                OsStr::new("Case.txt"),
                OsStr::new("case.txt"),
            )
            .expect("a case-only respell is one syscall, not a collision");
        assert_eq!(fs::read(t.path().join("case.txt")).expect("read"), b"x");
    }

    #[test]
    fn a_vanished_source_is_not_found() {
        let t = tempfile::tempdir().expect("tempdir");
        let err = PlatformRenameOps
            .rename_noreplace(&pin(t.path()), OsStr::new("gone"), OsStr::new("also-gone"))
            .expect_err("nothing to rename");
        assert_eq!(err, RenameErr::NotFound);
    }

    /// `ident_at` and `walk`'s `symlink_metadata` must read the same two numbers,
    /// because `apply` compares one against the other. They come from different
    /// APIs -- `fstatat` through `rustix` versus `std` -- and the underlying fields
    /// are signed on some targets, so agreement is asserted rather than assumed.
    #[cfg(unix)]
    #[test]
    fn ident_matches_std_metadata() {
        use std::os::unix::fs::MetadataExt as _;
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a"), b"aaa").expect("write");
        fs::create_dir(t.path().join("d")).expect("mkdir");
        std::os::unix::fs::symlink("a", t.path().join("l")).expect("symlink");

        let d = pin(t.path());
        for name in ["a", "d", "l"] {
            let mine = PlatformRenameOps
                .ident_at(&d, OsStr::new(name))
                .expect("ident_at");
            let theirs = fs::symlink_metadata(t.path().join(name)).expect("lstat");
            assert_eq!(mine.dev, theirs.dev(), "dev disagrees for {name}");
            assert_eq!(mine.ino, theirs.ino(), "ino disagrees for {name}");
            assert_eq!(mine.nlink, theirs.nlink(), "nlink disagrees for {name}");
        }
    }

    /// `ident_at` must not follow a symlink: what gets renamed is the link's own
    /// directory entry, so the link is what has to be identified.
    #[cfg(unix)]
    #[test]
    fn ident_at_does_not_follow_a_symlink() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("target"), b"x").expect("write");
        std::os::unix::fs::symlink("target", t.path().join("link")).expect("symlink");

        let d = pin(t.path());
        let link = PlatformRenameOps
            .ident_at(&d, OsStr::new("link"))
            .expect("link");
        let target = PlatformRenameOps
            .ident_at(&d, OsStr::new("target"))
            .expect("target");
        assert_ne!(link.ino, target.ino, "ident_at followed the link");
    }

    /// **The pin, asserted directly.** This is the property an adversarial review
    /// found missing: the checks and the rename must refer to the same directory
    /// even if the *path* that named it stops pointing there. Renaming the
    /// directory away and dropping a different one in its place must not redirect
    /// the rename -- it lands in the directory that was opened, by inode.
    #[cfg(unix)]
    #[test]
    fn the_rename_follows_the_pinned_directory_not_the_path() {
        let t = tempfile::tempdir().expect("tempdir");
        let real = t.path().join("real");
        fs::create_dir(&real).expect("mkdir");
        fs::write(real.join("a b.txt"), b"REAL").expect("write");

        // Pin it, exactly as `apply` does, before anything moves.
        let d = pin(&real);

        // Now the swap: the directory we pinned is renamed away, and an impostor
        // takes the original path with a file of the same name.
        fs::rename(&real, t.path().join("moved")).expect("rename dir");
        fs::create_dir(&real).expect("mkdir impostor");
        fs::write(real.join("a b.txt"), b"IMPOSTOR").expect("write");

        PlatformRenameOps
            .rename_noreplace(&d, OsStr::new("a b.txt"), OsStr::new("a_b.txt"))
            .expect("rename through the pinned handle");

        // The rename landed in the pinned (now relocated) directory...
        assert_eq!(
            fs::read(t.path().join("moved/a_b.txt")).expect("read"),
            b"REAL",
            "the rename did not follow the directory it was checked against"
        );
        // ...and the impostor at the original path was never touched.
        assert_eq!(
            fs::read(real.join("a b.txt")).expect("read"),
            b"IMPOSTOR",
            "a file that was never checked was renamed"
        );
        assert!(!real.join("a_b.txt").exists());
    }
}
