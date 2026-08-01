//! The write path, end to end through the real binary (plan §7.1, `WP5b`).
//!
//! Every test here points `XDG_STATE_HOME` at its own temporary directory, so a
//! run never touches the developer's journal and `undo --last` means "the batch
//! this test just made".
//!
//! The two that matter most, and the reason this file exists rather than more
//! unit tests:
//!
//! * [`crash_mid_batch_is_recoverable`] — `kill -9` in the middle of a batch. The
//!   §8.4 row, and the one property the whole journal design is staked on.
//! * [`a_destination_that_appeared_after_the_walk_is_refused`] — the apply-time
//!   TOCTOU row, made deterministic without a sleep or a debug hook.

use assert_cmd::Command;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// A run in `cwd`, journalling into `state`.
fn detoxrs(cwd: &Path, state: &Path) -> Command {
    let mut c = Command::cargo_bin("detoxrs").expect("binary builds");
    c.current_dir(cwd).env("XDG_STATE_HOME", state);
    c
}

/// A tree and a journal root, kept alive together.
struct Fixture {
    tree: TempDir,
    state: TempDir,
}

impl Fixture {
    fn new() -> Self {
        Self {
            tree: tempfile::tempdir().expect("tempdir"),
            state: tempfile::tempdir().expect("tempdir"),
        }
    }
    fn run(&self) -> Command {
        detoxrs(self.tree.path(), self.state.path())
    }
    fn path(&self, name: &str) -> PathBuf {
        self.tree.path().join(name)
    }
    fn write(&self, name: &str, body: &[u8]) {
        fs::write(self.path(name), body).expect("write");
    }
    /// Every journal file, oldest first.
    fn journals(&self) -> Vec<PathBuf> {
        let dir = self.state.path().join("detoxrs").join("journal");
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out: Vec<PathBuf> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
        out.sort();
        out
    }
}

#[test]
fn exec_renames_and_says_how_to_undo() {
    let f = Fixture::new();
    f.write("Screen Shot.png", b"shot");

    let out = f
        .run()
        .args(["-x", "Screen Shot.png"])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    let text = String::from_utf8(out.stdout).expect("utf8");

    assert!(text.contains("1 renamed"), "{text}");
    assert!(text.contains("detoxrs undo "), "{text}");
    assert_eq!(fs::read(f.path("Screen_Shot.png")).expect("read"), b"shot");
    assert!(!f.path("Screen Shot.png").exists());
    assert_eq!(f.journals().len(), 1, "one journal per batch");
}

/// The round trip through the CLI, which is the only form a user ever sees.
#[test]
fn undo_last_puts_the_batch_back() {
    let f = Fixture::new();
    for name in ["a file.txt", "b file.txt", "c file.txt"] {
        f.write(name, name.as_bytes());
    }
    let before = census(f.tree.path());

    f.run().args(["-x", "-r", "."]).assert().success();
    assert_ne!(before, census(f.tree.path()), "-x must actually rename");

    let out = f.run().args(["undo", "--last"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    assert_eq!(before, census(f.tree.path()), "undo must restore the tree");
}

/// An undo is a batch of renames like any other, so it has its own journal and
/// undoing it re-applies the original run. Two journals in, three out.
#[test]
fn an_undo_can_itself_be_undone() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");

    f.run().args(["-x", "-r", "."]).assert().success();
    f.run().args(["undo", "--last"]).assert().success();
    assert!(f.path("a file.txt").exists());

    f.run().args(["undo", "--last"]).assert().success();
    assert!(f.path("a_file.txt").exists(), "the undo was undone");
    assert_eq!(f.journals().len(), 3);
}

#[test]
fn undo_list_names_the_batches_and_undo_needs_one() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");

    let empty = f.run().args(["undo", "--list"]).output().expect("runs");
    assert_eq!(empty.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&empty.stdout).contains("no recorded batches"));

    f.run().args(["-x", "-r", "."]).assert().success();
    let listed = f.run().args(["undo", "--list"]).output().expect("runs");
    let id = String::from_utf8(listed.stdout)
        .expect("utf8")
        .trim()
        .to_owned();
    assert!(id.ends_with('Z') || id.contains('-'), "batch id: {id}");

    // Naming the batch explicitly must work as well as --last.
    f.run().args(["undo", &id]).assert().success();
    assert!(f.path("a file.txt").exists());

    // And neither is not a default.
    let bare = f.run().arg("undo").output().expect("runs");
    assert_eq!(bare.status.code(), Some(2));
    assert!(stderr(&bare).contains("--last"), "{}", stderr(&bare));
}

