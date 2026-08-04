---
number: 7
title: Exit non-zero when requested work was not done
status: accepted
date: 2026-08-04
---

# Exit non-zero when requested work was not done

## Context and Problem Statement

`detoxrs` is meant to be usable from scripts, where the exit status is the only
signal a caller checks. Two classes of not-done work exited 0:

- An `-x` run that left an unresolved conflict.
- A subtree the walk could not read, reported as `0 failed`, exit 0 — the tool
  could not look at part of the tree it was told to clean and said nothing was
  wrong.

Both classes have the same shape: **nothing was attempted, so nothing failed.**
Every existing test covered an attempted-and-failed item, which is why a green
suite coexisted with both defects. `failed` counted attempts, and the exit code
was derived from `failed`.

A preview is a different case from an apply. `detoxrs somedir` reporting pending
conflicts is doing its job, and must not start exiting non-zero for it — that
would make the normal path look like a failure.

## Decision Drivers

- A caller must be able to distinguish "did what you asked" from "could not".
- Reporting conflicts is the preview's purpose, not a failure of it.
- An incomplete plan is not a valid preview, whether or not `-x` was passed.
- The contract must be documented, because scripts depend on it.

## Considered Options

- Derive the exit code from attempted-and-failed items only
- Exit non-zero for any run where requested work was not done, preview included
- Exit non-zero only for `-x` runs, leaving all previews at 0

## Decision Outcome

Chosen option: **exit non-zero for any run where requested work was not done**,
with the preview/apply distinction drawn on _whether the failure prevents the
output from being trustworthy_ rather than on whether `-x` was passed:

| Code | Meaning                                                                                                                                                      |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 0    | Everything asked for was done. A preview reporting a conflict stays 0 — nothing was attempted.                                                               |
| 1    | An `-x`/`undo` run that could not do everything asked, **or any run whose walk could not see part of the tree** — an incomplete plan is not a valid preview. |
| 2    | Usage, walk or plan errors; nothing was attempted at all.                                                                                                    |

Fixed at the point where the run's outcome becomes a status, not per symptom:
`Summary` carries `conflicts` and `broken_symlinks` alongside `failed`, and
`walk::snapshot` returns the unreadable paths instead of dropping them, threaded
through both the preview and exec paths — including the `report_nothing`
short-circuit, which hardcoded 0.

The unreadable-subtree case deliberately applies to previews too. A preview whose
plan is missing part of the tree is misleading in exactly the way this ADR exists
to prevent: the user approves a list that silently omits files.

This is a **behaviour change for scripts**: a preview over a tree with an
unreadable subtree now exits 1 where it previously exited 0.

### Consequences

- Good, because `detoxrs -x … || handle_failure` now works as a caller expects.
- Good, because adding a new category of not-done work means extending one
  `Summary` and one `exit_code()`, not auditing every call site.
- Good, because the contract is written in `--help`, so it is checkable.
- Bad, because it is a breaking change for any existing script that treated a
  preview's 0 as "tree is readable".
- Bad, because 1 now covers several distinct situations — item failed, conflict
  unresolved, subtree unreadable, symlink broken — so a caller that wants to
  distinguish them must parse `--json` rather than branch on the status.

### Confirmation

Integration tests over the real binary and filesystem assert the status for each
class: conflict under `-x` exits 1, the same conflict in a preview exits 0, an
unreadable subtree exits 1 in both preview and exec, and a broken symlink exits 1.
The `chmod 000` tests restore permissions through a `Drop` guard so a failure
cannot leave the suite unrunnable, and skip themselves under root, which ignores
the permission bits.

## Pros and Cons of the Options

### Derive the exit code from attempted-and-failed items only

- Good, because "failed" maps cleanly onto a single counter.
- Good, because it never reports failure for a run that did everything it tried.
- Bad, because the interesting failures are the ones where nothing was tried —
  a conflict is never attempted, and an unreadable directory yields no items.
- Bad, because it is what shipped, and it hid two defects behind a green suite.

### Exit non-zero only for `-x` runs, leaving all previews at 0

- Good, because previews stay side-effect-free and quiet, which is a defensible
  reading of "preview never fails".
- Good, because no existing script behaviour changes.
- Bad, because a preview over a partly unreadable tree is exactly the case a
  script most needs to catch — it is the one that silently under-reports work.
- Bad, because it makes the status depend on the flag rather than on whether the
  output can be trusted.

## More Information

Found as C-8 in the second adversarial review of M1, reproduced for both classes.
Another reviewer asserted exit codes were correct, citing tests that all covered
attempted-and-failed items — the one category where the behaviour was already right.

A smaller related inconsistency remains open: `undo --last` with no batches exits
2 while `undo --list` exits 0. Tracked as a low-severity finding, not fixed here.
