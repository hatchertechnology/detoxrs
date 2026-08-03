# Handoff — state as of 2026-08-03 (M1 complete, self-review applied)

Written at the end of a long orchestration session, so a fresh session can resume without
re-deriving anything. Read `docs/plans/unified-draft-plan.md` first; it is the spec being
implemented. `docs/owner-decisions.md` overrides everything.

## Where the work stands

| Stage                                                     | Status                                 |
| --------------------------------------------------------- | -------------------------------------- |
| Research: detox inputs, options, config, filters, env     | done, validated, stage-3 reviewed      |
| Research: online user feedback + compiled synthesis       | done, stage-3 reviewed                 |
| Rust project setup per the ideal-project-setup guide      | done                                   |
| Stage-3 review sweep of all 22 files under `docs/`        | done (3 reviewers + arbiter each)      |
| Cross-document propagation into the proposal and code     | done                                   |
| Three implementation plans + opus unified draft + my pass | done                                   |
| 20 plan-required proposal amendments                      | applied                                |
| **M1 WP1-3** pure transform core                          | **done, gate green**                   |
| **M1 WP4** collision engine                               | **done, gate green**                   |
| **M1 WP5a** preview-only binary                           | **done, gate green, tool runs**        |
| **M1 WP5b** write path: fsops, apply, journal, undo       | **done, gate green, `kill -9` passes** |
| Implementation review (separate team)                     | **not started — resume here**          |
| Opus adjudication of that review                          | not started                            |

`just gate` is green: fmt, clippy pedantic+nursery at `-D warnings`, tests, MSRV 1.93.0,
`dep-budget` 6/11 (`rustix` joined at WP5b, as planned). Both crates are
`#![forbid(unsafe_code)]`.

M1 is functionally complete:

```
$ cargo run -q -p detoxrs -- -r /some/tree        # preview
$ cargo run -q -p detoxrs -- -x -r /some/tree     # apply, journalled
$ cargo run -q -p detoxrs -- undo --last          # put it back
```

Preview still cannot write, and that is asserted rather than assumed:
`preview_never_writes_anything` compares a recursive census across ten non-`-x` invocations. The
`-x` path is `tests/apply.rs`, which owns the two rows that gate M1.

## What WP5b actually shipped

`crates/detoxrs/src/{fsops.rs,fsops/fallback.rs,apply.rs,journal.rs}`, the `undo` subcommand,
`-x` switched from refusal to execution, and exit code 1 made real. Every non-negotiable from the
previous handoff held; two things came out differently and both are recorded in module docs:

- **`RenameFlags::NOREPLACE` via `rustix` worked exactly as measured.** One safe call, no `#[cfg]`
  split inside `fsops::unix`, no FFI. `rustix` is a `cfg(unix)` target dependency so the Windows
  best-effort tier still compiles, reaching a rename through `fallback::check_then_rename` and
  reporting `"atomicity": "check-then-rename"` rather than claiming atomicity it does not have.
- **No `policy_digest` in the journal header.** There is no hash function in the dependency budget
  and a digest nobody can recompute documents nothing, so the policy's fields are written verbatim.
- **A non-UTF-8 _directory_ path is journalled as `dir_bytes`.** Names cannot need this (an
  undecodable name is `Skipped`), but the directory holding them can be undecodable on Linux, and a
  journal that records an approximation of a path cannot undo.

**One real bug was found by a test, not by review, which keeps the streak intact (seven now).**
`undo --last` originally mixed the pid into the batch id suffix, so two batches created in the same
second sorted by a hash rather than by time — and undoing an undo reverted the _original_ batch.
Caught by `an_undo_can_itself_be_undone`. The suffix is now the subsecond clock scaled to four
fixed-width hex digits, which is monotonic within a second and sorts lexicographically the way it
compares numerically. **If you change the journal filename format, that ordering property is what
you must preserve;** `list()` sorting by name is what `--last` means.

The `kill -9` test (`crash_mid_batch_is_recoverable`) watches the journal rather than the clock:
it spawns a 1000-item batch, kills the child as soon as five `done` records exist, and then asserts
off the on-disk journal that at most one item has an `intent` with no outcome, that `undo --last`
restores every completed rename, and that the tree neither gained nor lost an entry. It asserts it
was _actually_ interrupted rather than passing vacuously if the machine is fast.

## The self-review pass, 2026-08-03

Not a substitute for the separate-team review — the author reading their own code catches the
mechanical defects and misses the design ones. Four real defects, all in WP5b, all now fixed with a
test written first that reproduced each one:

