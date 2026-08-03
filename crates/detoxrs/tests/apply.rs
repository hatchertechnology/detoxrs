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
    assert_eq!(fs::read(f.path("Screen_Shot.png")).expect("read"), b"shot");
    assert!(!f.path("Screen Shot.png").exists());
    assert_eq!(f.journals().len(), 1, "one journal per batch");

    // The printed hint must be a working command, not just plausible text. An
    // earlier version of this test asserted only that "detoxrs undo " appeared,
    // which a mutation run showed would pass with any id at all -- including an
    // empty one or another run's.
    let id = text
        .lines()
        .find_map(|l| l.strip_prefix("Undo with: detoxrs undo "))
        .expect("the report names a batch to undo")
        .trim()
        .to_owned();
    let undone = f.run().args(["undo", &id]).output().expect("runs");
    assert_eq!(undone.status.code(), Some(0), "{:?}", stderr(&undone));
    assert!(
        f.path("Screen Shot.png").exists(),
        "the id printed by -x did not undo that run"
    );
}

/// `undo --last` must mean the most recently created batch, and it must not depend
/// on the wall clock to know which that is: `SystemTime::now()` can step backwards
/// and an earlier design ordered batches by a timestamp in the filename.
#[test]
fn last_means_most_recently_created() {
    let f = Fixture::new();
    for name in ["a file.txt", "b file.txt", "c file.txt"] {
        f.write(name, b"x");
    }

    // Three batches in rapid succession, each renaming exactly one file.
    for name in ["a file.txt", "b file.txt", "c file.txt"] {
        f.run().args(["-x", name]).assert().success();
    }
    let journals = f.journals();
    assert_eq!(journals.len(), 3);

    // Sorting by name must equal creation order, which is what `list()` relies on.
    let mut sorted = journals.clone();
    sorted.sort();
    assert_eq!(sorted, journals, "filenames do not sort in creation order");

    // And --last must revert the third run, not the first.
    f.run().args(["undo", "--last"]).assert().success();
    assert!(
        f.path("c file.txt").exists(),
        "--last undid the wrong batch"
    );
    assert!(
        f.path("a_file.txt").exists(),
        "an earlier batch was disturbed"
    );
    assert!(
        f.path("b_file.txt").exists(),
        "an earlier batch was disturbed"
    );
}

/// A batch with no completion record either crashed or is still being written by a
/// live run. Undoing it is allowed -- that is the crash-recovery path -- but it must
/// say so and it must not report success, because a run still in progress will keep
/// renaming items this undo has already put back, leaving a half-cleaned tree.
#[test]
fn undoing_an_unfinished_batch_warns_and_does_not_report_success() {
    let f = Fixture::new();
    f.write("a_file.txt", b"x");
    let dir = f.state.path().join("detoxrs").join("journal");
    fs::create_dir_all(&dir).expect("mkdir");
    let ident = {
        let md = fs::symlink_metadata(f.path("a_file.txt")).expect("lstat");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            (md.dev(), md.ino())
        }
        #[cfg(not(unix))]
        {
            (0_u64, 0_u64)
        }
    };
    // A journal that records one completed rename and then simply stops: exactly
    // what a live run's file looks like partway through.
    fs::write(
        dir.join("000001-20260803T170000Z.jsonl"),
        format!(
            "{{\"v\":1,\"batch\":\"000001-20260803T170000Z\"}}\n\
             {{\"op\":\"intent\",\"dev\":{},\"ino\":{},\"kind\":\"file\",\"dir\":{:?},\"from\":\"a file.txt\",\"to\":\"a_file.txt\"}}\n\
             {{\"op\":\"done\",\"ino\":{}}}\n",
            ident.0,
            ident.1,
            f.tree.path().to_str().expect("utf8 tempdir"),
            ident.1
        ),
    )
    .expect("write journal");

    let out = f.run().args(["undo", "--last"]).output().expect("runs");

    assert!(
        f.path("a file.txt").exists(),
        "the recorded item was reverted"
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "an unfinished batch must not report success"
    );
    assert!(
        stderr(&out).contains("no completion record"),
        "{}",
        stderr(&out)
    );
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

