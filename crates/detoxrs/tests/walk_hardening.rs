//! Filesystem-level regression tests for three walk/plan defects (adversarial
//! review `docs/reviews/m1-write-path-adversarial-review.md`, C3/C8/C9).
//!
//! Every assertion here reads the real filesystem after running the real
//! binary -- never a mock, never a value compared to itself -- because that is
//! exactly the shortcut the review found missing in this repo's prior test
//! layer.

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// A run in `cwd`, journalling into its own throwaway state dir so it never
/// touches the developer's real undo journal.
fn detoxrs(cwd: &Path) -> Command {
    let mut c = Command::cargo_bin("detoxrs").expect("binary builds");
    c.current_dir(cwd).env("XDG_STATE_HOME", cwd.join(".state"));
    c
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

// ---- C3: a trailing slash must not turn a symlink guard into an escape -----

/// `detoxrs -x -r somelink` must never touch anything outside the named tree,
/// no matter which of the two ordinary spellings of `somelink` is used.
/// Without the fix, `somelink/` (the spelling shell tab-completion produces)
/// makes `lstat` dereference the link, `md.is_dir()` lies, and the walk
/// descends into -- and renames inside -- whatever the link points at.
#[test]
fn symlink_trailing_slash_does_not_escape_the_tree() {
    #[cfg(unix)]
    {
        let root = TempDir::new().expect("tempdir");
        let outside = root.path().join("outside");
        let tree = root.path().join("tree");
        fs::create_dir_all(&outside).expect("mkdir outside");
        fs::create_dir_all(&tree).expect("mkdir tree");
        let victim = outside.join("victim file.txt");
        fs::write(&victim, "secret").expect("write victim");
        std::os::unix::fs::symlink("../outside", tree.join("dirlink")).expect("symlink");

        // Without the trailing slash: already correct, pinned as a baseline so
        // a future regression here is caught by the same test.
        detoxrs(&tree)
            .args(["-x", "-r", "dirlink"])
            .assert()
            .success();
        assert!(
            victim.exists(),
            "no slash: victim must still exist at its original name"
        );
        assert_eq!(read(&victim), "secret");

        // With the trailing slash: this is the defect. Before the fix, this
        // renamed `victim file.txt` to `victim_file.txt` *outside* `tree/`.
        let _ = detoxrs(&tree).args(["-x", "-r", "dirlink/"]).assert();
        assert!(
            victim.exists(),
            "trailing slash must not make the walk escape `tree/` -- \
             `victim file.txt` was renamed outside the named tree"
        );
        assert!(
            !outside.join("victim_file.txt").exists(),
            "trailing slash escaped the tree and renamed the file outside it"
        );
        assert_eq!(
            read(&victim),
            "secret",
            "the file outside the tree must be untouched, not merely present"
        );
    }
}

/// The mis-kinding half of C3: a symlinked directory named with a trailing
/// slash must still classify as `[symlink]`, and applying its own rename must
/// not fail with the misleading "changed since the preview" diagnosis that a
/// wrongly-followed `lstat` produces (the link's recorded identity was the
/// *target's*, so the apply-time identity recheck saw a mismatch that was
/// never really there).
#[test]
fn symlink_trailing_slash_previews_as_a_symlink_not_a_directory() {
    #[cfg(unix)]
    {
        let root = TempDir::new().expect("tempdir");
        let target = root.path().join("target dir");
        fs::create_dir_all(&target).expect("mkdir target");
        std::os::unix::fs::symlink("target dir", root.path().join("dir link")).expect("symlink");

        // No slash: the existing, already-correct behaviour.
        let out = detoxrs(root.path())
            .args(["-r", "-v", "dir link"])
            .output()
            .expect("run");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("[symlink]"),
            "no slash: expected a [symlink] tag, got: {stdout}"
        );

        // With the slash: same link, same claim, and the `-x` that follows
        // must not misreport a race that never happened.
        let out = detoxrs(root.path())
            .args(["-r", "-v", "dir link/"])
            .output()
            .expect("run");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("[symlink]"),
            "trailing slash must not hide the [symlink] tag behind a \
             dereferenced directory metadata read, got: {stdout}"
        );

        detoxrs(root.path())
            .args(["-x", "-r", "dir link/"])
            .assert()
            .success();
        assert!(
            root.path().join("dir_link").exists(),
            "the link's own rename must succeed, not be refused as \
             \"changed since the preview\""
        );
    }
}

// ---- C8: dedup and collision keys must use identity, not path spelling ----

