# Handoff — state as of 2026-08-01 (WP5b complete)

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

## Resume here: implementation review

M1's code is done; nobody outside the session that wrote it has read it. The review pass and the
opus adjudication of that review are the two remaining rows in the table above.

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
