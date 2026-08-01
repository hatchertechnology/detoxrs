---
plan: C
mandate: specification- and test-driven — the executable specification (§8 property tests,
  snapshot tests, filesystem matrix, fuzzing) is built first and drives the implementation;
  an unimplemented property is a failing test, not a TODO
author: Plan Author C (one of three independent plan authors; this plan is not the
  consensus document and does not attempt to be)
date: 2026-07-31
documents_read:
  - docs/owner-decisions.md
  - docs/research/00-proposal-rust-detox-successor.md (full: §0-§11, Appendix A,
    Review record stage 3, Propagation record stage 3)
  - docs/rust-setup-notes.md
  - docs/rust-setup-ci.md
  - Cargo.toml, justfile, crates/detoxrs-core/src/lib.rs, crates/detoxrs/src/main.rs
    (current repository state — confirms the workspace compiles and both crates are
    placeholders)
  - Skimmed for cross-reference only: docs/research/10-13 (upstream source-derived
    behavior), docs/rust-setup-governance.md, docs/rust-setup-release.md,
    docs/rust-setup-supply-chain.md (not required for this plan's content; consulted to
    avoid contradicting already-built CI/governance machinery)
---

# Plan C: spec/test-driven build order for detoxrs

This plan is written under one instruction that overrides ordinary engineering
instinct: **the tests in proposal §8 are not a quality gate bolted on at the end, they
are the executable version of §3/§5, and building them first is how the pipeline gets
written.** Every milestone below names the test file and the property function before
it names the implementation. Where a milestone's implementation is a single `todo!()`
away from its test passing, that is the point — the test specified the function, the
function did not specify the test.

Two things this plan is not: it is not the safety-architecture-first plan (build
`fsops`/journal/apply before the transform contract is nailed down) and it is not the
thin-vertical-slice plan (get `detoxrs -x ~/Downloads` doing _something_ on day one). §9
argues for this ordering over both, honestly, including its cost: no human-usable
binary until Milestone 7.

---

## 1. Auditing §8 for decidability

Before any test is written, each property needs a yes/no answer to two questions:
**(a) is this decidable as stated** — can a test actually check it without needing
something the design doesn't provide — **and (b) does the design in §3/§5 actually
satisfy it**, or does the property fail on paper before it fails in CI. The stage-3
review already caught one of these (§3.14's `Unrepresentable` fix, retroactively
required by a falsified Safety-closure property on `***`). This section re-runs that
check across the rest of §8.1/§8.2 and finds three more issues of the same shape — none
as severe as the one that forced a new variant, but each is a gap between what the
property says and what the current type shapes let a test check.

### 1.1 §8.1 property-by-property

| Property                                | Decidable as written?                                                                                                                                                               | Design satisfies it?                                                                                                                                                                                                                                                                                                                        |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Totality                                | Yes. Generate arbitrary `Vec<u8>` + arbitrary resolved `Policy`; assert `transform` never panics and returns `Name(o)` satisfying Safety-closure/Non-empty or `Unrepresentable(_)`. | Yes, by §3.14's construction — this is exactly the property that forced that construction, so as of the current proposal text it holds by design, not by luck.                                                                                                                                                                              |
| Idempotence                             | Yes, for the `Name(_)` branch, given a fixed resolved `Policy`.                                                                                                                     | Yes, but **only if `NotConverged` is actually reachable and tested**, not merely declared possible. See M4 below: the fixed-point loop's bound (3 iterations) must be exercised with inputs chosen to approach it, or this property is "decidable" in principle while the implementation quietly never takes the branch that makes it true. |
| Safety closure                          | Yes. Direct character-class check on the output.                                                                                                                                    | Yes, given the narrowed delete class (already fixed in review).                                                                                                                                                                                                                                                                             |
| **Length bound**                        | **No, not as stated, against the current type shapes.** See §1.3 below — this is the plan's primary new finding.                                                                    | Cannot be evaluated until the ambiguity is resolved; the fix is proposed in §1.3 and adopted in Milestone 4.                                                                                                                                                                                                                                |
| No grapheme splitting                   | Yes. Grapheme-cluster count via `unicode-segmentation`, both on the stem path (§3.10 step 2) and the whole-name fallback (step 3).                                                  | Yes, provided both paths share one truncation function (a single `truncate_graphemes(&str, limit) -> &str`-shaped helper) rather than two hand-written loops that could drift. This is a Milestone 4 implementation constraint, not a new property.                                                                                         |
| Non-empty                               | Yes, trivial for `Name(_)`.                                                                                                                                                         | Yes, by §3.14.                                                                                                                                                                                                                                                                                                                              |
| Dotfile preservation                    | Yes. `starts_with('.')` and "exactly one" is a simple prefix-count check on both `x` and `o`.                                                                                       | Yes, given stage 10's dot-preservation rule as specified in §3.8.                                                                                                                                                                                                                                                                           |
| Decode is total and never re-interprets | Yes. Pure property of `decode` alone; no `Policy` dependency in practice even though the signature carries one.                                                                     | Yes — this is the regression test for `café.txt -> cafÃ©.txt`, and with `Repaired` gone there is no third branch to assert the absence of.                                                                                                                                                                                                  |
| **Stage independence**                  | **Decidable only if the implementation exposes each stage as an independently substitutable function.** See §1.4.                                                                   | Cannot be evaluated as a property of the _specification_ alone; it is a property of how `pipeline.rs` is _written_, and the proposal does not constrain that. This plan makes it a hard requirement on `pipeline.rs`'s shape (Milestone 3).                                                                                                 |

### 1.2 §8.2 property-by-property

| Property                    | Decidable as written?                                                                                                                                                                                                                                                                                                                                                                                                   | Design satisfies it?                                                                                                                                                                                                                                                                                                                         |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No collision                | Yes, purely in-memory: `plan()` takes `Vec<Entry>`, no I/O (§5.1), so an arbitrary-entries generator is enough.                                                                                                                                                                                                                                                                                                         | Yes.                                                                                                                                                                                                                                                                                                                                         |
| **No pre-existing clobber** | **Decidable only for the pure half.** See §1.5 — the property as stated covers collision layer 1/2's plan-time check but cannot, by construction, cover the apply-time TOCTOU recheck or the kernel-level no-clobber (layers 2's second half and 3, §5.3), because those need real I/O and `plan()` has none.                                                                                                           | Partially: the plan-time half is satisfied and testable; the apply-time half needs a _different_ test, which §8.4's table does not currently list. Proposed as a new filesystem-matrix row in Milestone 8.                                                                                                                                   |
| Order safety                | Yes. Pure check on `Plan.items` ordering vs `depth`.                                                                                                                                                                                                                                                                                                                                                                    | Yes, given deepest-first construction.                                                                                                                                                                                                                                                                                                       |
| No sibling chains           | Yes, given near-swap generators as the property text itself demands.                                                                                                                                                                                                                                                                                                                                                    | Yes, and provably so per §5.3's algebraic argument — this is the one property in the whole document with a written proof rather than just a design intention, which makes it the cheapest to trust and the most informative to fail: a failure here is evidence Idempotence broke, not evidence the collision engine has a new bug to chase. |
| Bounded renumbering         | Yes, including the pathological small-`max_len` case explicitly named in §5.3.                                                                                                                                                                                                                                                                                                                                          | Yes, given the stated `N = 2..999` bound and the fallback to `Conflict`.                                                                                                                                                                                                                                                                     |
| Determinism                 | Yes. Shuffle input order, replan, compare.                                                                                                                                                                                                                                                                                                                                                                              | Yes, given NFC-bytes-of-source-name as the stable sort key (§5.3).                                                                                                                                                                                                                                                                           |
| Undo round-trip             | Decidable, **but only if the apply/undo logic is written against an injectable `RenameOps` and journal-writer**, so an in-memory filesystem model can stand in for the real one. The proposal's own trait (§5.4) already has the right shape for this; the gap is only that §7.1 places `fsops.rs`'s _implementation_ in the binary crate without saying where the _trait_ and the apply-loop logic that calls it live. | Satisfiable, addressed as an explicit design decision in Milestone 6/8 below (put the trait, and as much of the apply/undo control flow as is I/O-free, where a mock can reach it without a filesystem).                                                                                                                                     |

