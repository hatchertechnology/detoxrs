---
plan: A
mandate: safety-first, correctness-before-features
author: Plan Author A (independent of Plan Authors B and C)
date: 2026-07-31
status: draft — one of three independent plans; a later reviewer merges them
documents-read:
  - docs/owner-decisions.md
  - docs/research/00-proposal-rust-detox-successor.md (full: §0-§11, Appendix A, Review record, Propagation record)
  - docs/rust-setup-notes.md
  - docs/rust-setup-ci.md (skimmed via justfile/CI wiring already described in rust-setup-notes.md)
  - docs/research/10-detox-cli-surface.md through 13-detox-build-env-and-runtime-inputs.md (skimmed for upstream behavior, cited only where load-bearing)
  - docs/research/user_feedback_online.md (skimmed for demand signal, cited only where load-bearing)
  - justfile, Cargo.toml, crates/detoxrs-core/src/lib.rs, crates/detoxrs/src/main.rs (current repo state)
---

# Plan A: safety-first, correctness-before-features

## 0. Mandate, stated as a falsifiable rule

Sequence the work so that the properties which are **hard to retrofit** exist, are tested, and are
green in CI **before** anything that could destroy a user's file is buildable at all. Concretely:
the two-phase `walk -> plan -> apply` separation (§5.1), the no-clobber rename path (§5.4), the
collision engine (§5.3), and the crash-safe journal with `undo` (§5.5) must all exist, and their
own property-test gates (§8.2, plus the totality/idempotence half of §8.1) must be green, before
`-x` is wired to anything users would actually want to run day to day. The transform pipeline's
remaining cosmetic stages (§3.2 stages 2, 5, 6, 8, 9, 11, 12 in part) are comparatively cheap to add
later, because adding a pure function to an already-safe pipe cannot regress the safety envelope
around it — that asymmetry (plan-level invariants are load-bearing infrastructure; transform-level
stages are additive) is the whole argument for this plan's ordering, and §9 below defends it against
the "build the visible transform first" alternative directly.

This costs a slower first _usable_ release. It buys a v0.1 that is safe to run unattended on a real
home directory from the day it exists, because the two hardest-to-retrofit properties — "never
returns an unsafe name" (Totality) and "never destroys a file, ever" (the §8.2 plan properties plus
the journal) — are gates on milestone 1, not aspirations for v1.0.

## 1. Milestone list

| #   | Name                                                   | One-line scope                                                                                                                                                                                                                                          |
| --- | ------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1  | Safety envelope, minimal pipeline                      | walk (incl. `-r`) → plan → apply → journal/undo, `rustix` no-clobber rename with fallback, collision engine, a deliberately thin transform (decode, NFC-for-comparison, invisible-strip, control-char deletion, dash/dot/space trim, `Unrepresentable`) |
| M2  | Transform pipeline to full v0.1 default set            | `safe_map`'s separator/keep classes, `collapse`, grapheme-safe `truncate`, full `finalize` fixed-point loop; spikes 2/13/14 matrices run and recorded; `--json`, verbosity, `insta` snapshots                                                           |
| M3  | Collision-adjacent and platform spikes closed for v1.0 | spikes 5, 11, 15 closed (case-only rename on network FS, `Unrepresentable` frequency on real trees, hardlink respell safety); can start once M1's `fsops.rs`/`plan.rs` exist, runs concurrently with M2                                                 |
| M4  | Configuration and customization                        | config file discovery/precedence, `[profile.*]`, `[[rule]]`, `--keep`/`--strip`, `--case`, `--ascii`, `url_decode` stage, `--print-config`                                                                                                              |
| M5  | Portability and operational surface                    | `--target windows/portable`, per-directory length-limit auto-detection (`statfs`), `--plan-out`/`apply <plan>`, `--stdin`, shell completions/man pages                                                                                                  |
| M6  | v1.0 hardening and release                             | full CI filesystem matrix green (tier-1 Linux + macOS), fuzz target in CI, mixed-script warning (cheap subset of §3.12), Windows best-effort build+unit-test in CI, `MIGRATING-FROM-DETOX.md`, `--help-transforms`, packaging items 1-4 (§9.4)          |

Each milestone ships a working `detoxrs` binary that passes `just gate` and is independently
demoable; none depends on a milestone later than itself.

## 2. Estimation unit

Calendar dates are not meaningful for a plan with one implementer of unknown availability. Instead:
**1 unit ≈ 100-150 lines of reviewed Rust, including the tests that gate it** — calibrated against
proposal §7.3's own checkable budget (v0.1: 1200-1800 LOC; v1.0: 2200-3000 LOC), so a milestone's
unit count is a claim about _size and test burden_, not time. A milestone with heavy property-test
or fixture-matrix work is rated in units even where its production code is small, because the
property tests are the deliverable for this mandate, not a checkbox after it.

## 3. Milestone 1, in file-by-file detail

This is the milestone someone starts on immediately, so it is specified at the level the other
milestones are not.

### 3.1 Scope

A `detoxrs` that:

- Takes one or more path arguments, `-r`/`--recursive`, `-x`/`--exec`, `--on-collision
number|skip|fail` (default `number`, per `docs/owner-decisions.md`), and a `detoxrs undo
<BATCH-ID>` subcommand.
- Walks each argument (recursing only under `-r`), snapshotting the whole entry list before any
  rename, skipping `.git`/`.hg`/`.svn` unconditionally and dotfiles during recursion (§5.6),
  never following a symlinked directory.
- Runs every entry's basename through a **thin** transform: `decode` (§3.2 stage 1), NFC folding
  for internal comparison only (§6.2), invisible/bidi/Tag/`Cf`/`Cs`/`Co` stripping (stage 4),
  control-character deletion (the delete-class slice of stage 7 — no separator/keep
  classification yet), leading-dash and trailing-dot/space trimming (stage 10), and a
  bounded fixed-point re-run of stage 10 that resolves to `Unrepresentable` rather than
  reintroducing an unsafe name (stage 13, §3.14).
