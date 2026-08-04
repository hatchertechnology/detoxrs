---
number: 6
title: Detect symlinks broken by a rename, do not repair them
status: accepted
date: 2026-08-04
---

# Detect symlinks broken by a rename, do not repair them

## Context and Problem Statement

Renaming entries inside a tree invalidates relative symlinks that pointed at the
old names. Cleaning a directory containing `l ink -> t arget.txt` renames the
target and leaves the link dangling.

The run reported `0 failed`, exit 0. The user was told the clean succeeded and was
left with broken links and no indication which ones, or that anything had happened
at all.

There are three possible behaviours: rewrite the link targets to follow the
rename, detect and report the breakage, or continue to ignore it. The tool is
named for renaming files, so rewriting file contents is a materially different
promise.

## Decision Drivers

- Reporting success for a tree that is now partly broken is the failure mode; that
  much has to change regardless of which option is chosen.
- A rename tool that edits what files point at is doing something the user did not
  ask for.
- Undo must remain meaningful: whatever apply does, `undo` has to be able to
  reverse.

## Considered Options

- Rewrite relative symlink targets to follow the rename
- Detect breakage and report it, leaving the links alone
- Refuse to rename anything that a symlink in the batch points at

## Decision Outcome

Chosen option: **detect and report, do not repair.** `apply::run` snapshots each
symlink's resolved target before the batch (only when it resolved) and re-checks
afterwards. Anything that resolved before and does not after is reported: a stderr
warning naming the link, a summary line, a `broken_symlinks` array in `--json`, and
a contribution to the exit code per
[ADR-0007](0007-exit-non-zero-when-requested-work-was-not-done.md).

Repair is rejected for M1 because rewriting a symlink target is not a rename. It
means writing new content into a file the user did not name, based on an inference
about what they meant. It also raises questions this decision does not have good
answers for: what to do about links pointing into the tree from outside it, about
absolute targets, about links whose target is renamed by a later batch, and how
`undo` reverses a content edit it did not journal.

Links that were **already** dangling before the run are not reported. The tool did
not break those, and reporting them would train users to ignore the warning.

Because `undo` shares the same `run()`, it inherits the detection: an undo that
breaks a link reports it too.

### Consequences

- Good, because the tool no longer claims success over a tree it partly broke.
- Good, because the user gets the specific link names and can repair or revert.
- Good, because `undo` gets the same protection with no extra code.
- Bad, because the user is left to do the repair. For a tree with many internal
  relative links, cleaning it is now a two-step job.
- Bad, because detection is a resolve-before and resolve-after per symlink, so a
  tree with many links pays for it on every run.
- Bad, because detection is only as good as the snapshot: a link created **during**
  the batch by another process is not covered, and a link whose target is outside
  the walked set is only noticed if it resolved before.

### Confirmation

Integration tests over a real filesystem: a batch that breaks a relative symlink
exits 1 and names the link, and a link that was already dangling before the run
keeps the run at exit 0. Both fail on the unfixed code.

## Pros and Cons of the Options

### Rewrite relative symlink targets to follow the rename

- Good, because the tree is left fully working with no user action, which is what
  someone cleaning a source checkout probably wants.
- Good, because the information needed is already in the plan — every old-to-new
  mapping is known.
- Bad, because it writes to files the user did not name, on an inference about
  intent, which is outside what "rename these files" promises.
- Bad, because it needs a journal format that can record and reverse a content
  edit, not just a rename, so `undo` stops being a pure inverse of `rename`.
- Bad, because the edge cases are genuinely hard: absolute targets, links from
  outside the tree, chains, and links that are themselves being renamed.

### Refuse to rename anything a symlink points at

- Good, because nothing is ever broken and no repair machinery is needed.
- Good, because it is trivially reversible.
- Bad, because one link can veto cleaning the file it points at, so a single
  stale link makes part of the tree permanently un-cleanable.
- Bad, because it inverts the tool's purpose: the messy name wins over the clean
  one.

## More Information

Found as C-9 in the second adversarial review of M1. The reviewer reported the
silence rather than proposing repair, and the adjudication kept it that way.

Repair remains a legitimate future feature, most plausibly behind an explicit flag
so the content-editing behaviour is opted into rather than assumed. It would need
a journal record type that can reverse a target rewrite.
