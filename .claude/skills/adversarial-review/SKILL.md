---
name: adversarial-review
description: Use when asked for an adversarial, critical, or multi-agent review of this crate — "review the whole app", "assume it's broken", "team review", "have N agents review this" — or before tagging a release. Also use when re-checking a prior review's verdicts, or when a green test suite is being treated as evidence of correctness.
---

# Adversarial review

A review team finds defects a single reader misses, but only if two things
hold: every reviewer works in a **tree nobody else can mutate**, and a **PASS
costs more than a FAIL**. Without the first, verdicts are noise. Without the
second, half the team reports "no problems found" as "no problems exist".

Reviews are **read-only**. Reviewers report; they do not fix. See the repo
rule in `~/.claude/CLAUDE.md` — finding a defect is not authorization to
change code.

## Isolation is non-negotiable

Any reviewer that runs `cargo test`, mutates code to test a guard, or builds
the binary gets its own worktree **and** its own `CARGO_TARGET_DIR`. A shared
target dir means one reviewer reads a binary built from another's source.

```sh
REV=opus-1                                     # reviewer id
BASE="${TMPDIR:-/tmp}/detoxrs-review"
WT="$BASE/$REV"
git worktree add --detach "$WT" HEAD
export CARGO_TARGET_DIR="$BASE/target-$REV"    # per reviewer, never shared
cd "$WT" && cargo test --workspace
```

The target dir sits **beside** the worktree, not inside it: `.gitignore`
covers `target`, not an arbitrary name, so a build dir inside the worktree
shows up as untracked and the clean-tree check below can never pass.

Teardown, after the report is written:

```sh
git status --short                     # must be clean in the worktree
cd - && git worktree remove --force "$WT" && rm -rf "$CARGO_TARGET_DIR"
git worktree list                      # only the main tree should remain
```

The **main tree is off limits** to reviewers. Only the coordinator writes
there, and only report files under `docs/reviews/`.

## What every reviewer gets

- **Stance:** assume the code is broken; prove it. A PASS requires stated
  evidence — a command that ran, a mutation the suite caught, a traced
  invariant. Absence of a found bug is `NOT ESTABLISHED`, never PASS.
- **Scope:** the whole app, plus one distinct emphasis (transform
  correctness, fs safety/data loss, journal round-trip, CLI contract, test
  adequacy via mutation, security/hostile input, plan-apply state).
- **Hands-on:** build it, run it, create hostile trees in temp dirs outside
  the repo. `-x` never runs against the repo or `$HOME`. The journal is
  XDG-based — isolate it with `XDG_STATE_HOME`.
- **Mutations:** one at a time, reverted immediately, `git status` clean
  before finishing. An unreverted mutation is a failed review.
- **Output:** `docs/reviews/<topic>-<reviewer>.md` — per finding: severity,
  `file:line`, a concrete failure scenario (inputs → wrong output), and
  confidence CONFIRMED (reproduced) vs PLAUSIBLE (reasoned). Plus a verdict
  table: area → PASS-with-evidence / FAIL / NOT ESTABLISHED.

Prior reviews in `docs/reviews/` are hypotheses, not facts. Never trust a
"fixed" claim; re-verify.

## Then adjudicate

One reviewer's report is a claim, not a result. A final agent reads all of
them and rules:

- **Dedupe** findings reported under different ids; keep the best repro.
- **Adjudicate conflicts** — decide who is right and why. Do not average, do
  not "both have a point". Re-verify in a clean copy when the reports don't
  settle it.
- **Re-rank severity globally.** Reviewers calibrate independently. Silent
  unrecoverability outranks cosmetics whatever the original said.
- **Audit the clean verdicts.** An unsupported PASS is `NOT ESTABLISHED`.
- **Reproduce every CRITICAL/HIGH yourself**, or downgrade confidence and say
  you could not.
- **Refute contamination artifacts.** A failure only one reviewer saw, in an
  area another reviewer was mutating, is probably theirs. Test that.

## Baseline failures this process exists to prevent

All observed in a real 7-reviewer run on this repo.

| Failure                                                                     | Consequence                                                                                     |
| --------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Reviewers shared one checkout while one held live mutations                 | 5 contradictory suite verdicts in 6 runs; phantom `NotConverged`; 3 of 7 reviews voided         |
| Two reviewers returned near-total PASS with no evidence                     | 3 of the 7 worst findings sat inside a PASS                                                     |
| A green suite was read as correctness                                       | An Order-safety property test _structurally could not fail_ for the HIGH defect in its own area |
| An agent stashed only its own files, then called the failure "pre-existing" | Blamed a regression on prior code; it was introduced two commits earlier                        |
| An agent deleted a proptest seed recording a real failure                   | Suite green by omission                                                                         |
| A seed was hand-written to pin a case                                       | Proptest regenerates from the hash; the input is pinned only by comment                         |

## Red flags — stop

- About to run `cargo test` in a tree another agent can write to
- Writing PASS because you looked and nothing seemed wrong
- Reporting a suite failure without checking who else is mutating that file
- `git stash` to test "before" behavior while other commits touch the same logic
- Deleting or regenerating a snapshot/seed so the suite goes green
- Fixing a defect you just found

## Quick reference

| Step                  | Command                                       |
| --------------------- | --------------------------------------------- |
| Per-reviewer tree     | `git worktree add --detach "$WT" HEAD`        |
| Per-reviewer build    | `export CARGO_TARGET_DIR="$BASE/target-$REV"` |
| Full gate             | `just gate`                                   |
| Report format check   | `just fmt-check-file docs/reviews/<f>.md`     |
| Verify nothing leaked | `git worktree list && git status --short`     |