### 1.3 New finding: Length bound is not decidable against the current `Policy` shape

§3.1 states plainly that the resolved `Policy` reaching `transform` has `max_len` as "a
concrete number." §3.10 states, with equal plainness, that the design "satisfies **both**
metrics simultaneously (bytes <= byte limit AND UTF-16 units <= unit limit) whenever the
volume is unknown." §8.1's Length-bound property statement inherits the second sentence
verbatim: "`o` satisfies both the byte and UTF-16-unit limit for the resolved policy."

Those two sentences describe two different shapes for the same field. A single scalar
`max_len: usize` cannot carry two independent caps at once, and the two caps are not
interchangeable: a name made entirely of astral-plane emoji has roughly half as many
UTF-16 units as bytes (each codepoint is 4 UTF-8 bytes but one UTF-16 surrogate pair,
i.e. 2 units), so a limit that is safe under one metric is not automatically safe under
the other in either direction. Concretely: if `max_len` is interpreted as a UTF-16-unit
cap of 255 (APFS's number) and enforced only in that unit, a 255-astral-codepoint name
passes the unit check at exactly 510 UTF-16 units... no — worked correctly: 255 astral
codepoints = 255 surrogate pairs = 510 UTF-16 units already over a units-cap of 255, so
that particular direction self-corrects. The failure direction that actually bites is
the _byte_ side: a name of ~130 astral emoji is only 260 UTF-16 units (just over a
255-unit cap, correctly truncated) but 520 UTF-8 bytes — comfortably over ext4's
255-_byte_ cap. Enforcing only the UTF-16-unit number that "the resolved policy" carries
produces an output that is safe on APFS and violates ext4's limit outright, on a
filesystem where nobody ever enforced the byte side because there was only one number to
enforce. **This is exactly the case the property is written to catch, and the property
cannot catch it if the type it is checking never receives the second number.**

Proposed fix, adopted in Milestone 4: the resolved policy that reaches `transform` (and
therefore the type the proptest constructs) carries **two concrete fields**, not one —
`max_len_bytes: usize` and `max_len_utf16: usize`. Per-directory auto-detection (§3.10)
sets both from the detected filesystem (ext4: bytes = 255, utf16 = `usize::MAX`; APFS:
utf16 = 255, bytes = `usize::MAX`; unknown/NTFS/exFAT-assumed: both = 255, matching
§3.10's own "conservative intersection" language generalized to two always-populated
fields instead of one sometimes-implicit one). A user's `--max-len N` sets both fields to
`N`, which is the literal reading of "we satisfy both metrics simultaneously... and it
costs nothing" applied to the override path as well as the auto path. `Truncate`'s
grapheme-safe algorithm (§3.10 steps 2/3) shrinks until _both_ fields are satisfied,
which was already the intended behavior — this fix makes the type able to say so, and
makes Length bound a property that can actually be violated by a bug and caught by a
test, rather than one that is vacuously true because only one axis was ever checked.

### 1.4 New finding: Stage independence requires `pipeline.rs` to expose substitutable stages

§8.1's Stage-independence property reads: "Disabling stage N changes only what stage N is
documented to change: the output with stage N off equals the output of the pipeline with
stage N replaced by identity." Checking that requires literally constructing two
pipelines that differ in exactly one stage. Five of the thirteen stages — 7 (`safe_map`),
9 (`collapse`), 10 (`trim`), 12 (`truncate`), 13 (`finalize`) — have **no CLI flag that
disables them** (§2.4's flag list confirms this: `--keep`/`--strip` move characters
between classes but do not turn stage 7 off; there is no flag for 9, 10, or 13 at all;
`--max-len` resizes stage 12 but does not disable it). If `pipeline.rs` is written as one
function that inlines all thirteen steps in sequence — the natural first draft of "the 13
stages, in order, and only here" (§7.1's own description of the file) — then the property
test for those five stages has no seam to substitute through, and the only way to write
the test is to duplicate the pipeline's logic inside the test file with one step skipped,
which is not testing the implementation at all.

This is not a falsified property — nothing here contradicts §3/§5 — but it is a
requirement on _how_ `pipeline.rs` gets written that the proposal never states, and a
plan that calls itself test-driven has to state it before the file is written, not
discover it while trying to write the test. **Adopted as a hard constraint from
Milestone 3 onward: every stage is a separate, independently callable function with the
same `&str -> String` (or `&str -> Cow<str>`) shape, and `pipeline::run` (or an internal
`pipeline::run_with_override(input, policy, overrides: &[StageOverride])`) composes them
by iterating an ordered list of function pointers/enum-tagged closures, not by inlining
thirteen blocks in one body.** The property test then builds its own second pipeline by
substituting `identity` at index N-1 in that same list — no duplicated logic, and the
production `pipeline::run` is exactly the N=all-stages-on case of the same machinery the
test exercises. This has no effect on the shipped CLI surface: none of stages 7/9/10/12/13
gains a public flag, the override mechanism is `pub(crate)` (or gated behind a
`#[cfg(test)]`/`internal-testing` feature if `pub(crate)` cannot reach an external `tests/`
integration-test crate — Rust integration tests are separate crates, so this plan uses a
small `#[doc(hidden)] pub` re-export restricted to what the test crate needs, documented
as "test-only, not part of the public API," rather than weakening the crate's real
surface).

### 1.5 New finding: "No pre-existing clobber" needs a filesystem-matrix companion, not just a property test

§5.3 describes three collision layers: (1) intra-batch, in `plan()`, pure; (2)
pre-existing destination, checked "from the walk snapshot **plus a fresh
`symlink_metadata` at apply time**"; (3) kernel-level no-clobber at the syscall. §8.2's
"No pre-existing clobber" property is written against `plan()`'s output alone, which can
only ever exercise the _walk-snapshot_ half of layer 2 — the half that has no I/O and can
be property-tested in-memory. The fresh recheck (the half that actually defends against
something changing between the walk and the apply, which is the whole reason layer 2 has
two clauses instead of one) cannot be exercised by any test that never touches a real
filesystem, because there is nothing to have changed. That gap is not a bug in the
property — it is correctly scoped to what `plan()` can see — but it means §8.2 alone
gives a false sense that "no clobber" is fully covered when the TOCTOU-defending half of
the guarantee currently has **no test anywhere in §8**, including §8.4's filesystem
matrix, which does not list this case.

**Proposed addition, adopted in Milestone 8's filesystem matrix:** a "TOCTOU collision
during apply" row — compute a plan against a snapshot, then create a file at one item's
destination _after_ the snapshot but _before_ `apply` runs, then run `apply` and assert
the affected item is reported as a fresh conflict (not clobbered, not silently
overwritten, not panicking), on both Linux and macOS. This is the test that actually
proves layer 2's second clause and layer 3 do their job; without it, "no pre-existing
clobber" as a phrase in the release-gate list is true of the planner and unproven of the
tool.

### 1.6 Fuzzing and snapshots

§8.5's fuzz target (`decode` + `transform`, oracle = the §8.1 property set) is decidable
exactly to the extent §8.1 is: once Milestones 2-5 land the properties as executable
`proptest` assertions, the same assertions become the fuzz oracle with zero additional
design work — this is one of the two concrete instances of "closing a spike as a side
effect of the test suite existing" this plan can point to (the other is §11 spike 12,
the fixed-point bound, addressed in Milestone 9). §8.3's snapshot tests are decidable as
stated and raise no design question; the only engineering problem they raise — how the
corpus is stored — is answered in full in §5 below.

