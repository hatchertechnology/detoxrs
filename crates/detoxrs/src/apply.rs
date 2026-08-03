//! Phase two: the apply loop (proposal §5.1, §5.3, §5.8; plan §7.3).
//!
//! A deliberate departure from all three implementation plans, which put this
//! loop in `main.rs`. It lives here so the fresh identity recheck, the
//! `EROFS`/`ENOSPC` abort and the per-item error taxonomy can be driven by a test
//! against a real temporary tree and a fault-injecting [`RenameOps`], instead of
//! only through a spawned binary.
//!
//! Three things happen per item, in this order, and the order is the safety
//! property:
//!
//! 1. **A fresh `symlink_metadata` on the source**, compared against the identity
//!    the walk recorded. Something else may have replaced this name since the
//!    snapshot; renaming whatever now sits there would be renaming a file the
//!    user never previewed.
//! 2. **A fresh `symlink_metadata` on the destination.** This is collision layer
//!    2's other half — `plan()` has no I/O, so it can only ever check the frozen
//!    snapshot, and §8.2's property can only ever exercise that half. A file
//!    created at a planned destination *after* the walk is caught here, and by the
//!    kernel behind it, which is what makes this belt-and-braces rather than
//!    duplicated.
//! 3. **`intent`, fsync, rename, `done`/`failed`.** Never a rename before a
//!    durable intent.
//!
//! [`undo`] is the same loop with the same recheck, the same no-clobber rename and
//! its own journal, which is what makes an undo itself undoable without a second
//! code path.
//!
//! **Declared divergence from §5.5.** That section says undo "runs through the same
//! no-clobber rename path and the same collision engine as a forward run, so undo
//! can also report conflicts". It runs through the same rename path but **not** the
//! collision engine: `plan()` derives destinations by transforming names, and an
//! undo's destinations are the recorded originals, so putting them through the
//! planner would re-clean them instead of restoring them. The consequence is that
//! an occupied destination comes back as a per-item `AlreadyExists` failure rather
//! than as a conflict with a renumbered alternative -- which is the behaviour we
//! want anyway, because an undo that invents `Report-2.pdf` to restore into has not
//! undone anything. Recorded here rather than left for a reader to notice; an
//! adversarial review found this undeclared.

use crate::fsops::RenameOps;
use crate::journal::{JournalWrite, UndoItem};
use crate::report::escape;
use detoxrs_core::plan::{Ident, PlanItem, Resolution};
use std::io::Write;

/// What happened to one item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemResult {
    /// Renamed on disk, and recorded as such in the journal.
    Renamed,
    /// Not a `Rename` in the plan, or the batch aborted before reaching it.
    NotAttempted,
    /// Attempted and refused, with the reason as it was reported to the user.
    Failed(String),
}

/// What happened to the batch.
#[derive(Debug, Default)]
pub struct Summary {
    /// One entry per input item, in the input's order.
    pub outcomes: Vec<ItemResult>,
    /// How many renames happened.
    pub renamed: usize,
    /// How many items were attempted and refused.
    pub failed: usize,
    /// Why the rest of the batch was skipped, if it was.
    pub aborted: Option<String>,
}

impl Summary {
    /// `0` when every attempted item succeeded and nothing aborted, `1`
    /// otherwise. Exit `2` is for usage, walk and plan errors, which happen
    /// before this function is reached.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        if self.failed > 0 || self.aborted.is_some() {
            1
        } else {
            0
        }
    }
}