/// A run with nothing to rename must not leave a journal behind. An empty batch
/// is not merely litter: it becomes the newest one, so `undo --last` would stop
/// meaning "undo what I just did" after any no-op `-x` run.
#[test]
fn a_run_with_nothing_to_rename_leaves_no_journal() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");
    f.run().args(["-x", "-r", "."]).assert().success();
    assert_eq!(f.journals().len(), 1);

    // Everything is clean now, so this run has nothing to do.
    let out = f.run().args(["-x", "-r", "."]).output().expect("runs");
    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    assert_eq!(f.journals().len(), 1, "a no-op run wrote a journal");

    // And --last still means the batch that actually did something.
    f.run().args(["undo", "--last"]).assert().success();
    assert!(f.path("a file.txt").exists());
}

/// A journal is only durable if it still means something tomorrow, from wherever
/// you happen to be standing. The plan carries directories as the user spelled
/// them on the command line — `.`, `nested dir` — so an undo run from anywhere
/// else has to resolve them against the *recorded* directory, not the current one.
#[test]
fn undo_works_from_a_different_working_directory() {
    let f = Fixture::new();
    fs::create_dir(f.path("sub dir")).expect("mkdir");
    f.write("sub dir/a file.txt", b"x");
    let before = census(f.tree.path());

    f.run().args(["-x", "-r", "."]).assert().success();

    // Somewhere else entirely, with a decoy of the same relative name to catch an
    // undo that resolves `.` against the wrong root.
    let elsewhere = tempfile::tempdir().expect("tempdir");
    fs::create_dir(elsewhere.path().join("sub_dir")).expect("mkdir");
    fs::write(elsewhere.path().join("sub_dir/a_file.txt"), b"decoy").expect("write");

    let out = detoxrs(elsewhere.path(), f.state.path())
        .args(["undo", "--last"])
        .output()
        .expect("runs");

    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    assert_eq!(before, census(f.tree.path()), "undo must restore the tree");
    assert_eq!(
        fs::read(elsewhere.path().join("sub_dir/a_file.txt")).expect("read"),
        b"decoy",
        "the decoy must not have been touched"
    );
}

/// A batch id names a file inside the journal directory and nothing else. It comes
/// straight from the command line, so it is a trust boundary however harmless the
/// worst case looks.
#[test]
fn a_batch_id_cannot_escape_the_journal_directory() {
    let f = Fixture::new();
    let outside = f.state.path().join("secret.jsonl");
    fs::write(&outside, b"{\"v\":1}\n").expect("write");

    for id in [
        "../secret",
        "..%2fsecret",
        "/etc/passwd",
        "sub/../../secret",
    ] {
        let out = f.run().args(["undo", id]).output().expect("runs");
        assert_eq!(out.status.code(), Some(2), "{id} was accepted");
        assert!(stderr(&out).contains("batch id"), "{id}: {}", stderr(&out));
    }
}

/// `-q` is "errors only", and that has to mean the same thing on the write path as
/// on the read path.
///
/// The per-item failure here used to be a pre-existing destination
/// (`b_file.txt` already on disk), which C9's fix now plans around instead of
/// failing (see `a_destination_occupied_at_snapshot_time_is_numbered_away`),
/// so this test switched to a read-only containing directory for the same
/// reason `undo_last_skips_a_batch_where_every_rename_failed` did: a
/// deterministic `EACCES` that does not depend on which layer catches a
/// pre-existing name.
#[cfg(unix)]
#[test]
fn quiet_suppresses_the_applied_report_but_not_errors() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");

    let ok = f
        .run()
        .args(["-x", "-q", "-r", "."])
        .output()
        .expect("runs");
    assert_eq!(ok.status.code(), Some(0), "{:?}", stderr(&ok));
    assert!(ok.stdout.is_empty(), "stdout was: {:?}", ok.stdout);
    assert!(
        f.path("a_file.txt").exists(),
        "-q must not stop the renames"
    );

    // An error still has to arrive, on stderr, where errors go.
    f.write("b file.txt", b"x");
    let mut perms = fs::metadata(f.tree.path()).expect("stat").permissions();
    let writable = perms.clone();
    {
        use std::os::unix::fs::PermissionsExt as _;
        perms.set_mode(0o555);
    }
    fs::set_permissions(f.tree.path(), perms).expect("chmod");

    let bad = f
        .run()
        .args(["-x", "-q", "b file.txt"])
        .output()
        .expect("runs");
    fs::set_permissions(f.tree.path(), writable)
        .expect("chmod back, or the tempdir cannot clean up");

    assert_eq!(bad.status.code(), Some(1));
    assert!(bad.stdout.is_empty());
    assert!(!stderr(&bad).is_empty());
}

