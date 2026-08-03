//! The two demotion rungs below [`super::PlatformRenameOps`] (proposal §5.4).
//!
//! Both are entered on an **observed error**, never on a prediction. There is no
//! capability probe: `rustix` does not wrap `getattrlist`, and a probe's only job
//! would be to guess at open time what the rename call reports anyway.
//!
//! 1. `EEXIST` where the destination is the *same inode* as the source. A
//!    same-inode respell (case-only, or NFD -> NFC) measured `Ok(())` on APFS, so
//!    the naive expectation was that this rung would never fire on a tier-1
//!    platform -- **false** (C5): `ln 'a b.txt' 'a_b.txt'` then renaming
//!    `a b.txt` reaches it on stock APFS, because a second hardlink also shares
//!    the source's inode. It never unlinks anything: it renames one name onto
//!    itself for a genuine respell, and is refused as an occupied destination
//!    for a hardlink -- see [`is_same_entry_not_hardlink`], which tells the two
//!    apart by re-reading `dir`'s own entries rather than trusting the inode's
//!    volume-wide link count (a first cut used `nlink == 1` and broke a
//!    respell of a file with an unrelated hardlink elsewhere; see that
//!    function's docs).
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
/// Two hardlinks to one inode also answer `true` here. **That used to be treated
/// as safe; it is not** (C5). POSIX does require `rename` over two names for the
/// same file to succeed and perform no other action, so neither name is
/// destroyed -- but "perform no other action" is exactly the bug: the caller
/// still sees `Ok(())` for a rename that renamed nothing, the dirty name stays on
/// disk, and the journal records a `done` for a no-op. This function alone
/// cannot tell a hardlink from the case-only/NFD-NFC respell rung 1 exists for;
/// [`is_same_entry_not_hardlink`] is what a caller must use to decide whether
/// falling through to a plain rename is safe. Kept `pub` and separately tested
/// because "two hardlinks share an inode" is a fact worth pinning on its own.
#[must_use]
pub fn same_inode(ops: &dyn RenameOps, dir: &Dir, from: &OsStr, to: &OsStr) -> bool {
    let (Ok(a), Ok(b)) = (ops.ident_at(dir, from), ops.ident_at(dir, to)) else {
        return false;
    };
    a.dev == b.dev && a.ino == b.ino
}

/// Is `to` the *same directory entry* as `from`, spelled differently -- rather
/// than a second, distinct hardlink to the same inode?
///
/// `same_inode` cannot make this distinction: a hardlink also answers
/// `dev == dev && ino == ino`. **Neither can `nlink` -- a first cut of this fix
/// used `nlink == 1` and a verifier broke it live:** `nlink` counts links to
/// the inode *anywhere on the volume*, so a file genuinely being respelled
/// here that happens to have an unrelated hardlink in some other directory has
/// `nlink >= 2` and was wrongly refused as occupied, turning a rename that
/// used to work into a false `AlreadyExists`.
///
/// The real question is not "how many links does this inode have" but "does a
/// directory entry literally spelled `to` already exist, distinct from
/// `from`?" -- [`dir_has_literal_entry`] answers exactly that by re-reading
/// `dir` itself (through the pinned descriptor, never a re-resolved path) and
/// comparing raw bytes, not `ident_at`'s case/normalization-folding lookup. A
/// case-only or NFD -> NFC respell has **no** literal entry spelled `to` --
/// the lookup that found `to`'s identity got there only by folding, and landed
/// on `from`'s own entry, because a case-insensitive or normalizing volume
/// cannot hold two entries that fold to the same string. A hardlinked
/// destination, by contrast, *is* a literal entry of its own: `to` is a real
/// name on disk, sitting on top of the same inode as `from` rather than being
/// another spelling of it. Whether some third, unrelated entry in `dir` also
/// happens to share the inode (another hardlink, minding its own business
/// under its own name) is irrelevant either way, which is why this checks for
/// `to` by name rather than counting every entry that shares the inode.
///
/// C5 is the reason this exists at all: a hardlink whose other name is *not in
/// the snapshot* (a single-file argument, or a non-recursive directory
/// argument) reaches this rung anyway, `apply`'s step 2 waives the occupancy
/// check for it because it shares the source's inode, and this rung used to
/// wave it through as a "respell" -- a false success reported as `1 renamed`,
/// exit 0, with the journal recording a rename that never happened.
#[must_use]
pub fn is_same_entry_not_hardlink(
    ops: &dyn RenameOps,
    dir: &Dir,
    from: &OsStr,
    to: &OsStr,
) -> bool {
    same_inode(ops, dir, from, to) && !dir_has_literal_entry(dir, to)
}