/// Apply a plan. Only [`Resolution::Rename`] items are touched; everything else
/// is already a decision to leave a name alone.
///
/// Writes one line per attempted item to `out`, and per-item failures to stderr.
/// Never panics and never returns `Err`: a journal failure is a batch abort
/// recorded in [`Summary::aborted`], because by then some items may already have
/// been renamed and the caller needs the tally, not an error that discards it.
pub fn run(
    items: &[PlanItem],
    ops: &dyn RenameOps,
    journal: &mut dyn JournalWrite,
    out: &mut impl Write,
) -> Summary {
    let mut s = Summary {
        outcomes: vec![ItemResult::NotAttempted; items.len()],
        ..Summary::default()
    };

    for (i, item) in items.iter().enumerate() {
        if s.aborted.is_some() {
            break;
        }
        if item.resolution != Resolution::Rename {
            continue;
        }

        match attempt(item, ops, journal, out) {
            Ok(()) => {
                s.outcomes[i] = ItemResult::Renamed;
                s.renamed += 1;
            }
            Err(Fail::Item(why)) => {
                eprintln!(
                    "detoxrs: {}: {why}",
                    escape(item.dir.join(&item.from).as_os_str())
                );
                s.outcomes[i] = ItemResult::Failed(why);
                s.failed += 1;
            }
            Err(Fail::Batch(why)) => {
                eprintln!("detoxrs: {why}");
                s.outcomes[i] = ItemResult::Failed(why.clone());
                s.failed += 1;
                s.aborted = Some(why);
            }
        }
    }
    s
}

/// Put a recorded batch back, newest rename first.
///
/// Each item is re-verified against the identity the forward run recorded, so a
/// file that something else has touched since is refused rather than forced —
/// per item, not per batch, because one meddled-with file is no reason to abandon
/// the other 399.
pub fn undo(
    items: &[UndoItem],
    ops: &dyn RenameOps,
    journal: &mut dyn JournalWrite,
    out: &mut impl Write,
) -> Summary {
    let plan: Vec<PlanItem> = items.iter().map(UndoItem::as_plan_item).collect();
    run(&plan, ops, journal, out)
}

/// A per-item failure, or one that ends the batch.
enum Fail {
    Item(String),
    Batch(String),
}

fn attempt(
    item: &PlanItem,
    ops: &dyn RenameOps,
    journal: &mut dyn JournalWrite,
    out: &mut impl Write,
) -> Result<(), Fail> {
    // Step 0: pin the directory. Everything below happens through this one
    // handle, so the entry that is checked and the entry that is renamed are
    // provably in the same directory even if the path that named it is renamed,
    // replaced, or swapped for a symlink while we work. Resolving `item.dir` a
    // second time down at the rename is what an adversarial review reproduced as a
    // wrong-file rename with a falsely successful journal record.
    let dir = ops
        .open(&item.dir)
        .map_err(|e| Fail::Item(format!("cannot open the containing directory: {e}")))?;

    // Step 1: is this still the entry that was previewed?
    let fresh = ops
        .ident_at(&dir, &item.from)
        .map_err(|e| Fail::Item(format!("no longer readable since the preview: {e}")))?;
    if !same_entry(fresh, item.ident) {
        return Err(Fail::Item(
            "changed since the preview (a different file now has this name); not renamed"
                .to_owned(),
        ));
    }

    // Step 2: has anything appeared at the destination since the walk? A
    // same-inode respell is not an occupant -- that is the case-only and
    // NFD -> NFC rename, where source and destination are one file.
    if let Ok(occupant) = ops.ident_at(&dir, &item.to)
        && !same_entry(occupant, item.ident)
    {
        return Err(Fail::Item(format!(
            "{} appeared since the preview; not renamed",
            escape(item.to.as_os_str())
        )));
    }

    // Step 3: intent, durably, before anything moves.
    journal.intent(item).map_err(|e| {
        Fail::Batch(format!(
            "cannot record the undo journal ({e}); the batch stops here rather than renaming \
             something it could not record. Nothing was renamed for this item."
        ))
    })?;

    match ops.rename_noreplace(&dir, &item.from, &item.to) {
        Ok(()) => {
            // A `done` that cannot be written is not worth stopping for: the
            // rename happened, and `undo` treats an intent with no outcome as
            // the interrupted item, which is exactly the right reading.
            drop(journal.done(item));
            // C6: this used to be `Fail::Batch` on write error, which turned a
            // broken stdout (`detoxrs -x -r . | head -1`) into an aborted batch
            // reported as exit 2 -- the code `main.rs` and `--help` both
            // document as "nothing was attempted at all" -- after renames had
            // already happened. The rename above is done and durably
            // journalled; a reader that stopped listening to the progress
            // line is not a reason to call this item failed, nor to stop
            // attempting the rest of a batch the user explicitly asked for.
            // So the write result is dropped, same as `done`'s: the *closing*
            // summary write in `main.rs::exec` is where a broken pipe still
            // has to be visible, because that write is the only remaining
            // place the caller learns the batch's outcome at all.
            drop(writeln!(
                out,
                "{}  ->  {}",
                escape(item.dir.join(&item.from).as_os_str()),
                escape(item.to.as_os_str())
            ));
            Ok(())
        }
        Err(e) => {
            drop(journal.failed(item, e));
            let why = e.to_string();
            Err(if e.aborts_batch() {
                Fail::Batch(format!(
                    "{why}; every remaining item would fail the same way, so the rest of the \
                     batch was not attempted"
                ))
            } else {
                Fail::Item(why)
            })
        }
    }
}

