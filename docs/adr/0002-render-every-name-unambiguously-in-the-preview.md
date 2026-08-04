---
number: 2
title: Render every name unambiguously in the preview
status: accepted
date: 2026-08-04
---

# Render every name unambiguously in the preview

## Context and Problem Statement

`detoxrs` previews by default and renames only when asked. That design puts the
whole safety burden on the preview: the user reads a list of `old -> new` pairs
and approves it. If the rendering of a name can differ from the name itself, the
approval is meaningless.

`report::escape_text` escaped Unicode category `Cc` and the literal `<`. An
adversarial review enumerated ten character classes against the binary and found
eight reached the terminal raw, including bidirectional overrides (`U+202E`,
`U+202D`), zero-width characters, Tags, `U+2028`/`U+2029` and non-breaking
spaces. The tool cites CVE-2021-42574 (Trojan Source) as the reason its own
stage 4 strips bidi characters, so a name carrying `U+202E` could render in the
preview as something other than what would be renamed.

The transform deliberately passes `Zl`, `Zp` and `Zs` through — a documented M4
deferral, not a defect. So the display layer cannot assume its input has already
been cleaned.

## Decision Drivers

- The preview is a safety control. A control that can be lied to is not one.
- Whatever the transform passes through, the reader must still be able to see it.
- Two distinct filenames must never render identically, or the preview merges
  rows that are actually different files.
- The escape vocabulary should not be a second thing to learn.

## Considered Options

- Escape `Cc` only, and strip the rest in the transform instead
- Escape every character that can mislead a reader, at the display layer
- Render names as quoted debug strings (`{:?}`)

## Decision Outcome

Chosen option: **escape every character that can mislead a reader, at the
display layer**. The predicate reuses `invisible::is_invisible` (stage 4's own
set, so no second classification of "invisible" exists), plus `Zl`/`Zp`, plus
`Zs` other than plain `U+0020`. `Co` is excluded: a private-use character is
neither invisible, reordering, nor a space lookalike, so it misleads nobody.
`Cs` cannot occur in a `char`.

The rule is about what a **reader** can be deceived by, which is why it is
broader than what the transform removes. A non-breaking space survives the
transform and now displays as `<u+00a0>`, so the user can see why a name that
looks clean is being changed — or is not.

Escaping is required to stay **injective**: every escaped scalar maps to a unique
token, and a literal `<` is itself escaped, so a filename cannot forge a token
that looks like ours. Without that, two different names could render the same and
the preview would misreport which file is which.

Display escaping applies to human output only. `--json` string fields keep their
existing escaped-plus-`utf8`-flag shape, and no field that previously carried raw
bytes gained display escaping.

### Consequences

- Good, because the preview can no longer be made to disagree with the rename.
- Good, because characters the transform deliberately keeps are now visible
  rather than silently present.
- Good, because reusing `is_invisible` means widening stage 4 widens the display
  automatically.
- Bad, because names containing exotic-but-harmless characters are noisier to
  read.
- Bad, because the display rule is deliberately broader than the transform rule,
  so "escaped in the preview" and "removed by the transform" are two different
  sets a contributor has to keep straight.

### Confirmation

Unit tests assert the escaped form for each newly covered class, that plain
`U+0020` is untouched, that private-use characters are not escaped, and that
escaping is injective. Verified end to end by piping a preview containing
`U+202E`, `U+200B` and `U+00A0` through `od` and confirming no raw sequence
reaches stdout.

## Pros and Cons of the Options

### Escape `Cc` only, and strip the rest in the transform

- Good, because there is one rule instead of two.
- Bad, because it forces an M4 decision (what to do about `Zs`, `Zl`, `Zp`) to be
  made early, under time pressure, as a side effect of a display bug.
- Bad, because the transform can never remove everything a reader can be fooled
  by — a name can be legitimately confusing without containing anything unsafe.

### Render names as quoted debug strings (`{:?}`)

- Good, because Rust's `Debug` escaping is injective for free.
- Good, because it is one line.
- Bad, because every path in every line gains surrounding quotes and doubled
  backslashes, which is worse to read for the common case of an ordinary name.
- Bad, because it escapes by Rust's rules, not by "can this mislead a reader" —
  it would leave `U+00A0` and `U+202E` untouched, missing the actual defect.

## More Information

Found as C-7 in the second adversarial review of M1, where it was reported as
MEDIUM by one reviewer, adjudicated up to HIGH, and asserted as a PASS by a third
reviewer whose own transcript printed raw `U+202E` beneath the claim.

The transform's own treatment of `Zl`/`Zp`/`Zs` is out of scope here and remains
deferred to M4.
