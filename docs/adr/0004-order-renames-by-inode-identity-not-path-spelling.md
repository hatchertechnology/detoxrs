---
number: 4
title: Order renames by inode identity, not path spelling
status: accepted
date: 2026-08-04
---

# Order renames by inode identity, not path spelling

## Context and Problem Statement

A batch that renames both a directory and things inside it has to rename the
contents first. Rename the parent first and every planned child path is stale, so
the child renames fail with `ENOENT` and the batch is left half applied.

`deterministic_order` sorted on `Reverse(e.dir.components().count())` — a property
of how a path was **spelled**, not of where it sits in the tree. Two arguments can
name the same directory with different component counts:

```sh
detoxrs -x -r "de ep/d ir" "de ep/../de ep"
```

`de ep/../de ep` has three components, `de ep/d ir` has two, so the argument that
textually looks deeper sorted first — even though it is the parent. Result: the
parent was renamed first, both nested renames failed `ENOENT`, and the tree was
left half cleaned at exit 1.

`Entry` already carried a `depth` field, which looks like the obvious fix and is
not: `depth` resets to 0 at each `-r` argument root, so one real directory reached
through two arguments carries two different depths. Substituting it reproduces the
same failure.

## Decision Drivers

- A half-applied batch is the worst outcome the apply phase can produce.
- The ordering key must be a property of the filesystem, not of user-supplied text.
- The property test guarding this must be able to fail; the one that existed could
  not.

## Considered Options

- Sort on `dir.components().count()`
- Sort on `Entry::depth`
- Sort on nesting depth derived from `(dev, ino)` identity chains
- Canonicalize every path first, then sort on the canonical spelling

## Decision Outcome

Chosen option: **derive nesting depth from `(dev, ino)` identity chains**.
`structural_depth` computes, for each entry, whether its `dir_ident` is some other
directory entry's own `ident`, recursively. Identity is what the filesystem
actually thinks, so no spelling can defeat it — `de ep/../de ep` and `de ep` have
the same inode and therefore the same depth, whatever their text looks like.

The general invariant: **deeper entries along the same chain are renamed before
shallower ones.** Ordering between unrelated subtrees is unconstrained, so this is
a partial order made total only for determinism.

The Order-safety property test could not have caught the defect: its generator
emitted a single canonical spelling per directory, and the check itself compared
with textual `Path::starts_with`. Both were fixed as part of this decision — a
generator that gives each entry one of two spellings while keeping identities tied
to the real level, and a check that walks identity chains. Its refuting power is
demonstrated rather than assumed: restoring the old component-count key makes it
fail with a minimal counterexample.

### Consequences

- Good, because ordering is now immune to path spelling, `..` segments, symlinked
  roots and duplicate arguments naming one directory.
- Good, because it reuses identity data the walk already collects for the
  apply-time TOCTOU checks; no new syscalls.
- Good, because the property test that guards it is proven able to fail.
- Bad, because `structural_depth` is O(n) per entry in the worst case rather than a
  field read, so ordering a very large batch costs more than a sort on an integer
  already in hand.
- Bad, because `Entry::depth` still exists and still looks like the right key for
  this. Its doc comment now says why it is not, which is weaker than deleting it.

### Confirmation

`prop_plan`'s `order_safety` property, quantified over generated snapshots with
varying spellings and checked against identity chains. A named integration test
runs the two-argument repro through the real binary and asserts the batch applies
fully. Restoring the old key must make `order_safety` fail — checked when the fix
landed.

## Pros and Cons of the Options

### Sort on `Entry::depth`

- Good, because it is already computed and is a single integer compare.
- Good, because it is correct for the common case of one argument.
- Bad, because it is walk-relative: it resets per `-r` argument, so the same
  directory reached two ways gets two values. Verified to reproduce the original
  defect.

### Canonicalize every path first, then sort on the canonical spelling

- Good, because it makes the textual sort correct by removing the variation.
- Good, because it needs no new ordering concept.
- Bad, because `canonicalize` resolves symlinks, which changes what the tool is
  being asked to rename — a symlinked root must not be descended, and canonicalizing
  would erase the distinction.
- Bad, because it is a syscall per entry, and it can fail on a path the walk
  already successfully read.
- Bad, because it still keys on text, so it is one surprising spelling away from
  the same class of defect.

## More Information

Found as C-2 in the second adversarial review of M1. Reported HIGH by one reviewer
with a reproduction, and asserted as PASS by another whose test used a single
canonical spelling — the one input class where the defect cannot appear. That
disagreement is why the review's adjudication pass exists.

The test-adequacy half was tracked separately as C-16 and fixed in the same
change, since a guard that cannot fail is not a guard.