/// Is this the same directory entry the walk saw?
///
/// `(dev, ino)` only. `mtime` is not compared: a rename does not change it, so a
/// mismatch would mean an ordinary write to a file whose *name* is still the one
/// previewed, and refusing to rename that would be refusing for no reason.
/// `nlink` is not compared either, for the same reason.
///
/// On a platform where [`RenameOps::ident_at`] cannot produce real numbers -- the
/// Windows best-effort tier, where `walk` also records zeroes rather than faking
/// them from `file_index` -- both sides are zero and this degenerates to "the name
/// still exists", which the `ident_at` call itself already established. Stated
/// rather than hidden: the identity guarantee is a tier-1 guarantee.
const fn same_entry(fresh: Ident, recorded: Ident) -> bool {
    fresh.dev == recorded.dev && fresh.ino == recorded.ino
}

#[cfg(test)]
mod tests {
    use super::{ItemResult, run, undo};
    use crate::fsops::Dir;
    use crate::fsops::{PlatformRenameOps, RenameErr, RenameOps};
    use crate::journal::{JournalWrite, UndoItem};
    use detoxrs_core::plan::{EntryKind, Ident, PlanItem, Resolution};
    use std::cell::RefCell;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::Path;
    use std::rc::Rc;
    use std::time::UNIX_EPOCH;

    /// A journal that keeps its records in memory, so the loop's protocol can be
    /// asserted without a state directory.
    #[derive(Default)]
    struct FakeJournal {
        lines: RefCell<Vec<String>>,
        fail_intent: bool,
    }

    impl JournalWrite for FakeJournal {
        fn intent(&mut self, item: &PlanItem) -> std::io::Result<()> {
            if self.fail_intent {
                return Err(std::io::Error::other("disk on fire"));
            }
            self.lines
                .borrow_mut()
                .push(format!("intent {:?}", item.from));
            Ok(())
        }
        fn done(&mut self, item: &PlanItem) -> std::io::Result<()> {
            self.lines
                .borrow_mut()
                .push(format!("done {:?}", item.from));
            Ok(())
        }
        fn failed(&mut self, item: &PlanItem, _why: RenameErr) -> std::io::Result<()> {
            self.lines
                .borrow_mut()
                .push(format!("failed {:?}", item.from));
            Ok(())
        }
    }

    /// §8.4's "`RENAME_NOREPLACE` unsupported" row needs a rename that fails a
    /// given way on demand, and no filesystem here provides one. The checks
    /// delegate to the real thing so only the rename is faulted.
    struct AlwaysFails(RenameErr);

    impl RenameOps for AlwaysFails {
        fn open(&self, dir: &Path) -> Result<Dir, RenameErr> {
            PlatformRenameOps.open(dir)
        }
        fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr> {
            PlatformRenameOps.ident_at(dir, name)
        }
        fn rename_noreplace(&self, _d: &Dir, _f: &OsStr, _t: &OsStr) -> Result<(), RenameErr> {
            Err(self.0)
        }
    }