/// `detoxrs -x -r . sub` names the same tree twice under two spellings.
/// `./sub/f.txt` and `sub/f.txt` are one file; the snapshot must contain it
/// once, so the rename happens once and does not error on a phantom ENOENT
/// for a "second" instance that was never a second file.
#[test]
fn overlapping_directory_arguments_do_not_double_plan_one_file() {
    let root = TempDir::new().expect("tempdir");
    fs::create_dir_all(root.path().join("sub")).expect("mkdir sub");
    let dirty = root.path().join("sub").join("f g.txt");
    fs::write(&dirty, "one file").expect("write");

    let assert = detoxrs(root.path()).args(["-x", "-r", ".", "sub"]).assert();
    let out = assert.get_output();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "overlapping arguments for one physical file must not fail: {stderr}"
    );
    assert!(
        !root.path().join("sub").join("f g.txt").exists(),
        "the dirty name must be gone"
    );
    assert!(
        root.path().join("sub").join("f_g.txt").exists(),
        "the file must have been renamed exactly once"
    );
    // If it had been double-planned, the destination would either not exist
    // (the second attempt ENOENT'd) or the tree would show some renumbered
    // artefact of a phantom second source; neither is present.
    assert!(!root.path().join("sub").join("f_g-2.txt").exists());
}

/// Two *different* files reachable through two spellings of the same
/// directory (`x y/a b.txt` and `./x y/a  b.txt`) that both transform to
/// `a_b.txt`. Before the fix, the two spellings put the collision in two
/// invisible-to-each-other universes, so the preview promised both a
/// clobbering rename and "0 conflicts". After the fix, the collision engine
/// must see both sources at once and number the loser -- never clobber, and
/// never claim there is nothing to resolve.
#[test]
fn overlapping_directory_spellings_still_collide() {
    let root = TempDir::new().expect("tempdir");
    let dir = root.path().join("x y");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(dir.join("a b.txt"), "ONE").expect("write one");
    fs::write(dir.join("a  b.txt"), "TWO").expect("write two");

    detoxrs(root.path())
        .args(["-x", "x y/a b.txt", "./x y/a  b.txt"])
        .assert()
        .success();

    // Neither original survives under its old dirty name...
    assert!(!dir.join("a b.txt").exists());
    assert!(!dir.join("a  b.txt").exists());
    // ...and both distinct files survive under distinct clean names: no
    // clobber, which is the one outcome that would lose data.
    let a_b = dir.join("a_b.txt");
    let numbered = dir.join("a_b-2.txt");
    assert!(a_b.exists(), "the first-numbered destination must exist");
    assert!(
        numbered.exists(),
        "the collision must have been detected and renumbered, not silently \
         dropped as \"0 conflicts\""
    );
    let contents: Vec<String> = [read(&a_b), read(&numbered)]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(
        contents,
        vec!["ONE".to_owned(), "TWO".to_owned()],
        "both distinct files must survive; a clobber would lose one of them"
    );
}

// ---- C9: layer 2 must see a pre-existing destination outside the walk ------

/// `detoxrs 'a b.txt'` where `a_b.txt` already exists on disk. The walk never
/// enumerates `a_b.txt` for a non-recursive single-file argument, so before
/// the fix the preview promised an impossible rename and `-x` refused it as a
/// race that never happened. After the fix, the destination must be numbered
/// up front and applying it must actually succeed.
#[test]
fn a_pre_existing_destination_outside_the_walk_is_seen() {
    let root = TempDir::new().expect("tempdir");
    fs::write(root.path().join("a b.txt"), "dirty").expect("write dirty");
    fs::write(root.path().join("a_b.txt"), "clean").expect("write clean");

    // The preview must not promise the unnumbered name: it cannot be taken.
    let out = detoxrs(root.path())
        .args(["a b.txt"])
        .output()
        .expect("run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("a_b.txt\n") || stdout.contains("a_b-2.txt"),
        "preview must not promise the taken name `a_b.txt` outright: {stdout}"
    );

    detoxrs(root.path())
        .args(["-x", "a b.txt"])
        .assert()
        .success();

    assert!(
        root.path().join("a_b.txt").exists(),
        "the pre-existing file must survive untouched"
    );
    assert_eq!(read(&root.path().join("a_b.txt")), "clean");
    assert!(
        root.path().join("a_b-2.txt").exists(),
        "the dirty file must have been renumbered around the taken name"
    );
    assert_eq!(read(&root.path().join("a_b-2.txt")), "dirty");
    assert!(!root.path().join("a b.txt").exists());
}