- Builds a `Plan` with the three-layer collision engine (§5.3): intra-batch map, pre-existing
  destination check, and the plan-time cycle assertion. `number`/`skip`/`fail` all work.
  `Determinism` holds because ordering is by NFC bytes of the source name, not `readdir()` order.
- Previews by default (P5); `-x` applies via `rustix::fs::renameat_with` +
  `RenameFlags::NOREPLACE`, with the observed-`EEXIST`/same-inode fallback to plain `rename(2)`,
  and the check-then-rename fallback when the flag is unsupported (§5.4).
- Journals every apply as append-only JSONL with `intent`/`done`/`failed` records (§5.5), and
  `detoxrs undo <BATCH-ID>` replays in reverse with the `(dev, ino, mtime)` re-check.
- Installs a `SIGINT`/`SIGTERM` flag handler that stops the apply loop cleanly between items
  (§5.8) and reports a clean prefix.
- Exits 0 (nothing to do or all succeeded), 1 (any per-item failure, `EROFS`/`ENOSPC` abort,
  `EMFILE` before any rename), matching §2.4's exit-code contract at the level M1 implements it.

Explicitly **not** in M1: config file, `[[rule]]`, `--ascii`/`--case`/`--keep`/`--strip`,
`url_decode`, `safe_map`'s separator/keep classes, `collapse`, `truncate`, `--target`, `--json`,
`--plan-out`/`apply <plan>`, `--stdin`, colorized/verbose preview. §3.4 below states the retrofit
cost of each deferral and it is uniformly low, because none of them touches `walk`, `plan`,
`fsops`, or `journal`.

### 3.2 `crates/detoxrs-core` — no I/O, no `clap`, no `std::fs`

```
crates/detoxrs-core/src/
  lib.rs
  decode.rs
  invisible.rs        # generated at build time from UCD data checked into the repo
  classes.rs           # M1: delete-class (Cc + DEL + NUL) only
  pipeline.rs
  plan.rs
```

`decode.rs`:

```rust
/// The one and only place a name becomes text. §3.4 (owner decision 2026-07-31):
/// valid UTF-8, or `Opaque`. No repair, no guess, no lossy conversion.
pub enum Decoded {
    Utf8(String),
    Opaque,
}

/// Total, never panics, never re-interprets. `Utf8(s)` always round-trips to
/// exactly the input bytes -- this is the executable form of the "Decode is
/// total" property (§8.1) and the regression test for detox's `café.txt ->
/// cafÃ©.txt` (doc 01 §7).
///
/// Deliberate deviation from proposal §3.1's signature
/// (`decode(raw: &OsStr, p: &Policy) -> Decoded`): `decode` has no toggle in
/// v1.0 -- repair was dropped by owner decision, so there is no flag for this
/// function to read. Threading a `Policy` through a function with nothing to
/// look up in it is dead parameter, not fidelity to the design; if a future
/// `--repair-encoding` lands post-1.0 (§3.4), that is the moment to add the
/// parameter back, with a real field behind it.
pub fn decode(raw: &std::ffi::OsStr) -> Decoded;
```

`classes.rs` (M1 scope only):

```rust
/// M1 ships only the delete class: Unicode `Cc` (C0 including newline/tab,
/// C1) plus DEL and NUL. The separator and keep classes (§3.7) arrive in M2
/// with `safe_map`; introducing them piecemeal here would make M1's
/// "Stage independence" story (§8.1) untestable, since there would be no
/// stage boundary to disable.
#[must_use]
pub fn is_delete_class(c: char) -> bool;
```

`invisible.rs`:

```rust
/// Bidi controls (U+202A-202E, U+2066-2069, U+200E/200F), zero-width
/// (U+200B/200C/200D/2060/FEFF), Unicode Tags (U+E0000-E007F), and remaining
/// `Cf`/`Cc`/`Cs`/`Co`. Table generated at build time from UCD data checked
/// into the repo (P7: no `unicode-security`/`unicode_skeleton` dependency for
/// this). `Cc` overlaps `classes::is_delete_class`'s set; that overlap is
/// intentional (both stages agree control characters go), not a bug -- see
/// the inline note in pipeline.rs stage 4/7 ordering.
#[must_use]
pub fn is_invisible(c: char) -> bool;
```

`pipeline.rs`:

```rust
use crate::decode::Decoded;

/// Every field maps 1:1 to a flag or config key that exists **yet**. M1's
/// `Policy` has exactly the fields M1's stages read: `on_collision` lives in
/// the CLI/plan layer, not here, because it is a plan-time concern (§5.3),
/// not a transform-time one. Deliberately not `#[non_exhaustive]`: every
/// field this struct gains in M2+ is a field some property test must also
/// gain a generator for, and an exhaustive struct makes a missed field a
/// compile error in the test crate instead of a silent gap.
pub struct Policy {
    pub separator: char, // wired through from M1 even though nothing in M1 emits it yet:
                          // the field exists so M2's safe_map does not need a Policy shape change,
                          // which would be a breaking change to every M1 test fixture.
}

pub enum Unrepresentable {
    ReducesToEmpty,
    ReducesToDotOrDotDot,
    NotConverged,
}

pub struct Outcome {
    pub text: String,
    pub stages: Vec<StageDelta>, // for -vv (M2) and snapshot tests (M2); populated from M1
                                  // so the type does not change shape later.
}

pub enum TransformResult {
    Name(Outcome),
    Unrepresentable(Unrepresentable),
}

