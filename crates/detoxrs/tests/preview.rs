//! End-to-end preview cases, driving the real binary over a temporary tree
//! (plan §7.1, `WP5a`).
//!
//! Every case runs with the fixture root as the working directory and `.` (or a
//! relative name) as the argument, so the output contains no absolute temporary
//! path and can be snapshotted verbatim.
//!
//! The defining test of this work package is `binary_never_writes_anything`:
//! at `WP5a` there is no write code, and that is asserted rather than assumed.

use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tempfile::TempDir;

fn detoxrs(cwd: &Path) -> Command {
    let mut c = Command::cargo_bin("detoxrs").expect("binary builds");
    c.current_dir(cwd);
    c
}

/// A tree covering every case `WP5a` has to render: a name needing cleanup, a
/// name already clean, a pre-existing collision, a dotfile, VCS metadata, a
/// nested directory, a symlink to a file and a symlink to a directory.
fn fixture() -> TempDir {
    let t = tempfile::tempdir().expect("tempdir");
    let r = t.path();
    for name in [
        "Screen Shot.png",
        "already_clean.txt",
        "IMG 0042.JPG",
        "IMG_0042.JPG",
        ".dot file.txt",
    ] {
        fs::write(r.join(name), b"x").expect("write");
    }
    fs::create_dir(r.join("nested dir")).expect("mkdir");
    fs::write(r.join("nested dir/inner file.txt"), b"x").expect("write");
    fs::create_dir(r.join(".git")).expect("mkdir");
    fs::write(r.join(".git/HEAD file"), b"x").expect("write");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("already_clean.txt", r.join("link name.txt")).expect("symlink");
        std::os::unix::fs::symlink("nested dir", r.join("dir link")).expect("symlink");
    }
    t
}

fn stdout_of(cmd: &mut Command) -> String {
    let out = cmd.output().expect("runs");
    assert!(
        out.status.success(),
        "exited {:?}: {}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("stdout is UTF-8")
}

/// The whole recursive preview, pinned. Covers cleanup, unchanged, the
/// renumbered collision, dotfile and `.git` exclusion during recursion, the
/// nested directory, and both symlinks in one artifact.
#[cfg(unix)]
#[test]
fn recursive_preview_is_stable() {
    let t = fixture();
    insta::assert_snapshot!(stdout_of(detoxrs(t.path()).args(["-r", "-v", "."])));
}

#[test]
fn a_name_needing_cleanup_shows_its_replacement() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).arg("Screen Shot.png"));
    assert!(out.contains("Screen Shot.png"), "{out}");
    assert!(out.contains("Screen_Shot.png"), "{out}");
    assert!(out.contains("1 to rename"), "{out}");
}

#[test]
fn an_already_clean_name_is_unchanged() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args(["-v", "already_clean.txt"]));
    assert!(out.contains("(unchanged)"), "{out}");
    assert!(out.contains("0 to rename, 1 unchanged"), "{out}");
}

/// `-v` lists unchanged entries; the default preview does not (that is what the
/// flag is for in M1). The summary counts them either way.
#[test]
fn unchanged_entries_are_hidden_without_verbose() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).arg("already_clean.txt"));
    assert!(!out.contains("(unchanged)"), "{out}");
    assert!(out.contains("1 unchanged"), "{out}");
}

/// `IMG 0042.JPG` wants `IMG_0042.JPG`, which already exists, so the preview
/// shows the renumbered destination before anything is applied.
#[test]
fn a_collision_shows_its_renumbered_resolution() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args(["IMG 0042.JPG", "IMG_0042.JPG"]));
    assert!(out.contains("IMG_0042-2.JPG"), "{out}");
}

#[test]
fn on_collision_skip_leaves_the_collision_alone() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args([
        "--on-collision",
        "skip",
        "IMG 0042.JPG",
        "IMG_0042.JPG",
    ]));
    assert!(!out.contains("IMG_0042-2.JPG"), "{out}");
    assert!(out.contains("conflict"), "{out}");
}

#[test]
fn on_collision_fail_refuses_the_batch() {
    let out = {
        let t = fixture();
        detoxrs(t.path())
            .args(["--on-collision", "fail", "IMG 0042.JPG", "IMG_0042.JPG"])
            .output()
            .expect("runs")
    };
    assert_eq!(out.status.code(), Some(2));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("refused"), "stderr was: {err}");
}

/// A dotfile is skipped while recursing but processed when named.
#[test]
fn dotfiles_are_named_only() {
    let t = fixture();
    let recursed = stdout_of(detoxrs(t.path()).args(["-r", "-v", "."]));
    assert!(!recursed.contains(".dot file.txt"), "{recursed}");

    let named = stdout_of(detoxrs(t.path()).arg(".dot file.txt"));
    assert!(named.contains(".dot_file.txt"), "{named}");
}

