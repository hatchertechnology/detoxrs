//! Regression tests for two defects from the second adversarial review of M1
//! (C-8, C-9): a run that could not do what it was asked used to exit `0`, and
//! a batch that broke a relative symlink reported `0 failed`.
//!
//! Every test drives the real binary against a real temporary tree, and
//! `XDG_STATE_HOME` is pointed at a throwaway directory so a run here never
//! touches the developer's own undo journal.

use assert_cmd::Command;
use std::fs;
use std::path::Path;

fn detoxrs(cwd: &Path, state: &Path) -> Command {
    let mut c = Command::cargo_bin("detoxrs").expect("binary builds");
    c.current_dir(cwd).env("XDG_STATE_HOME", state);
    c
}

/// Puts a directory's permissions back on drop, even if the test that made it
/// `000` panics before reaching its own cleanup -- a `chmod 000` directory
/// left behind by a failed assertion would break every test run after it.
#[cfg(unix)]
struct RestorePerms<'a>(&'a Path);

#[cfg(unix)]
impl Drop for RestorePerms<'_> {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
    }
}

/// A `-x` run whose only work is an unresolved conflict must exit non-zero:
/// the rename it was asked to do did not happen, and `--on-collision skip`
/// never gets a second chance at that name.
#[test]
fn a_conflict_that_is_never_attempted_still_fails_the_exit_code() {
    let tree = tempfile::tempdir().expect("tempdir");
    let state = tempfile::tempdir().expect("tempdir");
    fs::write(tree.path().join("a b.txt"), b"a").expect("write");
    fs::write(tree.path().join("a_b.txt"), b"b").expect("write");

    let out = detoxrs(tree.path(), state.path())
        .args(["-x", "--on-collision", "skip", "-r", "."])
        .output()
        .expect("runs");

    assert_eq!(
        out.status.code(),
        Some(1),
        "an unresolved conflict must not exit 0: stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    // Both original files must still be exactly where they were: a refused
    // conflict is not a partial rename.
    assert!(tree.path().join("a b.txt").exists());
    assert!(tree.path().join("a_b.txt").exists());
}

/// The same conflict, previewed rather than applied, must stay exit `0`:
/// a preview's whole purpose is to *report* a pending conflict, and a plain
/// `detoxrs somedir` with nothing wrong must never start exiting non-zero
/// just because a name collides.
#[test]
fn a_conflict_in_a_plain_preview_still_exits_zero() {
    let tree = tempfile::tempdir().expect("tempdir");
    let state = tempfile::tempdir().expect("tempdir");
    fs::write(tree.path().join("a b.txt"), b"a").expect("write");
    fs::write(tree.path().join("a_b.txt"), b"b").expect("write");

    let out = detoxrs(tree.path(), state.path())
        .args(["--on-collision", "skip", "-r", "."])
        .output()
        .expect("runs");

    assert_eq!(out.status.code(), Some(0));
}

/// A subtree the walk could not read at all must not be reported as a clean
/// `0 failed` / exit `0` run: the tool never even looked at part of what it
/// was told to clean.
///
/// `chmod 000` on a directory is only a permission barrier for a non-root
/// process; skip rather than false-fail under a `root` test runner, where the
/// bits are ignored and the subtree is perfectly readable. The permissions
/// are restored by a guard that runs even if an assertion below panics, so
/// this test can never leave a `000` directory behind for the rest of the
/// suite.
#[test]
fn an_unreadable_subtree_fails_the_exit_code_not_just_a_warning() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        let tree = tempfile::tempdir().expect("tempdir");
        let state = tempfile::tempdir().expect("tempdir");
        let secret = tree.path().join("secret");
        fs::create_dir(&secret).expect("mkdir");
        fs::write(secret.join("h idden.txt"), b"x").expect("write");
        fs::write(tree.path().join("o k.txt"), b"y").expect("write");

        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).expect("chmod 000");
        let _restore = RestorePerms(&secret);

        if fs::read_dir(&secret).is_ok() {
            eprintln!("skipping: running as root, chmod 000 is not a barrier");
            return;
        }

        let out = detoxrs(tree.path(), state.path())
            .args(["-x", "-r", "."])
            .output()
            .expect("runs");

        assert_eq!(
            out.status.code(),
            Some(1),
            "an unreadable subtree must not exit 0: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            tree.path().join("o_k.txt").exists(),
            "the readable sibling was still cleaned"
        );
    }
}

/// The same unreadable subtree, previewed rather than applied, must also
/// fail: the plan printed is not the plan for the whole tree the user named.
#[test]
fn an_unreadable_subtree_fails_a_plain_preview_too() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        let tree = tempfile::tempdir().expect("tempdir");
        let state = tempfile::tempdir().expect("tempdir");
        let secret = tree.path().join("secret");
        fs::create_dir(&secret).expect("mkdir");
        fs::write(secret.join("h idden.txt"), b"x").expect("write");
        fs::write(tree.path().join("o k.txt"), b"y").expect("write");

        fs::set_permissions(&secret, fs::Permissions::from_mode(0o000)).expect("chmod 000");
        let _restore = RestorePerms(&secret);

        if fs::read_dir(&secret).is_ok() {
            eprintln!("skipping: running as root, chmod 000 is not a barrier");
            return;
        }

        let out = detoxrs(tree.path(), state.path())
            .args(["-r", "."])
            .output()
            .expect("runs");

        assert_eq!(out.status.code(), Some(1));
    }
}

/// A batch that renames a symlink's target (or the link itself) inside the
/// same tree must not report `0 failed` / exit `0` when the link is left
/// dangling: the tool detects and reports this, rather than silently
/// claiming success or silently rewriting the link's target (see `apply.rs`'s
/// module doc for the detect-vs-repair judgement call).
#[test]
fn a_batch_that_breaks_a_relative_symlink_is_not_reported_as_clean() {
    #[cfg(unix)]
    {
        let tree = tempfile::tempdir().expect("tempdir");
        let state = tempfile::tempdir().expect("tempdir");
        fs::write(tree.path().join("t arget.txt"), b"TARGET").expect("write");
        std::os::unix::fs::symlink("t arget.txt", tree.path().join("l ink")).expect("symlink");

        let out = detoxrs(tree.path(), state.path())
            .args(["-x", "-r", "."])
            .output()
            .expect("runs");

        assert_eq!(
            out.status.code(),
            Some(1),
            "a newly-dangling symlink must not exit 0: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let link = tree.path().join("l_ink");
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "the link itself must still exist, just dangling"
        );
        assert!(
            fs::metadata(&link).is_err(),
            "the link must actually be dangling for this test to mean anything"
        );
    }
}

/// A symlink that was already dangling before the batch is not this batch's
/// doing, and must not be reported as a newly-broken link.
#[test]
fn a_symlink_that_was_already_dangling_is_not_reported() {
    #[cfg(unix)]
    {
        let tree = tempfile::tempdir().expect("tempdir");
        let state = tempfile::tempdir().expect("tempdir");
        // Points at a name that never existed.
        std::os::unix::fs::symlink("no such file.txt", tree.path().join("l ink")).expect("symlink");

        let out = detoxrs(tree.path(), state.path())
            .args(["-x", "-r", "."])
            .output()
            .expect("runs");

        assert_eq!(
            out.status.code(),
            Some(0),
            "a link that was already dangling is not a new failure: stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