/// Pure. No I/O, no allocation of paths, no knowledge of any other file.
/// M1's pipeline: decode -> (Opaque short-circuits to a separate `Skipped`
/// path in walk.rs, never reaching here) -> NFC-fold for comparison only
/// (the *returned* text is not forced to NFC output in M1 -- see §3.4 below,
/// item "stage 3 split") -> invisible-strip -> delete-class -> trim
/// (leading `-`; trailing `.`/space; preserve one leading `.`) -> bounded
/// re-run of trim only, up to 3 iterations -> Unrepresentable check.
pub fn transform(d: &Decoded, p: &Policy) -> TransformResult;
```

**Deliberate M1 narrowing, stated so it is not discovered from a diff later:** proposal §3.5 makes
NFC the _default output_, not just the comparison key. M1 folds to NFC for **comparison only**
(feeding `plan.rs`'s collision map) and does **not** rewrite the visible output to NFC yet, because
NFC-as-output requires `unicode-normalization`'s `to_nfc()` on the emitted string, which M1 pulls in
anyway for the comparison fold — so this is a one-line change, not a missing dependency. It is
narrowed in M1 specifically to keep the "what does this milestone actually promise" list short and
auditable; M2 turns the fold into a rewrite and gains the full "Idempotence under normalization"
half of §8.1's Idempotence property. This is the single largest M1 divergence from proposal §3.2's
stage list and it is called out here, not left for a reviewer to find.

`plan.rs`:

```rust
use std::path::PathBuf;
use std::ffi::OsString;

pub enum EntryKind { File, Dir, Symlink, Other }

/// (dev, ino, nlink, mtime) captured at walk time. Read by `apply`'s
/// stale-plan recheck (only load-bearing from M5's `--plan-out`/`apply`
/// onward) and by `undo`'s "did this file move since?" check (M1).
pub struct Ident { pub dev: u64, pub ino: u64, pub nlink: u64, pub mtime: std::time::SystemTime }

pub enum Reason { ReducesToEmpty, ReducesToDotOrDotDot, NotConverged, Opaque }

pub enum Conflict { /* smallest-free-N exhausted within the 998-probe bound, §5.3 */ Unresolvable }

pub enum Resolution {
    Rename,
    Unchanged,
    Skipped(Reason),
    Conflict(Conflict),
}

pub struct PlanItem {
    pub dir: PathBuf,
    pub from: OsString,
    pub to: OsString,
    pub kind: EntryKind,
    pub ident: Ident,
    pub depth: u32,
    pub resolution: Resolution,
}

pub struct Plan { pub items: Vec<PlanItem> }

pub enum OnCollision { Number, Skip, Fail }

/// No I/O. Takes the frozen walk snapshot and produces a `Plan`. Implements:
/// - Layer 1 (intra-batch): map keyed by `(dir, nfc_fold(to))`; >1 source per
///   key is a conflict, resolved per `OnCollision`.
/// - Layer 2 (pre-existing destination): checked against the snapshot's own
///   entries (a fresh `symlink_metadata` re-check at apply time is
///   `fsops.rs`'s job, not this function's -- `plan` has no I/O).
/// - Renumbering: N = 2..999 against existing-plus-already-allocated names;
///   `Conflict::Unresolvable` if none fits (§5.3's stated, not assumed, bound).
/// - The cycle assertion: refuses the entire batch (returns `Err`, not a
///   `Plan`) if any `Rename` item's `to` equals another `Rename` item's
///   `from` in the same directory -- an internal-consistency bug, not a
///   user-facing conflict (§5.3's proof: this should be provably unreachable
///   given Idempotence, so its existence is a bug report against `transform`).
pub fn plan(
    entries: &[(PathBuf, OsString, EntryKind, Ident, u32)],
    outcomes: &[TransformResult],
    on_collision: OnCollision,
) -> Result<Plan, PlanError>;

pub enum PlanError { InternalInconsistency(String) }
```

### 3.3 `crates/detoxrs` — the binary

```
crates/detoxrs/src/
  main.rs
  cli.rs        # clap derive: paths, -r, -x, --on-collision, `undo` subcommand
  walk.rs       # snapshot walk; VCS/dotfile skip; symlink non-follow
  fsops.rs      # RenameOps trait + rustix impl + fallback; no cfg-split needed (§5.4)
  journal.rs    # JSONL write, read, replay
  report.rs     # human preview; exit codes
```

`cli.rs`:

```rust
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "detoxrs")]
pub struct Cli {
    pub paths: Vec<PathBuf>,
    #[arg(short = 'r', long = "recursive")]
    pub recursive: bool,
    #[arg(short = 'x', long = "exec")]
    pub exec: bool,
    #[arg(long = "on-collision", default_value = "number")]
    pub on_collision: OnCollisionArg, // number | skip | fail
    #[command(subcommand)]
    pub command: Option<Command>,
}

pub enum Command {
    Undo { batch_id: String },
}
```

`walk.rs`:

```rust
use detoxrs_core::plan::{EntryKind, Ident};
use std::ffi::OsString;
use std::path::PathBuf;

pub struct Entry {
    pub dir: PathBuf,
    pub name: OsString,
    pub kind: EntryKind,
    pub ident: Ident,
    pub depth: u32,
}

/// Freezes the entry list before any rename happens (§5.1's non-negotiable
/// ordering). Non-recursive: only the named path's own basename. Recursive
/// (`-r`): full subtree, `.git`/`.hg`/`.svn` always skipped, dotfiles skipped
/// unless named explicitly, symlinked directories never descended into and
/// never will be (§5.6 -- this is not a flag in M1 or ever). An unreadable
/// directory is reported and the walk continues (matches detox, doc 13
/// §4.4); `EMFILE`/`ENFILE` aborts the whole run before any rename (§5.8).
pub fn snapshot(paths: &[PathBuf], recursive: bool) -> Result<Vec<Entry>, WalkError>;
```

`fsops.rs`:

```rust
use std::ffi::OsStr;
use std::path::Path;

pub enum RenameErr {
    AlreadyExists,
    PermissionDenied,
    ReadOnlyFilesystem,
    NoSpace,
    NameTooLong,
    NotFound,
    Unsupported,
}

pub trait RenameOps {
    /// Fails with `AlreadyExists` rather than clobbering. Never falls back to
    /// a clobbering call. The only rename entry point (§5.4) -- there is no
    /// separate `rename_case_only`; that method was designed, measured, and
    /// deleted upstream in the proposal (§5.4's "measurably false" section)
    /// and M1 does not resurrect it.
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}