#[test]
fn vcs_metadata_is_never_touched() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args(["-r", "-v", "."]));
    assert!(!out.contains("HEAD file"), "{out}");
    assert!(!out.contains(".git"), "{out}");
}

/// The link's own name is cleaned, its kind is shown, and the directory it
/// points at is never descended (§5.6): `inner file.txt` appears exactly once,
/// under the real directory, not a second time under `dir link`.
#[cfg(unix)]
#[test]
fn symlinks_are_renamed_but_never_followed() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args(["-r", "-v", "."]));
    assert!(out.contains("[symlink]"), "{out}");
    assert!(out.contains("dir_link"), "{out}");
    assert_eq!(out.matches("inner file.txt").count(), 1, "{out}");
    assert!(!out.contains("dir link/"), "{out}");
}

/// Without `-r`, a directory argument has only its own basename cleaned and
/// nothing inside it is touched (§5.6). With `-r`, the whole subtree.
#[test]
fn recursion_flag_decides_whether_children_are_processed() {
    let t = fixture();
    let shallow = stdout_of(detoxrs(t.path()).arg("nested dir"));
    assert!(shallow.contains("nested_dir"), "{shallow}");
    assert!(!shallow.contains("inner file.txt"), "{shallow}");
    assert!(shallow.contains("1 to rename"), "{shallow}");

    let deep = stdout_of(detoxrs(t.path()).args(["-r", "nested dir"]));
    assert!(deep.contains("inner_file.txt"), "{deep}");
}

#[test]
fn json_goes_to_stdout_and_parses() {
    let t = fixture();
    let out = stdout_of(detoxrs(t.path()).args(["-r", "--json", "."]));
    assert!(out.starts_with('{'), "{out}");
    assert!(out.contains("\"resolution\": \"rename\""), "{out}");
    assert!(out.contains("\"applied\": false"), "{out}");
    // No parser dependency: check the one structural property that matters, that
    // the document is balanced, plus the keys above.
    assert_eq!(
        out.matches('{').count(),
        out.matches('}').count(),
        "unbalanced JSON: {out}"
    );
}

/// An undecodable name is rendered with `<hh>` escapes and never printed raw
/// (§6.1). The raw byte must not appear anywhere in the output.
#[cfg(unix)]
#[test]
fn undecodable_names_are_escaped_never_raw() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let t = tempfile::tempdir().expect("tempdir");
    let name = OsStr::from_bytes(b"Bj\xf6rk - Vespertine.mp3");
    if fs::write(t.path().join(name), b"x").is_err() {
        // APFS refuses to create a filename that is not valid UTF-8 at the
        // syscall level (owner decision, 2026-07-31: this is why spike 6 was
        // never validated). The rendering itself is unit-tested in `report`.
        eprintln!("skipped: this filesystem will not create a non-UTF-8 name");
        return;
    }

    let out = detoxrs(t.path()).args(["-r", "."]).output().expect("runs");
    assert!(out.status.success());
    assert!(
        !out.stdout.contains(&0xf6),
        "raw invalid byte reached stdout"
    );
    let text = String::from_utf8(out.stdout).expect("stdout stays UTF-8");
    assert!(text.contains("Bj<f6>rk"), "{text}");
    assert!(text.contains("not valid UTF-8"), "{text}");
}

/// The dry-run default (§2.1), asserted rather than assumed: **no invocation
/// without `-x` changes a filesystem.** `WP5a` could prove this by construction,
/// because no write code existed; from `WP5b` on it is a property of one branch in
/// `main::run`, which makes it worth a test rather than a paragraph. `-x` is
/// deliberately absent from this list — it is the one thing that is supposed to
/// write, and it has its own file.
#[test]
fn preview_never_writes_anything() {
    let t = fixture();
    let before = census(t.path());

    for args in [
        vec!["."],
        vec!["-r", "."],
        vec!["-r", "-v", "."],
        vec!["-r", "--json", "."],
        vec!["-q", "-r", "."],
        vec!["-n", "-r", "."],
        vec!["-r", "--on-collision", "skip", "."],
        vec!["-r", "--on-collision", "fail", "."],
        vec!["Screen Shot.png"],
        vec!["nested dir"],
    ] {
        detoxrs(t.path()).args(&args).output().expect("runs");
    }

    assert_eq!(before, census(t.path()), "the tree changed");
}

/// Every path under `root`, with its kind, size and mtime. Directory entries and
/// mtimes are what a rename or a create would move; atime deliberately is not,
/// since merely reading a directory updates it.
fn census(root: &Path) -> BTreeMap<PathBuf, (bool, u64, SystemTime)> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).expect("readable") {
            let path = entry.expect("entry").path();
            let md = fs::symlink_metadata(&path).expect("lstat");
            if md.is_dir() {
                stack.push(path.clone());
            }
            out.insert(
                path.strip_prefix(root).expect("under root").to_path_buf(),
                (
                    md.is_symlink(),
                    md.len(),
                    md.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                ),
            );
        }
    }
    out
}