/// Does `dir` contain a directory entry spelled *exactly* `name`, byte for
/// byte?
///
/// Re-reads the directory through the pinned descriptor: `Dir::read_from` on
/// an `AsFd` opens `.` relative to that descriptor rather than resolving a
/// path (rustix's own `_read_from` does the `openat(fd, ".")`, not a path
/// lookup from the root), so this still cannot be redirected by anything that
/// happens to the path that originally named `dir` -- the same property
/// [`super::PlatformRenameOps::open`] exists to give every other check in this
/// module.
///
/// Any error partway through the scan fails closed: a directory that could
/// not be fully re-read is treated as though the literal entry might exist, so
/// the caller refuses the rename as occupied rather than risking another false
/// success.
///
/// On the non-Unix best-effort tier there is no directory descriptor to
/// re-read, and [`ident_at_path`] already reports `dev: 0, ino: 0` for every
/// name (see its docs), so `same_inode` cannot tell two different files apart
/// there either; this function is `#[cfg(unix)]`-only, and
/// [`is_same_entry_not_hardlink`]'s Windows-tier behaviour is the
/// `#[cfg(not(unix))]` twin directly below.
///
// ponytail: O(n) rescan of the directory per EEXIST, so O(n^2) over a batch
// whose destinations all collide. Correct first: this only runs on the demoted
// rung, and only when the destination is already occupied by the source's own
// inode. If it ever shows up in a profile, the upgrade is a single
// spelling-exact lookup of `to` rather than a full scan -- which needs a
// primitive that can say "is there an entry spelled exactly this?" without the
// volume's case/normalization folding answering for it.
#[cfg(unix)]
fn dir_has_literal_entry(dir: &Dir, name: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt as _;
    let Ok(mut entries) = rustix::fs::Dir::read_from(&dir.fd) else {
        return true;
    };
    loop {
        match entries.read() {
            Some(Ok(entry)) => {
                if entry.file_name().to_bytes() == name.as_bytes() {
                    return true;
                }
            }
            Some(Err(_)) => return true,
            None => return false,
        }
    }
}