/// `rustix::fs::renameat_with(dir_fd, from, dir_fd, to, RenameFlags::NOREPLACE)`.
/// One call, no `#[cfg(...)]` split needed for Linux vs macOS (§5.4): `rustix`
/// maps `NOREPLACE` to `renameat2(RENAME_NOREPLACE)` under
/// `#[cfg(linux_kernel)]` and to `renameatx_np(RENAME_EXCL)` under
/// `#[cfg(apple)]`. `#![forbid(unsafe_code)]` holds throughout this module.
pub struct PlatformRenameOps;

impl RenameOps for PlatformRenameOps {
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        // 1. Call renameat_with(NOREPLACE).
        // 2. On EEXIST where `symlink_metadata(to)` reports the same (dev, ino)
        //    as `from`: fall back to plain rename(2) for this one item, warn
        //    once naming the mount (observed-error fallback, §5.4 -- not
        //    predicted, and this is spikes 13/14's open question in the wild).
        // 3. On EINVAL/ENOSYS/EOPNOTSUPP: demote this mount to the
        //    check-then-rename fallback (symlink_metadata(dest) then rename),
        //    report `"atomicity": "check-then-rename"`, warn once per mount.
        todo!()
    }
}
```

`journal.rs`:

```rust
use std::path::PathBuf;

/// One file per batch: `$XDG_STATE_HOME/detoxrs/journal/<UTC-ts>-<id>.jsonl`
/// (`$HOME/.local/state/...` fallback). Hand-built JSON per line via
/// `serde_json::Value`/`serde_json::to_writer`, not a derived struct, because
/// the record shapes (`intent`/`done`/`failed`) are three small fixed shapes
/// -- deriving `Serialize` for three tiny enums is not simpler than
/// constructing three `Value::Object`s directly, and skipping `serde`'s
/// derive machinery here keeps `serde` itself out of M1's direct-dependency
/// count until config (M4) actually needs it.
pub fn journal_path(batch_id: &str) -> PathBuf;

/// Writes an `intent` record and fsyncs **before** the rename happens. If
/// this fails, the rename does not happen either (§5.8: an unjournaled
/// rename is the one thing `undo` cannot reverse).
pub fn write_intent(/* ... */) -> std::io::Result<()>;

pub fn write_outcome(/* done | failed */) -> std::io::Result<()>;

/// Replays a batch's journal in reverse. For each `done` item, verifies the
/// current `to` name still resolves to the recorded `(dev, ino)`; refuses
/// (does not force) that single item if it does not, and continues with the
/// rest. Runs through the same `RenameOps`/collision path as a forward run.
pub fn undo(batch_id: &str, ops: &dyn RenameOps) -> Result<UndoReport, UndoError>;
```

`report.rs` (M1: minimal — no `--json`, no color, no verbosity levels; those are M2):

```rust
use detoxrs_core::plan::Plan;

/// Human preview: `from -> to` lines, a note per `Skipped`/`Conflict` item,
/// a one-line summary. Printed unconditionally; `-x` prints the same report
/// after applying, annotated with per-item outcome.
pub fn print_preview(plan: &Plan);