    /// Records the rename into a log shared with the journal double, so the
    /// *interleaving* of the two can be asserted rather than each in isolation.
    struct LoggingOps(Rc<RefCell<Vec<String>>>);

    impl RenameOps for LoggingOps {
        fn open(&self, dir: &Path) -> Result<Dir, RenameErr> {
            PlatformRenameOps.open(dir)
        }
        fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr> {
            PlatformRenameOps.ident_at(dir, name)
        }
        fn rename_noreplace(&self, dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
            self.0.borrow_mut().push("rename".to_owned());
            PlatformRenameOps.rename_noreplace(dir, from, to)
        }
    }

    /// A journal that appends to a caller-supplied log.
    struct LoggingJournal(Rc<RefCell<Vec<String>>>);

    impl JournalWrite for LoggingJournal {
        fn intent(&mut self, _i: &PlanItem) -> std::io::Result<()> {
            self.0.borrow_mut().push("intent".to_owned());
            Ok(())
        }
        fn done(&mut self, _i: &PlanItem) -> std::io::Result<()> {
            self.0.borrow_mut().push("done".to_owned());
            Ok(())
        }
        fn failed(&mut self, _i: &PlanItem, _w: RenameErr) -> std::io::Result<()> {
            self.0.borrow_mut().push("failed".to_owned());
            Ok(())
        }
    }

    /// Counts `open()` calls, delegating everything else to the real
    /// implementation. This is C4's guard: `fsops::tests` pins a `Dir` the
    /// *test* creates and asserts a property of `fsops` alone, so it
    /// structurally cannot see `apply::attempt` acquiring a second handle --
    /// which is where the original defect (`docs/HANDOFF.md`'s "worst finding
    /// of the previous pass") actually lived. An adversarial review proved
    /// that guard blind by adding one line to `apply.rs` that re-opens
    /// `item.dir` right before the rename, reinstating the original defect
    /// with `cargo test` fully green; the reinstated line only showed up as
    /// an occasional wrong-file rename under an active directory-swap race
    /// (5/30 iterations, 14 false journal successes). This double sits at the
    /// layer the defect lived in and turns "sometimes, under a race" into
    /// "every single time, no race required": one pinned directory means
    /// exactly one `open()` per item, and a regression that resolves it a
    /// second time is a wrong count, not a wrong rename that might get lucky.
    #[derive(Default)]
    struct CountingOps {
        opens: RefCell<u32>,
    }

    impl RenameOps for CountingOps {
        fn open(&self, dir: &Path) -> Result<Dir, RenameErr> {
            *self.opens.borrow_mut() += 1;
            PlatformRenameOps.open(dir)
        }
        fn ident_at(&self, dir: &Dir, name: &OsStr) -> Result<Ident, RenameErr> {
            PlatformRenameOps.ident_at(dir, name)
        }
        fn rename_noreplace(&self, dir: &Dir, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
            PlatformRenameOps.rename_noreplace(dir, from, to)
        }
    }

    /// A writer whose every write fails, standing in for a closed stdout pipe
    /// (`detoxrs -x -r . | head -1`) without needing a real subprocess.
    struct FailingWriter;