---

## 2. Executable-specification build order

The rule for every milestone from here on: **the test file is named and written before
the implementation file it tests.** Where a milestone's test would need a function that
doesn't exist yet, the function is added with a body that makes the test fail
honestly (a real, if minimal, implementation attempt — never a `todo!()` left in a
commit that claims the milestone is done; `just gate`'s `test` step must be green at
every milestone boundary, never red-with-an-excuse).

This section is the map; Milestone 1 (the first one, per the task's request for "unusual
detail") is expanded fully in §4. The rest are specified at the grain a reviewer needs to
sequence and estimate them, not re-derive them.

---

## 3. Milestones

Estimates are in **T** (test-points): roughly, one T is "one property/test file plus the
minimal code that makes it pass," calibrated against the proposal's own LOC budget
(§7.3: v0.1 1200-1800 lines, v1.0 2200-3000 lines) rather than calendar time, since the
task asks for justified units, not dates. A milestone's T is stated as
non-test-code-lines ÷ 60, rounded, cross-checked against the named test files — it is a
sizing signal for sequencing discussions, not a velocity commitment.

Dependencies (runtime and dev) are called out per-milestone with a running total; the
full schedule is also tabulated in §7 for a single-glance view. Every milestone ends with
`just gate` green; milestones that add runtime dependencies also re-run `just
dep-budget` explicitly in their exit criteria, since that is the one gate a milestone
could silently blow past.

### M1 — Decode, the fixture corpus, and the first property test (detoxrs-core)

**Scope:** `Decoded`, `decode()`, and the fixture-corpus infrastructure everything after
this milestone reuses. No pipeline, no `Policy` fields beyond the empty struct the
signature requires.

**Files:** `crates/detoxrs-core/src/decode.rs`, `src/policy.rs` (minimal), `src/lib.rs`
(wiring, placeholder removed), `tests/support/mod.rs` + `tests/support/corpus.rs`,
`tests/prop_decode.rs`, `tests/snap_decode_corpus.rs`.

**Dependencies added:** dev-only — `proptest`, `insta`. Runtime: none.

**Tests first:** `decode_is_total_and_never_reinterprets` (proptest, §8.1); a snapshot of
the corpus's Utf8/Opaque classification (insta).

**Exit criteria:** `cargo test -p detoxrs-core` green; `just gate` green; `just
dep-budget` unchanged at 0/11.

**Size:** 1T. Full detail in §4.

### M2 — Character classes, `safe_map`, `collapse`, `trim` (stages 7, 9, 10)

**Scope:** the three character-class sets (§3.7) as pure functions over `char`/`&str`;
`safe_map`, `collapse` (same-character-run collapsing, §3.8, including the leading-dot
worked example), `trim` (§3.8's leading `-`, trailing dot/space, one preserved leading
`.`). No `normalize`, no `invisible_strip`, no truncation yet — this milestone
deliberately runs a three-stage sub-pipeline so Safety-closure and Non-empty become
checkable early, before the harder stages (normalize/invisible/truncate) exist.

**Files:** `crates/detoxrs-core/src/classes.rs`, `src/pipeline.rs` (introduced here, per
§1.4's constraint: a stage-list/override-capable composer from day one, not retrofitted
later), `tests/prop_pipeline_stage2.rs` (properties scoped to the three-stage
sub-pipeline: Safety closure minus the length/grapheme clauses, which need stages not yet
present).

**Dependencies added:** none.

**Exit criteria:** Safety-closure holds for the delete/separator classes on the
three-stage pipeline; the `.!file.txt -> .file.txt` worked example (§3.8) is a literal
test case, not just prose; `pipeline.rs`'s stage-list shape is exercised by a
throwaway "identity-substitution round-trips" sanity test that Milestone 3 will build
Stage-independence proper on top of.

**Size:** 2T.

### M3 — `normalize`, `invisible_strip` (stages 3, 4) and Stage independence

**Scope:** NFC normalization (always-NFC internal comparison is a later, plan-level
concern — this milestone is only the output-normalization stage); the invisible/bidi/tag
deletion set, generated at build time from UCD data checked into the repo (§7.1's
`invisible.rs`, build-time generator, no network fetch — the generator script and its
input data file are part of this milestone's file list, not deferred).

**Files:** `crates/detoxrs-core/src/invisible.rs`, `build.rs` (UCD-table generator,
checked-in UCD source data under `crates/detoxrs-core/data/`), `tests/prop_stage_independence.rs`.

**Dependencies added:** runtime — `unicode-normalization`. Running total: 1/11.

**Tests first:** the full Stage-independence property, now meaningful across five
implemented stages (3, 4, 7, 9, 10), built on the override mechanism from M2.

**Exit criteria:** Stage independence green across all five implemented stages,
including the `--no-invisible-strip` case (this is also the regression test that the
delete class was correctly narrowed in the proposal's own stage-3 review — a duplicate
delete/invisible set would make this property fail immediately).

**Size:** 3T (the UCD generator is the bulk of it).

### M4 — `truncate`, `finalize`, and the `Unrepresentable` outcome (stages 12, 13)

**Scope:** the resolved-`Policy` fix from §1.3 (`max_len_bytes` + `max_len_utf16`, both
concrete); grapheme-safe extension-aware truncation (§3.10, all four steps including the
"<= 4 bytes of UTF-8" extension lookback and the whole-name fallback); the bounded
fixed-point loop (re-running 9/10 only, per the v0.1 scope note in §10 — stage 11 is not
implemented until M11) with `Unrepresentable(ReducesToEmpty | ReducesToDotOrDotDot |
NotConverged)`.

**Files:** `crates/detoxrs-core/src/truncate.rs`, `src/policy.rs` (the two-field length
cap), `src/pipeline.rs` (finalize loop), `tests/prop_length_bound.rs`,
`tests/prop_idempotence.rs`, `tests/prop_no_grapheme_splitting.rs`,
`tests/prop_totality.rs`.

**Dependencies added:** runtime — `unicode-segmentation`. Running total: 2/11.

**Tests first, and in a specific order that matters:** the `***`-shaped case from the
proposal's own §3.14 history is written as a **named, literal unit test** first
(`transform("***", default_policy()) == Unrepresentable(ReducesToEmpty)`) before the
proptest is generalized — this is the regression test for the exact defect the stage-3
review found, and a literal case is worth keeping even once the property subsumes it,
because a property-test shrinker's minimal counterexample for a future regression is not
guaranteed to be `***` again. Then Length bound (now decidable per §1.3's fix). Then
Idempotence, with the proptest generator biased toward inputs likely to need the full
3-iteration bound (repeated `.`/`-`/`_` runs interacting with truncation and reserved
stems) so `NotConverged` is not merely possible in principle but actually exercised —
addressing the risk noted in §1.1's Idempotence row.

**Exit criteria:** all four properties green; the literal `***` test passes; `--target
windows` is _not_ wired into the loop yet (matches §10's v0.1 scope: "v0.1's stage 13
fixed-point loop re-runs 9/10 only").

**Size:** 4T. This is the single largest core milestone — the proposal's own review
called out the collision engine and the journal as the parts that blow LOC estimates;
this is the third.

### M5 — `rules` (stage 5, literal-only) and the complete v0.1 transform

**Scope:** literal find/replace `[[rule]]` application (regex mode deferred to M10, since
it needs the `regex` dependency and this milestone should not spend budget early); wiring
stages 1, 3, 4, 5(literal), 7, 9, 10, 12, 13 into one `pipeline::transform` matching the
proposal's own v0.1 stage list exactly (§10).

**Files:** `crates/detoxrs-core/src/rules.rs`, `src/pipeline.rs` (final assembly),
`src/lib.rs` (public `transform` entry point).

**Dependencies added:** none.

**Exit criteria:** every §8.1 property now runs against the **assembled** v0.1 pipeline,
not a sub-pipeline — this is the milestone where "the executable specification for v0.1's
transform is fully proven" becomes a true sentence, checkable by running `cargo test -p
detoxrs-core` and reading the property names off the terminal. `--url-decode` (stage 2)
and `--target` (stage 11) are absent, matching §10's stated v0.1 boundary; the corpus
snapshot from M1 is extended to the full pipeline's `-vv`-shaped stage trace.

**Size:** 2T.

### M6 — `plan.rs`: the collision engine, in memory, no I/O

**Scope:** `Entry`, `Plan`, `PlanItem`, `Resolution`, the three-layer-minus-syscall
collision engine (layers 1 and 2's snapshot half), deterministic renumbering (`N =
2..999`), the sibling-chain assertion (§5.3's proof, encoded as a plan-time internal
consistency check), and — per §1.2's Undo-round-trip finding — the `RenameOps` **trait
definition** (not its real implementation) placed here in `detoxrs-core`, since a trait
is an interface, not I/O, and putting it here is what lets Milestone 8 write a
zero-filesystem mock against it.

**Files:** `crates/detoxrs-core/src/plan.rs`, `tests/prop_plan_no_collision.rs`,
`tests/prop_plan_no_clobber.rs`, `tests/prop_plan_order_safety.rs`,
`tests/prop_plan_no_sibling_chains.rs`, `tests/prop_plan_bounded_renumbering.rs`,
`tests/prop_plan_determinism.rs`.

**Dependencies added:** none.

**Exit criteria:** all six §8.2 plan-time properties green (Undo round-trip is deferred
to M8, since it needs the apply loop, not just `plan()`); the no-sibling-chains generator
explicitly includes near-swap pairs (`a_b`/`a-b`, `A.txt`/`a.txt` under `--case lower`) as
the property text demands, not just random inputs that are unlikely to ever land near a
swap.

**Size:** 4T (the second of the three LOC-heavy pieces the proposal's own review named).

### M7 — The binary: CLI, walk, real rename, preview — **the tool becomes usable**

**Scope:** everything needed for `detoxrs [-r] [-x] <path>...` to actually rename files
on a real Linux or macOS filesystem: `cli.rs` (clap derive), `walk.rs` (snapshot walk,
`walkdir`, VCS-metadata skip, symlink-never-descend, dotfile skip), `fsops.rs`
(`rustix::fs::renameat_with` + `RenameFlags::NOREPLACE`, the real `RenameOps` impl),
`fsops/fallback.rs` (check-then-rename), `report.rs` (human preview, `--json`, exit
codes, the `<hh>`-escape display for `Opaque` names promised in §3.4/§6.1 — this is
where that rendering actually gets written, not M1).

**Files:** `crates/detoxrs/src/{main,cli,walk,fsops,fsops/fallback,report}.rs`,
`crates/detoxrs/tests/cli/*.toml` (trycmd), `crates/detoxrs/tests/help.rs` (assert_cmd +
insta for `--help`).

**Dependencies added:** runtime — `clap` (derive), `walkdir`, `rustix` (feature `fs`),
`serde_json`. Running total: 6/11.
Dev — `assert_cmd`, `trycmd`. Running total: 4 dev-deps (proptest, insta, assert_cmd,
trycmd).

**Tests first:** the `--help` snapshot (a contract, per §8.3, so it is pinned before the
CLI struct is finalized, not after); the filesystem-matrix rows that do not depend on the
journal existing yet — case-only rename (both APFS variants, ext4, tmpfs, a
case-insensitive Linux mount), NFD->NFC rename (both APFS variants), length-limit probe
(ext4, tmpfs, both APFS variants), `RENAME_NOREPLACE` unsupported (an injectable-failure
`RenameOps` wrapper, so this row does not need genuinely exotic hardware to pass in CI —
see §6), non-UTF-8 name (Linux tmpfs only, per §8.4's own note that APFS refuses to
create one), symlink-to-`../..` escape, rename-during-walk (5000 entries).

**Exit criteria:** all of the above pass on Linux and macOS in CI; `detoxrs -x -r
<dir>` performs real, safe, no-clobber renames end to end. **This is the milestone where
a human can point the tool at a real directory and trust the result**, discussed further
in §9.

**Size:** 5T (`report.rs` alone was called out in the proposal's own estimate as 250-400
lines).

### M8 — Journal, undo, crash safety, and the TOCTOU test from §1.5

**Scope:** `journal.rs` (JSONL `intent`/`done`/`failed` protocol, fsync-before-rename),
the `undo` subcommand, SIGINT/SIGTERM handling (§5.8), and — because M6 put the
`RenameOps` trait somewhere a mock can reach it — an in-memory `RenameOps` + in-memory
journal-writer pair used to run the Undo-round-trip property (§8.2) as a fast, seeded
`proptest` rather than only as a slow real-filesystem integration test. The real-disk
half still gets its own tests: crash-mid-batch (kill after N renames, assert journal
replay identifies the exact interrupted item) and the new **TOCTOU-collision-during-apply**
row proposed in §1.5.

**Files:** `crates/detoxrs/src/journal.rs`, `src/cli.rs` (`undo` subcommand),
`detoxrs-core` gains no new production code here beyond what M6 already placed (the
`RenameOps` trait); `crates/detoxrs-core/tests/support/mem_fs.rs` (the in-memory
filesystem test double, dev-only, never compiled into the shipped binary),
`tests/prop_plan_undo_roundtrip.rs`, `crates/detoxrs/tests/crash_mid_batch.rs`,
`crates/detoxrs/tests/toctou_collision.rs`.

**Dependencies added:** none new (JSONL already covered by `serde_json` from M7).

**Exit criteria:** Undo round-trip green against the mock; crash-mid-batch and TOCTOU
tests green against real tmp directories on Linux and macOS.

**Size:** 4T (the journal was the proposal's own third named LOC-heavy piece).

### M9 — Fuzzing and the huge-tree benchmark

**Scope:** one `cargo-fuzz` target over `decode` + `transform`, oracle = the full §8.1
property set (this is the moment those properties get reused rather than re-derived, per
§1.6); a `criterion` benchmark over a 200k-entry synthetic tree (walk + plan + apply),
`#[ignore]`d by default so it never runs in the fast `gate` path, wired into a separate,
manually-triggered CI job; the fixed-point-loop iteration count is logged by the fuzz
harness under `--target windows`-shaped tight `--max-len` policies, which is §11 spike
12's own proposed closing experiment and falls out of this milestone for free.

**Files:** `fuzz/fuzz_targets/decode_transform.rs` (separate crate, its own
`Cargo.toml`), `crates/detoxrs/benches/huge_tree.rs`.

**Dependencies added:** dev — `criterion`. Running total: 5 dev-deps. The `fuzz/` crate's
own dependency (`libfuzzer-sys`) lives outside the workspace's dependency budget
entirely, since `cargo-fuzz` scaffolding is conventionally its own crate — noted, not
hidden.

**Exit criteria:** fuzz target runs a bounded smoke-test corpus clean in CI on every
push (a short, seeded run — not an unbounded nightly fuzz campaign, which is a separate,
manually-triggered job); the benchmark produces a recorded baseline number.

**Size:** 2T.

### M10 (≈ proposal's v0.2) — Config, profiles, `[[rule]]` regex, `--ascii`, `url_decode`

**Scope:** `config.rs` (discovery, first-match-wins precedence, §4.3), `[profile.*]`,
regex-mode `[[rule]]` and `--exclude` globs, `--keep`/`--strip`, `--case`, `--ascii`
(stage 6), stage 2 (`url_decode`, all-or-nothing per §3.11), and `--print-config` with
its two hard requirements from §4.3 (resolve-don't-echo; validate-everything-compilable,
exit 2 not exit 0 on an unrunnable config — this is the direct fix for upstream's `-L`
failure mode and gets its own golden test, not just prose).

**Files:** `crates/detoxrs/src/config.rs`, `crates/detoxrs-core/src/rules.rs` (regex
mode added), extended `pipeline.rs` (stages 2, 6).

**Dependencies added:** runtime — `serde`, `toml`, `regex`, `deunicode`. Running total:
10/11.

**Tests first:** the all-or-nothing url-decode property extended into
`prop_pipeline_stage2.rs` from M2 (a malformed escape must leave the whole name
untouched, per §3.11's `50%-70%.txt` example — a literal test case); the
`--print-config` exit-2-on-invalid-rule golden test, written _before_ `--print-config`'s
happy path, specifically because that ordering is what stops the upstream `-L` mistake
from being reproduced by accident.

**Exit criteria:** `just dep-budget` at 10/11, explicitly checked and recorded (one slot
of headroom remains for `terminal_size`, per §7.2's own undecided 11th line — this plan
recommends resolving that question in this milestone rather than carrying it further,
since `report.rs` already exists by M7 and the column-alignment question is answerable
by then).

**Size:** 5T.

### M11 (≈ proposal's v0.3) — `--target`, plan files, `--stdin`, per-directory limits

**Scope:** `reserved.rs` (Windows reserved stems + illegal-character set, §6.5's
conservative default, applied only under `--target windows/portable`, joining the
fixed-point loop as stage 11 exactly as §10 specifies for this later milestone);
`--plan-out`/`apply` with the `(dev, ino, mtime)` stale-plan recheck (§5.7); `--stdin`;
per-directory length-limit detection via `rustix::fs::statfs` feeding the two-field
`Policy` from M4's fix; `clap_complete`/`clap_mangen` output.

**Files:** `crates/detoxrs-core/src/reserved.rs`, `crates/detoxrs/src/limits.rs`,
plan-file (de)serialization in `crates/detoxrs/src/report.rs` or a new `planfile.rs`.

**Dependencies added:** dev — `clap_complete`, `clap_mangen`. Running total: 7 dev-deps
(matches the proposal's §7.2 dev-dependency list exactly, in full).

**Exit criteria:** stage 11 joins the finalize loop and the fixed-point-bound fuzz
harness from M9 is re-run under `--target windows` (this is where spike 12's "richest
interaction corner" actually gets exercised, since stage 11 wasn't implemented until
now); stale-plan detection has its own trycmd case (mutate the tree between `--plan-out`
and `apply`, assert refusal).

**Size:** 4T.

### M12 (≈ proposal's v1.0 hardening) — The full filesystem matrix and spike closures

**Scope:** every §8.4 row (plus the two additions from §1.5 and §6) running in CI across
the full Linux+macOS matrix described in §6; spike 13, 14, 15 closures (macOS incapable
volume, Linux case-insensitive-mount case-only rename, `nlink > 1` respell) as concrete
CI-run tests rather than open questions; `MIGRATING-FROM-DETOX.md`, `--help-transforms`;
packaging items 1-4 from §9.4 (out of this plan's test-driven remit except insofar as
release automation itself gets smoke-tested, which it already is per
`docs/rust-setup-release.md`).

**Files:** CI workflow additions only (`.github/workflows/`), no new `detoxrs`/
`detoxrs-core` source files expected — if this milestone needs new source files, that is
itself a signal a property was missing earlier, not a normal part of this milestone's
scope.

**Dependencies added:** none.

**Exit criteria:** §8.4's table, with the §1.5 and §6 additions, is a literal checklist
against CI job names; every row is either green in CI or explicitly marked untested with
the reason (per owner decision — no row may claim verified Windows/NTFS/exFAT behavior).

**Size:** 3T (mostly CI/infrastructure, not new logic).

---

## 4. Milestone 1 in full detail

### 4.1 `Cargo.toml` changes

```toml
# crates/detoxrs-core/Cargo.toml — add:
[dev-dependencies]
proptest = "1"
insta = "1"
```

No `[dependencies]` change. `just dep-budget` counts `[dependencies]` only (confirmed by
reading the recipe: it unions `dependencies` tables across `crates/*/Cargo.toml`), so
this milestone does not move the 0/11 counter.

### 4.2 `crates/detoxrs-core/src/policy.rs` (new, minimal)

```rust
//! Policy: every field maps 1:1 to a flag and a config key (proposal §3.1).
//! Grown incrementally; this milestone needs only enough for `decode`'s
//! signature to compile.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    // Fields land as the stages that need them are implemented (M2 onward).
    // `decode` does not currently branch on any field; the parameter exists
    // because §3.1 specifies `decode(raw: &OsStr, p: &Policy) -> Decoded`,
    // and this plan keeps signatures matching the proposal rather than
    // dropping a parameter now and re-adding it in M4.
}
```

### 4.3 `crates/detoxrs-core/src/decode.rs` (new)

```rust
//! Stage 1: OsStr -> text, or Opaque. No legacy decoder exists in this
//! binary (owner decision 2026-07-31; proposal §3.4). This is the entire
//! stage.

use crate::policy::Policy;
use std::ffi::OsStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    Utf8(String),
    Opaque,
}

#[must_use]
pub fn decode(raw: &OsStr, _policy: &Policy) -> Decoded {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        match std::str::from_utf8(raw.as_bytes()) {
            Ok(s) => Decoded::Utf8(s.to_owned()),
            Err(_) => Decoded::Opaque,
        }
    }
    #[cfg(windows)]
    {
        // WTF-8 over UTF-16 (§6.1): unpaired surrogates cannot form a valid
        // `&str`, so `OsStr::to_str` already gives exactly the Utf8/Opaque
        // split this stage needs.
        match raw.to_str() {
            Some(s) => Decoded::Utf8(s.to_owned()),
            None => Decoded::Opaque,
        }
    }
}
```

### 4.4 `crates/detoxrs-core/src/lib.rs` (rewritten)

Removes `placeholder_version` (its own doc comment already says "delete once the
pipeline needs a real public API" — this milestone is that moment) and wires the new
module:

```rust
#![forbid(unsafe_code)]

mod decode;
mod policy;

pub use decode::{decode, Decoded};
pub use policy::Policy;
```

### 4.5 The fixture corpus: `crates/detoxrs-core/tests/support/corpus.rs` (new)

This is the answer to the task's fixture-corpus requirement (expanded fully in §5, since
it is reused by every later milestone) — introduced here because M1 is the first
consumer.

```rust
//! Fixture corpus for §8.3's required nasty-name list. Byte-string literals,
//! not files-on-disk-named-this: some of these are not valid UTF-8, and APFS
//! refuses to create such a name at the syscall level (proposal §3.4,
//! doc 05's Load-Bearing Uncertainties), so "check in a file with this
//! literal name" is not portable across the Linux+macOS test hardware this
//! project has. A Rust byte-string literal has no such restriction: `b"..."`
//! is `&'static [u8]`, not `&str`, so it can hold a lone invalid byte just
//! as easily as it holds `café.txt`.

pub struct Fixture {
    pub label: &'static str,
    pub bytes: &'static [u8],
    /// False for names that cannot be created as a real directory entry on
    /// at least one of this project's tier-1 filesystems (APFS, per the
    /// citation above). Filesystem-matrix tests (M7 onward) filter on this;
    /// pure in-memory tests (proptest/insta, from this milestone on) ignore
    /// it entirely, since they never touch a filesystem.
    pub disk_constructible_everywhere: bool,
}

pub const ALL: &[Fixture] = &[
    Fixture { label: "cafe_nfc", bytes: b"caf\xc3\xa9.txt", disk_constructible_everywhere: true },
    Fixture { label: "cafe_nfd", bytes: b"cafe\xcc\x81.txt", disk_constructible_everywhere: true },
    Fixture { label: "bidi_202e", bytes: "\u{202E}evil.txt".as_bytes(), disk_constructible_everywhere: true },
    Fixture { label: "zwsp_200b", bytes: "a\u{200B}b.txt".as_bytes(), disk_constructible_everywhere: true },
    Fixture { label: "ascii_300byte", bytes: &[b'a'; 300], disk_constructible_everywhere: false /* exceeds ext4 too, deliberately */ },
    Fixture { label: "astral_emoji_128", bytes: "\u{1F600}".repeat(128).as_bytes().to_vec().leak(), disk_constructible_everywhere: false },
    Fixture { label: "con_txt", bytes: b"CON.txt", disk_constructible_everywhere: true },
    Fixture { label: "hidden_file", bytes: b".hidden file", disk_constructible_everywhere: true },
    Fixture { label: "dotdot_weird", bytes: b"..weird..name..", disk_constructible_everywhere: true },
    Fixture { label: "percent_20", bytes: b"100%20done.txt", disk_constructible_everywhere: true },
    Fixture { label: "percent_25", bytes: b"100%25 done.txt", disk_constructible_everywhere: true },
    Fixture { label: "percent_malformed", bytes: b"50%-70%.txt", disk_constructible_everywhere: true },
    Fixture { label: "libstdcpp", bytes: b"libstdc++.so", disk_constructible_everywhere: true },
    Fixture { label: "music_separator", bytes: b"a_-_b.mp3", disk_constructible_everywhere: true },
    Fixture { label: "icon_cr", bytes: b"Icon\r", disk_constructible_everywhere: true },
    Fixture { label: "lone_dash", bytes: b"-", disk_constructible_everywhere: true },
    Fixture { label: "all_punctuation", bytes: b"***", disk_constructible_everywhere: true },
    Fixture { label: "cp1252_bjork", bytes: b"Bj\xf6rk - Vespertine.mp3", disk_constructible_everywhere: false /* invalid UTF-8: APFS refuses */ },
    Fixture { label: "invalid_utf8_lone_ff", bytes: b"\xff", disk_constructible_everywhere: false },
];
```

(The exact `leak()` use for the repeated-emoji entry is a placeholder shown for
concreteness; the real implementation may instead generate such entries
programmatically in the test rather than as `const` data — either is fine, and this
plan does not mandate one over the other. What it does mandate is: no entry in this
table is ever materialized as an actual filename during a pure/property/snapshot test,
and `disk_constructible_everywhere: false` is exactly the set excluded from real-disk
fixture use from M7 onward.)

### 4.6 `crates/detoxrs-core/tests/support/mod.rs`

```rust
pub mod corpus;

#[cfg(unix)]
pub fn os_str_from_bytes(bytes: &[u8]) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(bytes.to_vec())
}