/// 0 = nothing to do or everything that was attempted succeeded; 1 = any
/// per-item failure occurred, or the batch aborted on EROFS/ENOSPC/EMFILE.
#[must_use]
pub fn exit_code(plan: &Plan) -> i32;
```

`main.rs` wires `cli::Cli::parse()` → `walk::snapshot` → `detoxrs_core::plan::plan` → (if `-x`)
`fsops`+`journal`, else `report::print_preview` and exit. The `SIGINT`/`SIGTERM` handler (§5.8) is
installed once, before the apply loop, as a plain `AtomicBool` flag checked between items — no
signal-handling crate; `std::os::unix`'s raw signal primitives plus a static flag are enough for a
"stop between items" handler and pulling a dependency for it would fail P7's falsifier outright.

### 3.4 What M1 defers, and the retrofit cost

| Deferred                                        | To    | Retrofit cost                                                                                                                                                                                                               |
| ----------------------------------------------- | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `safe_map`'s separator/keep classification      | M2    | Low. `classes.rs` gains two more classifier functions; `pipeline.rs` gains one stage between invisible-strip and collapse. Nothing in `plan.rs`/`fsops.rs`/`journal.rs` changes.                                            |
| `collapse`, `truncate`                          | M2    | Low-medium. `truncate` needs `unicode-segmentation` (new dependency) and interacts with `finalize`'s fixed-point loop, which M1 already built for stage 10 alone — M2 widens the loop's stage set, it does not redesign it. |
| NFC as _output_, not just comparison key        | M2    | Trivial (one line, dependency already present from M1's comparison fold).                                                                                                                                                   |
| `url_decode`, `--ascii`, `--case`, config/rules | M4    | Low. Each is an additional pure stage or a CLI/config-layer concern; none touches the safety envelope.                                                                                                                      |
| `--target`, plan files, `--stdin`               | M5    | Low-medium. `--plan-out`/`apply <plan>` reuses `plan.rs` and `journal.rs` verbatim; the stale-plan recheck is the one new piece, and it reuses `Ident` from M1.                                                             |
| `--json`, verbosity, color                      | M2/M6 | Trivial. `report.rs` grows output formats; nothing upstream of it changes.                                                                                                                                                  |

No entry in this table touches `walk.rs`'s snapshot-before-rename invariant, `plan.rs`'s collision
engine, `fsops.rs`'s no-clobber call, or `journal.rs`'s crash protocol. That is the property this
plan is optimizing for: everything expensive to get wrong later is already built and tested by the
time anything easy to get wrong later is even started.

### 3.5 Exit criteria (tests that must pass)

- **Property tests, §8.1 subset applicable to M1's thin pipeline:** Totality, Idempotence (over
  M1's stage set), Decode-is-total, Non-empty, Dotfile preservation. Safety closure is checked only
  for the classes M1 actually removes (delete-class, leading `-`, trailing dot/space) — asserting it
  over separator/keep classes that do not exist yet would be vacuously true and worth nothing, so
  the property's assertion is scoped explicitly to M1's stage set in the test file, with a comment
  pointing at M2 where it widens.
- **Property tests, §8.2, in full:** No collision, No pre-existing clobber, Order safety, No sibling
  chains, Bounded renumbering, Determinism. These depend only on `plan.rs` and the walk snapshot
  shape, not on which transform stages exist, which is why M1 can make them fully green rather than
  partially green.
- **Undo round-trip** (§8.2): against an in-memory filesystem model for the property test, and
  against a real temp directory for an `assert_cmd`-driven integration test.
- **Filesystem matrix subset of §8.4** that M1's scope actually exercises: case-only rename
  (macOS both APFS variants, Linux ext4/tmpfs), `RENAME_NOREPLACE` unsupported → fallback → warns
  → never clobbers, rename-during-walk (5000-entry tree, every entry visited exactly once),
  crash-mid-batch (kill after N renames; replay identifies the interrupted item; undo restores the
  completed prefix). Length-limit and NFD→NFC-specific rows move to M2 with `truncate`/output-NFC.
- **Spikes 2, 13, 14 run and recorded** (see §5 below) as part of M1's own exit criteria, not
  deferred, because they gate exactly the rename path M1 ships.
- `just gate` green throughout: `fmt-check`, `clippy -D warnings`, `test`, `msrv`, `dep-budget`
  (≤ 11, and M1 alone uses 4 — see §4).

## 4. Test plan per milestone, mapped to §8

| Milestone | §8.1 (transform properties)                                                                                      | §8.2 (plan properties)                                            | §8.3 (snapshot)                                                               | §8.4 (filesystem matrix)                                                                 | §8.5 (fuzz)                                              |
| --------- | ---------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| M1        | Totality, Idempotence, Decode-total, Non-empty, Dotfile preservation; Safety closure scoped to M1's stage set    | **All, fully green** — this is the milestone's actual deliverable | none yet                                                                      | subset: case-only rename, unsupported-flag fallback, rename-during-walk, crash-mid-batch | none yet                                                 |
| M2        | Full §8.1 table green, including Length bound, No grapheme splitting, Stage independence                         | re-run, unaffected (plan.rs did not change)                       | `--help` (as it stands at M2), `-vv` per-stage trace for the canonical corpus | length-limit probe rows, NFD→NFC rows                                                    | target stood up, not yet in CI                           |
| M3        | unaffected                                                                                                       | unaffected                                                        | none new                                                                      | spike 5/15 hardlink and network-FS cases added as `assert_cmd` cases once measured       | unaffected                                               |
| M4        | Stage independence re-checked with rules/case/ascii toggled                                                      | unaffected                                                        | `--print-config`, config-driven `-vv` traces                                  | none new                                                                                 | unaffected                                               |
| M5        | unaffected by `--target unix`; `--target windows/portable` gains its own Safety-closure/Stage-independence cases | Order safety / stale-plan recheck extended to `apply <plan>`      | `--json` shape, plan-file shape                                               | Windows-tier compile+unit only (best-effort, no filesystem claim)                        | unaffected                                               |
| M6        | full table, every stage combination fuzzed                                                                       | full table                                                        | full corpus (§8.3's minimum list)                                             | full matrix, tier-1 green on every commit                                                | running in CI, iteration-count instrumented for spike 12 |

**Non-negotiable gates, stated once rather than repeated per row:** Totality, Safety closure (over
whatever stage set currently exists), Idempotence, No collision, No pre-existing clobber, Order
safety, Undo round-trip. These seven cannot lag behind the milestone that introduces their subject
matter — a milestone that adds a stage without extending Safety closure to that stage's outputs is
not done. **Properties allowed to lag:** Length bound and No grapheme splitting (meaningless until
`truncate` exists, M2), Stage independence (meaningless with one undifferentiated stage set, gets
real teeth once there are enough stages to disable one without disabling its neighbors, M2), Bounded
renumbering (mechanically present from M1 but only exercised by real collisions once names get more
varied in M2+), Determinism (true from M1, re-asserted rather than re-earned at each milestone).

## 5. Dependency introduction schedule

Running total against the ≤ 11 direct-dependency budget (§7.2). "Cumulative" is the count after each
milestone; `just dep-budget` enforces the ceiling from M1 onward, not just at the end.

| Milestone | New direct deps                                          | Why here, not earlier or later                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | Cumulative |
| --------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- |
| M1        | `clap`, `rustix` (feature `fs`), `walkdir`, `serde_json` | `clap`: even four flags plus a subcommand is not worth hand-rolling and its help text is a snapshot contract from day one (M2). `rustix`: the entire no-clobber rename story is this crate; it is the reason M1 exists. `walkdir`: recursion (`-r`) is in scope from M1 because deepest-first apply order has nothing to protect without a subtree to walk, and hand-rolling a correct, symlink-safe recursive walk is exactly the kind of code doc 03's own portability lessons warn is easy to get subtly wrong. `serde_json`: the journal is the safety net; hand-rolling JSON escaping for arbitrary `OsStr`-derived path bytes in the one artifact `undo` depends on is a correctness risk in the safety-critical path, which is the opposite of what P7's "50 lines of our own code will do" test is for — the 50 lines are exactly where a byte-escaping bug would hide. | 4          |
| M2        | `unicode-normalization`, `unicode-segmentation`          | `unicode-normalization`: M1 already needs NFC folding for comparison (§6.2), so this dependency's _arrival_ is at M1 in spirit; it is listed at M2 because M1's `Cargo.toml` can defer it by folding with a hand-rolled comparison shim only if that shim is provably correct, and it is not worth writing one just to delay a dependency the design calls for regardless — **so this row is a correction to keep honest: M1 actually pulls `unicode-normalization`, making M1's true cumulative count 5, not 4.** `unicode-segmentation`: mandatory for grapheme-safe `truncate`, no substitute (§3.10, and `sanitize-filename`'s codepoint-boundary bug is the documented reason not to reach for a lighter alternative).                                                                                                                                                     | 6          |
| M3        | none                                                     | Spike closure is measurement against the fallback path M1 already built; it needs zero new production dependencies. Dev-only `assert_cmd` cases exercise mounts the CI matrix (M6) sets up.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | 6          |
| M4        | `serde`, `toml`, `regex`                                 | Config parsing needs a format and a merge story; `toml` needs `serde`'s derive to be worth using at all (hand-rolling TOML deserialization is exactly the "should this exist" question the ladder answers no to). `regex`: `[[rule]] regex = true` and `--exclude` globs-as-regex both need it; deferred to M4 because nothing before this milestone has a regex-shaped input.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  | 9          |
| M5        | `deunicode`                                              | `--ascii` transliteration tables; deferred past M4 deliberately even though M4 is "the config milestone," because `--ascii` is opt-in taste (P4) and has zero interaction with config _parsing_ — it is a pipeline stage keyed by a flag, and grouping it with `--target`/portability in M5 keeps M4 scoped to "how do settings reach `Policy`," not "every remaining opt-in flag."                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | 10         |
| M6        | `terminal_size` (maybe)                                  | Proposal §7.2 flags this as the 11th line "until deleted." Resolved at M6, not earlier, because column-alignment cosmetics for the preview are the least safety-relevant thing in the whole budget and deserve to be decided last, against a real preview output, not speculatively. If a fixed two-column layout suffices (likely, given `report.rs`'s simple `from -> to` shape), this row is **struck** and the final count is 10, not 11.                                                                                                                                                                                                                                                                                                                                                                                                                                   | 10 or 11   |

Dev-only dependencies (`insta`, `trycmd`, `assert_cmd`, `proptest`, `criterion`, `clap_complete`,
`clap_mangen`) are outside this budget per proposal §7.2 and arrive when their milestone needs them:
`proptest` and `assert_cmd` at M1 (the property/integration tests are M1's deliverable), `insta` at
M2 (first snapshot corpus), `criterion` at M6 (huge-tree benchmark), `clap_complete`/`clap_mangen`
at M5.

**Correction folded into the table above, stated once more because it matters for anyone auditing
the count:** M1's honest dependency list is `clap`, `rustix`, `walkdir`, `serde_json`,
`unicode-normalization` — five, not four — because comparison-NFC is a §6.2 correctness requirement
of the collision engine, which is M1's own deliverable, not a M2 nice-to-have. The table above keeps
`unicode-normalization` listed under "M2" only to match where the _visible output_ rewrite lands;
`just dep-budget`'s number will read 5 as soon as M1's `Cargo.toml` is committed, and this document
says so rather than letting the schedule and the enforced number disagree.

## 6. Spike handling

| Spike                                                   | Where it sits                                                                                                                                                                                                                                                                                                                                                                                                      | Blocks / parallel                                                                                                                     | Assumption in the meantime                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Name availability                                    | Before any milestone — largely pre-closed. `docs/owner-decisions.md` already ran the crates.io check (`detoxrs` available, `dtx` binary-collision risk flagged). What remains — GitHub org, Debian namespace, USPTO/trademark search, `dtx` binary-name clearance on Homebrew/coreutils — is a non-engineering task with no code dependency, so it runs **before M1's first public push**, not inside a milestone. | Blocks any public commit (per the document's own gating table)                                                                        | Working name `detoxrs` stands; do not publish to crates.io until the trademark search completes.                                                                                                                                                                                                                                 |
| 2. `renameat2(NOREPLACE)` matrix                        | Inside M1, as an exit criterion (§3.5), immediately after `fsops.rs`'s skeleton exists — it needs the actual call to test against, not a mock.                                                                                                                                                                                                                                                                     | Blocks M1's _release_, not M1's code-complete state; runs concurrently with the rest of M1's implementation once `fsops.rs` compiles. | Runtime demotion on `EINVAL`/`ENOSYS`/`EOPNOTSUPP` per mount, as already designed (§5.4) — this is the fallback path regardless of what the matrix shows; the matrix tells us how often it fires.                                                                                                                                |
| 13. Incapable macOS volume return value                 | Same slot as spike 2 — same call, same owner (macOS side), run the moment an old-HFS+/exFAT/SMB image can be mounted via `hdiutil`.                                                                                                                                                                                                                                                                                | Blocks M1 release.                                                                                                                    | Assumed to fail with `EINVAL`/`ENOTSUP`/`ENOSYS`, not silently drop the flag and clobber. If the matrix shows silent clobbering on any format, the `getattrlist` probe §5.4 deliberately dropped comes back **for that format only**, as a scoped fix, not a redesign — flagged as this plan's single largest re-plan risk (§7). |
| 14. Linux case-only rename over case-insensitive mounts | Same slot, Linux side: ext4 (casefold on/off), vfat, exfat, tmpfs.                                                                                                                                                                                                                                                                                                                                                 | Blocks M1 release.                                                                                                                    | Assumed to behave like APFS (`Ok(())` on same-inode respell); the observed-`EEXIST` fallback already in `fsops.rs` catches it if not, per M1's own design — so even a bad matrix result is absorbed without a code change, only a "how often does the fallback fire" answer.                                                     |
| 5. Case-only rename on network filesystems              | M3, parallel with M2. Needs `fsops.rs` (M1) and access to an SMB/NFS mount, which is an infrastructure setup task independent of pipeline work.                                                                                                                                                                                                                                                                    | Gates v1.0 ("everything else" tier), not any specific milestone's code.                                                               | Direct case-only rename assumed to work per doc 06's APFS/reasoning; if it fails on any tested filesystem, the temp-name dance returns **only for that filesystem**, triggered by the observed error, never as a default.                                                                                                        |
| 15. `nlink > 1` respell safety                          | M3, parallel with M2. Needs two hardlinks and the collision engine (M1), nothing from M2.                                                                                                                                                                                                                                                                                                                          | Gates v1.0.                                                                                                                           | Respells proceed without a `nlink` guard, per §5.4/§5.6's argument that the collision engine keys on `(dir, name)` and reasons correctly about two entries sharing an inode. If the measurement disagrees, the fix is a new `Skipped` reason gated on `nlink > 1`, a small, additive change to `plan.rs` — not a redesign.       |
| 7. NFC-by-default                                       | Starts at M2 (once NFC-as-output exists) as a background research task: run the planner in report-only mode over large real trees and count NFD→NFC collisions. Never blocks a milestone; it can change M2's default before M6, or not, based on what it finds.                                                                                                                                                    | Gates v1.0.                                                                                                                           | NFC rewrite stays default-on per the proposal's current reasoning.                                                                                                                                                                                                                                                               |
| 8. Auto-number default                                  | Ongoing from M1 onward — the owner already decided (`docs/owner-decisions.md`), but the plan keeps counting real conflicts as dogfooding accumulates, per the document's own "closes with user feedback on the v0.1 preview" criterion.                                                                                                                                                                            | Informational; does not gate anything in this plan.                                                                                   | `number` stays the default, as decided.                                                                                                                                                                                                                                                                                          |
| 11. `Unrepresentable` frequency                         | M3, once M2's fuller pipeline exists (more stages producing more `Unrepresentable` cases is when the count becomes meaningful) — run over Downloads/media-library/archive-extraction corpora.                                                                                                                                                                                                                      | Gates v1.0.                                                                                                                           | Skipping stays the only behavior; no placeholder flag is added ahead of the count, per the proposal's own instruction.                                                                                                                                                                                                           |
| 12. Fixed-point bound of 3                              | M6, instrumented into the fuzz target once it exists, run under `--target windows` with tight `--max-len` (the richest interaction, per the proposal). Cheap enough to also spot-check manually in M2 once `finalize` has more than one interacting stage.                                                                                                                                                         | Informational; §3.14 already makes non-convergence safe regardless of the answer.                                                     | Bound stays 3 until measured.                                                                                                                                                                                                                                                                                                    |
| 3, 4. Windows 11 reserved names; NTFS/exFAT limits      | **Not scheduled to close in this plan.** No Windows machine, no NTFS/exFAT volume exists (`docs/owner-decisions.md`). M5/M6 build the conservative, documented-as-assumption behavior and stop there.                                                                                                                                                                                                              | Blocks Windows tier-1 promotion only; does not block v1.0 as scoped by this plan (Windows stays best-effort).                         | Conservative pre-Windows-11 reserved-name rule; both byte and UTF-16-unit limits enforced simultaneously. **No milestone in this plan claims verified Windows filesystem behavior**, per the owner's explicit rule.                                                                                                              |
| 6. CP1252 repair measurement                            | Not scheduled. Moot per owner decision; retained only as a specification for a possible post-1.0 `--repair-encoding`, which is out of scope for every milestone in this plan.                                                                                                                                                                                                                                      | None.                                                                                                                                 | Non-UTF-8 names stay `Opaque`, skipped, reported, forever within this plan's scope.                                                                                                                                                                                                                                              |
| 9. Parallelism benchmark                                | Not scheduled unless a real performance complaint arrives after M6.                                                                                                                                                                                                                                                                                                                                                | None currently.                                                                                                                       | Single-threaded.                                                                                                                                                                                                                                                                                                                 |
| 10. Distro aggregate count                              | Not scheduled; cosmetic, only matters if a README wants the aggregate figure.                                                                                                                                                                                                                                                                                                                                      | None.                                                                                                                                 | Per-distro citations (already primary-confirmed) suffice.                                                                                                                                                                                                                                                                        |

## 7. Risk register

| Risk                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      | What could force a redesign                                                                                                                                                                                                                                                                                                                                                                               | Early detection signal                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spike 13 finds a macOS volume format that **silently clobbers** instead of erroring on an unsupported `NOREPLACE`                                                                                                                                                                                                                                                                                                                                                                                         | The demotion-on-error design (§5.4) assumes every unsupported case is _observable_ as an error. A silent-clobber format would mean `fsops.rs`'s whole error-taxonomy approach has a hole that no amount of `Result`-plumbing catches, because there is no error to plumb.                                                                                                                                 | M1's spike-13 exit criterion itself: run the matrix against an old HFS+/exFAT/SMB image before M1 ships, not after. If it fires, the fix is narrow (bring back a scoped `getattrlist` probe for that format), but the detection has to happen at M1, because M1 is the only point in this plan where the assumption is load-bearing but not yet shipped to a user.                                                                                                                                    |
| The journal's crash-safety protocol (`intent`/fsync/rename/`done`) turns out not to be atomic enough on some real filesystem/OS combination (e.g., `fsync` semantics differ, or a power-loss test shows a `done` record can be lost after the rename it describes succeeded)                                                                                                                                                                                                                              | This is the single property this whole mandate is staked on. If it is wrong, "we can never lose track of a file" is false, and the fix could touch `journal.rs`'s wire format, not just its implementation.                                                                                                                                                                                               | M1's crash-mid-batch test (§3.5) run with `kill -9`, not `kill -TERM`, and ideally with a real power-cut simulation (`libfiu` or a VM snapshot-and-kill) before M1 is called done, not as a nice-to-have added at M6. If the SIGKILL case ever shows a `done` record surviving without its rename, or a rename surviving without any record, the wire format needs a checksum or a two-phase commit marker, which is a breaking change to every journal ever written — cheap now, expensive after M6. |
| `walkdir`'s symlink-safety guarantees turn out to have an edge case (loop, `.`/`..` symlink, or a race between snapshot and apply that lets a directory become a symlink mid-batch) that `detoxrs`'s own "never descend into a symlinked directory" rule does not independently re-verify                                                                                                                                                                                                                 | Doc 05's open spike (#20, symlink loops) was never closed upstream either; trusting a dependency's edge-case handling for a security-relevant property (§5.6, motivated by issue #23's blast-radius incident) without an independent assertion is exactly the kind of "we trusted the library" failure P7 exists to prevent for _dependency choice_, but says nothing about for _dependency correctness_. | The rename-during-walk / symlink-to-`../..`-under-`-r` test in §8.4, run early (M1, not deferred to M6's full matrix) and specifically including a symlink created _between_ snapshot and apply in a race-simulation test, not just a static one.                                                                                                                                                                                                                                                     |
| The collision-engine's cycle-refusal proof (§5.3) rests on `transform` being genuinely idempotent; if a future stage (M2's `truncate`, especially) is accidentally non-idempotent in some corner (e.g., truncation-then-renumbering interacting with `finalize`'s loop in a way the 3-iteration bound does not actually cover), the "cycles are structurally impossible" argument silently stops holding, and the plan-time assertion becomes the only thing standing between a bug and a corrupted batch | This is the risk most likely to surface _after_ M1, when M2 adds the interacting stages the proof was written to anticipate but never tested against. A redesign here would mean either raising the fixed-point bound with a real invariant (spike 12) or, worse, discovering the assertion fires in production and a whole batch refuses to run for a reason a user cannot self-diagnose.                | The plan-time internal-consistency assertion (§3.2's `plan.rs`) is wired to a loud, named error from M1 onward, and M2's exit criteria (§4) require the fuzz target (brought forward from M6 if this signal appears) to run specifically against `--target windows` with tight `--max-len`, per spike 12's own closing experiment, the moment `truncate` lands — not deferred to M6 as originally scheduled.                                                                                          |
| `rustix`'s API for `renameat_with`/`RenameFlags` changes shape in a later release in a way that is not purely additive (unlikely, given it wraps a stable syscall contract, but this whole design's `unsafe`-free story depends on one crate's safe wrapper existing and staying safe)                                                                                                                                                                                                                    | If `rustix` ever regresses this wrapper (removes `#[cfg(apple)]` support, or the wrapper itself needs `unsafe` in a future major version), the entire "no FFI shim, `forbid(unsafe_code)` in both crates" story — the thing that let this project drop ~60 lines of `unsafe` and a probe outright — reverses, and a real `libc`-based shim comes back, with its own `unsafe`-audit budget.                | Pin `rustix` in `Cargo.lock`/`deny.toml` and re-verify the compile-and-run-on-APFS check (already done once per `docs/rust-setup-notes.md`) on every `rustix` major-version bump, not just on first adoption. This is a `cargo update` review discipline, not a milestone.                                                                                                                                                                                                                            |

