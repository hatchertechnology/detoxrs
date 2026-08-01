---
plan: unified draft (the one that gets implemented)
date: 2026-07-31
supersedes-for-execution:
  - docs/plans/plan-a-safety-first.md
  - docs/plans/plan-b-thin-slice.md
  - docs/plans/plan-c-test-driven.md
authority-order:
  - docs/owner-decisions.md
  - docs/research/00-proposal-rust-detox-successor.md (incl. Review record + Propagation record, stage 3)
  - docs/rust-setup-{notes,ci,governance,release,supply-chain}.md
verified-on-this-machine-2026-07-31:
  - "rustix 1.1.4 renameat_with(NOREPLACE) under #![forbid(unsafe_code)] on APFS: Ok(()) on a
    same-inode case-only respell; Err(EEXIST/17) onto a distinct occupied destination"
  - "APFS refuses invalid-UTF-8 filenames at the syscall level (EILSEQ/errno 92), for both
    b'bad\\xffname.txt' and b'Bj\\xf6rk - Vespertine.mp3'"
  - "no unsafe-free signal-handler registration exists: std exposes none, and rustix's
    kernel_sigaction is `pub unsafe fn` and linux_raw-only"
  - "just dep-budget reads only the [dependencies] table of crates/*/Cargo.toml; currently 0/11"
spine: >
  Plan B's spine — the tool is usable at M1 — carrying Plan A's safety envelope unabridged and
  Plan C's property suite as M1's gate rather than as later milestones. The three plans do not
  actually disagree about what to build or in what dependency order; they disagree about when a
  binary exists. Plan C's own argument for withholding it ("prove the transform before I/O touches
  it") is a claim about the order in which code and tests are written, which is a per-commit
  discipline, not a milestone boundary: transform and plan are pure, so their property tests need
  no CLI, and writing them first inside one milestone buys the identical guarantee at none of the
  cost. Plan A's argument for a thin M1 transform is weaker than Plan A itself admits — its own
  retrofit table rates every deferred transform stage "Low", which is the definition of something
  there is no safety reason to defer. So M1 is the proposal's own v0.1 (§10) whole: stages 1, 3, 4,
  7, 9, 10, 12, 13, the full collision engine, the crash-safe journal, undo, kernel-level
  no-clobber rename — gated by every §8.1/§8.2 property that its stage set makes non-vacuous, by
  the filesystem matrix rows it exercises, and by a `kill -9` crash test (Plan A's exit criterion,
  adopted verbatim). The owner has confirmed this is a real publicly packaged tool intended as the
  successor detox users migrate to, and that cuts toward B's spine rather than against it: spikes
  7, 8 and 11 close only against real user trees, `user_feedback_online.md` records that detox
  generates almost no public discussion, and the way to get any signal at all is to put a safe,
  honest, narrower tool in front of people early. What we give up is stated in §2.
---

# Unified draft plan: detoxrs to v1.0

## 1. Adjudication record

| Decision point                     | Plan A position                                                        | Plan B position                                                     | Plan C position                                                       | Chosen                                                   | Reason                                                                                                                                                                                                                                                                                       |
| ---------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Spine / build order                | Safety envelope first, thin transform                                  | Thin vertical slice, usable immediately                             | Executable specification first, 12 milestones                         | **B's spine**                                            | A and B build the same safety code in the same order; A only narrows the transform, and rates every deferral "Low" retrofit cost itself. C's ordering buys no correctness a within-milestone TDD rule does not (see §2).                                                                     |
| When the tool becomes usable       | M1, but barely (control chars + leading dash only)                     | M1, fully                                                           | M7                                                                    | **M1, fully**                                            | Spikes 7, 8, 11 close only on real trees; the owner's "real publicly packaged tool" call makes early real-tree signal load-bearing, not a nicety.                                                                                                                                            |
| M1 transform breadth               | Stages 1, 4, part of 7, 10, 13                                         | Stages 1, 3, 4, 7, 9, 10, 12, 13                                    | Stages 1, 3, 4, 5(literal), 7, 9, 10, 12, 13 by M5                    | **1, 3, 4, 7, 9, 10, 12, 13** (B)                        | This is the proposal's own v0.1 list (§10). A's narrowing produces a binary that does not visibly clean filenames; C adds literal `[[rule]]` before there is a config file to hold a rule.                                                                                                   |
| Length bound decidability (C §1.3) | not raised                                                             | uses `max_len_bytes`/`max_len_utf16` already, without arguing it    | **defect: split into two concrete fields**                            | **Accepted, with one clause rejected**                   | The arithmetic checks out (130 astral emoji = 260 UTF-16 units, 520 bytes). Rejected: C's "`--max-len N` sets both fields to N", which silently over-truncates on a volume whose unit is known. See §5.1.                                                                                    |
| Stage independence seam (C §1.4)   | acknowledges the property is vacuous early, no seam                    | not raised                                                          | **defect: every stage a substitutable `&str -> String` in a list**    | **Accepted in substance, rejected in mechanism**         | The gap is real. But stages 12 and 13 do not have that signature (12 takes stem/ext/limits, 13 is a loop over 9/10/11), so a uniform function list is a false abstraction. A stage mask over the linear stages, plus direct tests for 12/13, is smaller and honest. See §5.2.                |
| TOCTOU / apply-time clobber (C §1) | crash test at M1, no TOCTOU row                                        | not raised                                                          | **defect: new filesystem-matrix row**                                 | **Accepted in full**                                     | §8.2's property is written against `plan()`, which has no I/O; §8.4 has no row for the fresh `symlink_metadata` recheck. The guarantee's TOCTOU half is currently untested anywhere. Cheap to fix.                                                                                           |
| `RenameOps` trait location         | binary crate (`fsops.rs`)                                              | binary crate (`fsops.rs`)                                           | `detoxrs-core`, so a mock can reach it                                | **Binary crate** (A/B)                                   | C's goal is right and its means unnecessary: an in-memory `RenameOps` is reachable from the binary crate's own `#[cfg(test)]` module. Moving a `&Path`-shaped trait into the pure crate leaks filesystem vocabulary into a crate whose whole point is not having any.                        |
| Invisible-character set at M1      | build-time UCD generator at M1                                         | named codepoint set at M1, generator later                          | build-time UCD generator at M3                                        | **Named set at M1; generator at M4**                     | The named set covers the whole CVE-2021-42574 class the stage exists for. The generator is amortized: `scripts.rs`'s UCD Script table (§3.12, needed at M4) uses the same generator, so building it at M4 is one generator for two tables instead of one generator, twice-visited.           |
| Length limit source at M1          | truncation deferred to M2 entirely                                     | hardcoded 255 bytes AND 255 UTF-16 units                            | two concrete fields, `statfs` detection at M11                        | **Hardcoded 255/255 in the two-field shape**             | §3.10's own table: ext4 is 255 bytes, APFS 255 UTF-16 units. Both tier-1 platforms are exactly 255, so the constant is wrong only on filesystems nobody is running yet, and only in the over-truncating direction.                                                                           |
| SIGINT/SIGTERM handling            | "std's raw signal primitives plus a static flag", no dependency        | flagged unresolved; would defer a milestone rather than add a crate | M8, feasibility not addressed                                         | **Rejected from v1.0 entirely**                          | A's claim is false: verified today that std exposes no signal API and rustix's `kernel_sigaction` is `unsafe` and Linux-only. Any handler costs the last budget slot for a cosmetic. The journal's `intent`/fsync/rename/`done` protocol already covers the strictly harsher `kill -9` case. |
| Journal crash test rigor           | **`kill -9`, ideally a power-cut simulation, as an M1 exit criterion** | crash-mid-batch in M1's `trycmd` set                                | subprocess + SIGKILL at M8                                            | **A's, verbatim, at M1**                                 | A correctly names this as the one property the whole design is staked on and the one bug discovered by losing a file rather than by a failing test. `SIGKILL`, not `SIGTERM`.                                                                                                                |
| Estimation unit                    | 1 unit ≈ 100-150 reviewed lines incl. tests                            | story points, ~50 lines each                                        | T = non-test lines ÷ 60                                               | **A's unit, recalibrated in §3**                         | A's unit is anchored to §7.3's checkable LOC budget. B's total (~55 SP) lands near the v1.0 budget by accident while its M1 (13 SP ≈ 650 lines) is under half the proposal's own v0.1 range of 1200-1800. C's T-scale is calibrated correctly and converts at 1 unit ≈ 2T.                   |
| Spike 2 / 14 gating                | block M1's release                                                     | block the release _announcement's claims_, not the merge            | close partially in CI at M7/M12                                       | **B's for 2 and 14**                                     | Demotion-on-error is the shipped design regardless of what the matrix says; the worst unmeasured outcome is an extra warning line. The matrix converts "assumed" to "measured" for release notes.                                                                                            |
| Spike 13 gating                    | blocks M1's release                                                    | run alongside 2 and 14, non-blocking                                | closes in CI at M12                                                   | **A's**                                                  | Spike 13 is categorically unlike 2 and 14: it asks whether the detection mechanism itself works. A volume that silently drops the flag produces no error to demote on, so it is the one result the design cannot absorb.                                                                     |
| Spike 15 (`nlink > 1`)             | M3 measurement, carried as assumption                                  | carried as an open assumption                                       | **promote to an ordinary CI row** (`hard_link` + rename)              | **C's**                                                  | C is plainly right that this needs no exotic hardware. Carrying it open was inertia.                                                                                                                                                                                                         |
| Fixture corpus storage             | not addressed                                                          | corpus split across milestones, storage not addressed               | **Rust byte-string constants + `disk_constructible_everywhere` flag** | **C's, adopted whole**                                   | Verified today: APFS refuses `b"bad\xffname.txt"` and `b"Bj\xf6rk..."` with EILSEQ. A checked-in file named with the payload is impossible; a `b"..."` literal is not.                                                                                                                       |
| Fuzz target timing                 | M6, brought forward if the cycle risk fires                            | M6                                                                  | M9                                                                    | **M2**                                                   | Its oracle is the §8.1 property set, which exists at M1's close, so the target is nearly free; it is the cheapest insurance against the defect class that dominated upstream's tracker (five external crash reports, §8.4); and spike 12 closes off its iteration counter.                   |
| `terminal_size` (11th slot)        | resolve at M6                                                          | resolve at M6                                                       | resolve at M10                                                        | **Resolve at M2**                                        | `report.rs` exists at M1 and M2 is the verbosity/colour milestone. Expected answer: not needed, final count 10, slot reserved.                                                                                                                                                               |
| Config file timing                 | M4                                                                     | M3, with `--exclude` split out to M2 ahead of it                    | M10                                                                   | **B's split**                                            | `--exclude` needs no persistence to be useful and is the highest-value/lowest-cost item after M1 (macOS-origin trees, `.DS_Store`/`Icon\r`). Bundling it with the config lift delays it for nothing.                                                                                         |
| Preview/`-x`, no-clobber, journal  | non-negotiable                                                         | non-negotiable ("refuses to defer")                                 | non-negotiable                                                        | **Unanimous; recorded so no later milestone reopens it** | Also B's refusal list in full: two-phase snapshot/plan/apply, `rustix` no-clobber as the only rename entry point, `OsStr`-at-the-boundary discipline, the crash-safe journal.                                                                                                                |