/// §8.4's apply-time TOCTOU row, deterministic and with no sleep in it.
///
/// The race is real rather than simulated: collision layer 2 compares against the
/// **snapshot**, and naming one file on the command line puts exactly one entry in
/// that snapshot. `Screen_Shot.png` therefore exists on disk and is invisible to
/// the planner, which is precisely the state a concurrent writer would create
/// between the walk and the rename.
#[test]
fn a_destination_that_appeared_after_the_walk_is_refused() {
    let f = Fixture::new();
    f.write("Screen Shot.png", b"source");
    f.write("Screen_Shot.png", b"squatter");

    let out = f
        .run()
        .args(["-x", "Screen Shot.png"])
        .output()
        .expect("runs");

    assert_eq!(out.status.code(), Some(1), "a per-item failure is exit 1");
    assert!(
        stderr(&out).contains("appeared since the preview"),
        "{}",
        stderr(&out)
    );
    assert_eq!(
        fs::read(f.path("Screen_Shot.png")).expect("read"),
        b"squatter",
        "the pre-existing file must be byte-identical"
    );
    assert_eq!(
        fs::read(f.path("Screen Shot.png")).expect("read"),
        b"source"
    );
}

#[test]
fn json_reports_what_was_applied_and_with_what_guarantee() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");

    let preview = f
        .run()
        .args(["--json", "a file.txt"])
        .output()
        .expect("runs");
    let text = String::from_utf8(preview.stdout).expect("utf8");
    assert!(text.contains("\"applied\": false"), "{text}");
    assert!(text.contains("\"result\": null"), "{text}");

    let applied = f
        .run()
        .args(["-x", "--json", "a file.txt"])
        .output()
        .expect("runs");
    let text = String::from_utf8(applied.stdout).expect("utf8");
    assert!(text.contains("\"applied\": true"), "{text}");
    assert!(text.contains("\"result\": \"renamed\""), "{text}");
    assert!(text.contains("\"atomicity\":"), "{text}");
}

/// A journal that cannot be created means nothing is renamed at all, because a
/// rename that is not recorded is the one thing `undo` cannot reverse (§5.8).
/// `XDG_STATE_HOME` pointing at a plain file is the cheapest way to make
/// `create_dir_all` fail without root.
#[test]
fn no_journal_means_no_renames() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");
    let blocker = f.state.path().join("not-a-dir");
    fs::write(&blocker, b"").expect("write");
    let before = census(f.tree.path());

    let out = detoxrs(f.tree.path(), &blocker)
        .args(["-x", "-r", "."])
        .output()
        .expect("runs");

    assert_eq!(out.status.code(), Some(2), "nothing was attempted");
    assert!(stderr(&out).contains("undo journal"), "{}", stderr(&out));
    assert_eq!(before, census(f.tree.path()), "nothing may have moved");
}