/// Best-effort tier: no directory descriptor to re-scan, and `same_inode`
/// itself cannot distinguish two files here (`ident_at_path` zeroes `dev`/`ino`
/// for everything). Answering `true` (as if `to` were always a literal entry)
/// would make `is_same_entry_not_hardlink` refuse every same-inode case,
/// including the ordinary respell -- a regression, not a fix, on a tier that
/// already cannot see the difference. Falls back to the pre-C5 behaviour
/// instead; the degraded identity guarantee is [`Dir`]'s documented,
/// owner-accepted limit for Windows.
#[cfg(not(unix))]
fn dir_has_literal_entry(_dir: &Dir, _name: &OsStr) -> bool {
    false
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
/// count as occupied, which following it would not. A destination that is a
/// second hardlink to the source's own inode is occupied too (C5): only a
/// same-entry respell -- [`is_same_entry_not_hardlink`] -- waives this check,
/// because that is the one case where `to` names the file being renamed rather
/// than something else sitting on top of it.
pub fn check_then_rename(
    ops: &dyn RenameOps,
    dir: &Dir,
    from: &OsStr,
    to: &OsStr,
) -> Result<(), RenameErr> {
    if ops.ident_at(dir, to).is_ok() && !is_same_entry_not_hardlink(ops, dir, from, to) {
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

    /// The missing coverage an adversarial review found: `check_then_rename`'s
    /// `!same_inode` carve-out (now `!is_same_entry_not_hardlink`) had zero
    /// direct coverage, and a reviewer who deleted the guard entirely still saw
    /// 11/11 `fsops` tests and 14/14 `apply` tests pass. This is the demoted-path
    /// twin of `fsops::tests::a_hardlinked_destination_is_a_conflict_not_a_false_success`:
    /// it drives the exact same C5 scenario through `check_then_rename` itself,
    /// the rung used by any filesystem lacking `renameat2`/`renameatx_np` and by
    /// the whole Windows tier, so deleting the guard again fails a test here
    /// even though the atomic rung is a `#[cfg(unix)]`-only code path.
    #[cfg(unix)]
    #[test]
    fn check_then_rename_refuses_a_hardlinked_destination() {
        use std::os::unix::fs::MetadataExt as _;
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"CONTENT").expect("write");
        fs::hard_link(t.path().join("a b.txt"), t.path().join("a_b.txt")).expect("link");

        let err = check_then_rename(
            &PlatformRenameOps,
            &pin(t.path()),
            OsStr::new("a b.txt"),
            OsStr::new("a_b.txt"),
        )
        .expect_err("a hardlink is an occupied destination, not a same-entry respell");
        assert_eq!(err, RenameErr::AlreadyExists);

        assert!(t.path().join("a b.txt").exists());
        assert!(t.path().join("a_b.txt").exists());
        assert_eq!(
            fs::read(t.path().join("a b.txt")).expect("read"),
            b"CONTENT"
        );
        assert_eq!(
            fs::read(t.path().join("a_b.txt")).expect("read"),
            b"CONTENT"
        );
        assert_eq!(
            fs::metadata(t.path().join("a b.txt")).expect("stat").ino(),
            fs::metadata(t.path().join("a_b.txt")).expect("stat").ino(),
            "still the same hardlinked file, untouched"
        );
    }

    /// The demoted-path equivalent of `fsops::tests::a_case_only_respell_succeeds`:
    /// the guard `check_then_rename_refuses_a_hardlinked_destination` pins must
    /// still let a genuine same-entry respell through. On a case-sensitive
    /// filesystem this passes trivially (the destination is simply free); on a
    /// case-insensitive one it is the real exercise of `is_same_entry_not_hardlink`
    /// answering `true` because no literal entry spelled `case.txt` exists --
    /// the lookup for it folded onto `Case.txt`'s own entry.
    #[test]
    fn check_then_rename_case_only_respell_succeeds() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("Case.txt"), b"x").expect("write");
        check_then_rename(
            &PlatformRenameOps,
            &pin(t.path()),
            OsStr::new("Case.txt"),
            OsStr::new("case.txt"),
        )
        .expect("a case-only respell is one entry seen twice, not a hardlink");
        assert_eq!(fs::read(t.path().join("case.txt")).expect("read"), b"x");
    }

    /// **Regression pin for the fix's own first, wrong cut.** A verifier found
    /// that `is_same_entry_not_hardlink`'s original `nlink == 1` discriminator
    /// breaks a genuine respell as soon as the source has *any* hardlink,
    /// anywhere -- `nlink` is volume-wide, not directory-local, so a file
    /// respelled here that is also hardlinked from some unrelated directory
    /// fails an `nlink == 1` test even though `dir` itself still has exactly
    /// one entry naming it. This is the case that broke: hardlink `Case.txt`
    /// to a name in a second, unrelated directory (so `nlink == 2`), then
    /// respell `Case.txt` -> `case.txt` in the original directory. The
    /// respell must still succeed -- requirement 1 from the adversarial
    /// review's correction. On a case-sensitive filesystem this passes
    /// trivially, same as the sibling test above; on a case-insensitive one
    /// (the default on the macOS CI this crate targets) it is the real
    /// exercise: `dir_has_literal_entry` must say "no" for `case.txt` even
    /// though the shared inode's `nlink` is 2.
    #[cfg(unix)]
    #[test]
    fn a_respell_succeeds_even_when_the_source_has_an_unrelated_hardlink() {
        use std::os::unix::fs::MetadataExt as _;
        let t = tempfile::tempdir().expect("tempdir");
        let elsewhere = tempfile::tempdir().expect("second tempdir, a different directory");
        fs::write(t.path().join("Case.txt"), b"x").expect("write");
        fs::hard_link(
            t.path().join("Case.txt"),
            elsewhere.path().join("unrelated_link"),
        )
        .expect("link");
        assert_eq!(
            fs::metadata(t.path().join("Case.txt"))
                .expect("stat")
                .nlink(),
            2,
            "the setup must actually produce nlink >= 2, or this test proves nothing"
        );

        check_then_rename(
            &PlatformRenameOps,
            &pin(t.path()),
            OsStr::new("Case.txt"),
            OsStr::new("case.txt"),
        )
        .expect(
            "a same-entry respell must succeed regardless of a hardlink in an unrelated \
             directory",
        );
        assert_eq!(fs::read(t.path().join("case.txt")).expect("read"), b"x");
    }
}