## 2. Defending the spine against A and C

**Against C, which is the sharper disagreement.** C's central claim is that fixing the transform's
contract with property tests before any I/O exists means the I/O layer is built once instead of
re-tested every time a stage moves. That is true and it is also not an argument for six
binary-less milestones. `transform` and `plan` are pure by construction (§3.1, §5.1) — that is the
entire reason §8's property tests are possible — so their property tests never needed a CLI to
exist, and nothing about writing them first requires withholding `main.rs`. C has conflated two
different things: _tests before implementation_, which is a commit-level discipline this plan
adopts outright (§4, work-package order), and _test-only milestones_, which is a delivery
decision with a real cost and no additional correctness. C's own §9 concedes the cost
("six milestones... looks, from outside `cargo test`'s output, like nothing happened") and prices
the benefit as avoided rework. That price is wrong: under this plan the transform's properties are
green before `fsops.rs` is written, because they are in an earlier work package of the same
milestone.

What C is right about, and what this plan takes from it wholesale: the §8 audit for decidability
(three real defects, §5), the fixture-corpus solution (§6), the discipline of naming a test file
before the file it tests, the promotion of spike 15, and the observation that `pipeline.rs`'s
_shape_ is a testability requirement the proposal never states.

**Against A.** A's ordering argument is correct and this plan does not weaken it: everything hard
to retrofit — the snapshot walk, the collision engine, the no-clobber rename, the journal, `undo`
— is in M1, unabridged, and gated by the full §8.2 property set plus the crash test. Where A goes
too far is in also deferring the transform stages, on an asymmetry argument (plan-level invariants
are infrastructure, transform stages are additive) that is sound but proves less than A uses it
for: if adding a stage later cannot regress the safety envelope, then adding it _now_ cannot
either. A's own §3.4 table rates every deferred stage "Low" or "Low-medium". So the deferral buys
a shorter audit list and costs the one thing that cannot be bought later, which is real user
trees. A also makes one claim that is simply false — a `forbid(unsafe_code)`, dependency-free
signal handler via "`std::os::unix`'s raw signal primitives" — and this plan rejects the feature
rather than the constraint (§5.4).

What A is right about, and what this plan takes: the `kill -9` crash test as an M1 exit criterion
rather than a hardening-milestone nicety; the framing of spike 13 as the one spike whose failure
mode the design cannot absorb; the `rustix`-major-bump re-verification discipline; the honest
statement that Windows spikes 3 and 4 make every Windows-facing default an assumption with a
structurally unknown retrofit cost.

**What this plan gives up, stated plainly.** M1 is the largest milestone in any of the three plans
— 10 to 14 units, the whole of the proposal's v0.1 LOC budget, with the property suite and the
filesystem matrix subset as gates rather than follow-ups. It will feel long before it feels
useful, and there is no shippable artifact before it. A stakeholder who wants something visible
sooner does not get it here either; what they get is that when it arrives it is the real tool and
not a demo whose transform still has to be re-verified. The named fallback is in §8, risk 12: if
M1's plan-engine work package is not gate-green by the time cumulative production lines pass the
midpoint of the v0.1 budget, split the binary work package into its own milestone and accept C's
ordering for that one boundary.

## 3. Estimation unit

**1 unit = one reviewable change of roughly 100-150 lines of production Rust, plus the tests that
gate it.** Calibrated against proposal §7.3's own checkable budget (v0.1: 1200-1800 lines; v1.0:
2200-3000), so a unit count is a claim about size and test burden, not about time. No calendar
dates appear in this plan, deliberately: there is one implementer of unknown availability.