// ---- C9 follow-up: a recursive directory argument's own basename ----------

/// `detoxrs -r -v somedir` where a sibling `somedir`-cleaned already exists.
/// `walk_into` only ever looks *inside* the named directory, never beside it,
/// so seeding the pre-existing-destination check only in the non-recursive
/// branch (this round's first attempt) left a recursive directory argument's
/// own basename uncovered: the preview promised an impossible rename and
/// `-x` fell back on the apply-time recheck instead of the preview agreeing
/// with it up front.
#[test]
fn a_recursive_directory_arguments_own_basename_still_collides() {
    let root = TempDir::new().expect("tempdir");
    let dirty = root.path().join(format!("Weird{}", '\u{200b}'));
    let clean = root.path().join("Weird");
    fs::create_dir_all(&dirty).expect("mkdir dirty");
    fs::create_dir_all(&clean).expect("mkdir clean");
    fs::write(dirty.join("marker.txt"), "sibling").expect("write marker");

    let dirty_name = dirty.file_name().unwrap().to_str().unwrap().to_owned();

    // The preview must not promise the taken name `Weird/` outright.
    let out = detoxrs(root.path())
        .args(["-r", "-v", &dirty_name])
        .output()
        .expect("run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Weird-2"),
        "preview must number around the pre-existing `Weird/`, not promise \
         it outright: {stdout}"
    );

    detoxrs(root.path())
        .args(["-x", "-r", &dirty_name])
        .assert()
        .success();

    assert!(clean.exists(), "the pre-existing directory must survive");
    assert!(
        !clean.join("marker.txt").exists(),
        "the pre-existing directory's own (empty) contents must be untouched"
    );
    assert!(
        root.path().join("Weird-2").exists(),
        "the dirty directory must have been renumbered, not lost"
    );
    assert!(
        root.path().join("Weird-2").join("marker.txt").exists(),
        "renumbering the directory must carry its contents with it"
    );
}

// ---- C9 follow-up: seeding must not collide with itself --------------------

/// An NFD-spelled file, named as a single-file argument. `café.txt` (NFD on
/// disk) transforms to the NFC spelling of the same text. On a
/// normalization-insensitive lookup filesystem (APFS), `lstat`ing that NFC
/// candidate resolves right back to the very file being renamed -- it is not
/// a second occupant, and treating it as one made the planner number a
/// respell that has nothing to collide with (`café.txt` -> `café-2.txt`
/// instead of an in-place NFC respell).
#[cfg(target_os = "macos")]
#[test]
fn single_file_nfd_respell_does_not_spuriously_renumber() {
    let root = TempDir::new().expect("tempdir");
    let nfd_name = format!("cafe{}.txt", '\u{301}'); // "café.txt", decomposed
    fs::write(root.path().join(&nfd_name), "x").expect("write");

    let out = detoxrs(root.path())
        .args(["-x", &nfd_name])
        .output()
        .expect("run");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !root.path().join("caf\u{e9}-2.txt").exists(),
        "a lone file must never be numbered against itself"
    );
    let entries: Vec<String> = fs::read_dir(root.path())
        .expect("read_dir")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n != ".state")
        .collect();
    assert_eq!(
        entries,
        vec!["caf\u{e9}.txt".to_owned()],
        "exactly one file, respelled to NFC in place: {entries:?}"
    );
}

// ---- C9 follow-up: argument spelling must not change the outcome ----------

/// The same on-disk NFD file, named on the command line in NFC instead of
/// NFD. Recursive discovery reads `readdir`'s own bytes and never has this
/// ambiguity; a single-file argument used to trust the bytes typed on the
/// command line instead, so an NFC-spelled argument resolved (via APFS's
/// normalization-insensitive lookup) to the NFD file, decoded as already-NFC,
/// and reported "unchanged" -- leaving the NFD bytes on disk untouched with
/// no indication that a different spelling of the very same path would have
/// acted. The NFD-spelled argument is the baseline: both spellings of one
/// path must agree.
#[cfg(target_os = "macos")]
#[test]
fn single_file_argument_spelling_does_not_change_the_outcome() {
    for arg_form in ["nfc", "nfd"] {
        let root = TempDir::new().expect("tempdir");
        let nfd_name = format!("cafe{}.txt", '\u{301}'); // on-disk spelling
        fs::write(root.path().join(&nfd_name), "x").expect("write");
        let arg_name = if arg_form == "nfc" {
            "caf\u{e9}.txt".to_owned()
        } else {
            nfd_name.clone()
        };

        detoxrs(root.path())
            .args(["-x", &arg_name])
            .assert()
            .success();

        let entries: Vec<String> = fs::read_dir(root.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != ".state")
            .collect();
        assert_eq!(
            entries,
            vec!["caf\u{e9}.txt".to_owned()],
            "argument spelled as {arg_form}: the file must end up respelled \
             to NFC regardless of how the argument itself was spelled, \
             got: {entries:?}"
        );
    }
}