## 8. What this plan deliberately defers, and the retrofit cost

Beyond the M1-specific table in §3.4:

- **Everything in proposal §10's "Deliberately out of v1.0" table** (`--edit`, interactive prompts,
  confusable/skeleton warnings beyond the cheap M6 subset, full-width folding, content-derived
  names, parallelism, native distro packages, Windows tier-1 promotion). Retrofit cost: as stated in
  the proposal, none of these touch the safety envelope, which is this plan's whole point in
  deferring them.
- **The dependency-count decision on `terminal_size`** (§5, M6 row). Retrofit cost: zero either way —
  adding it later is one dependency and a formatting change; not adding it and discovering a real
  alignment need later is the same cost in reverse.
- **Confusable/skeleton warning beyond M6's "cheap subset"** (a UTS #39-generated table, v1.1 per
  the proposal). Retrofit cost: low. It is a new, self-contained detection-only stage; nothing about
  it interacts with the safety envelope, and it does not rewrite names, so it cannot regress
  Safety closure.
- **Spikes 3 and 4 (Windows).** Retrofit cost: **structurally unknown**, and this is the honest
  answer rather than a comfortable one. Every Windows-facing default in this plan (§6.5's
  conservative reserved-name rule, the simultaneous byte/UTF-16-unit length enforcement) is a
  documented assumption, not a measurement, and this plan does not schedule closing either spike
  because the owner has no Windows machine and no NTFS/exFAT volume. If either assumption turns out
  wrong once hardware exists, the fix is confined to `reserved.rs`/`limits.rs` and does not touch
  `walk`/`plan`/`fsops`/`journal` — but "confined to those files" is itself an untested claim about a
  platform this plan cannot test, and it is named here as exactly that.