/// C9: a destination that already existed **at snapshot time** must be caught
/// during planning and numbered away, not passed through preview as "0
/// conflicts" and then failed at apply time with a false "appeared since the
/// preview" -- `Screen_Shot.png` was never a race, it predates the walk, and
/// the walk now `lstat`s the candidate destination and freezes it as an
/// ordinary snapshot entry so `plan()`'s existing occupancy machinery sees it
/// with no new I/O layer.
///
/// This test used to be named
/// `a_destination_that_appeared_after_the_walk_is_refused` and asserted the
/// old, incorrect behaviour (a false apply-time race failure) -- that name is
/// now backwards: the destination in it does not appear after the walk at
/// all, it is there before the preview even runs, which is exactly C9's
/// point. The genuine post-walk race (a destination that appears *between*
/// the walk and the rename) is a different, still-real safety property,
/// covered by `apply::tests::a_destination_that_appears_after_planning_is_a_fresh_conflict`
/// in `src/apply.rs`, which drives `apply::run` directly against a real
/// filesystem with no `walk`/`plan` involved at all so C9's plan-time fix
/// cannot affect it.
#[test]
fn a_destination_occupied_at_snapshot_time_is_numbered_away() {
    let f = Fixture::new();
    f.write("Screen Shot.png", b"source");
    f.write("Screen_Shot.png", b"squatter");

    let out = f
        .run()
        .args(["-x", "Screen Shot.png"])
        .output()
        .expect("runs");

    assert_eq!(
        out.status.code(),
        Some(0),
        "a destination occupied before the walk must be planned around, not failed: {:?}",
        stderr(&out)
    );
    assert_eq!(
        fs::read(f.path("Screen_Shot.png")).expect("read"),
        b"squatter",
        "the pre-existing file must be untouched"
    );
    assert_eq!(
        fs::read(f.path("Screen_Shot-2.png")).expect("read"),
        b"source",
        "the source must have been renamed onto the next free numbered name"
    );
    assert!(!f.path("Screen Shot.png").exists());
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
/// 3. Every rename that actually happened is in the journal. This one compares the
///    journal against the *filesystem* rather than against itself, and it is here
///    because everything else in this test passes with the intent-before-rename
///    ordering fully inverted -- a mutation run demonstrated that.
/// 4. `undo --last` puts back every completed rename and leaves the interrupted
///    item alone rather than guessing.
///
/// Note on how much this test is worth: property 3 catches an inverted ordering
/// only when the `kill -9` happens to land inside the window the inversion opens,
/// measured at roughly a quarter of runs. It never misfires on correct code -- the
/// invariant always holds there -- but it is a probabilistic detector, not a gate.
/// The gate for the ordering itself is
/// `apply::tests::the_intent_is_recorded_before_the_rename_not_after`, which asserts
/// the interleaving directly and fails every time. Both are kept: one proves the
/// order, the other proves end-to-end recovery.
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

    // **Property 2b: every rename that actually happened is in the journal.**
    // This is the assertion the test was missing. Everything above compares the
    // journal against itself, and a mutation run proved that lets the whole
    // intent-before-rename ordering be inverted without failing: with the rename
    // first, each intent still has a `done` right after it, `unresolved` is still
    // 0, and the file renamed in the gap is simply absent from the journal. Only
    // the filesystem knows the difference.
    let journalled: std::collections::BTreeSet<&str> = recs
        .iter()
        .filter(|r| r.op == "intent")
        .map(|r| r.to.as_str())
        .collect();
    let unjournalled: Vec<String> = fs::read_dir(f.tree.path())
        .expect("readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("f_"))
        .filter(|n| !journalled.contains(n.as_str()))
        .collect();
    assert!(
        unjournalled.is_empty(),
        "{} rename(s) happened with no journal record at all, so undo can never \
         reverse them: {unjournalled:?}",
        unjournalled.len()
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

/// C6: a broken stdout must not report exit 2 -- the code documented as
/// "nothing was attempted at all" -- after renames have already happened, and
/// must not truncate the batch.
///
/// Reproduces the review's `detoxrs -x -r . | head -1`: read exactly one line
/// from the child's stdout, then drop the reader so the read end of the pipe
/// closes. Every later write from the child gets `EPIPE`, exactly as under a
/// real closed pipe.
#[test]
fn a_broken_pipe_does_not_report_exit_2_after_renames_happened() {
    let f = Fixture::new();
    for n in 1..=20 {
        f.write(&format!("k {n}.txt"), b"x");
    }

    // `assert_cmd::Command` has no `spawn`/piped-stdio surface (it always
    // collects output for assertion), so this one test uses `std::process`
    // directly, the same way `assert_cmd::Command::cargo_bin` finds the
    // binary under the hood.
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_detoxrs"))
        .current_dir(f.tree.path())
        .env("XDG_STATE_HOME", f.state.path())
        .args(["-x", "-r", "."])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");

    {
        let stdout = child.stdout.take().expect("piped stdout");
        let mut reader = std::io::BufReader::new(stdout);
        let mut line = String::new();
        std::io::BufRead::read_line(&mut reader, &mut line).expect("read one line");
        // `reader` (and the pipe's read end) drops here.
    }

    let status = child.wait().expect("wait");
    assert_ne!(
        status.code(),
        Some(2),
        "exit 2 means nothing was attempted, but renames already happened"
    );

    let renamed = (1..=20)
        .filter(|n| !f.path(&format!("k {n}.txt")).exists())
        .count();
    assert_eq!(renamed, 20, "a broken pipe must not truncate the batch");
}

/// C7 (route: every rename in a batch failed). A journal describing no
/// completed rename must not become `--last`'s target and shadow the real
/// batch underneath it.
///
/// [`a_destination_that_appeared_after_the_walk_is_refused`]'s technique (a
/// pre-existing destination) is deliberately *not* reused here:
/// `detoxrs-core`'s plan layer is under active repair in this same review
/// round (C8/C9), and which layer catches a pre-existing destination is
/// exactly what is changing there. A read-only containing directory fails
/// the rename itself (`EACCES`) with no dependency on collision detection at
/// any layer, which is the deterministic, per-item, non-aborting failure
/// §5.8 already specifies and `fsops` already has unit coverage for.
#[cfg(unix)]
#[test]
fn undo_last_skips_a_batch_where_every_rename_failed() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");
    f.run().args(["-x", "a file.txt"]).assert().success();
    assert_eq!(f.journals().len(), 1, "the real batch");

    f.write("b file.txt", b"x");
    let mut perms = fs::metadata(f.tree.path()).expect("stat").permissions();
    let writable = perms.clone();
    {
        use std::os::unix::fs::PermissionsExt as _;
        perms.set_mode(0o555); // read + execute, no write: renameat needs write on the dir.
    }
    fs::set_permissions(f.tree.path(), perms).expect("chmod");

    let doomed = f.run().args(["-x", "b file.txt"]).output().expect("runs");
    fs::set_permissions(f.tree.path(), writable)
        .expect("chmod back, or the tempdir cannot clean up");

    assert_eq!(doomed.status.code(), Some(1), "{:?}", stderr(&doomed));
    assert_eq!(f.journals().len(), 2, "the doomed run still gets a journal");

    let out = f.run().args(["undo", "--last"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    assert!(
        f.path("a file.txt").exists(),
        "--last must have reverted the real batch, not the empty one on top of it"
    );
}

/// C7 regression: a batch that crashed *before* its first `done` has an
/// `intent` with no outcome -- `replay.items` is empty exactly like the
/// all-failed and all-refused journals above, but the rename it started is
/// of genuinely unknown fate on disk (`replay.interrupted` is `Some`). An
/// eligibility test that only looked at `items.is_empty()` treated this the
/// same as "nothing happened," fell through to an older, already-clean
/// batch, silently reverted *that* one, and never mentioned the crash --
/// worse than never fixing C7 at all, since the pre-fix behaviour at least
/// landed on the crashed batch and exited 1 with a warning. `--last` is the
/// post-crash recovery command; it must never report success while a
/// half-applied rename from a real crash sits unreported underneath it.
///
/// The crashed batch is hand-crafted exactly like
/// `undoing_an_unfinished_batch_warns_and_does_not_report_success` does (a
/// live run's journal is indistinguishable from one that crashed), but with
/// no `done` line at all -- the state *before* the first rename completes,
/// not partway through a batch of several.
#[test]
fn undo_last_surfaces_a_crash_instead_of_reverting_an_older_batch() {
    let f = Fixture::new();
    // batch 000001: a real, completed rename -- the batch that must stay put.
    f.write("a file.txt", b"x");
    f.run().args(["-x", "a file.txt"]).assert().success();
    assert_eq!(f.journals().len(), 1);

    // batch 000002: crashed before its first `done`. Only an `intent`, no
    // outcome, no `end`.
    f.write("other.txt", b"y");
    let ident = {
        let md = fs::symlink_metadata(f.path("other.txt")).expect("lstat");
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            (md.dev(), md.ino())
        }
        #[cfg(not(unix))]
        {
            (0_u64, 0_u64)
        }
    };
    let dir = f.state.path().join("detoxrs").join("journal");
    fs::write(
        dir.join("000002-20260803T170600Z.jsonl"),
        format!(
            "{{\"v\":1,\"batch\":\"000002-20260803T170600Z\"}}\n\
             {{\"op\":\"intent\",\"dev\":{},\"ino\":{},\"kind\":\"file\",\"dir\":{:?},\"from\":\"other.txt\",\"to\":\"other-renamed.txt\"}}\n",
            ident.0,
            ident.1,
            f.tree.path().to_str().expect("utf8 tempdir"),
        ),
    )
    .expect("write journal");
    assert_eq!(f.journals().len(), 2);

    let out = f.run().args(["undo", "--last"]).output().expect("runs");

    assert_eq!(
        out.status.code(),
        Some(1),
        "a crashed batch must not be reported as a clean success: {:?}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("interrupted while renaming"),
        "the crash must be surfaced, not silently stepped over: {}",
        stderr(&out)
    );
    assert!(
        f.path("a_file.txt").exists(),
        "the real, older, already-clean batch must not have been reverted instead"
    );
}

/// C7 (route: a concurrent run's still-open journal). A journal with no
/// completed rename yet -- the state a live run's journal is in the instant
/// after `Journal::create` returns and before its first `intent` -- must be
/// skipped for `--last`, not picked and then warned about.
#[test]
fn undo_last_skips_a_journal_with_no_completed_rename() {
    let f = Fixture::new();
    f.write("a file.txt", b"x");
    f.run().args(["-x", "a file.txt"]).assert().success();
    assert_eq!(f.journals().len(), 1);

    let dir = f.state.path().join("detoxrs").join("journal");
    fs::write(
        dir.join("000002-20260803T170500Z.jsonl"),
        "{\"v\":1,\"batch\":\"000002-20260803T170500Z\"}\n",
    )
    .expect("write journal");
    assert_eq!(f.journals().len(), 2);

    let out = f.run().args(["undo", "--last"]).output().expect("runs");
    assert_eq!(out.status.code(), Some(0), "{:?}", stderr(&out));
    assert!(
        f.path("a file.txt").exists(),
        "the real batch underneath the in-progress journal must still be reachable via --last"
    );
    assert!(
        !stderr(&out).contains("no completion record"),
        "a journal that was correctly skipped for --last must not be warned about: {}",
        stderr(&out)
    );
}

/// C12: `--json` promises "JSON on stdout, diagnostics on stderr" with no
/// carve-out, but every exit-2 refusal used to write zero bytes to stdout.
#[test]
fn json_error_path_still_emits_a_json_document_on_exit_2() {
    let f = Fixture::new();
    let out = f
        .run()
        .args(["--json", "does-not-exist"])
        .output()
        .expect("runs");
    assert_eq!(out.status.code(), Some(2));
    let doc: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid JSON even on refusal");
    assert_eq!(doc["schema"], 1);
    assert!(doc["error"].is_string(), "{doc}");
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
