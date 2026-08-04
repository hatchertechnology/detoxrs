---
number: 3
title: Bind only the length axis the filesystem enforces
status: accepted
date: 2026-08-04
---

# Bind only the length axis the filesystem enforces

## Context and Problem Statement

Filenames have a length limit, but filesystems do not agree on the unit. ext4
counts bytes and does not care how many UTF-16 code units a name would occupy.
APFS counts UTF-16 code units and does not care how many bytes the name takes.

`Policy` already carried both `max_len_bytes` and `max_len_utf16`, and the
transform checked both. `Policy::default()` set both to 255 on every platform.
Binding both at once means the stricter one always wins, so on APFS the byte cap
fired for ordinary multi-byte names the filesystem would have accepted.

Measured on APFS: a 255-character CJK name (765 bytes) is accepted; 256 UTF-16
units is `ENAMETOOLONG` even at 512 bytes. So a 100-character CJK name — 300
bytes, 100 UTF-16 units, comfortably legal — was being truncated. Worse,
truncating several such names cut off exactly the part that distinguished them, so
they collided, and the collision was resolved by numbering and reported as
`2 renamed, 0 conflicts`, exit 0, with nothing saying a name had been shortened.

Truncating a name the filesystem would have accepted is destroying information for
no reason.

## Decision Drivers

- Never emit a name the filesystem will reject; `ENAMETOOLONG` at apply time is
  worse than truncating early.
- Never truncate a name the filesystem would have accepted.
- Truncation destroys information, so it must be visible to the user.
- The safety closure (a name is never over either limit) is verified by property
  tests and must not weaken.

## Considered Options

- Keep a single 255-byte cap everywhere
- Set the non-binding axis per `target_os` at compile time
- Query the real limit at runtime per filesystem (`statfs`/`pathconf`)

## Decision Outcome

Chosen option: **set the non-binding axis per `target_os`**. Each platform has
exactly one axis that its filesystems actually enforce, so the other is left
unbounded rather than given a second magic number. The real limit still binds and
the safety closure is unchanged — the property tests assert against
`p.max_len_bytes` and `p.max_len_utf16` rather than a hardcoded 255, so they
remain meaningful under per-platform defaults.

This was the smallest correct change: the pair of limits already existed and the
transform already checked both. Only the default was wrong.

Two reporting halves are part of the same decision, because a truncation the user
cannot see is nearly as bad as one that should not have happened:

- `Outcome.truncated` was computed and discarded. It is now threaded through
  `PlanItem` and surfaced as a preview note and a `--json` `truncated` key.
- A collision **caused by** truncation is reported as
  `Conflict::TruncationCollision` rather than silently renumbered. Numbering
  replaces the lost distinguishing bytes with an invented suffix, which is a
  worse outcome presented as a cleaner one.

### Consequences

- Good, because legal names are no longer truncated or collided on APFS.
- Good, because a shortened name is now visible in both output modes.
- Good, because no new configuration surface was added.
- Bad, because the effective limit now differs by platform, so the same tree
  cleaned on Linux and macOS can produce different names. This is inherent to the
  filesystems, but it is now `detoxrs`-visible rather than hidden behind a
  uniform-looking 255.
- Bad, because a network or FUSE filesystem mounted on either platform may
  enforce the other axis, and the compile-time default cannot know that. A name
  accepted by the plan could then be rejected at apply time, which surfaces as a
  per-item failure rather than data loss.

### Confirmation

`policy::tests` pins the platform-aware defaults and asserts the real per-platform
axis still binds. `plan::tests` pins that a truncation-induced collision is
reported rather than renumbered. The `length_bound` property test asserts both
axes on every generated name and policy.

## Pros and Cons of the Options

### Keep a single 255-byte cap everywhere

- Good, because behaviour is identical on every platform.
- Good, because it never emits an over-long name on any filesystem.
- Bad, because it truncates legal names on APFS, and truncation is destructive.
- Bad, because the resulting collisions are silent, which is how this was found.

### Query the real limit at runtime (`statfs`/`pathconf`)

- Good, because it is correct for network mounts, FUSE and exotic filesystems,
  which the compile-time default cannot be.
- Good, because it would make the limit a property of the directory being cleaned
  rather than of the build.
- Neutral, because `pathconf(_PC_NAME_MAX)` reports a byte count and gives no way
  to learn that a filesystem counts UTF-16 units instead, so it answers only half
  the question.
- Bad, because it puts a syscall per directory into the plan phase and makes the
  transform's output depend on the filesystem, which the property tests currently
  quantify over as a pure function of `(name, Policy)`.
- Bad, because it is substantially more machinery than the defect required.

## More Information

Found as C-3 in the second adversarial review of M1, merged from three separate
reports: the damage (legal names truncated and collided), the reachability proof
(`M1_MAX_LEN == NAME_MAX`, so for single-byte names truncation never fires on
ext4 or APFS — which is why only multi-byte names expose it), and the silence
(`truncated` computed and dropped).

Runtime limit discovery is the natural successor to this decision if network or
FUSE filesystems become a supported target.