/// **The exit criterion for M1** (plan §7.1, risk 1; §8.4 "Crash mid-batch").
///
/// `kill -9`, not `SIGTERM`: there is no signal handler to give the process a
/// chance to tidy up, which is the point. What must hold afterwards is entirely a
/// property of the on-disk journal:
///
/// 1. Every rename that completed has a `done` record.
/// 2. **At most one** item has an `intent` and no outcome, and that is the exact
///    interrupted item. More than one would mean the loop had renamed something it
///    had not yet finished recording.
/// 3. `undo --last` puts back every completed rename and leaves the interrupted
///    item alone rather than guessing.
#[cfg(unix)]
#[test]
fn crash_mid_batch_is_recoverable() {
    const N: usize = 1000;
    let f = Fixture::new();
    for i in 0..N {
        f.write(&format!("f {i}.txt"), b"x");
    }

    let mut child = std::process::Command::new(assert_cmd::cargo::cargo_bin("detoxrs"))
        .args(["-x", "-r", "."])
        .current_dir(f.tree.path())
        .env("XDG_STATE_HOME", f.state.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn");

    // Watch the journal, not the clock: kill as soon as the batch is provably
    // under way but nowhere near finished.
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        assert!(
            Instant::now() < deadline,
            "the batch never started renaming"
        );
        if let Some(j) = f.journals().first()
            && records(j).iter().filter(|r| r.op == "done").count() >= 5
        {
            break;
        }
        assert!(
            child.try_wait().expect("wait").is_none(),
            "the batch finished before it could be interrupted; raise N"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    child.kill().expect("SIGKILL");
    let status = child.wait().expect("wait");
    assert!(
        status.code().is_none(),
        "expected death by signal: {status:?}"
    );

    // Property 1 and 2, read straight off the journal.
    let journal = f.journals().first().cloned().expect("a journal exists");
    let recs = records(&journal);
    let done: Vec<&Record> = recs.iter().filter(|r| r.op == "done").collect();
    let intents = recs.iter().filter(|r| r.op == "intent").count();
    let failed = recs.iter().filter(|r| r.op == "failed").count();
    assert!(!done.is_empty(), "nothing was recorded as renamed");
    assert!(done.len() < N, "the batch was not actually interrupted");
    let unresolved = intents - done.len() - failed;
    assert!(
        unresolved <= 1,
        "{unresolved} items have an intent and no outcome; at most one may be unknown"
    );

    // Property 3: undo restores the completed prefix.
    let completed = completed_pairs(&recs);
    let out = detoxrs(f.tree.path(), f.state.path())
        .args(["undo", "--last"])
        .output()
        .expect("runs");
    assert!(
        out.status.code() == Some(0) || out.status.code() == Some(1),
        "{:?}",
        stderr(&out)
    );

    for (from, to) in &completed {
        assert!(
            f.path(from).exists(),
            "{from} was renamed to {to} and not put back"
        );
        assert!(!f.path(to).exists(), "{to} still exists after undo");
    }
    // Nothing was created or destroyed by any of this, whatever happened to the
    // interrupted item's name.
    assert_eq!(
        fs::read_dir(f.tree.path()).expect("readable").count(),
        N,
        "the tree gained or lost an entry"
    );
}

/// One journal record, reduced to the fields these tests read.
struct Record {
    op: String,
    from: String,
    to: String,
}

/// Parse a journal, ignoring the header and any line a crash truncated.
fn records(path: &Path) -> Vec<Record> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            Some(Record {
                op: v.get("op")?.as_str()?.to_owned(),
                from: v
                    .get("from")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_owned(),
                to: v
                    .get("to")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_owned(),
            })
        })
        .collect()
}

/// The `(from, to)` of every intent that has a `done` after it.
fn completed_pairs(recs: &[Record]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut pending: Option<(String, String)> = None;
    for r in recs {
        match r.op.as_str() {
            "intent" => pending = Some((r.from.clone(), r.to.clone())),
            "done" => {
                if let Some(p) = pending.take() {
                    out.push(p);
                }
            }
            "failed" => pending = None,
            _ => {}
        }
    }
    out
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Names, kinds and contents' sizes under `root`. Enough to catch a rename, a
/// create or a delete; deliberately not mtime, which `undo` legitimately leaves
/// alone.
fn census(root: &Path) -> BTreeMap<PathBuf, (bool, u64)> {
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
                (md.is_symlink(), md.len()),
            );
        }
    }
    out
}