// ---- C6: a same-directory hardlink must not steal an argument's identity --

/// `detoxrs -x 'a b.txt'` must rename `a b.txt`, never `c d.txt`, even when
/// the two share an inode. Before the fix, `real_entry_name` searched the
/// directory for *any* entry whose `(dev, ino)` matched and returned the
/// first one `readdir` listed -- which can be a hardlinked sibling the user
/// never named.
#[cfg(unix)]
#[test]
fn a_same_directory_hardlink_does_not_steal_the_named_argument() {
    let root = TempDir::new().expect("tempdir");
    let named = root.path().join("a b.txt");
    let sibling = root.path().join("c d.txt");
    fs::write(&named, "x").expect("write");
    fs::hard_link(&named, &sibling).expect("hardlink");

    detoxrs(root.path())
        .args(["-x", "a b.txt"])
        .assert()
        .success();

    assert!(
        root.path().join("a_b.txt").exists(),
        "the argument's own name must be the one renamed"
    );
    assert!(
        sibling.exists() && read(&sibling) == "x",
        "the hardlinked sibling the user never named must be left alone"
    );
}

// ---- C12: non-recursive numbering must see the whole directory ------------

/// A single-file, non-recursive argument must pick the same collision number
/// a recursive walk of the same tree would pick. Before the fix, only the
/// unnumbered destination was checked against the filesystem, so `plan()`
/// picked `-2` blind to an existing `a_b-2.txt` and `apply` refused the item,
/// misreporting the pre-existing file as having "appeared since the
/// preview".
#[test]
fn single_file_numbering_matches_what_recursive_discovery_would_pick() {
    let root = TempDir::new().expect("tempdir");
    fs::write(root.path().join("a b.txt"), "dirty").expect("write");
    fs::write(root.path().join("a_b.txt"), "taken").expect("write");
    fs::write(root.path().join("a_b-2.txt"), "also taken").expect("write");

    detoxrs(root.path())
        .args(["-x", "a b.txt"])
        .assert()
        .success();

    assert!(
        root.path().join("a_b-3.txt").exists(),
        "must skip past both occupied slots, the same as `-r .` would"
    );
    assert_eq!(read(&root.path().join("a_b.txt")), "taken");
    assert_eq!(read(&root.path().join("a_b-2.txt")), "also taken");
}

// ---- C14: argument resolution must not be quadratic in argument count -----

/// `detoxrs` invoked with every file in a directory as a separate argument
/// (what a shell glob produces) must read that directory a small, constant
/// number of times, not once per argument. Before the fix, `real_entry_name`
/// ran its own `read_dir` plus an `lstat` per entry for every top-level
/// argument, so N arguments sharing one parent did O(N^2) work.
///
/// A wall-clock assertion is flaky in general, but counting the actual work
/// (directory scans, `lstat` calls) would need instrumentation this module
/// does not have, and adding it only for a test is more machinery than the
/// bug is worth. Instead this pins a generous ceiling measured against the
/// unfixed code, not the fixed one: 2500 single-file arguments sharing one
/// directory took 7.2s for the quadratic version in an unoptimized (`dev`)
/// build on the development machine (the review measured 10.5s for 3000
/// args in a *release* build, so a debug build's constant is if anything
/// worse), and the fix runs the same input in well under a second -- an
/// order of magnitude of headroom in both directions, which is what makes
/// this bound a shape assertion in practice rather than a timing race.
#[test]
fn many_single_file_arguments_in_one_directory_do_not_rescan_it_quadratically() {
    let root = TempDir::new().expect("tempdir");
    let names: Vec<String> = (0..2500).map(|i| format!("f_{i:04}.txt")).collect();
    for name in &names {
        fs::write(root.path().join(name), "x").expect("write");
    }

    let start = std::time::Instant::now();
    let mut cmd = detoxrs(root.path());
    cmd.args(&names);
    cmd.assert().success();
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "2500 single-file arguments in one directory took {elapsed:?}; a \
         quadratic real_entry_name took 7.2s for the same input on this \
         machine in a debug build"
    );
}
