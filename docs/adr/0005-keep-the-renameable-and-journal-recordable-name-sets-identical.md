---
number: 5
title: Keep the renameable and journal-recordable name sets identical
status: accepted
date: 2026-08-04
---

# Keep the renameable and journal-recordable name sets identical

## Context and Problem Statement

The undo journal is the only thing standing between a user and a permanent loss of
their original filenames. Two independent predicates decided what it could carry:
`classes.rs` decided which characters the transform rewrites, and
`journal::is_plain_basename` decided which names a journal record may contain.
Nothing kept them in agreement.

They disagreed about the backslash. `\` is in `SEPARATOR_CLASS`, so the transform
rewrites it; `is_plain_basename` rejected any name containing it. So `-x` renamed
`back\slash.txt` to `back_slash.txt`, reported `1 renamed, 0 failed`, exit 0 — and
`undo` refused the record it had just written, reported `1 reverted, 0 refused`,
and left the original name gone for good. Silent, unrecoverable loss reported to
the user as success.

The guard itself was not pointless. `\` really is a path separator to
`MoveFileExW`, so a Windows build replaying a journal must refuse such a record.
It was the unconditional application that was wrong.

## Decision Drivers

- A rename that cannot be undone must not happen. Either both work or neither does.
- Records must be refused for real reasons; a journal that rejects legal names is
  as much a defect as one that accepts dangerous ones.
- The failure mode must not be silent. A refused record is recoverable; a refused
  record reported as a success is not.

## Considered Options

- Delete the backslash clause outright
- Gate the backslash clause on the replaying platform
- Refuse to rename any name the journal cannot record

## Decision Outcome

Chosen option: **gate the clause on the replaying platform** —
`!cfg!(windows) || !bytes.contains(&b'\\')`. Under `renameat` a backslash is an
ordinary byte in a basename; under `MoveFileExW` it is a separator and the record
genuinely is a multi-component path. Gating keeps the protection where the hazard
is real and removes it where it only caused loss.

The invariant this establishes, and the reason this ADR exists rather than just a
bug fix: **on any given build, the set of names `-x` will rename must equal the set
the journal can round-trip.** A divergence between those two sets is a data-loss
bug generator, not a validation nicety. `\` was the only divergence — `/` and
`.`/`..` cannot occur as a real directory entry — but the class is what matters.

The related accounting defect is part of the same decision. An item dropped during
replay was counted as neither reverted nor refused, so the tally could read
`1 reverted, 0 refused` while a name was lost. Every item now lands in exactly one
bucket and the printed tally equals the batch size, because a loss the user cannot
see is one they cannot act on.

### Consequences

- Good, because backslash names round-trip, verified end to end.
- Good, because the invariant is stated, so the next character added to
  `SEPARATOR_CLASS` has an obvious obligation attached.
- Good, because the undo tally can no longer hide a lost item.
- Bad, because the journal's validity now depends on the build that replays it, so
  a journal written on Linux and replayed by a Windows build can be refused. That
  is correct — the record really is unusable there — but it means a journal is not
  unconditionally portable.
- Bad, because the two predicates are still two predicates. Nothing mechanically
  enforces that they agree; the invariant lives in a test and this document.

### Confirmation

A round-trip test renames, replays and undoes a real backslash-named file, and
fails on the unfixed guard. `journal::tests` covers the replay accounting for an
intent that cannot be parsed but is later closed by a `done`.

The stronger confirmation would be a property test quantified over
`SEPARATOR_CLASS` asserting every rewritable character is journal-recordable. That
does not exist yet and is the obvious next guard.

## Pros and Cons of the Options

### Delete the backslash clause outright

- Good, because it is the smallest possible diff and fixes the reported symptom.
- Good, because all tests pass with it, so nothing in the suite argues against it.
- Bad, because it discards a real protection: a Windows build replaying the record
  would treat `back\slash.txt` as a two-component path and rename outside the
  pinned directory.
- Bad, because "the tests still pass" was never evidence here — the clause had no
  coverage in either direction.

### Refuse to rename any name the journal cannot record

- Good, because it satisfies the invariant from the other side, and is arguably
  the more conservative reading.
- Good, because it needs no platform reasoning at all.
- Bad, because it makes a class of perfectly legal Unix filenames permanently
  un-cleanable — exactly the names a tool like this exists to fix.
- Bad, because the refusal would be driven by a Windows constraint on platforms
  where it does not apply.

## More Information

Found as C-1 in the second adversarial review of M1, the only CRITICAL of that
pass. The accounting half was reported separately as C-11 and is recorded here
because it is what made C-1 invisible.

The guard had no test in either direction, which is why a divergence between two
predicates survived a full review, a fix pass and a green suite.
