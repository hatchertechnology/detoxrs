# Handoff — state as of 2026-08-01

Written at the end of a long orchestration session, so a fresh session can resume without
re-deriving anything. Read `docs/plans/unified-draft-plan.md` first; it is the spec being
implemented. `docs/owner-decisions.md` overrides everything.

## Where the work stands

| Stage                                                     | Status                            |
| --------------------------------------------------------- | --------------------------------- |
| Research: detox inputs, options, config, filters, env     | done, validated, stage-3 reviewed |
| Research: online user feedback + compiled synthesis       | done, stage-3 reviewed            |
| Rust project setup per the ideal-project-setup guide      | done                              |
| Stage-3 review sweep of all 22 files under `docs/`        | done (3 reviewers + arbiter each) |
| Cross-document propagation into the proposal and code     | done                              |
| Three implementation plans + opus unified draft + my pass | done                              |
| 20 plan-required proposal amendments                      | applied                           |
| **M1 WP1-3** pure transform core                          | **done, gate green**              |
| **M1 WP4** collision engine                               | **done, gate green**              |
| **M1 WP5a** preview-only binary                           | **done, gate green, tool runs**   |
| **M1 WP5b** write path: fsops, apply, journal, undo       | **NOT STARTED — resume here**     |
| Implementation review (separate team)                     | not started                       |
| Opus adjudication of that review                          | not started                       |

`just gate` is green: fmt, clippy pedantic+nursery at `-D warnings`, tests, MSRV 1.93.0,
`dep-budget` 5/11. Both crates are `#![forbid(unsafe_code)]`.

The tool is usable today for previewing:

```
$ cargo run -q -p detoxrs -- -r /some/tree
```

It **cannot write** — that is asserted by `binary_never_writes_anything`, which compares a
recursive census of entries, symlink-ness, sizes and mtimes across ten invocations including `-x`.

## Resume here: WP5b

Scope, per plan §7.1: `crates/detoxrs/src/{fsops,fsops/fallback,apply,journal}.rs` plus the `undo`
subcommand, and switching `-x` from refusal to execution.

Non-negotiables, all established by evidence rather than preference:

- **`rustix::fs::renameat_with` + `RenameFlags::NOREPLACE`** is the only rename entry point, on
  both Linux and macOS. No `unsafe`, no FFI shim, no `libc`. Verified twice by compiling and
  running a `forbid(unsafe_code)` program on APFS: `EEXIST` on an occupied destination, `Ok` on a
  free one, and `Ok` on a same-inode case-only respell.
- **There is no `rename_case_only` path.** It was deleted once measurement falsified its only
  stated reason (see the proposal's §5.4 and doc 06 row 4f).
- Journal protocol: write `intent`, fsync, rename, write `done` or `failed`. Append-only JSONL in
  `$XDG_STATE_HOME/detoxrs/journal/`.
- **Exit criterion that gates M1: a `kill -9` mid-batch test**, not `SIGTERM`. Journal replay must
  identify the exact interrupted item and `undo` must restore the completed prefix. This is risk 1
  in the plan and the one property the whole design is staked on.
- The new **TOCTOU collision during apply** matrix row (plan §5.3): create a file at a planned
  destination after the snapshot and before apply; assert a fresh conflict, the pre-existing file
  byte-identical, no panic, exit 1.
- Exit code 1 becomes real here (per-item failures). Until now only 0 and 2 exist.
- v1.0 ships **no signal handler** — decided, with reasons, in the proposal's §5.8.

## Open question needing an owner ruling

**`-r` semantics, and a contradiction inside the proposal.** WP5a chose: without `-r`, a directory
argument has only its own basename cleaned; nothing inside it is touched. Upstream `detox` instead
always processes a named directory's immediate children, and `-r` only controls deeper descent.

The proposal states WP5a's choice in §5.6, §2.4's `--help` block and §9.2 — but §2.2's worked
example shows `detoxrs ~/Downloads` listing that directory's contents, which implies upstream's
behavior. Three sections against one example. WP5a followed the three and recorded the discrepancy
in `walk.rs`'s module docs rather than resolving it silently.

This needs a decision, because it is the single most visible behavioral difference from the tool
being replaced, and because §2.2 is the part of the proposal a reader is most likely to copy.

## Process notes worth keeping

- **Every design defect so far was found by a property test or by running something, never by
  review of prose.** Six so far: the stage-13 empty-name fallback, the undecidable length bound,
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