#[cfg(windows)]
pub fn os_str_from_bytes(bytes: &[u8]) -> std::ffi::OsString {
    // Windows OsStr is WTF-8-over-UTF-16 (§6.1); round-tripping arbitrary
    // bytes through it for test purposes goes through a lossy WTF-8 decode.
    // Acceptable here because this helper backs unit/property tests that run
    // on Unix in CI (per owner decision, no verified Windows behavior is
    // claimed); it exists so the module compiles on Windows's best-effort
    // CI tier rather than to assert anything about it.
    use std::os::windows::ffi::OsStringExt;
    let wide: Vec<u16> = bytes.iter().map(|&b| u16::from(b)).collect();
    std::ffi::OsString::from_wide(&wide)
}
```

### 4.7 `crates/detoxrs-core/tests/prop_decode.rs` (new — written before `decode.rs` is finalized)

```rust
mod support;

use detoxrs_core::{decode, Decoded, Policy};
use proptest::prelude::*;
use support::os_str_from_bytes;

proptest! {
    #[test]
    fn decode_is_total_and_never_reinterprets(bytes in prop::collection::vec(any::<u8>(), 0..64)) {
        let os = os_str_from_bytes(&bytes);
        let policy = Policy::default();
        let result = decode(&os, &policy);
        match std::str::from_utf8(&bytes) {
            Ok(s) => prop_assert_eq!(result, Decoded::Utf8(s.to_owned())),
            Err(_) => prop_assert_eq!(result, Decoded::Opaque),
        }
    }
}
```

### 4.8 `crates/detoxrs-core/tests/snap_decode_corpus.rs` (new)

```rust
mod support;