Conversions from the source plans, for anyone reading them alongside this one: Plan B's story
point ≈ 0.4 unit; Plan C's T ≈ 0.5 unit. Two arithmetic notes from checking the plans rather than
trusting them:

- Plan B's M1 (13 SP, ≈ 650 lines at its own implied rate) is **under half** the proposal's own
  v0.1 range for the same scope. Its total (~55 SP ≈ 2750 lines) lands inside the v1.0 range, so
  the error is concentrated in the front-loaded milestone — which is exactly the one this plan
  adopts, so this plan states M1 as 10-14 units rather than inheriting B's number.
- Plan C's T-scale is internally consistent (M1-M9 ≈ 27T ≈ 1620 lines for roughly v0.1 plus the
  fuzz/bench work), which is why its scale is the one used to cross-check the conversion.

This plan totals **22-28 units** against a v1.0 budget of 18-24. That overrun is recorded, not
argued away: it is the signal §8 risk 12 watches.

## 4. Milestones

Every milestone leaves `just gate` green (fmt-check, clippy `-D warnings`, test, msrv,
dep-budget), is independently shippable, and depends only on earlier milestones. Conventional
Commits throughout.

| #   | Name                                                     | Units | Deps after |
| --- | -------------------------------------------------------- | ----- | ---------- |
| M1  | v0.1: the whole walking skeleton, usable                 | 10-14 | 6/11       |
| M2  | `url_decode`, `--exclude`, verbosity/colour, fuzz in CI  | 2-3   | 7/11       |
| M3  | Config file, discovery, profiles, `--print-config`       | 3     | 9/11       |
| M4  | `[[rule]]`, `--keep`/`--strip`, `--case`, `--ascii`, UCD | 3-4   | 10/11      |
| M5  | `--target`, plan files, `--stdin`, per-directory limits  | 3     | 10/11      |
| M6  | v1.0 hardening, full matrix, docs, packaging 1-4         | 2-3   | 10/11      |

### M1 — v0.1, usable (detailed file-by-file in §7)

Scope: `detoxrs [-r] [-x] [-n] [--on-collision number|skip|fail] [-v|-q] [--json] <PATH>...` plus
`detoxrs undo [--last | <BATCH-ID>] [--list]`. Pipeline stages 1, 3, 4, 7, 9, 10, 12, 13 (§3.2),
which is the proposal's v0.1 list. Snapshot walk with `-r`, VCS-metadata skip, dotfile skip during
recursion, symlinks never descended. Three-layer collision engine with `number`/`skip`/`fail`.
`rustix` no-clobber rename plus the two fallbacks. JSONL journal with `intent`/`done`/`failed` and
`undo`. Human preview, `--json`, exit codes 0/1/2.

Not in M1: config, profiles, rules, `url_decode`, `--ascii`, `--case`, `--keep`/`--strip`,
`--exclude`, `--target`, plan files, `--stdin`, `-vv` per-stage trace, per-directory length
detection, SIGINT handling (dropped from v1.0 entirely, §5.4).

Exit criteria, as tests that must pass:

- §8.1 properties, all of them that M1's stage set makes non-vacuous: Totality, Idempotence,
  Safety closure (case clause vacuous, stated as such in the test's doc comment), **Length bound
  against both fields** (§5.1), No grapheme splitting, Non-empty, Dotfile preservation, Decode is
  total and never re-interprets, **Stage independence across stages 3, 4, 7, 9, 10 via the mask
  seam** (§5.2). The literal `transform("***") == Unrepresentable(ReducesToEmpty)` unit test
  exists as a named case before the property that subsumes it (Plan C).
- §8.2 properties, all seven, fully green — including Undo round-trip against an in-memory
  `RenameOps` and journal writer in the binary crate's own test module.
- §8.4 rows this scope exercises: case-only rename (both APFS variants, ext4, tmpfs, a
  case-insensitive Linux mount); NFD→NFC rename (both APFS variants); length-limit probe (against
  the hardcoded constant); `RENAME_NOREPLACE` unsupported via an injectable-failure `RenameOps`
  (Plan C's point: this row tests our fallback, not the kernel, so it needs no exotic mount);
  non-UTF-8 name (Linux tmpfs only — APFS rejects it, verified); symlink-to-`../..` under `-r`,
  including a symlink created _between_ snapshot and apply (Plan A); rename-during-walk, 5000
  entries; **TOCTOU collision during apply** (§5.3, new row, both OSes); **spike 15's
  `hard_link` + respell row**; **crash mid-batch under `kill -9`, not `kill -TERM`** (Plan A) —
  journal replay identifies the exact interrupted item and `undo` restores the completed prefix.
- `insta` snapshots: `--help`; the human preview over the §8.3 corpus minus the entries that only
  mean something once stages 2/5/6/8 exist; the `<hh>`-escaped rendering of an `Opaque` name.
- `criterion` huge-tree benchmark (200k entries) recorded as a baseline, `#[ignore]`d out of
  `gate` (Plan B's argument, and §8.4's five external crash reports are the reason).
- `just dep-budget` reads 6/11.

Files: `crates/detoxrs-core/src/{lib,policy,decode,classes,invisible,truncate,pipeline,plan}.rs`;
`crates/detoxrs/src/{main,cli,walk,fsops,fsops/fallback,apply,journal,report}.rs`; the corpus and
test files named in §6 and §7.

### M2 — `url_decode`, `--exclude`, verbosity, fuzz in CI (2-3 units, +`regex` → 7/11)

Stage 2 (all-or-nothing per §3.11); `--exclude <GLOB>` compiled to `regex`, repeatable;
`--files-only`/`--dirs-only`; `-v`/`-vv`/`-q`; `--color`; the `-vv` per-stage trace and its
snapshot corpus; the `cargo-fuzz` target over `decode` + `transform` with the §8.1 property set as
oracle, running a bounded seeded corpus in CI on every push, seeded from `corpus::ALL` by a
generator rather than by hand; **the `terminal_size` decision, recorded either way**.

Exit: the all-or-nothing property (`50%-70%.txt` unchanged, a literal case); `--exclude 'Icon\r'`
leaves that literal name untouched; the fuzz target runs clean in CI; the fixed-point iteration
counter is instrumented (spike 12's data starts accumulating here).

### M3 — Config, discovery, profiles, `--print-config` (3 units, +`serde`, `toml` → 9/11)

§4.1-4.3 in full: TOML load; `--config` / `$DETOXRS_CONFIG` / nearest `.detoxrs.toml` / XDG, first
match wins, no merging; `[profile.NAME]` via `-p`; `--print-config` with resolve-don't-echo and
validate-everything-compilable.

Exit: each of the four discovery sources individually verified to win when it should; the
`--print-config` **exit-2-on-invalid-rule golden test written before the happy path** (Plan C's
ordering argument — that is what stops upstream's `-L` mistake being reproduced by accident); a
snapshot proving `max_len = 0` prints its "resolved per directory at walk time" comment rather
than a number, restated for both length fields per §5.1.

### M4 — `[[rule]]`, `--keep`/`--strip`, `--case`, `--ascii`, UCD tables (3-4 units, +`deunicode` → 10/11)

Stages 5, 6, 8; `--keep`/`--strip` moving characters between classes; the build-time UCD
generator, checked-in data, no network fetch — producing **both** `invisible.rs`'s full
`Cf`/`Cs`/`Co` closure (replacing M1's named set behind an unchanged `is_invisible(char) -> bool`)
and `scripts.rs`'s Script table for the §3.12 mixed-script warning (detection only, never
rewriting; no UTS #39 confusable table, which stays v1.1).

Exit: Stage independence re-run across stages 5/6/8 (the property whose violation is detox's own
#40/#86); `ü -> ue` via a user rule running before `--ascii` (§3.3's #117/#121 example); the
generator's output diffed into the repo as a reviewable artifact; Stage independence still green
for `--no-invisible-strip`, which is the regression test that the delete class stayed narrowed.

### M5 — `--target`, plan files, `--stdin`, per-directory limits (3 units, no new deps)

Stage 11 (`reserved.rs`, §6.5's conservative default, `--target`-gated) joining the finalize loop;
`--plan-out`/`apply` with the `(dev, ino, mtime)` stale-plan recheck; `--stdin`;
`rustix::fs::statfs`-derived per-directory limits replacing M1's constant behind the same two
`Policy` fields, in the commit that deletes the constant; `clap_complete`/`clap_mangen` output.

Exit: `CON.txt`/`nul.c` under `--target windows` as a `trycmd` case, documented as an assumption
and **not** as verified Windows behavior; stale-plan refusal test; the length-detection assertion
matching doc 06 Test 1's numbers on ext4/tmpfs and both APFS variants; the M2 fuzz harness re-run
with stage 11 in the loop, which is spike 12's actual richest-interaction corner.

### M6 — v1.0 hardening (2-3 units, no new deps)

Every §8.4 row plus this plan's two additions green in CI across the Linux + macOS matrix; spike
13's `hdiutil` HFS+/exFAT image row; spike 14's loopback rows; spike 5's env-var-gated row
skipping loudly with a named reason; Windows best-effort tier compiling and unit-testing in CI
with `continue-on-error` and no filesystem claim; `MIGRATING-FROM-DETOX.md`;
`--help-transforms`; packaging items 1-4 from §9.4.

Exit: §8.4's table is a literal checklist against CI job names, and every row is either green or
explicitly marked untested with its reason. No row claims verified Windows, NTFS, or exFAT
behavior.

## 5. The three adjudicated design changes

### 5.1 Length bound: two concrete fields — accepted, with C's override clause rejected

Plan C's finding is correct and the arithmetic holds: U+1F600 is 4 UTF-8 bytes and 2 UTF-16 units,
so ~130 astral emoji is 260 UTF-16 units (just over a 255-unit cap, correctly truncated) and 520
bytes (far over ext4's 255-byte cap). §3.1 says the resolved `Policy` carries `max_len` as "a
concrete number"; §3.10 and §8.1 both require both metrics to hold simultaneously. One scalar
cannot express that, so the property is vacuous on one axis — which is exactly the failure it was
written to catch.

Adopted: `Policy { max_len_bytes: usize, max_len_utf16: usize }`, both always concrete, both
always checked, `truncate` shrinking until both are satisfied. Auto-detection sets them per
§3.10's table: ext4 `bytes = 255, utf16 = usize::MAX`; APFS `utf16 = 255, bytes = usize::MAX`;
unknown volume, and NTFS/exFAT under their standing assumption, `both = 255`.

**Rejected: C's clause that `--max-len N` sets both fields to `N`.** On a volume whose unit is
known, that silently over-truncates every non-ASCII name — `--max-len 255` on APFS would cap bytes
at 255 on a filesystem that accepts up to 1020 — and it contradicts §3.10's stated semantics
("interpreted in the filesystem's own unit"). Instead: `--max-len N` sets the field corresponding
to the detected volume's own unit and leaves the other at its detected value; on an unknown volume
it sets both, because there both are 255 already. Same conservatism, no surprise, and the property
stays decidable because both fields are still concrete.

### 5.2 Stage independence: accepted in substance, C's mechanism rejected

C is right that five of thirteen stages have no CLI flag to disable them, and that without an
internal seam the property test can only reimplement the pipeline — which tests nothing. But C's
remedy (all thirteen stages as `&str -> String` in an iterated list of function pointers, plus a
`#[doc(hidden)] pub` re-export so an integration-test crate can reach the override) is wrong twice
over: stage 12 takes `(stem, ext, limits)` and stage 13 is a bounded loop **over** stages 9/10/11,
so neither can be an element of the list it would have to appear in; and weakening the public API
for a test is avoidable.

Adopted instead:

- Each linear stage (2, 3, 4, 5, 6, 7, 8, 9, 10, 11) is its own named `pub(crate)` function.
- One internal entry point, `pipeline::run_with(input: &str, p: &Policy, disabled: StageMask)`,
  where `StageMask` is a small bitset. `pipeline::transform` is `run_with(.., StageMask::NONE)`.
  A disabled stage is skipped, which is identity by definition — no second pipeline to keep in
  sync, no `Vec<fn>`.
- Stages 12 and 13 get direct tests rather than mask entries: `truncate` is tested against its own
  signature, and `finalize`'s loop is tested by the Idempotence and `NotConverged` properties,
  with the proptest generator biased toward inputs that approach the 3-iteration bound (C's point
  in its §1.1 Idempotence row, which is a good one).
- The Stage-independence property lives in `pipeline.rs`'s own `#[cfg(test)] mod tests`, not in
  `tests/`. `proptest!` works fine in a unit-test module, and nothing needs to become `pub`.

### 5.3 No-pre-existing-clobber: accepted in full, new matrix row

§8.2's property is quantified over `plan()`'s output, and `plan()` has no I/O, so it can only ever
exercise the walk-snapshot half of collision layer 2. The fresh `symlink_metadata` recheck at
apply time and the kernel's own refusal have no test anywhere in §8, including §8.4. Adopted as a
new §8.4 row, scheduled in M1's exit criteria:

> **TOCTOU collision during apply.** Compute a plan against a snapshot; create a file at one
> item's destination after the snapshot and before `apply` runs; run `apply`. Assert the affected
> item is reported as a fresh conflict, the pre-existing file is byte-identical afterwards, the
> process does not panic, and the exit code is 1. Linux and macOS.

Independently verified today that the kernel half of this holds on APFS: `renameat_with` with
`RenameFlags::NOREPLACE` onto a distinct occupied destination returns `Err(EEXIST)`, while a
same-inode case-only respell returns `Ok(())` (which is doc 06 row 4f reproduced, and the reason
Plan A's observed-`EEXIST` fallback is defensive-only rather than a normal path).

### 5.4 Rejecting all three on SIGINT handling

All three plans schedule SIGINT/SIGTERM handling (§5.8, added by the stage-3 review). Plan A
asserts it needs no dependency ("`std::os::unix`'s raw signal primitives plus a static flag"),
Plan B tries to write that sentence and visibly cannot finish it, Plan C schedules it at M8
without addressing feasibility. Verified today: **std exposes no signal-handler API at all**, and
`rustix`'s nearest equivalent is `pub unsafe fn kernel_sigaction`, gated to `linux_raw`. So the
only unsafe-free route is a crate (`signal-hook`/`ctrlc`), which under `forbid(unsafe_code)` is
legal — the attribute governs our code, not our dependencies — but costs the last free budget
slot.

**Decision: v1.0 ships no signal handler.** SIGINT terminates the process; `rename(2)` is a single
syscall and is not interrupted mid-flight; the journal writes and fsyncs its `intent` record
before the rename, so an interrupted batch leaves at most one item whose outcome is unknown, which
is precisely what the `kill -9` protocol already reports and what `undo --last` already reverts.
A clean between-items summary line on Ctrl-C is cosmetic. The upgrade path, named with its cost:
spend the 11th budget slot on `signal-hook` if a real user asks. The safety claim is unaffected,
and the test that proves it — `kill -9` mid-batch — is strictly harsher than the case a handler
would cover.

## 6. The fixture corpus

Adopted from Plan C essentially unchanged, because it is the only one of the three that solved it
and because its premise verified: APFS refuses invalid-UTF-8 names at the syscall level (EILSEQ,
errno 92, reproduced today on both `b"bad\xffname.txt"` and `b"Bj\xf6rk - Vespertine.mp3"`), so a
fixture checked in as a file whose _name_ is the payload is not merely awkward, it is impossible
on one of the two tier-1 platforms.

1. Canonical storage: `crates/detoxrs-core/tests/support/corpus.rs`, a Rust source file of
   `b"..."` constants covering §8.3's required list. Git-diffable, no binary blob, valid UTF-8 as
   source while holding arbitrary invalid-UTF-8 as data.
2. Each entry carries `disk_constructible_everywhere: bool`. Invalid-UTF-8 entries are `false`
   unconditionally; deliberately oversized entries are `false` because their job is exercising
   truncation in memory, not proving a filesystem accepts a 300-byte name.
3. Consumption: pure/property/snapshot tests read the bytes and build an `OsString` via
   `OsStringExt::from_vec`, never touching a filesystem and ignoring the flag. Filesystem-matrix
   tests filter on the flag, create entries straight from bytes with no `&str` step, and **log
   every skipped entry by name with an asserted skip count**, so a matrix run's coverage is
   auditable rather than silently thin.
4. The fuzz seed corpus (M2) is generated from `corpus::ALL` by a helper, with ordinary ASCII file
   names and fixture bytes as contents, so the seed set cannot drift from the property corpus.

One correction to C's sketch: the repeated-emoji entry is generated in a helper function, not
`const` data with a `leak()` call. `leak()` in a fixture table is a memory leak dressed as a
constant.

## 7. Milestone 1, file by file

`decode`, `plan`, and `fsops` signatures below are the contract; the work-package order under them
is the sequence someone starts on Monday.

### 7.1 Work-package order inside M1

Each work package ends with `just gate` green and its own tests. This is where Plan C's discipline
lives — the test file is named and written before the file it tests, and no `todo!()` survives a
commit that claims a package done.

| WP  | Content                                                                           | Tests written first                                                                                      |
| --- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 1   | `policy.rs`, `decode.rs`, `tests/support/{mod,corpus}.rs`                         | `prop_decode.rs` (Decode is total), `snap_decode_corpus.rs`                                              |
| 2   | `classes.rs`, `invisible.rs` (named set), `pipeline.rs` with the mask seam        | Safety closure, Non-empty, Dotfile preservation, Stage independence over 3/4/7/9/10, `.!file.txt` case   |
| 3   | `truncate.rs`, the finalize loop, `Unrepresentable`                               | literal `***` case, Length bound (both fields), No grapheme splitting, Idempotence, Totality             |
| 4   | `plan.rs`: three layers, renumbering, sibling-chain refusal                       | all six plan-time §8.2 properties, near-swap generators included                                         |
| 5   | `cli.rs`, `walk.rs`, `fsops.rs` + fallback, `apply.rs`, `journal.rs`, `report.rs` | `--help` snapshot pinned first; Undo round-trip against the in-memory double; the §8.4 subset; `kill -9` |

### 7.2 `crates/detoxrs-core`

```
crates/detoxrs-core/src/{lib,policy,decode,classes,invisible,truncate,pipeline,plan}.rs
crates/detoxrs-core/tests/support/{mod,corpus}.rs
```

```rust
// policy.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub separator: char,        // '_' in M1; --separator arrives with the config file (M3)
    pub max_len_bytes: usize,   // M1: 255 (§5.1); statfs-derived from M5
    pub max_len_utf16: usize,   // M1: 255 (§5.1)
}
// on_collision deliberately does NOT live here: it is a plan-time concern (§5.3), not a
// transform-time one, and Policy's contract is "every field maps 1:1 to a flag AND is read
// by some stage". Plan A put it in the CLI/plan layer for this reason; that is right.

impl Default for Policy { /* '_' , 255, 255 */ }
```

```rust
// decode.rs -- no Policy parameter. See §9 amendment 16: P2 makes encoding a
// non-policy, repair is gone by owner decision, so there is nothing for this
// function to look up. A dead parameter is not fidelity to the design.
pub enum Decoded { Utf8(String), Opaque }

#[must_use]
pub fn decode(raw: &std::ffi::OsStr) -> Decoded;   // to_str() on Unix; WTF-8 split on Windows
```

```rust
// classes.rs -- §3.7's table as code. Delete class is Cc + DEL + NUL ONLY: it must not
// re-include stage 4's set, or --no-invisible-strip is a dead flag and Stage independence
// is false (stage-3 review, ACCEPTED finding).
pub enum CharClass { Delete, Separator, Keep }
#[must_use] pub fn classify(c: char) -> CharClass;

// invisible.rs -- M1: the named set (bidi, zero-width, Tags). M4 swaps the body for the
// UCD-generated Cf/Cs/Co closure behind this unchanged signature.
#[must_use] pub fn is_invisible(c: char) -> bool;
```

```rust
// truncate.rs
pub struct Limits { pub bytes: usize, pub utf16: usize }

/// Largest prefix of `stem` on a GRAPHEME CLUSTER boundary such that stem+ext satisfies
/// both limits. Never is_char_boundary (§3.10 step 2; sanitize-filename's documented bug).
#[must_use] pub fn truncate_graphemes(s: &str, limits: &Limits) -> &str;
#[must_use] pub fn truncate(stem: &str, ext: &str, limits: &Limits) -> (String, bool);
/// §3.10 step 1: last `.`-suffix, plus the pair when the preceding segment is <= 4 BYTES
/// of UTF-8 and itself preceded by a `.` (`.tar.gz`).
#[must_use] pub fn split_extension(name: &str) -> (&str, &str);
```

Step 3's whole-name fallback calls `truncate_graphemes` too. One function, two callers — C's
observation that two hand-written loops would drift is correct and this is the cheap form of it.

```rust
// pipeline.rs
pub struct Outcome { pub text: String, pub truncated: bool }
pub enum Unrepresentable { ReducesToEmpty, ReducesToDotOrDotDot, NotConverged }
pub enum TransformResult { Name(Outcome), Unrepresentable(Unrepresentable) }

#[must_use] pub fn transform(input: &str, p: &Policy) -> TransformResult;

// §5.2's seam. pub(crate) + a #[cfg(test)] property test in this module.
pub(crate) struct StageMask(u16);
pub(crate) fn run_with(input: &str, p: &Policy, disabled: StageMask) -> TransformResult;
```

M1's stage order inside `run_with`: NFC (3) → invisible strip (4) → safe_map (7) → collapse (9) →
trim (10) → split_extension + truncate (12) → finalize (13): re-run 9/10 to a bounded fixed point
of 3 iterations, checking empty/`.`/`..` after each pass. `Outcome` carries no `Vec<StageDelta>`
in M1; it grows one at M2 when `-vv` makes a trace worth reading.

```rust
// plan.rs
pub struct Entry { pub dir: PathBuf, pub name: OsString, pub kind: EntryKind,
                  pub ident: Ident, pub depth: u32 }
pub enum EntryKind { File, Dir, Symlink, Other }
pub struct Ident { pub dev: u64, pub ino: u64, pub nlink: u64, pub mtime: SystemTime }
pub enum SkipReason { NotUtf8, Unrepresentable(Unrepresentable) }
pub enum Resolution { Rename, Unchanged, Skipped(SkipReason), Conflict(Conflict) }
pub enum Conflict { Unresolvable }          // 998 probes exhausted (§5.3's stated bound)
pub enum OnCollision { Number, Skip, Fail }
pub struct PlanItem { pub dir: PathBuf, pub from: OsString, pub to: OsString,
                      pub kind: EntryKind, pub ident: Ident, pub depth: u32,
                      pub resolution: Resolution }
pub struct Plan { pub items: Vec<PlanItem> }
pub enum PlanError { InternalInconsistency(String) }

/// No I/O. Takes the frozen snapshot; returns items sorted deepest-first by depth, with
/// ties broken by NFC bytes of the source name (never readdir order -- that is what
/// Determinism asserts). Layer 1: map keyed by (dir, nfc_fold(to)). Layer 2: the snapshot's
/// own entries; the fresh symlink_metadata recheck is apply.rs's job, not this function's.
/// Renumbering: N = 2..999, each candidate truncated to fit, against existing plus
/// already-allocated names. The sibling-chain check refuses the whole batch as an internal
/// error -- provably unreachable given Idempotence, so if it fires, Idempotence broke first.
pub fn plan(entries: &[Entry], p: &Policy, on_collision: OnCollision)
    -> Result<Plan, PlanError>;
```

### 7.3 `crates/detoxrs`

```
crates/detoxrs/src/{main,cli,walk,fsops,apply,journal,report}.rs
crates/detoxrs/src/fsops/fallback.rs
```

`cli.rs`: clap derive — `paths`, `-r/--recursive`, `-x/--exec`, `-n/--dry-run`
(`conflicts_with = exec`), `--on-collision`, `-v` (count), `-q/--quiet`, `--json`, and
`undo { --last, batch_id, --list }`.

`walk.rs`: `pub fn snapshot(paths: &[PathBuf], recursive: bool) -> Result<Vec<Entry>, WalkError>`.
`walkdir` with `max_depth(1)` unless recursive; `follow_links(false)` already gives §5.6's
symlink non-descent for free; `lstat` never `stat`; `.git`/`.hg`/`.svn` skipped unconditionally
and never configurable; dotfiles skipped while recursing, processed when named. Unreadable
directory reported and the walk continues; `EMFILE`/`ENFILE` aborts before any rename, because an
incomplete snapshot is the one thing §5.1 cannot tolerate.

`fsops.rs`:

```rust
pub enum RenameErr { AlreadyExists, PermissionDenied, ReadOnlyFilesystem, NoSpace,
                     NameTooLong, NotFound, Unsupported }

pub trait RenameOps {
    /// The ONLY rename entry point. Fails rather than clobbering; never falls back to a
    /// clobbering call. No rename_case_only -- designed, measured and deleted upstream in
    /// the proposal's propagation pass, and not resurrected here.
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}

pub struct PlatformRenameOps;   // rustix::fs::renameat_with(dirfd, from, dirfd, to, NOREPLACE)
```

No `#[cfg]` split: `rustix` maps `NOREPLACE` to `renameat2` under `#[cfg(linux_kernel)]` and
`renameatx_np(RENAME_EXCL)` under `#[cfg(apple)]`. Verified again today under
`#![forbid(unsafe_code)]` on APFS. Two fallbacks, both observed-error rather than predicted:
`EEXIST` where `symlink_metadata(to)` reports the same `(dev, ino)` as `from` → plain `rename(2)`
for that one item, warn once (defensive; measured not to fire on APFS);
`EINVAL`/`ENOSYS`/`EOPNOTSUPP` → demote this mount to check-then-rename, report
`"atomicity": "check-then-rename"`, warn once. M1 warns once **globally** via `OnceLock`;
per-mount granularity is Plan B's accepted debt, a `HashSet<PathBuf>` at the same call site.

`apply.rs` (new, and this is a deliberate departure from all three plans, which put the apply loop
in `main.rs`): the loop, the fresh `symlink_metadata` recheck, the `EROFS`/`ENOSPC` abort, and the
per-item error taxonomy live here, written against `&dyn RenameOps` and a journal-writer trait, so
the Undo round-trip property and the TOCTOU test can drive it from `#[cfg(test)]` with an
in-memory double. That is C's Undo-round-trip finding satisfied without moving anything into
`detoxrs-core`.

`journal.rs`: one file per batch at `$XDG_STATE_HOME/detoxrs/journal/<UTC-ts>-<id>.jsonl`
(`$HOME/.local/state/...` fallback). `record_intent` writes **and fsyncs before** the rename; if
that fails the rename does not happen, because an unjournaled rename is the one thing `undo`
cannot reverse. `record_done`/`record_failed` after. `replay_for_undo` walks a batch in reverse
and, per item, verifies the current name still resolves to the recorded `(dev, ino)`; refuses that
single item if not, and continues. Lines are built with `serde_json` rather than hand-escaped:
this artifact is the safety net, and a byte-escaping bug in path-derived data is exactly what P7's
"50 lines of our own code" test is not for.

`report.rs`: human preview (`from -> to`, a note per `Skipped`/`Conflict`, one summary line),
`--json`, `<hh>`-escaped rendering for `Opaque` names, exit codes 0/1/2. Fixed two-column layout,
no `terminal_size` (decision recorded at M2). Exit 3 (`--quiet` and nothing matched) deferred.

`main.rs`: `cli::Cli::parse()` → `walk::snapshot` → `plan::plan` → if `-x`, `journal` +
`apply::run`, else `report::print_preview`. The `if !exec { print; return; }` branch is the only
gate on the only call site of `rename_noreplace`, and there is no other code path to a rename.

### 7.4 M1 dependencies: 6/11

`clap` (derive), `rustix` (feature `fs`), `walkdir`, `serde_json`, `unicode-normalization`,
`unicode-segmentation`. Dev: `proptest`, `insta`, `assert_cmd`, `trycmd`, `criterion`.

Plan B's count is the correct one for this scope. Plan A's table said 4 and then corrected itself
to 5 mid-draft; the correction was right as far as it went, but A's M1 also excludes `truncate`,
which is why it does not need `unicode-segmentation` — under this plan's M1 scope it does, so 6 is
the honest number. Verified against the recipe: `just dep-budget` unions the `[dependencies]`
tables of `crates/*/Cargo.toml` and subtracts workspace members, so the path dependency on
`detoxrs-core` does not count and dev-dependencies do not either.

## 8. Spike schedule

Hardware: Linux (any distro) and macOS. No Windows, no NTFS, no exFAT.

| Spike                                  | Where                                                                    | Blocks                                                    | Runs in parallel with | If unclosed, the carried assumption                                                                                                                                                     |
| -------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------- | --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 name/trademark                       | before the first public push; non-engineering                            | any public commit and the crates.io claim                 | all of M1             | `detoxrs` stands as the working name (crates.io availability already checked); do not publish until the trademark search and `dtx` binary clearance are done                            |
| 2 `renameat2` matrix (Linux)           | M1 WP5, on Linux hardware; loopback images                               | the v0.1 **release announcement's claims**, not the merge | WP1-4                 | runtime demotion on `EINVAL`/`ENOSYS`/`EOPNOTSUPP` per mount, which is the shipped design either way; the matrix only tells us how often the fallback is normal                         |
| 13 macOS incapable volume              | M1 WP5, `hdiutil` HFS+/exFAT image                                       | **the v0.1 release**                                      | WP1-4                 | assumed to error, not to silently drop the flag. This is the one spike whose bad outcome the design cannot absorb, so it gates (Plan A)                                                 |
| 14 Linux case-insensitive mounts       | M1 WP5, loopback vfat/exfat/ext4-casefold                                | nothing                                                   | 2, 13                 | assumed to behave like APFS; the observed-`EEXIST` fallback absorbs the other answer with no code change                                                                                |
| 15 `nlink > 1` respell                 | M1 WP5, `std::fs::hard_link` + rename, CI row                            | nothing — **closed here**                                 | 2, 13, 14             | promoted from open question to ordinary CI row (Plan C)                                                                                                                                 |
| 12 fixed-point bound of 3              | M2 fuzz harness iteration counter; re-run at M5                          | nothing — informational, **closes at M5**                 | M3, M4                | bound stays 3; §3.14 makes non-convergence safe regardless                                                                                                                              |
| 7 NFC-by-default collision rate        | M1's release ask, over real macOS trees                                  | v1.0 ("everything else" tier)                             | M2-M5                 | NFC rewrite stays default-on                                                                                                                                                            |
| 8 auto-number default                  | already decided by the owner; count real conflicts from M1's release ask | nothing                                                   | M2-M5                 | `number` stays the default                                                                                                                                                              |
| 11 `Unrepresentable` frequency         | M1's release ask (`--json` output makes the count parseable)             | v1.0                                                      | M2-M5                 | skipping stays the only behavior; no placeholder flag ahead of the count                                                                                                                |
| 5 case-only rename on network FS       | test written at M1 WP5, env-var-gated on a mount point; skipped loudly   | v1.0                                                      | everything            | **cannot be closed without infrastructure.** Direct case-only rename assumed to work; a temp-name dance returns per-filesystem on an observed error, never as a default                 |
| 9 parallelism                          | `criterion` baseline recorded at M1; no decision                         | nothing                                                   | —                     | single-threaded                                                                                                                                                                         |
| 3, 4 Windows names / NTFS-exFAT limits | **not scheduled. Cannot be closed at all.**                              | Windows tier-1 promotion only                             | —                     | conservative pre-Windows-11 reserved-name rule; both length fields enforced simultaneously. **No milestone claims verified Windows behavior.** Retrofit cost structurally unknown (§10) |
| 6 CP1252 repair                        | moot (owner decision)                                                    | nothing                                                   | —                     | non-UTF-8 names stay `Opaque`, skipped, reported                                                                                                                                        |
| 10 distro aggregate count              | not scheduled; cosmetic                                                  | nothing                                                   | —                     | per-distro citations suffice                                                                                                                                                            |

Two spikes advanced today, recorded so the matrix run does not repeat them: the APFS half of
spike 13's premise (the flag works and refuses correctly on a capable APFS volume; the
_incapable_-volume case remains unrun), and doc 06 row 4f's same-inode-respell result, reproduced
independently.

## 9. Risk register

| Risk                                                                                                                                   | Source    | Early detection signal                                                                                                                                                                                                                                                                        |
| -------------------------------------------------------------------------------------------------------------------------------------- | --------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Journal crash protocol not atomic enough on a real filesystem; a `done` survives without its rename, or a rename without any record | A         | M1 WP5's `kill -9` mid-batch test, run before M1 is called done. If it fires, the wire format needs a checksum or a commit marker — cheap now, a migration story for every journal ever written after M1.                                                                                     |
| 2. A macOS volume format silently clobbers instead of erroring on unsupported `NOREPLACE`                                              | A         | Spike 13's `hdiutil` HFS+/exFAT image, gating the v0.1 release. Fix is narrow (a scoped `getattrlist`-equivalent probe for that format only), but demotion-on-error has no error to observe, so detection cannot be deferred to a user.                                                       |
| 3. `walkdir`'s symlink handling has an edge case, or a directory becomes a symlink between snapshot and apply                          | A         | The symlink-to-`../..` row **plus** the between-snapshot-and-apply race variant, both at M1 rather than M6. #20 (symlink loops) was never characterized upstream either.                                                                                                                      |
| 4. Idempotence breaks once `truncate` interacts with `finalize`, making the cycle proof stop holding silently                          | A         | The plan-time sibling-chain refusal is a loud named error from M1; the fuzz oracle runs from M2, not M6. Signal: the assertion fires, or Idempotence's shrinker returns a truncation-shaped counterexample.                                                                                   |
| 5. `rustix` changes `renameat_with`/`RenameFlags` non-additively, or its wrapper needs `unsafe` in a future major                      | A         | Pin in `Cargo.lock`/`deny.toml`; re-run the compile-and-run-on-APFS check on every `rustix` major bump. Done today; record the result each time. If it ever regresses, the `libc` shim and its unsafe-audit budget come back.                                                                 |
| 6. Hardcoded 255/255 limits wrong on some filesystem                                                                                   | B         | `ENAMETOOLONG` from a real run, which §5.8 already designates as evidence the detected limit was wrong. Direction of error is over-truncation, which the collision engine catches rather than silently merging.                                                                               |
| 7. The named invisible set misses a hazard the UCD closure would catch                                                                 | B         | A user report naming a surviving invisible character outside the bidi/zero-width/Tags set. Nothing in the tracker or `user_feedback_online.md` reports one. Closed at M4 regardless.                                                                                                          |
| 8. Stage independence degenerates into reimplementing the pipeline in the test                                                         | C         | Review signal, not a runtime one: any test file that contains a second copy of stage ordering. The mask seam (§5.2) exists so there is nothing to copy.                                                                                                                                       |
| 9. Length bound passes vacuously on one axis                                                                                           | C         | The property itself, once both fields are concrete: a 130-astral-emoji input is a named corpus entry, so a one-axis implementation fails at M1 WP3 rather than on ext4 in the field.                                                                                                          |
| 10. The apply-time TOCTOU recheck is never exercised                                                                                   | C         | The new §8.4 row, in M1's exit criteria.                                                                                                                                                                                                                                                      |
| 11. Nobody responds to the v0.1 release ask, so spikes 7, 8, 11 stay open anyway                                                       | B         | Zero responses within one release cycle of posting to the venues `user_feedback_online.md` names. Fallback: run spikes 7 and 11 in report-only mode over the owner's own Downloads/media/archive trees and label the sample size honestly.                                                    |
| 12. M1 is too large and stalls                                                                                                         | this plan | WP4 (`plan.rs`) not gate-green by the time cumulative production lines pass 900 (the midpoint of §7.3's v0.1 range). Response: split WP5 into its own milestone and accept C's ordering for that boundary.                                                                                    |
| 13. The dep-budget gate has a hole                                                                                                     | this plan | Verified: the recipe reads only `[dependencies]`, so `[build-dependencies]` and `[target.'cfg(..)'.dependencies]` escape it. Signal: the first PR adding a `build.rs` dependency or a target table. M4's UCD generator is std-only, so extend the recipe in the same PR that adds `build.rs`. |
| 14. Windows-facing defaults are assumptions and their retrofit cost is unknown                                                         | A         | None available — this is the honest entry. No Windows machine exists. Containment claim: a wrong assumption is confined to `reserved.rs`/`limits.rs`. That claim is itself untested, and is recorded as such rather than as a mitigation.                                                     |

## 10. Deliberately out of v1.0

| Out                                                                                                                        | Retrofit cost                                                                                                                                    |
| -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Legacy encoding repair / `--repair-encoding` (owner decision)                                                              | Medium, and gated on a false-positive measurement against a real corpus on Linux hardware, not on code. Returns as opt-in only, never a default. |
| SIGINT/SIGTERM handler (§5.4)                                                                                              | Low: one dependency (the reserved 11th slot) plus an `AtomicBool` checked between items. No safety property depends on it.                       |
| `--edit` ($EDITOR plan buffer) and any interactive prompt                                                                  | Low, additive. v1.1 (§2.3).                                                                                                                      |
| Confusable/skeleton warnings beyond M4's mixed-script detection                                                            | Low: a self-contained detection-only stage over a UTS #39 table from the M4 generator. It rewrites nothing, so it cannot regress Safety closure. |
| Full-width/halfwidth folding (#140)                                                                                        | Low: one opt-in stage. NFKC is the wrong hammer, which is why it waits.                                                                          |
| Per-mount fallback-warning granularity                                                                                     | Very low: `OnceLock` to `HashSet<PathBuf>` at one call site.                                                                                     |
| `--no-journal`, `--hidden`, exit code 3                                                                                    | Trivial, one flag each. Deferred until something needs them, not scheduled speculatively.                                                        |
| Parallelism                                                                                                                | Medium, and unjustified until the M1 `criterion` baseline says otherwise. A small capped pool, never a work-stealing one.                        |
| Content-derived names, moving files between directories, overwrite-on-collision, `detoxrc` parsing, `.gitignore` awareness | **Never.** §5.2 and P3 are safety properties, not missing features.                                                                              |
| Windows as tier 1                                                                                                          | Unknown, and honestly so — blocked on spikes 3 and 4, which no available hardware can close.                                                     |
| Native Debian/Fedora packages; `detoxrs-core` API stability                                                                | Post-1.0, once the dependency tree is stable (§9.4).                                                                                             |

## 11. Required proposal amendments

Changes to `docs/research/00-proposal-rust-detox-successor.md` that this plan implies. **Not
applied here.** Each is written to be applied mechanically.

1. **§3.1**, the `Policy` sketch and the paragraph beginning "`transform` is a pure function":
   replace the single `max_len` with `max_len_bytes` and `max_len_utf16`, and restate the resolved
   invariant as "both length fields are concrete numbers, never the CLI's `0 = auto` sentinel".
2. **§3.10**, after the limits table: add the field mapping — ext4 `bytes = 255, utf16 =
usize::MAX`; APFS `utf16 = 255, bytes = usize::MAX`; unknown volume and NTFS/exFAT under their
   standing assumption, both `255`.
3. **§3.10**, the `--max-len N` sentence: state that `N` sets the field for the detected volume's
   own unit and leaves the other at its detected value, and that on an unknown volume it sets
   both. Do not adopt "sets both to N" unconditionally.
4. **§3.10 step 2 and step 3**: state that both paths call one shared grapheme-truncation helper,
   not two loops.
5. **§8.1**, the Length-bound row: re-word to name `max_len_bytes` and `max_len_utf16` explicitly.
6. **§8.1**, the blanket scoping paragraph: "`max_len` a concrete number" becomes "both length
   fields concrete".
7. **§8.1**, the Stage-independence row: add that the substitution seam is an internal
   `pub(crate)` stage mask over the linear stages, that stages 12 and 13 are tested directly
   rather than through the mask, and that the property lives in an in-crate `#[cfg(test)]` module
   so no public API is widened for it.
8. **§7.1**, `pipeline.rs`'s description: add the requirement that each linear stage is its own
   named function and that one internal `run_with(input, policy, disabled)` composes them —
   `pipeline::transform` being the all-stages-on case. State that this is a testability
   requirement, not a style preference.
9. **§8.4**: add the "TOCTOU collision during apply" row, quoted in §5.3 above, with "Linux,
   macOS" as its Where.
10. **§8.2**, the No-pre-existing-clobber row: add that the property covers the plan-time half
    only, because `plan()` has no I/O, and cross-reference the new §8.4 row for the apply-time
    recheck and the kernel refusal.
11. **§5.8**: replace the SIGINT/SIGTERM paragraph with the v1.0 decision — no handler — and its
    reason: std exposes no signal API, `rustix::runtime::kernel_sigaction` is `pub unsafe fn` and
    Linux-only, a crate would spend the last budget slot on a cosmetic, and the
    `intent`/fsync/rename/`done` protocol already covers the strictly harsher `SIGKILL` case, with
    `undo --last` reverting the recorded prefix. Keep the `EROFS`/`ENOSPC`/`EMFILE` and
    error-taxonomy paragraphs untouched.
12. **Review record (stage 3)**, the row "No SIGINT/SIGTERM handling anywhere ... ACCEPTED": mark
    it **SUPERSEDED** per the document's own convention, pointing at the amended §5.8. The finding
    that the gap existed was correct; the remedy is not implementable under the constraints.
13. **§5.4**: add a repeat-measurement line — on 2026-07-31, `rustix` 1.1.4
    `renameat_with(NOREPLACE)` under `#![forbid(unsafe_code)]` on APFS returned `Ok(())` on a
    same-inode case-only respell and `Err(EEXIST/17)` onto a distinct occupied destination — and
    label the observed-`EEXIST` same-inode fallback defensive-only rather than a normal path.
14. **§8.3**: record the corpus storage decision — Rust `b"..."` constants in
    `crates/detoxrs-core/tests/support/corpus.rs` with a per-entry
    `disk_constructible_everywhere` flag, never a checked-in file whose name is the payload — and
    the verified reason: APFS refuses invalid-UTF-8 names with EILSEQ (errno 92), reproduced
    2026-07-31 for both `b"bad\xffname.txt"` and `b"Bj\xf6rk - Vespertine.mp3"`.
15. **§11 spike 15**: relabel from open question to **closeable and scheduled as an ordinary CI
    row** (`std::fs::hard_link` plus a respell, both OSes, no exotic hardware). Remove it from the
    "v1.0, everything else" gate row in the gating table.
16. **§11 spike 13**: add that it blocks the v0.1 _release_ specifically, unlike spikes 2 and 14,
    because a silently-clobbering volume produces no error for demotion-on-error to observe.
17. **§3.1**, `decode`'s signature: drop the `&Policy` parameter. With repair gone by owner
    decision there is no field for it to read, and P2 makes encoding a non-policy. Sweep §3.2's
    stage-1 row, §8.1's decode property, and §7.1's `decode.rs` line for the same signature.
18. **§7.2**: note that the enforced `just dep-budget` recipe reads only the `[dependencies]`
    table, so `[build-dependencies]` and `[target.'cfg(..)'.dependencies]` currently escape the
    cap, and state the intent that they count — to be applied to the recipe when the first
    `build.rs` lands (M4).
19. **§10 v0.1**: add the two behaviors this plan's v0.1 ships differently from the section's
    current implication — no signal handler, and a hardcoded `bytes = 255, utf16 = 255` limit pair
    until v0.3's per-directory detection replaces it — alongside the existing stage-2 and stage-11
    deferral notes.
20. **§7.1**: add `apply.rs` to the binary crate's layout, described as the apply loop plus the
    fresh `symlink_metadata` recheck written against `&dyn RenameOps` and a journal-writer trait,
    so the Undo round-trip property and the TOCTOU test can drive it from an in-memory double
    without moving the `RenameOps` trait into `detoxrs-core`.