1. **The journal recorded `"dir": "."`.** The plan carries directories as the user spelled them, so
   `undo` only worked from the original working directory and failed confusingly anywhere else. Now
   recorded absolute, via `std::path::absolute`, which is purely lexical and resolves no symlinks —
   that matters, because what gets renamed is a directory entry and resolving the path would record a
   different one. `undo_works_from_a_different_working_directory` has a same-named decoy tree to
   catch a regression that resolves against the wrong root.
2. **An `-x` run with nothing to rename still created a journal.** Not litter: the empty batch
   became the newest one, so `undo --last` stopped meaning "undo what I just did" after any no-op
   `-x`. No renames now means no journal.
3. **`-q` was ignored on the write path.** "Errors only" has to mean the same thing on both sides of
   the `-x` branch.
4. **`undo <BATCH-ID>` joined an unvalidated argument onto a path.** Only ever read, and every rename
   it describes still goes through the identity recheck, so nothing terrible was reachable — but a
   trust boundary is a trust boundary. Ids are now rejected if they contain a separator or `..`.

Two things were examined and deliberately left alone, both recorded where the code is rather than
only here:

- **The fsync is on the journal file, not its directory**, so the guarantee is "survives `kill -9`"
  and not "survives power loss". That is exactly the threat model §5.5 and §8.4 specify and the one
  that is tested. The upgrade is named in `journal.rs`'s module docs, with its cost (`F_FULLFSYNC` on
  Apple, where a plain directory `fsync` promises less than it looks like it does).
- **Rung 1 of the rename fallback treats two hardlinks to one inode as a same-inode respell.** POSIX
  requires `rename` over two names for one file to succeed and perform no other action, so neither
  name is destroyed; the planner will not normally produce such an item anyway. Argued in
  `fsops/fallback.rs`.

## Resume here: implementation review

M1's code is done and self-reviewed; nobody outside the session that wrote it has read it. The
separate-team review and the opus adjudication of that review are the two remaining rows above.
Points worth aiming a reviewer at, because they are where a defect would be expensive rather than
merely wrong: the apply loop's step order in `apply.rs`, the `undo`-reuses-`apply` decision (it buys
undo-of-undo for free, and it means a bug in `attempt` is a bug in both directions), and the batch-id
ordering property that `--last` depends on.

## Closed: `-r` semantics

Ruled by the owner on 2026-08-01 and recorded in `docs/owner-decisions.md`: §5.6, §2.4 and §9.2
win, §2.2's worked example is the wrong one. §2.2 now carries a warning block pointing at the
decision; the example is deliberately left visible rather than rewritten, because it is the passage
a reader is most likely to copy.

## Process notes worth keeping

- **Every design defect so far was found by a property test or by running something, never by
  review of prose.** Seven now, the newest being WP5b's `undo --last` ordering bug above. Six
  before it: the stage-13 empty-name fallback, the undecidable length bound,
  the stage-independence seam, the untested apply-time TOCTOU, the stage-13 NFC re-run set, and the
  collision tie-break that was keyed on the colliding key. Keep the rule that a property which
  cannot pass is reported, never weakened.
- Do not run `git add -A` while background agents are writing; it sweeps their work into unrelated
  commits. Use explicit paths.
- Agents must not edit `docs/research/00-proposal-rust-detox-successor.md` concurrently. Corrections
  are collected and applied in one propagation pass.
- `just fmt-check-file <paths>` for files you touched; never the repo-wide formatter while others
  are working.

## Deliberately unresolved

- Spikes 3 and 4 (Windows reserved names, NTFS/exFAT length limits) **cannot be closed** with
  Linux + macOS hardware. Every Windows-facing default is a documented assumption. Do not write
  documentation implying verified Windows behavior.
- Spike 2 (`renameat2` across Linux filesystems) is closeable and unrun; it gates the v0.1 release
  announcement's claims, not the merge.
- `SECURITY.md` and `CODE_OF_CONDUCT.md` still carry placeholder contacts and response times. The
  owner has confirmed this is a real public tool, so these are real obligations before first
  release. They must not be invented.
- The release workflow cannot run until someone creates the `release` GitHub Environment and its
  `RELEASE_APP_ID` / `RELEASE_APP_PRIVATE_KEY` secrets. No workflow in this repo has ever executed.
- SBOM generation is decided (`cargo-cyclonedx`, CycloneDX JSON) but not wired into
  `release.yml`, and `cargo-cyclonedx` is not installed, so `just sbom` fails. Named as Gap SBOM-1.