use detoxrs_core::{decode, Policy};
use support::{corpus, os_str_from_bytes};

#[test]
fn decode_corpus_classification() {
    let policy = Policy::default();
    let rendered: Vec<String> = corpus::ALL
        .iter()
        .map(|f| {
            let os = os_str_from_bytes(f.bytes);
            format!("{}: {:?}", f.label, decode(&os, &policy))
        })
        .collect();
    insta::assert_snapshot!(rendered.join("\n"));
}
```

This snapshot is the classification table only (Utf8 vs Opaque per corpus entry) — the
`<hh>`-escaped _rendering_ of an Opaque name for a human preview is `report.rs`'s job and
gets its own snapshot in M7. Conflating the two here would make this milestone depend on
code that does not exist yet.

### 4.9 Exit criteria, exact commands

```
cargo test -p detoxrs-core            # decode property + snapshot, green
just gate                             # fmt-check, clippy -D warnings, test, msrv, dep-budget
just dep-budget                       # still 0/11 direct runtime dependencies
```

`cargo insta review` accepts the new snapshot once, and the accepted `.snap` file is
committed alongside the test.

---

## 5. The test-fixture corpus (full answer)

Already introduced in §4.5; stated here as the general policy for every later milestone,
since §8.3 requires the same corpus to serve multiple test kinds with different
constraints:

1. **Canonical storage: a Rust source file of byte-string constants**
   (`crates/detoxrs-core/tests/support/corpus.rs`), never an on-disk file whose _name_ is
   the fixture. `b"..."` literals are `&[u8]`, immune to the "must be valid UTF-8" and
   "must be a legal name on this filesystem" constraints that block every other storage
   choice (a real file named `\xff`, a `.txt` containing that name as text, a tarball of
   real files). This is git-diffable, text-editable, requires no binary blob, and is the
   one representation that is simultaneously valid Rust source, valid UTF-8 _as source_,
   and capable of representing arbitrary invalid-UTF-8 _data_.
2. **Per-entry disk-constructibility flag.** Each fixture states whether it can be
   materialized as a real directory entry on every tier-1 filesystem this project tests
   against. Invalid-UTF-8 entries are `false` unconditionally (APFS refuses them
   outright, per §3.4/doc 05); oversized entries are `false` where they are deliberately
   larger than every tier-1 limit, since their purpose is to exercise truncation logic in
   memory, not to prove a filesystem accepts a 300-byte name (it will not, on any tier-1
   filesystem, which is the point of the test).
3. **Consumption by test kind:**
   - Pure/property/unit tests (M1-M6): read the byte constants directly, build an
     `OsStr` via `OsStr::from_bytes`/`OsString::from_vec` in memory, never touch a
     filesystem. No constructibility filter applies.
   - Snapshot tests (M1, M5, M7): render text output (classification tables, `-vv`
     traces, `<hh>`-escaped previews). The _rendered_ output is always valid UTF-8/ASCII
     even when the _input_ is not, so `.snap` files stay ordinary text regardless of
     which corpus entries feed them.
   - Filesystem-matrix tests (M7 onward): filter `corpus::ALL` on
     `disk_constructible_everywhere`, then create each surviving entry via
     `OsStr::from_bytes`/`OsString::from_vec` passed straight to `std::fs`/`rustix`, never
     through a `&str` step that would reject or mangle it. Entries filtered out are
     logged by name in the test's own output (`eprintln!` or a `#[test]`-level assertion
     that the skip count matches an expected number) so a matrix run's coverage is
     auditable, not silently thin — a corpus entry quietly never running anywhere is
     exactly the failure mode "no built-in default exclude list" (§2.2) warns readers
     about in a different context, and it is worth guarding against here too.
