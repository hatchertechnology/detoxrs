---
number: 1
title: Record architecture decisions
status: accepted
date: 2026-08-04
---

# Record architecture decisions

## Context and Problem Statement

Design decisions on this project have been recorded in three different shapes:
`docs/research/00-proposal-rust-detox-successor.md` for the reasoned design,
`docs/owner-decisions.md` for calls the owner made that the research could not
settle, and commit messages for everything decided while implementing. That
third category is the problem. Decisions like binding only the length axis the
filesystem enforces, reporting broken symlinks rather than repairing them, and
ordering renames by inode identity rather than path spelling each had real
alternatives and real consequences, and each is currently recoverable only by
reading a commit message. A planning tree that was deleted once already took
the milestone-scope record with it.

How should a decision with alternatives and consequences be recorded so it
survives the document it was made in?

## Decision Drivers

- A decision's alternatives and consequences are the part worth keeping; the
  outcome alone is not enough to re-evaluate it later.
- Records must survive reorganization. Anything that lives only in a planning
  document dies with that document.
- `docs/owner-decisions.md` already works for what it covers and has 18 inbound
  references. Replacing it would be churn for no gain.
- The overhead has to be small enough to actually happen mid-implementation.

## Considered Options

- `adrs` with MADR 4.0.0 in NextGen mode (YAML frontmatter)
- `adrs` in adr-tools/Nygard-compatible mode
- Keep `docs/owner-decisions.md` as the only decision log

## Decision Outcome

Chosen option: **`adrs` with MADR 4.0.0 in NextGen mode**, configured in
`adrs.toml` (`adr_dir = "docs/adr"`, `mode = "nextgen"`,
`templates.format = "madr"`). MADR prompts for considered options and per-option
consequences as first-class sections, which is exactly the material that was
being lost; NextGen's YAML frontmatter makes status and number machine-readable,
so `adrs doctor` and `adrs generate toc` work without parsing prose headings.

`docs/owner-decisions.md` stays, with a boundary: it records **owner calls that
override the research** — an authority statement, dated, with what it changes.
ADRs record **technical decisions with alternatives considered**. An owner call
needs no alternatives section, and a technical decision carries no authority to
override research, so the two do not overlap. When an owner call also needs its
alternatives recorded, the entry cites an ADR rather than restating it.

### Consequences

- Good, because a decision's rejected alternatives survive independently of any
  planning document.
- Good, because status and numbering are machine-readable, so repository health
  is checkable rather than a matter of review.
- Bad, because there are now two places a reader must check for a past
  decision. Mitigated by the boundary above, but it is a real cost.
- Bad, because only the M1 remediation decisions that had genuine alternatives were
  retrofitted (ADR 2-7). The rest were plain bug fixes with no decision to record,
  and stay in commit messages.

### Confirmation

`adrs doctor` is the fitness function — it checks numbering, status validity and
link integrity. `just fmt-check` covers formatting, since `docs/adr/` is inside
the repo-wide prettier glob.

## Pros and Cons of the Options

### `adrs` in adr-tools/Nygard-compatible mode

The format `adrs init` produces by default, and what this file originally was.

- Good, because it is the most widely recognized ADR shape.
- Good, because it is the shortest to write.
- Bad, because Context/Decision/Consequences has no slot for rejected
  alternatives, which is the material this project keeps losing.
- Bad, because status lives in a prose heading, so tooling must parse it.

### Keep `docs/owner-decisions.md` as the only decision log

- Good, because no new tool, directory or convention.
- Good, because it already has 18 inbound references and is demonstrably used.
- Bad, because its format records an authority and a date, not alternatives.
- Bad, because it is one file: a decision made about the transform pipeline and
  one made about release tooling land in the same append-only list.

## More Information

`adrs init` writes `doc/adr` regardless of `adr_dir` in `adrs.toml`; `adrs new`
and `adrs list` honor the config. That mismatch is why this repository briefly
had both `doc/` and `docs/`. If ADRs are ever re-initialized, move the directory
afterward and delete the `.adr-dir` file, which duplicates `adrs.toml`.

MADR 4.0.0: <https://adr.github.io/madr/>. Michael Nygard's original article,
which the Nygard format follows:
<https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions>.