    impl std::io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "broken pipe",
            ))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn item(dir: &Path, from: &str, to: &str) -> PlanItem {
        let md = fs::symlink_metadata(dir.join(from)).expect("lstat");
        PlanItem {
            dir: dir.to_path_buf(),
            from: OsString::from(from),
            to: OsString::from(to),
            kind: EntryKind::File,
            ident: ident(&md),
            depth: 0,
            resolution: Resolution::Rename,
            truncated: false,
        }
    }

    #[cfg(unix)]
    fn ident(md: &fs::Metadata) -> Ident {
        use std::os::unix::fs::MetadataExt as _;
        Ident {
            dev: md.dev(),
            ino: md.ino(),
            nlink: md.nlink(),
            mtime: UNIX_EPOCH,
        }
    }

    #[cfg(not(unix))]
    fn ident(_md: &fs::Metadata) -> Ident {
        Ident {
            dev: 0,
            ino: 0,
            nlink: 1,
            mtime: UNIX_EPOCH,
        }
    }

    #[test]
    fn a_rename_is_journalled_before_it_happens_and_after() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];

        let mut j = FakeJournal::default();
        let mut out = Vec::new();
        let s = run(&items, &PlatformRenameOps, &mut j, &mut out);

        assert_eq!(s.renamed, 1);
        assert_eq!(s.exit_code(), 0);
        assert_eq!(
            j.lines.borrow().as_slice(),
            [r#"intent "a b.txt""#, r#"done "a b.txt""#]
        );
        assert!(t.path().join("a_b.txt").exists());
    }

    /// §5.8: an unjournaled rename is the one thing undo cannot reverse, so a
    /// journal that cannot be written stops the batch *before* the rename.
    #[test]
    fn a_journal_failure_stops_the_batch_before_renaming() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];

        let mut j = FakeJournal {
            fail_intent: true,
            ..FakeJournal::default()
        };
        let s = run(&items, &PlatformRenameOps, &mut j, &mut Vec::new());

        assert_eq!(s.renamed, 0);
        assert_eq!(s.exit_code(), 1);
        assert!(s.aborted.is_some());
        assert!(t.path().join("a b.txt").exists(), "must not have renamed");
    }

    /// The TOCTOU row (§5.3, §8.4), driven directly: the destination appears
    /// after the item was planned and before apply runs.
    #[test]
    fn a_destination_that_appears_after_planning_is_a_fresh_conflict() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"source").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];
        // The race, made deterministic.
        fs::write(t.path().join("a_b.txt"), b"squatter").expect("write");

        let mut j = FakeJournal::default();
        let s = run(&items, &PlatformRenameOps, &mut j, &mut Vec::new());

        assert_eq!(s.renamed, 0);
        assert_eq!(s.failed, 1);
        assert_eq!(s.exit_code(), 1);
        assert!(matches!(s.outcomes[0], ItemResult::Failed(_)));
        assert_eq!(
            fs::read(t.path().join("a_b.txt")).expect("read"),
            b"squatter",
            "the pre-existing file must be byte-identical"
        );
        assert!(t.path().join("a b.txt").exists(), "source must be intact");
        // Refused before the journal was touched: nothing was intended.
        assert!(j.lines.borrow().is_empty());
    }

    /// The source is not the file that was previewed any more.
    ///
    /// Deterministic, unlike an earlier version of this test which replaced the
    /// file on disk and then asserted `renamed + failed == 1` -- a statement that
    /// is true whether or not the identity check exists, as a mutation run proved.
    /// Here the recorded identity cannot match anything, so refusing is the only
    /// correct outcome and the assertion says so.
    #[test]
    fn a_replaced_source_is_refused() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"original").expect("write");
        let mut items = vec![item(t.path(), "a b.txt", "a_b.txt")];
        items[0].ident = Ident {
            dev: u64::MAX,
            ino: u64::MAX,
            nlink: 1,
            mtime: UNIX_EPOCH,
        };

        let mut j = FakeJournal::default();
        let s = run(&items, &PlatformRenameOps, &mut j, &mut Vec::new());

        assert_eq!(s.renamed, 0);
        assert_eq!(s.failed, 1);
        assert!(
            matches!(&s.outcomes[0], ItemResult::Failed(why) if why.contains("changed since the preview")),
            "{:?}",
            s.outcomes[0]
        );
        assert_eq!(
            fs::read(t.path().join("a b.txt")).expect("read"),
            b"original",
            "the source must be untouched"
        );
        assert!(!t.path().join("a_b.txt").exists());
        // Refused before the journal was touched.
        assert!(j.lines.borrow().is_empty());
    }

    /// `EROFS` will fail every remaining item, so it must produce one message
    /// rather than one per entry (§5.8).
    #[test]
    fn a_read_only_filesystem_aborts_the_rest_of_the_batch() {
        let t = tempfile::tempdir().expect("tempdir");
        for n in ["a 1.txt", "a 2.txt", "a 3.txt"] {
            fs::write(t.path().join(n), b"x").expect("write");
        }
        let items = vec![
            item(t.path(), "a 1.txt", "a_1.txt"),
            item(t.path(), "a 2.txt", "a_2.txt"),
            item(t.path(), "a 3.txt", "a_3.txt"),
        ];

        let s = run(
            &items,
            &AlwaysFails(RenameErr::ReadOnlyFilesystem),
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );
        assert_eq!(s.failed, 1, "one report, not three");
        assert_eq!(s.outcomes[1], ItemResult::NotAttempted);
        assert_eq!(s.outcomes[2], ItemResult::NotAttempted);
        assert!(s.aborted.is_some());
    }

    /// A per-item error is per-item: the batch continues.
    #[test]
    fn a_permission_error_does_not_stop_the_batch() {
        let t = tempfile::tempdir().expect("tempdir");
        for n in ["a 1.txt", "a 2.txt"] {
            fs::write(t.path().join(n), b"x").expect("write");
        }
        let items = vec![
            item(t.path(), "a 1.txt", "a_1.txt"),
            item(t.path(), "a 2.txt", "a_2.txt"),
        ];

        let s = run(
            &items,
            &AlwaysFails(RenameErr::PermissionDenied),
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );
        assert_eq!(s.failed, 2);
        assert!(s.aborted.is_none());
    }

    /// **The ordering, asserted deterministically.** The design's central rule is
    /// that a durable `intent` is written *before* the rename, so a crash can never
    /// leave a rename nobody recorded.
    ///
    /// This test exists because the `kill -9` test in `tests/apply.rs` cannot
    /// enforce it: an adversarial review moved `journal.intent` to *after* the
    /// rename and that test passed 6 runs out of 6, because it only ever checks the
    /// journal against itself. Here the journal double and the rename share one
    /// event log, so the interleaving itself is the assertion and the inversion
    /// fails instantly and every time.
    #[test]
    fn the_intent_is_recorded_before_the_rename_not_after() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];

        let log = Rc::new(RefCell::new(Vec::new()));
        let s = run(
            &items,
            &LoggingOps(Rc::clone(&log)),
            &mut LoggingJournal(Rc::clone(&log)),
            &mut Vec::new(),
        );

        assert_eq!(s.renamed, 1);
        assert_eq!(
            log.borrow().as_slice(),
            ["intent", "rename", "done"],
            "the journal protocol's order is the safety property"
        );
    }

    /// A failed rename must still be bracketed by an intent, so an interrupted
    /// attempt is never invisible.
    #[test]
    fn a_failed_rename_is_also_journalled_intent_first() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];

        let log = Rc::new(RefCell::new(Vec::new()));
        run(
            &items,
            &AlwaysFails(RenameErr::PermissionDenied),
            &mut LoggingJournal(Rc::clone(&log)),
            &mut Vec::new(),
        );
        assert_eq!(log.borrow().as_slice(), ["intent", "failed"]);
    }

    /// The round trip, in one process: apply, then undo, and the tree is what it
    /// was. This is C's Undo-round-trip finding as a unit test; the crash-and-undo
    /// version lives in `tests/apply.rs`.
    #[test]
    fn undo_puts_every_rename_back() {
        let t = tempfile::tempdir().expect("tempdir");
        for n in ["a 1.txt", "b 2.txt"] {
            fs::write(t.path().join(n), n.as_bytes()).expect("write");
        }
        let items = vec![
            item(t.path(), "a 1.txt", "a_1.txt"),
            item(t.path(), "b 2.txt", "b_2.txt"),
        ];
        let s = run(
            &items,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );
        assert_eq!(s.renamed, 2);

        let back: Vec<UndoItem> = items
            .iter()
            .rev()
            .map(|i| UndoItem {
                dir: i.dir.clone(),
                current: i.to.clone(),
                original: i.from.clone(),
                ident: i.ident,
                kind: i.kind,
            })
            .collect();
        let u = undo(
            &back,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );

        assert_eq!(u.renamed, 2);
        assert_eq!(u.exit_code(), 0);
        for n in ["a 1.txt", "b 2.txt"] {
            assert_eq!(fs::read(t.path().join(n)).expect("read"), n.as_bytes());
        }
        assert!(!t.path().join("a_1.txt").exists());
    }

    /// An undo whose file was touched by something else in the meantime is
    /// refused for that item, not forced (§5.5).
    #[test]
    fn undo_refuses_an_item_that_something_else_replaced() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a 1.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a 1.txt", "a_1.txt")];
        run(
            &items,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );

        // Someone replaces the renamed file with a different one.
        fs::remove_file(t.path().join("a_1.txt")).expect("rm");
        fs::write(t.path().join("a_1.txt"), b"different").expect("write");

        let back = vec![UndoItem {
            dir: t.path().to_path_buf(),
            current: OsString::from("a_1.txt"),
            original: OsString::from("a 1.txt"),
            // An identity that cannot match anything on disk.
            ident: Ident {
                dev: u64::MAX,
                ino: u64::MAX,
                nlink: 1,
                mtime: UNIX_EPOCH,
            },
            kind: EntryKind::File,
        }];
        let u = undo(
            &back,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );
        assert_eq!(u.renamed, 0);
        assert_eq!(u.failed, 1);
        assert_eq!(
            fs::read(t.path().join("a_1.txt")).expect("read"),
            b"different",
            "the file something else put there must be untouched"
        );
    }

    /// C4's regression guard, at the layer the defect actually lived in.
    ///
    /// Verified against the actual defect, not just written and trusted: with
    /// `attempt` given one extra `ops.open(&item.dir)` right before the
    /// rename call -- the exact one-line reinstatement an adversarial review
    /// used to bring the original "worst finding" defect back with the suite
    /// green -- this test fails every time (`opens == 2`), no race needed.
    /// Removing that line restores the pass. Both checked by hand while
    /// writing this test; left as this comment because the check itself
    /// cannot run twice in one binary.
    #[test]
    fn attempt_opens_the_directory_exactly_once_per_item() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"x").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];

        let ops = CountingOps::default();
        let s = run(&items, &ops, &mut FakeJournal::default(), &mut Vec::new());

        assert_eq!(s.renamed, 1);
        assert_eq!(
            *ops.opens.borrow(),
            1,
            "attempt must pin the directory once and reuse that one handle for \
             the identity check, the occupancy check and the rename -- a second \
             `open()` call is exactly the regression an adversarial review \
             reproduced as a wrong-file rename with a falsely successful \
             journal record"
        );
    }

    /// C6: a progress line that cannot be written must not be reported as a
    /// failed rename, and must not stop the rest of the batch. The rename
    /// already happened and is already durably journalled by the time this
    /// write is attempted; the item has already succeeded.
    #[test]
    fn a_progress_write_failure_does_not_fail_the_item_or_abort_the_batch() {
        let t = tempfile::tempdir().expect("tempdir");
        for n in ["a 1.txt", "a 2.txt"] {
            fs::write(t.path().join(n), b"x").expect("write");
        }
        let items = vec![
            item(t.path(), "a 1.txt", "a_1.txt"),
            item(t.path(), "a 2.txt", "a_2.txt"),
        ];

        let s = run(
            &items,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut FailingWriter,
        );

        assert_eq!(
            s.renamed, 2,
            "a broken progress pipe must not be reported as a failed rename"
        );
        assert_eq!(s.failed, 0);
        assert!(s.aborted.is_none());
        assert!(t.path().join("a_1.txt").exists());
        assert!(t.path().join("a_2.txt").exists());
    }
}