4. **Fuzz corpus seeding (M9):** `cargo-fuzz` reads raw bytes from files under
   `fuzz/corpus/decode_transform/`; those files' _names_ are ordinary ASCII
   (`seed_bidi_202e`, `seed_invalid_utf8_ff`, ...) and their _contents_ are the fixture
   bytes, generated once from `corpus::ALL` by a small `xtask`/test helper rather than
   hand-copied, so the fuzz seed set never drifts from the property-test corpus it is
   supposed to be seeding.

No corpus entry is ever checked in as a file whose name is the payload. That is the
whole engineering answer, and it costs nothing extra to maintain because it is Rust
source like everything else in the crate.

---

## 6. The filesystem matrix, mapped to Linux + macOS

Per owner decision, no Windows machine, no NTFS/exFAT volume. Every row below is
annotated with where it actually runs.

| §8.4 row (+ this plan's additions)                                                         | Runs locally (owner's Linux/macOS)                                                                                              | Runs in CI                                                                                                                                                                                                                                 | Notes                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case-only rename                                                                           | Yes, all five sub-cases                                                                                                         | Yes — macOS runners for both APFS variants (`hdiutil` ephemeral images, per doc 06's own method); `ubuntu-latest` for ext4/tmpfs and a loopback-mounted case-insensitive mount (`vfat`/`exfat` image, or `ext4` with the casefold feature) | Fully coverable with hardware that exists.                                                                                                                                                                                                                                                                                                                                                              |
| NFD -> NFC rename                                                                          | Yes                                                                                                                             | Yes, macOS only (both APFS images)                                                                                                                                                                                                         | Same `hdiutil` mechanism as above.                                                                                                                                                                                                                                                                                                                                                                      |
| Length limit probe                                                                         | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | Four-way discriminator per doc 06 Test 1, encoded as an assertion, not just an ad hoc probe.                                                                                                                                                                                                                                                                                                            |
| `RENAME_NOREPLACE` unsupported                                                             | Yes, via an **injectable-failure** `RenameOps` wrapper                                                                          | Yes                                                                                                                                                                                                                                        | This plan deliberately does not require an exotic real mount for the CI-gated version of this row — an injected `EINVAL`/`ENOSYS`/`EOPNOTSUPP` from a test double proves the fallback-and-warn path works, which is the thing this row is actually testing. The _real-mount_ version (does an actual overlayfs/NFS/CIFS mount return one of those errnos) is spike 2's job, not this row's — see below. |
| Non-UTF-8 name                                                                             | Yes                                                                                                                             | Yes, Linux only (`ubuntu-latest`, tmpfs)                                                                                                                                                                                                   | APFS rejects the name outright; macOS is correctly excluded from this row, not silently skipped.                                                                                                                                                                                                                                                                                                        |
| Symlink to `../..` under `-r`                                                              | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | No exotic setup needed.                                                                                                                                                                                                                                                                                                                                                                                 |
| Rename-during-walk (5000 entries)                                                          | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | Synthetic tree, no filesystem dependency beyond "a filesystem."                                                                                                                                                                                                                                                                                                                                         |
| Crash mid-batch                                                                            | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | Subprocess + `SIGKILL` after N journal `done` records; standard on both.                                                                                                                                                                                                                                                                                                                                |
| Huge tree (200k entries)                                                                   | Yes                                                                                                                             | Yes, but `#[ignore]`d from `gate`, run in a separate scheduled/manual job                                                                                                                                                                  | Slow by design; `criterion` records a baseline, does not gate merges.                                                                                                                                                                                                                                                                                                                                   |
| **TOCTOU collision during apply** (§1.5, new)                                              | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | No special hardware; just a test that mutates the tree between snapshot and apply.                                                                                                                                                                                                                                                                                                                      |
| Spike 13 (macOS volume lacking `VOL_CAP_INT_RENAME_EXCL`)                                  | Yes                                                                                                                             | Yes, macOS only                                                                                                                                                                                                                            | An HFS+ or exFAT `hdiutil` image lacks the capability; closeable in CI with existing tooling.                                                                                                                                                                                                                                                                                                           |
| Spike 14 (Linux case-only rename, case-insensitive mounts)                                 | Yes for `ext4`-casefold, `vfat`, `exfat` (loopback images)                                                                      | Yes, same subset                                                                                                                                                                                                                           | `ntfs3` and CIFS need a kernel module or a server not guaranteed present on hosted runners — **owner-run, local-only**, not CI-gated; documented as such, not silently assumed passing.                                                                                                                                                                                                                 |
| Spike 15 (`nlink > 1` respell)                                                             | Yes                                                                                                                             | Yes, both OSes                                                                                                                                                                                                                             | `std::fs::hard_link` + rename; no exotic hardware required at all — this plan promotes it from "open question" to "ordinary CI row" outright.                                                                                                                                                                                                                                                           |
| Spike 2's full matrix (ext4/xfs/btrfs/tmpfs/overlayfs/NFSv4/CIFS/ZFS/old-kernel container) | ext4/xfs/btrfs/tmpfs/overlayfs: yes, via loopback images with root/sudo (available on both the owner's box and `ubuntu-latest`) | ext4/xfs/btrfs/tmpfs/overlayfs: yes                                                                                                                                                                                                        | NFSv4/CIFS/ZFS need a running server or kernel module not reliably present on hosted runners. **These three remain an owner-run, manually-triggered exercise**, recorded as closeable-not-closed per the proposal's own §11 framing — this plan does not claim to close them by CI automation alone, only to close the five that loopback images can reach.                                             |
| Spike 5 (case-only rename on network filesystems)                                          | Only if the owner mounts SMB/NFS locally                                                                                        | No                                                                                                                                                                                                                                         | Test code is written and gated behind an environment variable naming a mount point; skipped (loudly, with a named reason) when the variable is absent, so it is ready the moment infrastructure exists without ever blocking CI.                                                                                                                                                                        |
| Spikes 3, 4 (Windows 11 reserved names; NTFS/exFAT limits)                                 | No                                                                                                                              | No                                                                                                                                                                                                                                         | **Cannot run at all.** No Windows machine, no NTFS/exFAT volume exists in this project's hardware. The existing `test-windows` CI job (per `docs/rust-setup-ci.md`) stays `continue-on-error`, compile-and-unit-test only. Nothing in this plan's test suite may claim verified Windows filesystem behavior, per owner decision, and none of the above rows attempts to.                                |