## 9. Why this ordering, against "build the visible transform first"

The obvious alternative sequences differently: ship a thin CLI that runs `safe_map`/`collapse`/case
folding over a flat directory with no recursion, no journal, and a naive `std::fs::rename`, because
that gives a demoable "detoxrs cleans my filenames" experience in the first few hundred lines, and
gets fast feedback on the part of the design most likely to draw user complaints (§3.7's contested
`( )`/`[ ]` defaults, §3.6's transliteration stance). That is a legitimate strategy, and it is
plausibly what a fast-feedback-mandated Plan Author would write.

The case against it, under **this** mandate, is not that it is a bad strategy in general — it is that
it inverts which axis of this design is expensive to get wrong. The transform stages are, almost
without exception, pure functions with no shared state and no interaction with anything outside
`detoxrs-core` (§3.1's whole reason for existing): adding `safe_map`'s separator class in M2 cannot
break `plan.rs`'s collision engine, because `plan.rs` only ever sees `transform`'s _output_, never
its internals. The plan-level and journal-level invariants are the opposite: `plan.rs`'s cycle-proof
rests on Idempotence holding across the _whole_ pipeline, present and future (§5.3's proof is
universally quantified over "any future stage," not scoped to today's stages); the journal's
crash-safety protocol has to be right the first time a real rename happens, because a bug in it
is discovered by losing track of a file, not by a failing test — and by definition, the one place a
"just ship it and iterate" strategy cannot recover from its own mistake is the one place recovery
(`undo`) itself lives.

Put differently: a wrong `--ascii` default is a documented flag away from being right. A wrong
journal wire-format decision is a corrupted undo history away from being unfixable for every batch
already run against it. Building the safety envelope first is not caution for its own sake; it is
recognizing that only one of these two kinds of mistake is retrofittable, and sequencing the
non-retrofittable kind first is what "we can never lose a file" actually requires as an engineering
plan rather than a slogan.

**What this costs, stated plainly:** the M1 binary is not very _useful_ — it strips control
characters, invisible characters, and a leading dash, and that is genuinely most of what a first-time
user would call "detox does nothing to my messy filenames." A stakeholder who wants a demoable
cleaner in week one will be unhappy with M1's output. That unhappiness is the price of this mandate,
named rather than hidden: fast feedback on taste-driven defaults (§3.7's contested character-class
calls) is exactly the feedback this plan defers, on the argument that a wrong taste call is cheap to
change and a wrong safety call is not.
