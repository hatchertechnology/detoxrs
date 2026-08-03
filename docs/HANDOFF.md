# Handoff — state as of 2026-08-03 (M1 complete, six-reviewer adversarial pass applied)

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
Caught by `an_undo_can_itself_be_undone`. That subsecond-hex suffix is gone; a later fix (below)
replaced it with a directory-read sequence number, so batch ids are now `<seq:06>-<UTC-stamp>`
(e.g. `000001-20260803T184819Z`), monotonic by construction rather than by clock resolution.
**If you change the journal filename format, that ordering property is what you must preserve;**
`list()` sorting by name is what `--last` means.

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

## The adversarial review, 2026-08-03

Six reviewers, each given one attack surface and a standing order to assume the code was broken and
prove it. Read-only on the tree; anything needing a code change went in a copy. They found **seven
real defects**, four of them behavioural. Every fix below landed with a test written first that
reproduced the defect.

**The worst one, and the most interesting: the directory was not pinned.** `apply` checked identity
with `symlink_metadata(dir.join(from))` and then `fsops` re-resolved `dir` _by path_ down inside the
rename. Rename the directory in that gap and the rename lands on a file that was never checked,
while the journal records a **false success** against the original inode. Reproduced with
content-tagged files. Fixed properly rather than narrowly: `RenameOps` now hands out a `Dir` — an
open descriptor on Unix — and the identity check, the occupancy check and the rename all go through
that one handle. A path resolved twice is two directories; a descriptor resolved once is one. Guarded
by `the_rename_follows_the_pinned_directory_not_the_path`, which renames the pinned directory away
mid-test and asserts the rename follows the inode rather than the path.

**The flagship crash test did not test what it is named for.** Moving `journal.intent` to _after_ the
rename — the exact inversion the whole design forbids — passed `crash_mid_batch_is_recoverable` 6
runs out of 6. The cause is not timing: it is that every assertion in that test compares the journal
against _itself_. With the rename first, each intent still has a `done` after it, `unresolved` is
still 0, and the file renamed in the open window is simply absent from the journal, where nothing was
looking. Measured: the window is hit in roughly a quarter of killed runs, so the state was reachable
all along. Two fixes: a deterministic
`the_intent_is_recorded_before_the_rename_not_after` that threads one event log through the journal
double and the rename and asserts the interleaving (fails every time, in microseconds), and a new
property in the crash test that compares the journal against the **filesystem** — every rename that
happened must be journalled. Keep both and know what each is worth: the first is the gate, the second
is a probabilistic end-to-end detector.

**`replay` trusted the journal completely.** Outcome records carry the inode they close and it was
never compared, so a `done` consumed whatever intent happened to be pending; a journal with two
intents made one item vanish from both the undo set and the interrupted report, silently. A malformed
intent did the same and left `undo` printing "nothing to undo" with exit 0. Now every mismatch,
orphan and unparseable line lands in `Replay::anomalies`, is printed with its line number, and forces
a non-zero exit. For the one file the safety story rests on, silence was the wrong failure mode.

**`undo --last` depended on the wall clock.** Batches were named `<UTC-stamp>-<subsecond-hex>` and
`--last` took the lexical maximum, so a backward NTP step — routine on a laptop that sleeps — makes
the newer batch sort first and `undo --last` revert the wrong one. Ids are now
`<seq>-<UTC-stamp>`, with the sequence read from the directory, so ordering holds by construction
whatever the clock does. This **deleted** the nanosecond-derived suffix, which a mutation run had
separately shown was untested. `last_means_most_recently_created` pins it.

**`undo --last` could revert a batch that was still being written.** Reproduced: a running `-x` gets
its completed prefix reverted, the forward run carries on past those items, and the tree is left
permanently half-cleaned with exit 0. A batch now writes a terminal `end` record, and undoing a batch
without one warns explicitly and exits 1 — which still permits the crash-recovery path, because a
crashed batch has no `end` either and must remain undoable. Note for the spec: §5.8 analyses two
concurrent `-x` runs but never names `-x` racing `undo`; that gap is real and worth an amendment.

Two documentation defects: §5.5 says undo runs through the collision engine and it does not (declared
now in `apply.rs`, with the argument for why refusing beats renumbering when restoring), and
`Replay::items`'s comment had the undo ordering backwards.

### What the reviewers could not break, which is the other half of the result

Reported because a review that only lists findings is not auditable. The fsync-before-rename ordering
survived a real instrumented `kill -9` on the first item and mid-batch. 150 iterations of two
concurrent `-x` runs over one tree — ~6000 attempted renames — lost, duplicated and clobbered nothing,
producing exactly the "confusing report" §5.8 predicts. 30 simultaneous batches produced 30 distinct
journals. Two simultaneous `undo`s of one batch split the items cleanly. Directory-onto-directory and
file-onto-directory collisions renumber rather than merge. `ident_at` does not follow symlinks, and it
agrees with `walk`'s `std` reading of the same two numbers — now asserted, since the two come from
different APIs over fields that are signed on some targets. Of nine planted mutations, six were caught
directly, including a clobbering rename, a neutered `aborts_batch`, and skipped batch-id validation.

### Reviewer quality, recorded because it matters for the next pass

Two of the six returned confident all-clear verdicts that did not survive spot-checking. One cited a
hardlink test in a file containing no hardlink test, and marked the collision-engine requirement
CONFORMS while citing the very function that proves it does not. The other reported "188 cases, 0
issues" and credited `serde_json` for escaping that the hand-rolled `report::escape` actually does.
Both all-clears were discarded; the one real finding recovered from them came from checking a claim
that was inverted. **Weight a reviewer by its reproductions, not its verdict** — the four that
instrumented code, hand-authored hostile inputs, or planted mutations found everything of value.

### Still open

- The fsync no-op survives the whole suite, and that is honest rather than fixable: `kill -9` does not
  discard page cache, so only real power loss would tell the difference. The ceiling and its upgrade
  path are named in `journal.rs`.
- The demoted `check_then_rename` TOCTOU remains theoretical here: no available filesystem refuses
  `RENAME_NOREPLACE`/`RENAME_EXCL`, so that rung cannot be exercised on this hardware. Unchanged from
  the spec's own admission.
- `volume_case`'s probe has a benign race (a wrong case guess can only misreport a collision, never
  cause a wrong rename, since the apply-time recheck and the kernel act independently of it).

## Resume here: opus adjudication

The review is done and its findings are applied. What has not happened is an independent read of
_these_ fixes — the dirfd refactor changed the `RenameOps` trait shape and touched every call site,
and it was written by the same author the review was auditing.

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