---

## 7. Dependency and dev-dependency schedule

Runtime (cap: 11, enforced by `just dep-budget`):

| Milestone | Adds                                      | Running total |
| --------- | ----------------------------------------- | ------------- |
| M1-M2     | (none)                                    | 0/11          |
| M3        | `unicode-normalization`                   | 1/11          |
| M4        | `unicode-segmentation`                    | 2/11          |
| M5-M6     | (none)                                    | 2/11          |
| M7        | `clap`, `walkdir`, `rustix`, `serde_json` | 6/11          |
| M8-M9     | (none)                                    | 6/11          |
| M10       | `serde`, `toml`, `regex`, `deunicode`     | 10/11         |
| M11       | (none)                                    | 10/11         |
| M12       | (none)                                    | 10/11         |

One slot of headroom remains at every milestone from M10 onward for `terminal_size`
(§7.2's own undecided 11th line). This plan recommends resolving that question inside
M10, since `report.rs` exists by M7 and the column-alignment question is answerable
without waiting further — carrying an open dependency slot past v0.2 for a question that
was answerable at v0.1 is the kind of drift a "<= 11" budget is supposed to prevent
becoming a rounding error nobody revisits.

Dev-only (not counted against the runtime cap, but every one appears in supply-chain
scans — `cargo audit`/`cargo deny`/`cargo vet`/Trivy all already run against the whole
lockfile per `docs/rust-setup-supply-chain.md`, so "dev-only" does not mean "free," it
means "not packaged into the shipped binary"):

| Milestone | Adds                           | Running total | Why                                                                                                                                                            |
| --------- | ------------------------------ | ------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1        | `proptest`, `insta`            | 2             | The two tools this entire plan is built on: property tests against `transform`, snapshots for the corpus and `--help`.                                         |
| M7        | `assert_cmd`, `trycmd`         | 4             | CLI-level golden tests only become possible once a CLI exists.                                                                                                 |
| M9        | `criterion`                    | 5             | The huge-tree benchmark; deferred to its own milestone so it is not added before there is anything worth benchmarking.                                         |
| M11       | `clap_complete`, `clap_mangen` | 7             | Shell completions/man-page generation, tied to the CLI surface stabilizing at the `--target`/plan-files milestone, matching the proposal's own v0.3 placement. |

Final dev-dependency count: **7**, matching proposal §7.2's dev-dependency list exactly
(`proptest`, `insta`, `trycmd`, `assert_cmd`, `criterion`, `clap_complete`,
`clap_mangen`), introduced incrementally rather than all at M1 — the setup notes'
recorded reason for deferring all of them ("no application logic yet to test") stops
applying to each one exactly at the milestone that gives it something to test, which is
the whole point of scheduling them instead of adding them in a batch.

The `fuzz/` crate (M9) is a separate Cargo workspace member with its own
`libfuzzer-sys` dependency; it is outside the `<= 11` runtime budget (which is about the
_shipped binary's_ packaging cost, per §7.2's own Debian-source-package rationale, and
`cargo-fuzz` targets are never shipped) but is still real supply-chain surface and is
named here rather than left for someone to discover it exists.

---

## 8. Spike handling

| Spike                                              | Closed by this plan's test suite as a side effect?                                                                                                                                                                                                                              | Where                                                                                     |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| 1 (name availability)                              | No — not a test-suite question at all.                                                                                                                                                                                                                                          | Out of scope for this plan.                                                               |
| 2 (`renameat2` matrix)                             | **Partially.** ext4/xfs/btrfs/tmpfs/overlayfs close via loopback images in CI (M7/M12); NFSv4/CIFS/ZFS remain an owner-run manual exercise.                                                                                                                                     | M7, M12; see §6.                                                                          |
| 3, 4 (Windows 11 names; NTFS/exFAT limits)         | No — no hardware exists to run them.                                                                                                                                                                                                                                            | Stays open per owner decision.                                                            |
| 5 (case-only rename on network FS)                 | No, unless the owner supplies a mount point; test code is ready either way.                                                                                                                                                                                                     | M12 (env-var-gated).                                                                      |
| 6 (CP1252 repair measurement)                      | Moot; not this plan's concern.                                                                                                                                                                                                                                                  | N/A.                                                                                      |
| 7 (NFC-rewrite collision rate on real trees)       | No — needs real user trees, not synthetic property inputs.                                                                                                                                                                                                                      | Out of scope.                                                                             |
| 8 (auto-numbering the right default)               | No — needs real usage feedback.                                                                                                                                                                                                                                                 | Out of scope.                                                                             |
| 9 (parallelism)                                    | Partially — the M9 benchmark produces the data a human decision needs, but does not make the decision.                                                                                                                                                                          | M9.                                                                                       |
| 10 (aggregate packaging count)                     | No.                                                                                                                                                                                                                                                                             | Out of scope.                                                                             |
| **11 (`Unrepresentable` frequency)**               | No — needs a count over real trees, which no synthetic test can substitute for; the _code path_ itself is fully covered regardless (M4).                                                                                                                                        | Out of scope for closing the question; in scope for making the mechanism safe either way. |
| **12 (fixed-point bound of 3)**                    | **Yes**, as a direct side effect of M9's fuzz harness logging iteration counts under `--target windows`-shaped tight `--max-len` policies — this is the second of the two spikes this plan closes essentially for free by having built the property machinery it needed anyway. | M9, M11 (once stage 11 exists to make the interaction rich).                              |
| 13 (macOS incapable-volume errno)                  | **Yes**, via an `hdiutil` HFS+/exFAT image, CI-runnable on macOS.                                                                                                                                                                                                               | M12.                                                                                      |
| 14 (Linux case-insensitive-mount case-only rename) | **Partially** — `ext4`-casefold/`vfat`/`exfat` yes; `ntfs3`/CIFS no (no server/module guaranteed on hosted runners).                                                                                                                                                            | M12.                                                                                      |
| **15 (`nlink > 1` respell)**                       | **Yes**, fully — this plan promotes it from "open question" to an ordinary hardlink-plus-rename test with no exotic requirements.                                                                                                                                               | M12.                                                                                      |

Two spikes (12, 15) close outright as a consequence of building the test suite this plan
specifies; one (13) closes with existing macOS tooling; two (2, 14) close partially,
with the unclosed remainder named rather than silently dropped.

---

## 9. Why this order, and what it costs

**The alternative orderings, and why this plan rejects them for v1.0's actual hazard
profile:**

- **Safety-architecture-first** (build `fsops`/`journal`/`apply` before `transform`'s
  contract is fixed) risks building real I/O machinery — the crash-safety protocol, the
  syscall fallback ladder, the undo journal — around a transform whose behavior is still
  moving. Every one of those pieces has to be re-tested whenever a pipeline stage
  changes shape, because they consume `PlanItem`s built from `Outcome`s built from
  `transform`. Fixing the contract with property tests first (M1-M6) means the I/O layer
  (M7-M8) is built once, against a target that has already been proven not to produce an
  unsafe name — which is the actual thing that matters, since the failure this project
  exists to prevent (`café.txt -> cafÃ©.txt`, `rnr`'s panic on non-UTF-8) is a transform
  bug, not an I/O bug.
- **Thin-vertical-slice-first** (get `detoxrs -x ~/Downloads` doing _something_ on day
  one) produces a demo faster, at the cost of validating Safety closure, Idempotence, and
  the collision engine only against hand-picked example tests until whatever milestone
  eventually gets around to the property suite — which, on that ordering, is naturally
  the _last_ thing done, since it is the least demo-visible. That is precisely the
  ordering that let detox ship `café.txt -> cafÃ©.txt` and let `rnr` ship a
  `.unwrap()` panic on a non-UTF-8 name: both are transforms that were tested by example,
  not by property, and both examples looked fine until a byte sequence nobody thought to
  hand-write came along. This plan's mandate is explicitly to not repeat that.

**What this plan's ordering costs, stated plainly rather than argued away:** there is no
runnable binary — nothing a human can point at a real directory — until **Milestone 7**.
Milestones 1 through 6 (roughly half the plan, and the majority of `detoxrs-core`'s
logic) produce no user-visible artifact at all; they produce a fully property-tested
pure library that nothing outside its own test suite calls yet. A stakeholder who wants
to see the tool "do something" has six milestones to wait through, and every one of them
looks, from outside `cargo test`'s output, like nothing happened. That is a real cost
against a thin-slice plan's early demo, and this plan does not minimize it: **the tool
is not usable by a human until M7**, by design, because M7 is exactly where the pipeline
this plan has spent six milestones proving safe finally touches a real filesystem for
the first time — and by that point, the two things a demo would have been implicitly
vouching for (the transform is safe, the collision engine cannot corrupt data) are
already proven rather than assumed. The trade this plan makes is: slower to a demo,
faster to a demo you don't have to re-verify by hand every time a stage's behavior
shifts.
