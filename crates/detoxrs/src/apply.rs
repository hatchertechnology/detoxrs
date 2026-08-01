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
    let src = item.dir.join(&item.from);
    let dst = item.dir.join(&item.to);

    // Step 1: is this still the entry that was previewed?
    let fresh = std::fs::symlink_metadata(&src)
        .map_err(|e| Fail::Item(format!("no longer readable since the preview: {e}")))?;
    if !same_entry(&fresh, item.ident) {
        return Err(Fail::Item(
            "changed since the preview (a different file now has this name); not renamed"
                .to_owned(),
        ));
    }

    // Step 2: has anything appeared at the destination since the walk? A
    // same-inode respell is not an occupant -- that is the case-only and
    // NFD -> NFC rename, where source and destination are one file.
    if let Ok(occupant) = std::fs::symlink_metadata(&dst)
        && !same_entry(&occupant, item.ident)
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

    match ops.rename_noreplace(&item.dir, &item.from, &item.to) {
        Ok(()) => {
            // A `done` that cannot be written is not worth stopping for: the
            // rename happened, and `undo` treats an intent with no outcome as
            // the interrupted item, which is exactly the right reading.
            drop(journal.done(item));
            writeln!(
                out,
                "{}  ->  {}",
                escape(src.as_os_str()),
                escape(item.to.as_os_str())
            )
            .map_err(|e| Fail::Batch(format!("cannot write output: {e}")))
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
#[cfg(unix)]
fn same_entry(md: &std::fs::Metadata, recorded: Ident) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    md.dev() == recorded.dev && md.ino() == recorded.ino
}

/// Windows is best-effort (owner decision, 2026-07-31): `walk` records `dev` and
/// `ino` as zero there rather than faking them from `file_index`, so this check
/// degenerates to "the name still exists", which is what the `symlink_metadata`
/// call above already established. Stated rather than hidden: the identity
/// guarantee is a tier-1 guarantee.
#[cfg(not(unix))]
fn same_entry(_md: &std::fs::Metadata, _recorded: Ident) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{ItemResult, run, undo};
    use crate::fsops::{PlatformRenameOps, RenameErr, RenameOps};
    use crate::journal::{JournalWrite, UndoItem};
    use detoxrs_core::plan::{EntryKind, Ident, PlanItem, Resolution};
    use std::cell::RefCell;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    use std::path::Path;
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
    /// given way on demand, and no filesystem here provides one.
    struct AlwaysFails(RenameErr);

    impl RenameOps for AlwaysFails {
        fn rename_noreplace(&self, _d: &Path, _f: &OsStr, _t: &OsStr) -> Result<(), RenameErr> {
            Err(self.0)
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
    #[test]
    fn a_replaced_source_is_refused() {
        let t = tempfile::tempdir().expect("tempdir");
        fs::write(t.path().join("a b.txt"), b"original").expect("write");
        let items = vec![item(t.path(), "a b.txt", "a_b.txt")];
        fs::remove_file(t.path().join("a b.txt")).expect("rm");
        fs::write(t.path().join("a b.txt"), b"impostor").expect("write");

        let s = run(
            &items,
            &PlatformRenameOps,
            &mut FakeJournal::default(),
            &mut Vec::new(),
        );
        // On a filesystem that reuses inode numbers immediately this can pass the
        // identity check, in which case the rename is correct and the assertion
        // below is the one that matters: exactly one of the two names exists.
        assert_eq!(s.renamed + s.failed, 1);
        assert_eq!(
            u8::from(t.path().join("a b.txt").exists())
                + u8::from(t.path().join("a_b.txt").exists()),
            1
        );
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
}
