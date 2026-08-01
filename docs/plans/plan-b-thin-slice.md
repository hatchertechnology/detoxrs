---
plan: B
mandate: thin vertical slice — fastest path to a tool a real user can run
author: Plan Author B (of three independent authors)
date: 2026-07-31
documents_read:
  - docs/owner-decisions.md
  - docs/research/00-proposal-rust-detox-successor.md (full, including Review record
    (stage 3), Propagation record (stage 3), and Rejected-and-why sections at the end)
  - docs/research/user_feedback_online.md
  - docs/rust-setup-notes.md
  - docs/rust-setup-ci.md
  - docs/rust-setup-governance.md
  - docs/rust-setup-release.md
  - docs/rust-setup-supply-chain.md
  - repo state: Cargo.toml, justfile, crates/detoxrs/src/main.rs,
    crates/detoxrs-core/src/lib.rs (read directly, not from the proposal)
note: >
  This is one of three independent plans under different mandates. It does not
  attempt consensus with the other two. A later reviewer merges them.
---

# Plan B: thin vertical slice

## 0. What "thin slice" means here, precisely

**The slice.** `detoxrs [-r] [-x] [--on-collision number|skip|fail] [-v|-q] [--json] <PATH>...`
plus `detoxrs undo [--last | <BATCH-ID>] [--list]`. Preview by default; `-x` renames. No config
file, no profiles, no custom rules, no transliteration, no `--target`, no plan files, no `--stdin`.

**What a user can do with it.** Point it at a real, messy directory — the best-evidenced use case
in `user_feedback_online.md` (§"Top use cases", items 1-4: cross-OS import cleanup, pre-backup
sanitization, bulk cleanup of thousands of accented/spaced files, the `convmv -> detox -r ->
rename -> mogrify` pipeline) — preview it, apply it, and undo it if unhappy. What they cannot do
yet: keep that decision (no config), exclude a handful of named files from the run except the
three VCS directories and dotfiles-during-recursion (no `--exclude`), fix a stray `%20`, or ask
for lowercase/ASCII-only output. Those are the very next milestones (§2), not missing safety.

This is not a novel scope I invented to be aggressive: it is **exactly** the proposal's own v0.1
(§10, "the walking skeleton"), and the proposal's own reasoning for the cut line is the one I am
adopting rather than second-guessing: _"the MVP boundary is drawn at 'safety architecture
complete, customization absent,' because §5 is the part that is hard to retrofit and §4 is the
part that is trivial to add."_ My contribution as the thin-slice author is not to argue for a
different cut — the research already found the right one — but to (a) argue for shipping it
_before_ building anything past it, in unusual file-by-file detail, and (b) find the places inside
that already-thin scope where even more can be shaved without weakening the safety guarantee,
because "thin slice" is a discipline that should be applied recursively, not just once at the
milestone boundary.

### 0.1 Pipeline stages: in and out, and why the subset is coherent

From §3.2's 13-stage table, v0.1 ships stages **1, 3, 4, 7, 9, 10, 12, 13** and defers **2, 5, 6,
8, 11**.

| #   | Stage             | In M1?                          | Why                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| --- | ----------------- | ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `decode`          | **Yes**                         | Not optional. This is the whole fix for detox's worst bug class (§3.4, P2) and it is one branch (`Utf8`/`Opaque`), not a feature to phase in.                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 2   | `url_decode`      | No (M2)                         | On by default in the full design, but it is "the fiddly part" (§10) — an all-or-nothing validity check per name. Deferring it is _visible_: `invoice%20final.pdf` stays as-is in M1. Stated, not hidden.                                                                                                                                                                                                                                                                                                                                                                              |
| 3   | `normalize` (NFC) | **Yes, hardcoded on**           | No `--normalize` flag in M1 — there is exactly one mode. Comparison inside the planner is always NFC regardless of the flag in the full design anyway (§3.2 row 3), so hardcoding it changes nothing about correctness, only removes a flag surface.                                                                                                                                                                                                                                                                                                                                  |
| 4   | `invisible_strip` | **Yes, narrowed**               | The security-relevant reason this stage exists at all is CVE-2021-42574-class bidi/zero-width abuse (§3.12), not general Unicode hygiene. M1 hardcodes the _named_ codepoint set from §3.2 row 4 (bidi U+202A-202E/U+2066-2069/U+200E/200F, zero-width U+200B/200C/200D/2060/FEFF, Tags U+E0000-E007F) rather than a UCD-generated closure over all of `Cf`/`Cs`/`Co`. This is a deliberate narrowing _inside_ an already-in-scope stage, argued in §3 below.                                                                                                                         |
| 5   | `rules`           | No (M3)                         | The one customization slot. Nothing to customize until there's somewhere durable to write a rule, i.e. a config file.                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 6   | `ascii`           | No (M4)                         | Off by default even in v1.0 (P4). Deferring an opt-in, off-by-default, lossy transform costs nothing to a v0.1 user who never asked for it.                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 7   | `safe_map`        | **Yes**                         | The actual "make it sane" transform. Hardcoded delete/separator/keep classes (§3.7), no `--keep`/`--strip` override yet.                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 8   | `case`            | No (M4)                         | `keep` is already the default; there is no visible difference until a user asks for `lower`/`upper`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 9   | `collapse`        | **Yes**                         | Cheap, and without it stage 7 produces visibly wrong output (`" & "` -> `"___"` uncollapsed). Shipping 7 without 9 would be a worse product than shipping neither.                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 10  | `trim`            | **Yes**                         | Same argument: leading `-`, trailing dot/space, and the one-leading-dot rule are inseparable from a safe-map stage that can produce them.                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 11  | `target`          | No (M5)                         | Identity under the default `--target unix` (§10's own note: "this one costs nothing observable"). The one thing it would add globally — trailing dot/space stripping — already lives in stage 10.                                                                                                                                                                                                                                                                                                                                                                                     |
| 12  | `truncate`        | **Yes, with a hardcoded limit** | Grapheme-safe truncation is a correctness property (§8.1, "No grapheme splitting"), not a nicety, so it ships. What's narrowed: the limit is a **hardcoded conservative constant** (255 bytes AND 255 UTF-16 units, simultaneously — §3.10's own "satisfy both metrics" rule, just without the per-directory `statfs` detection that picks a _looser_ limit on a filesystem that allows one). No filesystem gets truncated more aggressively than the real limits in §3.10's table permit; some get truncated slightly more eagerly than necessary. That is conservative, not unsafe. |
| 13  | `finalize`        | **Yes, over stages 9/10 only**  | Matches the proposal's own v0.1 note verbatim: "v0.1's stage 13 fixed-point loop re-runs 9/10 only" because stage 11 isn't in scope yet.                                                                                                                                                                                                                                                                                                                                                                                                                                              |

Net effect: a user gets space/punctuation cleanup, safe-character mapping, run-collapsing,
trimming, grapheme-safe truncation, and the invisible-character security fix — the transform that
actually matters for messy import trees — without url-decoding, custom rules, transliteration,
case-folding, or Windows targeting. Every one of those five deferred items is either off-by-default
in the full design anyway, or additive syntax that changes nothing about names that don't use it.
None of them touch the safety architecture.

### 0.2 The non-negotiable guarantee, stated exactly

**Never a clobbering rename, guaranteed by exactly two mechanisms, both present from commit one:**

1. **Preview is the only thing that happens without `-x`.** `main.rs`'s top-level branch is `if
!exec { print_report(&plan); return; }` — there is no code path in M1 that can reach a `rename`
   call without that flag being true. This is not a convention to remember; it is one `if`
   statement gating the only call to `RenameOps::rename_noreplace`.
2. **The one rename primitive is kernel-level no-clobber.** `rustix::fs::renameat_with(...,
RenameFlags::NOREPLACE)` — `renameat2(RENAME_NOREPLACE)` on Linux, `renameatx_np(RENAME_EXCL)`
   on macOS, both from safe code, both under `#![forbid(unsafe_code)]` (§5.4, already applied in
   both crates per `docs/rust-setup-notes.md`). If the destination exists, the syscall itself
   refuses with `EEXIST`; there is no window where the tool "checks then renames" except in the
   documented, `--json`-flagged fallback path for filesystems that don't support the flag, and that
   fallback still never calls a clobbering `rename` — it refuses if `symlink_metadata` finds the
   destination occupied.

Nothing else is required to make this true. Collision detection (layers 1 and 2 of §5.3), the
journal, and `undo` are all in M1 too, but they are not what makes clobbering impossible — the
syscall is. That separation matters for this plan specifically: it means the parts I could cut
for speed (journal, `undo`, collision numbering) are _convenience and recoverability_, not the
safety property itself, and I did not cut them (see §1), but I want the boundary stated so a
reviewer can see exactly what would remain true even if they were.

---

## 1. Milestones after the slice

Ordered by value delivered per unit of effort, not by the proposal's own v0.1/v0.2/v0.3/v1.0
grouping — I've resequenced within that grouping where a cheap, high-visibility item was bundled
with an expensive one. Estimates are in **story-points (SP)**, where 1 SP is roughly "a single
cohesive module with its own unit tests, reviewable in one sitting" — deliberately not calendar
time, which depends on who's available.

### M1 — the slice itself (detailed file-by-file in §2)

Scope: everything in §0. **Exit criteria:**

- `cargo test --workspace` passes, including the property-test subset in §2.6.
- `just gate` green.
- Manual acceptance: running `detoxrs -x -r <a directory the author copies from a real
Downloads folder>` produces a diff a human agrees is "sane," and `detoxrs undo --last`
  restores it exactly.
- A `trycmd` case reproducing §2.2's first two worked examples from the proposal (the
  `Screen Shot ... .png` and `Mario & Luigi` renames) byte-for-byte, **minus** the `%20` line,
  which is correctly _not_ decoded yet.

Estimate: **13 SP.** (Breakdown in §2.7.)

### M2 — url_decode, `--exclude`, `--files-only`/`--dirs-only`, verbosity/color

Scope: stage 2 (§3.11's all-or-nothing rule); a repeatable `--exclude <GLOB>` CLI flag (globs
compiled to `regex`, introducing that dependency); `--files-only`/`--dirs-only`; `-v`/`-vv`/`-q`;
`--color`. **Deliberately no config file yet** — `--exclude` is a CLI flag because a _pattern
list_ needs no persistence to be useful for a single invocation, and shipping it before the config
system exists is the single highest-value/lowest-cost item left in the backlog: it's what lets a
user keep `.DS_Store`/`Icon\r`/`*.crdownload` out of a run _today_, which is exactly the friction
`user_feedback_online.md`'s evidence points at (macOS-originated import trees, item 1 in its "Top
problems" table). Files touched: `crates/detoxrs-core/src/percent.rs` (new), `pipeline.rs`
(insert stage 2), `crates/detoxrs/src/cli.rs`, `walk.rs` (exclude filtering), `Cargo.toml` (add
`regex`).
Exit criteria: property test "percent-decode is all-or-nothing" (a name with any malformed `%xx`
decodes to itself unchanged); `trycmd` case with `%20`/`%25`/malformed-`%` fixtures matching
§3.11's worked examples; `--exclude 'Icon\r'` verified to leave that literal name untouched in a
`trycmd` fixture.
Estimate: **5 SP.**

### M3 — config file, discovery, profiles, `--print-config`

Scope: §4.1-4.3 in full: TOML load, `--config`/`$DETOXRS_CONFIG`/nearest-`.detoxrs.toml`/XDG
fallback with first-match-wins (no merging), `[profile.NAME]` selected by `-p`, `--print-config`
with the resolve-don't-echo and validate-everything-compilable rules from §4.3. This is the
milestone that resolves the UX tension in §2.1: it is what lets `exec = true` in a user's config
turn `detoxrs ~/Downloads` into an apply-on-first-run tool for that user, without ever changing
the binary's own default. Files touched: `crates/detoxrs/src/config.rs` (new, ~150 lines per
§7.2's own estimate), `cli.rs` (config resolution wiring), `Cargo.toml` (add `serde`, `toml`).
Exit criteria: a `trycmd` fixture tree with `--config`, `$DETOXRS_CONFIG`, nearest-file, and XDG
fallback each individually verified to win when it should; `--print-config` snapshot test
(`insta`) for a config with a deliberately invalid regex, asserting exit 2 and the _same_ error
message a real run gives, per §4.3's `-L`-lesson requirement; a snapshot test proving `max_len = 0`
is printed with its "resolved per directory at walk time" comment rather than a number.
Estimate: **8 SP.**

### M4 — `[[rule]]`, `--keep`/`--strip`, `--case`, `--ascii`

Scope: stages 5, 6, 8; the `deunicode` dependency (feature-gated, default on); mixed-script
warning (§3.12's "cheap subset" — detection only, never rewriting, no confusable-table work,
which stays out of v1.0 per §3.12/§10). Files touched: `crates/detoxrs-core/src/rules.rs` (new),
`scripts.rs` (new, hand-written mixed-script check over a small hardcoded confusable-script set —
**not** the UTS #39 generated table, which is v1.1 per §3.12; this is a further narrowing inside
an already-deferred stage, same move as §0.1's invisible-strip narrowing), `pipeline.rs` (insert
5/6/8), `classes.rs` (`--keep`/`--strip` override support).
Exit criteria: property test "Stage independence" (§8.1) specifically for stages 5/6/8, since
that's the property detox's own history violated (#40/#86); snapshot fixtures for `ü -> ue` via a
user rule running before `--ascii` (§3.3's `#117`/`#121` worked example).
Estimate: **8 SP.**

### M5 — `--target`, plan files, `--stdin`, per-directory length detection

Scope: stage 11; `--plan-out`/`apply` (§5.7); `--stdin` (no filesystem access, §2.4); replacing
M1's hardcoded 255/255 constant with the real per-directory `statfs`-based detection from §3.10.
This is where the debt named in §3 gets paid down. Files touched: `crates/detoxrs-core/src/
reserved.rs` (new), `crates/detoxrs/src/limits.rs` (new), `journal.rs` extended for plan-apply
staleness check (`(dev, ino, mtime)` recheck per §5.7).
Exit criteria: `trycmd` fixture for `CON.txt`/`nul.c` under `--target windows`; a stale-plan test
(mutate the tree between `--plan-out` and `apply`, assert refusal); a length-detection test on
both APFS images and ext4/tmpfs matching doc 06 Test 1's numbers, superseding M1's hardcoded
constant in the same commit that deletes it.
Estimate: **8 SP.**

### M6 — v1.0 hardening

Scope: Windows best-effort tier building and unit-tested in CI (already scaffolded per
`rust-setup-ci.md`; this is "make the CLI code compile and unit-test there," not "verify Windows
filesystem behavior" — see §5); the full §8 test matrix green on both APFS variants and three
Linux filesystems; `MIGRATING-FROM-DETOX.md`, `--help-transforms`; packaging items 1-4 from §9.4;
the `cargo-fuzz` target running in CI (§8.5).
Estimate: **13 SP** (mostly test-matrix and CI plumbing, not new product surface).

**Running total after M6: ~55 SP**, none of it spent before M1 ships.

---

## 2. M1 in file-by-file detail

### 2.1 `crates/detoxrs-core/` (no I/O, no `clap`, no `std::fs` — unchanged constraint)

**`src/lib.rs`** — replaces the placeholder. Re-exports `policy`, `decode`, `classes`,
`invisible`, `pipeline`, `truncate`, `plan`. Deletes `placeholder_version` (its only job was
giving `main.rs` something honest to print before real logic existed — §7.1's TODO comment says
exactly this).

**`src/policy.rs`** (new, ~30 lines)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    pub separator: char,           // fixed '_' in M1; no --separator flag yet
    pub on_collision: OnCollision,
    pub max_len_bytes: usize,      // hardcoded 255 in M1 (§3 debt item)
    pub max_len_utf16: usize,      // hardcoded 255 in M1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnCollision { Number, Skip, Fail }

impl Default for Policy {
    fn default() -> Self {
        Self { separator: '_', on_collision: OnCollision::Number,
               max_len_bytes: 255, max_len_utf16: 255 }
    }
}
```

`--separator` is not exposed as a flag in M1 even though it costs little — it's excluded because
exposing one taste flag without the config file to hold it invites "why can I set this on the CLI
but not save it," which is exactly the kind of surprise §4.3 warns about. It arrives in M3 with
the rest of the customization surface it belongs next to.

**`src/decode.rs`** (new, ~15 lines)

```rust
pub enum Decoded { Utf8(String), Opaque }

#[must_use]
pub fn decode(raw: &std::ffi::OsStr) -> Decoded {
    match raw.to_str() {
        Some(s) => Decoded::Utf8(s.to_owned()),
        None => Decoded::Opaque,
    }
}
```

No `Policy` parameter — P2 (§1, design principles) means encoding is never a policy input, so the
signature should not imply it is one. `to_str()` on Unix `OsStr` is exactly the UTF-8 validity
check §3.4 specifies; nothing hand-rolled here.

**`src/classes.rs`** (new, ~40 lines) — hardcoded, no `--keep`/`--strip` override in M1:

```rust
pub enum CharClass { Delete, Separator, Keep }

#[must_use]
pub fn classify(c: char) -> CharClass {
    if c.is_control() { return CharClass::Delete; }               // Cc, incl. DEL/NUL
    if matches!(c, ' ' | '"' | '\'' | '`' | '$' | '!' | '*' | '?'
                 | '[' | ']' | '<' | '>' | '|' | ';' | '&' | ':'
                 | '\\' | '/' | '(' | ')') { return CharClass::Separator; }
    CharClass::Keep
}
```

This is §3.7's table as code, verbatim, no table lookup structure — it is genuinely this small.

**`src/invisible.rs`** (new, ~25 lines) — the narrowed set from §0.1, not the UCD generator:

```rust
#[must_use]
pub fn is_invisible(c: char) -> bool {
    matches!(c as u32,
        0x202A..=0x202E | 0x2066..=0x2069 | 0x200E | 0x200F   // bidi
        | 0x200B..=0x200D | 0x2060 | 0xFEFF                    // zero-width
        | 0xE0000..=0xE007F)                                    // Unicode Tags
}
```

No build-time UCD generation script in M1. That script (§7.1's `invisible.rs` comment: "generated
from UCD; build-time script, data checked in") is real work — a codegen step, a data file, a
regeneration story — for the general `Cf`/`Cs`/`Co` closure the full design wants. The named set
above is the one CVE-2021-42574 and the tracker's own `#120`/`#116` (§3.12) are actually about.
Swapping this function's body for a generated table later is a same-signature, same-module change
— see the debt argument in §3.

**`src/truncate.rs`** (new, ~60 lines)

```rust
pub struct Limits { pub bytes: usize, pub utf16: usize }

#[must_use]
pub fn truncate(stem: &str, ext: &str, limits: &Limits) -> (String, bool) {
    // 1. grapheme-safe truncation of `stem` via unicode_segmentation::UnicodeSegmentation,
    //    to the largest prefix satisfying both limits.bytes and limits.utf16 once `ext`
    //    is re-appended. Returns (new_name, was_truncated).
    // 2. if `ext` alone doesn't fit, truncate the whole name (stem+ext) as one unit,
    //    same grapheme-cluster algorithm, per §3.10 step 3.
}
```

Extension-splitting (the `.tar.gz` <=4-byte-lookback rule, §3.10 step 1) is a separate small
function `split_extension(name: &str) -> (&str, &str)` in the same file, called from
`pipeline.rs` before `truncate`.

**`src/pipeline.rs`** (new, ~80 lines)

```rust
pub struct Outcome { pub text: String, pub truncated: bool }
pub enum TransformResult { Name(Outcome), Unrepresentable(Unrepresentable) }
pub enum Unrepresentable { ReducesToEmpty, ReducesToDotOrDotDot, NotConverged }

#[must_use]
pub fn transform(input: &str, p: &Policy) -> TransformResult {
    // NFC normalize (unicode_normalization::UnicodeNormalization::nfc)
    // -> strip invisibles (invisible::is_invisible)
    // -> safe_map (classes::classify)
    // -> collapse runs of '.', '-', '_', p.separator
    // -> trim (leading '-'; leading/trailing separator; trailing '.'/' '; one leading '.')
    // -> truncate (truncate::truncate, using p.max_len_bytes/max_len_utf16)
    // -> finalize: re-run collapse+trim up to 3 times (bounded fixed point, §3.14),
    //    checking for empty/"."/".." after each pass
}
```

This is stages 3/4/7/9/10/12/13 from §3.2, in that order, exactly as scoped in §0.1's table. No
`Vec<StageDelta>`/`Vec<Note>` tracing yet (the full `Outcome` shape in §3.1 has both, for `-vv` and
snapshot tests) — M1's `Outcome` carries only what M1's own `-v` needs (`truncated: bool`), and
grows when `-vv`'s per-stage trace becomes a real feature (M4, alongside the stages that make a
trace worth reading).

**`src/plan.rs`** (new, ~150 lines) — the collision engine, unnarrowed, because this is squarely
inside §5 (the hard-to-retrofit half):

```rust
pub struct Entry { pub dir: std::path::PathBuf, pub name: std::ffi::OsString,
                    pub kind: EntryKind, pub ident: Ident }
pub enum EntryKind { File, Dir, Symlink, Other }
pub struct Ident { pub dev: u64, pub ino: u64, pub nlink: u64, pub mtime: std::time::SystemTime }

pub struct PlanItem { pub dir: std::path::PathBuf, pub from: std::ffi::OsString,
                       pub to: std::ffi::OsString, pub kind: EntryKind, pub ident: Ident,
                       pub depth: u32, pub resolution: Resolution }
pub enum Resolution { Rename, Unchanged, Skipped(SkipReason), Conflict }
pub enum SkipReason { NotUtf8, Unrepresentable(Unrepresentable) }

#[must_use]
pub fn plan(entries: Vec<Entry>, p: &Policy) -> Vec<PlanItem> {
    // 1. decode + transform each entry's name (skip Opaque/Unrepresentable per §3.4/§3.14)
    // 2. build (dir, NFC(to)) -> Vec<source> map; anything with >1 source is an
    //    intra-batch conflict (§5.3 layer 1)
    // 3. check each `to` against the frozen snapshot for a pre-existing, unrelated
    //    occupant (§5.3 layer 2)
    // 4. resolve conflicts per p.on_collision: Number (deterministic N=2..999 search,
    //    §5.3), Skip, or Fail (refuse the whole batch -- returns Err upstream, not
    //    modeled in this signature; M1's caller checks for any Conflict and aborts
    //    before calling apply if on_collision == Fail)
    // 5. sort output deepest-first by `depth` (§5.1)
}
```

The sibling-chain assertion from §5.3 ("if any Rename item's destination equals the `from` of
another Rename item, refuse the entire batch") is a `debug_assert!`-gated check plus a real
runtime check in `plan()` — cheap, and it is the thing that turns "we have a proof this can't
happen" into "and we'll notice immediately if it ever does."

### 2.2 `crates/detoxrs/` (the binary)

**`src/main.rs`** — replaces the placeholder `println!`. Parses CLI, resolves a hardcoded
`Policy::default()` overridden only by the M1 flags (`--on-collision`), walks, plans, prints the
report, and — only if `-x` — opens a journal and applies.

**`src/cli.rs`** (new, ~50 lines)

```rust
#[derive(clap::Parser)]
pub struct Cli {
    pub paths: Vec<std::path::PathBuf>,
    #[arg(short = 'r', long)] pub recursive: bool,
    #[arg(short = 'x', long)] pub exec: bool,
    #[arg(short = 'n', long, conflicts_with = "exec")] pub dry_run: bool,
    #[arg(long, value_enum, default_value = "number")] pub on_collision: OnCollisionArg,
    #[arg(short = 'v', action = clap::ArgAction::Count)] pub verbose: u8,
    #[arg(short = 'q', long)] pub quiet: bool,
    #[arg(long)] pub json: bool,
    #[command(subcommand)] pub command: Option<Command>,
}
#[derive(clap::Subcommand)]
pub enum Command { Undo { #[arg(long)] last: bool, batch_id: Option<String>,
                           #[arg(long)] list: bool } }
```

**`src/walk.rs`** (new, ~60 lines)

```rust
pub fn snapshot(paths: &[std::path::PathBuf], recursive: bool)
    -> std::io::Result<Vec<detoxrs_core::plan::Entry>> {
    // walkdir::WalkDir per path. max_depth(1) unless `recursive`, matching §5.6's
    // "one argument, one name" vs "one argument, its whole tree" rule -- no third
    // behavior, and no --hidden flag yet, so dotfiles are always skipped while
    // recursing and always processed when named explicitly (§5.6, unconditional
    // in M1, since --hidden doesn't exist until it has a use: M2 doesn't need it
    // either, so it's not promised until customization work resumes).
    // Skips .git/.hg/.svn unconditionally (§4.4, never configurable, ever).
    // Uses symlink_metadata (lstat), never stat. walkdir's default
    // follow_links(false) already refuses to descend into a symlinked
    // directory -- zero extra code needed for the §5.6 hazard.
}
```

**`src/fsops.rs`** (new, ~50 lines)

```rust
pub trait RenameOps {
    fn rename_noreplace(&self, dir: &std::path::Path, from: &std::ffi::OsStr,
                         to: &std::ffi::OsStr) -> Result<(), RenameErr>;
}
pub struct RealRenameOps;
impl RenameOps for RealRenameOps {
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr> {
        use rustix::fs::{renameat_with, RenameFlags, CWD};
        let dirfd = rustix::fs::openat(CWD, dir, OFlags::DIRECTORY | OFlags::RDONLY, Mode::empty())?;
        match renameat_with(&dirfd, from, &dirfd, to, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(()),
            Err(e) if is_unsupported(e) => fallback::check_then_rename(dir, from, to),
            Err(e) => Err(e.into()),
        }
    }
}
```

`RenameOps` is a trait specifically so `plan.rs`'s consumers (the apply loop in `main.rs`) can be
tested against a fake in-memory implementation without touching a real filesystem — this is the
one seam in the binary crate that gets a mock, because it's the one seam where "trust the syscall,
don't re-test the kernel" and "test our own retry/demotion logic" are genuinely different jobs.

**`src/fsops/fallback.rs`** (new, ~25 lines) — `check_then_rename`, `symlink_metadata` then plain
`rename`, warns once **globally** (a single `OnceLock<()>`), not per-mount. Per-mount tracking
(§5.4's stated design) is deferred: M1 handles it as a global "you hit the fallback path at least
once" warning rather than naming every distinct mount that needed it. Cheap to generalize into a
`HashSet<PathBuf>` keyed by mount point later; not worth the extra code before there's a multi-
filesystem run to observe it on.

**`src/journal.rs`** (new, ~90 lines) — the intent/fsync/rename/done protocol from §5.5, unabridged:

```rust
pub struct Journal { file: std::fs::File, batch_id: String }
impl Journal {
    pub fn open_new(state_dir: &Path) -> std::io::Result<Self>;   // creates <ts>-<id>.jsonl
    pub fn record_intent(&mut self, item: &PlanItem) -> std::io::Result<()>;  // + fsync
    pub fn record_done(&mut self, ident: &Ident) -> std::io::Result<()>;
    pub fn record_failed(&mut self, ident: &Ident, err: &str) -> std::io::Result<()>;
}
pub fn replay_for_undo(batch_id: &str, state_dir: &Path) -> std::io::Result<Vec<UndoItem>>;
pub fn list_batches(state_dir: &Path) -> std::io::Result<Vec<BatchMeta>>;
```

This is not narrowed relative to the full design — §5.5's crash-safety protocol is safety
architecture, not customization, so it ships whole in M1. `--no-journal` (the opt-out) is **not**
in M1: it's a flag whose only job is disabling a feature M1 users haven't yet had a reason to find
slow, so it's deferred to whichever milestone first needs it for a huge-tree scripted use case
(most naturally alongside M5's plan files).

**`src/report.rs`** (new, ~80 lines) — human preview text matching §2.2's worked examples (minus
the `%20` line, per §0.1), `--json` via `serde_json`, exit codes 0/1/2 (exit code 3, "`--quiet` and
nothing matched," is a one-line addition deferred to whichever milestone first has a reason to
special-case an empty walk — not worth its own test fixture in M1).

### 2.3 Dependency additions for M1

Six direct dependencies, against the budget of 11 (§4):

| Crate                   | Why in M1 specifically                                                                                                                                                           |
| ----------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clap` (derive)         | Arg parsing, `-x`/`-r`/`-n`/`--on-collision`/`-v`/`-q`/`--json`, the `undo` subcommand.                                                                                          |
| `serde_json`            | `--json` output shape; also the journal's JSONL lines (one JSON object per line, hand-serialized field by field is more code than using the crate we already need for `--json`). |
| `unicode-normalization` | NFC.                                                                                                                                                                             |
| `unicode-segmentation`  | Grapheme-safe truncation — the one correctness property (§8.1) that cannot be hand-rolled without re-deriving UAX #29.                                                           |
| `walkdir`               | Recursive walk with the symlink-non-descent default already doing the §5.6 work for free.                                                                                        |
| `rustix` (feature `fs`) | The no-clobber rename, from safe code, on both tier-1 platforms, per §5.4. Non-negotiable — see §0.2.                                                                            |

`regex`, `serde`, `toml`, `deunicode` are **not** in M1: nothing in M1's scope needs them
(§0.1 excludes stages 2/5/6/8, and `--exclude` isn't a flag yet either). They arrive in M2-M4 per
§5's schedule. `terminal_size` is not in M1: the preview layout is a fixed two-column format
(name -> new name), which needs no terminal width to render correctly; it stays the 11th,
possibly-unneeded budget line exactly as §7.2 frames it.

### 2.4 What M1 explicitly does not attempt to make safe (because it doesn't need to)

- **EXDEV.** Structurally impossible (§5.2) — M1 never takes a destination directory argument,
  so there is no code path to test against it.
- **Batch atomicity.** Never attempted, in any milestone (§5.4: "we will not pretend otherwise").
- **Concurrent invocations.** Explicit non-goal per §5.8; M1 inherits that non-goal rather than
  re-deriving it, since the argument ("no-clobber renames mean neither run can destroy the
  other's file") holds regardless of which milestone is shipping.

### 2.5 Interrupts and I/O failure in M1

SIGINT handling (§5.8) ships in M1: a flag checked between apply-loop items, clean summary +
journal close on interrupt. This is cheap (a handful of lines around `signal-hook`-free use of
`std::sync::atomic` set from `ctrlc`-style registration via the standard library's own
`std::os::unix`-level signal primitives is more machinery than needed — the simplest correct
version is a `static ATOMIC_BOOL` set by a minimal signal handler installed via `libc`-free
`std::process`... on reflection, this needs one crate or ~15 lines of platform-conditional code).
**Decision:** M1 uses the check-a-flag-between-items pattern with the flag set by a small
hand-written handler (no new dependency — `rustix` itself does not expose signal handling, so this
is a `std`-only, `unsafe`-free registration, which is achievable on both tier-1 platforms with
`std::os::unix::process`... if that proves not to fit cleanly under `forbid(unsafe_code)` without
a crate, the fallback is to defer SIGINT handling one milestone (to M2) rather than reach for
`libc`/`signal-hook` and grow the budget for a nice-to-have. This is flagged here rather than
resolved with false confidence, because it is the one item in M1 whose exact zero-dependency
shape is not yet proven — see the open item at the end of §2.7.

The `EROFS`/`ENOSPC`-aborts-the-batch behavior and the per-item error taxonomy (§5.8) ship in M1
in the one hand-written error enum (`RenameErr`, in `fsops.rs`) — this is a `match` arm count, not
a subsystem, so there's no reason to defer it.

### 2.6 Tests (exit criteria, not aspiration)

`detoxrs-core` property tests (`proptest`, dev-dependency — added in M1, not before, since this
is the first milestone with logic to test against):

- **Totality, Idempotence, Safety closure** restricted to the classes M1 implements (no
  `--case`, so the case-mapping clause of Safety closure is vacuous in M1 and stated as such in
  the test's doc comment).
- **Length bound** against the hardcoded 255/255 constants.
- **No grapheme splitting.**
- **Non-empty.**
- **Dotfile preservation.**
- **Decode is total and never re-interprets** (§8.1's exact statement — this is the direct
  regression test for `café.txt -> cafÃ©.txt`, and it's non-negotiable from commit one).

`plan.rs` property tests: **No collision**, **No pre-existing clobber**, **Order safety**, **No
sibling chains**, **Bounded renumbering**, **Determinism**. All six from §8.2's table — none
deferred, because this module is squarely inside §5.

`insta` snapshots (dev-dependency): `--help` text; the human preview for a fixture list that is
the §8.3 corpus **minus** the entries that only make sense once stage 2/5/6/8 exist (the `%20`/
`%25` pair moves to M2's snapshot set; `café.txt` NFC/NFD, the U+202E/U+200B names, the CP1252
byte string, and the invalid-UTF-8 lone `\xff` all stay, because stages 1/3/4/13 are exactly what
they exercise).

`trycmd` + `assert_cmd` (dev-dependencies): the filesystem matrix rows from §8.4 that M1's scope
actually touches — case-only rename, length-limit probe (against the hardcoded constant, not the
per-directory detector), `RENAME_NOREPLACE`-unsupported fallback, non-UTF-8 name, symlink-to-`../..`
non-descent, rename-during-walk, crash-mid-batch/journal-replay. The 200k-entry huge-tree
benchmark (`criterion`, dev-dependency) ships in M1 too — it's cheap to write against a pipeline
that already exists, and it's the regression test for the crash-bug class detox's own tracker
shows dominating (§8.4's five external crash reports), which is exactly the class a thin-slice
author should not be casual about deferring.

### 2.7 M1 estimate breakdown (13 SP total)

| Piece                                                                                                                                                                                                                                    | SP                                                                       |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| `detoxrs-core`: policy/decode/classes/invisible/truncate/pipeline                                                                                                                                                                        | 3                                                                        |
| `plan.rs` (collision engine, three layers, renumbering, sibling-chain check)                                                                                                                                                             | 3                                                                        |
| `fsops.rs` + fallback (the rustix call, demotion-on-error, mock `RenameOps` for tests)                                                                                                                                                   | 2                                                                        |
| `journal.rs` (intent/fsync/done protocol + replay)                                                                                                                                                                                       | 2                                                                        |
| `walk.rs` + `cli.rs` + `report.rs` + `main.rs` wiring                                                                                                                                                                                    | 1                                                                        |
| Property tests (§2.6, core + plan)                                                                                                                                                                                                       | 1                                                                        |
| Snapshot + `trycmd` + huge-tree benchmark                                                                                                                                                                                                | 1                                                                        |
| **Open item, not yet costed above:** SIGINT handling's exact zero-dependency shape (§2.5). If it needs a crate, that's a budget conversation (still well inside 11), not a scope conversation — flagged rather than hidden in the total. | (~0.5, folded into the wiring line above as a risk, not a separate line) |

---

## 3. Where debt is accepted, and the interest rate

| Deferred                                                                                    | What breaks if never fixed                                                                                                                                                                                                                                                                                                                                              | Retrofit cost                                                                                                                                                                                                                                                                                                                                                    | Interest rate                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No config file / no `--exclude` in M1                                                       | Users can't keep `.DS_Store`/`Icon\r` out of a run except via the three hardcoded VCS names; every run re-specifies nothing, because there's nothing to specify yet                                                                                                                                                                                                     | **Low.** §4.1's whole design point is "config keys are named identically to flags" — M3 adds a load path in front of already-existing `Policy` fields, touching no pipeline code. `--exclude` itself lands in M2, ahead of the config file, precisely because it's cheap and doesn't need one (CLI-only glob list).                                              | **Low**, but not zero: every week without `--exclude` is a week of first-run friction on real macOS-originated trees, which is the scenario `user_feedback_online.md` documents best. This is why `--exclude` is M2, not bundled into M3's bigger config lift. |
| Hardcoded 255-byte/255-UTF-16-unit length limit instead of per-directory `statfs` detection | On a filesystem with a _tighter_ real limit than 255 (none known in the research), truncation would under-fire; on one with a _looser_ limit, M1 truncates names that didn't need it. Neither loses data — truncation only ever produces a shorter, still-safe name, and a collision from over-eager truncation is caught by the collision engine, not silently merged. | **Medium.** §3.1 is explicit that a resolved `Policy` (a concrete `max_len`) is what keeps `transform`'s purity and the per-directory limit coexisting — the _interface_ is already shaped for this (`Policy` carries a number, never the CLI's `0=auto` sentinel), so M5 replaces a constant with a `statfs`-derived one behind the same field, not a redesign. | **Low and flat**, not compounding — ext4 and APFS are exactly 255 either way (§3.10's table), so the two platforms this project tier-1-tests on are unaffected by the constant being wrong on some other filesystem nobody's running yet.                      |
| `invisible.rs`'s named-codepoint set instead of a UCD-generated `Cf`/`Cs`/`Co` closure      | A general-category invisible character outside the named set (bidi/zero-width/Tags) survives into the output. The security-relevant class (Trojan Source) is fully covered; the completeness gap is cosmetic Unicode hygiene, not a hazard.                                                                                                                             | **Low.** Same function signature (`is_invisible(char) -> bool`), same call site in `pipeline.rs`. Swapping a hand-written `matches!` for a generated table is an internals-only change with zero API surface to migrate callers off of.                                                                                                                          | **Very low.** Nobody in `user_feedback_online.md` or the upstream tracker reports a _general_ invisible-character complaint distinct from the named hazard classes (§3.12 cites `#120`/`#116`, both of which the named set covers).                            |
| Per-mount fallback-warning granularity collapsed to one global warning                      | A user on a mixed-filesystem tree (e.g., an ext4 root with an NFS mount inside it) sees one warning instead of one per distinct filesystem that needed the fallback                                                                                                                                                                                                     | **Low.** `OnceLock<()>` to `HashSet<PathBuf>` keyed by mount point, same call site.                                                                                                                                                                                                                                                                              | **Very low.** Cosmetic, and only visible at all on a filesystem combination outside tier-1 testing.                                                                                                                                                            |
| `nlink > 1` respell behavior (§11 spike 15) carried as an open assumption, not resolved     | If a hardlinked file's respell interacts badly with the planner in some untested way, M1 ships the same "preview note, no refusal" posture the full design does — this is not a narrowing I invented, it's the proposal's own stated-open status                                                                                                                        | **Medium**, same as the full design's own accounting: adding a refusal for `nlink > 1` later is a new `Skipped` reason and a §5.6 doc change, not a rewrite.                                                                                                                                                                                                     | Same rate the full research already accepted; M1 doesn't add or remove risk here.                                                                                                                                                                              |

**Refused to defer, because retrofitting after real use would mean redesigning a live on-disk
contract:**

- **The two-phase snapshot/plan/apply split and deepest-first apply order (§5.1).** Once a journal
  format and an `undo` command exist and users have relied on them, changing the fundamental
  ordering guarantee they were built against means either a journal version bump with migration
  logic or asking early users to discard their undo history. Building it right the first time
  costs nothing extra in M1 (it's the natural way to write the walk/plan/apply functions anyway)
  and buying it later costs a compatibility story nobody wants to write.
- **`rustix`'s no-clobber rename as the _only_ rename entry point.** This is §0.2's whole
  argument: it's the one thing that makes the "never lose a file" claim true regardless of what
  else ships thin. There is no cheaper version of this that is still true.
- **`OsStr`-at-the-boundary discipline (§6.1, P6).** Introducing a single `.to_string_lossy()` or
  `String`-typed filename anywhere and then trying to remove it later means auditing every call
  site that came to depend on the lossy version, which is exactly the kind of retrofit that's
  cheap to avoid and expensive to undo. Zero cost to do right from line one; real cost to fix
  after a `String`-typed API leaks into a public signature.
- **The journal's crash-safety protocol (§5.5) and preview-by-default + explicit `-x` (§2.1,
  P5).** Both are named directly in the mandate's hard constraints as non-negotiable even in a
  thin slice, and both are represented in M1 without narrowing.

---

## 4. Dependency introduction schedule

| Milestone | New direct deps                                                                            | Cumulative        | Notes                                                                                                                                                                                                                                                             |
| --------- | ------------------------------------------------------------------------------------------ | ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1        | `clap`, `serde_json`, `unicode-normalization`, `unicode-segmentation`, `walkdir`, `rustix` | **6 / 11**        | All justified in §2.3.                                                                                                                                                                                                                                            |
| M2        | `regex`                                                                                    | **7 / 11**        | `--exclude` globs compiled to regex; later reused for `[[rule]] regex = true` in M4, so this is not a single-purpose addition.                                                                                                                                    |
| M3        | `serde`, `toml`                                                                            | **9 / 11**        | Config parsing. Two crates, one budget row per §7.2's own accounting convention (`serde`+`toml` "one row but two packages"), counted here as two lines against the cap for honesty, matching the proposal's own correction of an earlier draft that undercounted. |
| M4        | `deunicode`                                                                                | **10 / 11**       | Feature-gated, default-on; §3.6.                                                                                                                                                                                                                                  |
| M5        | none                                                                                       | **10 / 11**       | `--target`/plan files/`--stdin`/length-detection all reuse existing deps (`serde_json` for plan files, `rustix::fs::statfs` for limits).                                                                                                                          |
| M6        | possibly `terminal_size`                                                                   | **10 or 11 / 11** | Only if the fixed two-column layout turns out to need it once real preview output from wide trees is seen — resolve before v1.0 tags, per §7.2's own instruction, not before.                                                                                     |

Dev-only dependencies (`proptest`, `insta`, `trycmd`, `assert_cmd`, `criterion`, `clap_complete`,
`clap_mangen`) are added starting in M1 as soon as there's logic to test, and don't count against
the direct-dependency budget (`just dep-budget` only scans `[dependencies]`, not
`[dev-dependencies]` — confirmed by reading the recipe in `justfile`).

---

## 5. Spike handling

Owner has Linux + macOS, no Windows, no NTFS/exFAT (`docs/owner-decisions.md`).

| Spike                                                                                                                               | My slice's posture                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **2** (`renameat2` matrix across Linux filesystems)                                                                                 | **Closeable, and I'd run it right after M1 code-complete, not before.** The design already has runtime demotion built in (`EINVAL`/`ENOSYS`/`EOPNOTSUPP` -> fallback), so an unrun matrix does not block M1 shipping — worst case on an unmeasured filesystem is one extra warning line, never a clobber. Running it right after M1 turns "assumed" into "measured" for the release notes, and it's cheap now that the hardware exists. Do not gate the M1 branch merge on it; do gate the _v0.1 release announcement's_ claims on it.                                                                                                                                                                                     |
| **13** (macOS incapable-volume errno)                                                                                               | Same posture as spike 2: demotion-on-error already covers the unknown case safely (§5.4's own framing: "silently dropping the flag is the failure mode that matters," and that's the one failure mode this spike would catch). Run it alongside spike 2 before the release announcement, not before the code.                                                                                                                                                                                                                                                                                                                                                                                                              |
| **14** (Linux `RENAME_NOREPLACE` on case-insensitive mounts)                                                                        | Same posture. M1's fallback never unlinks anything regardless of the answer.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| **3, 4** (Windows 11 reserved names; NTFS/exFAT length limits)                                                                      | **Stay open, unconditionally, per the owner decision.** My slice makes no Windows filesystem claims at all — Windows isn't even in M1-M5's tier-1 scope, and M6's Windows work is "compiles and unit-tests in CI," never "verified on real NTFS." I assume the conservative pre-Windows-11 reserved-name rule (§6.5) exactly as the full design does, and I do not narrow that assumption for speed, because getting it wrong in the _permissive_ direction (assuming Windows 11's relaxed rule) is the one mistake that's expensive to discover after the fact — a file that's fine on Windows 11 and unreadable-as-named on an SMB share from 2015 is exactly the failure mode a conservative default exists to prevent. |
| **5** (case-only rename on network filesystems)                                                                                     | Informs M1's fallback path (never unlinks on an unmeasured filesystem) but doesn't block it — same shape as 2/13/14.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| **7, 8, 9, 11** (NFC-rewrite collision rate; auto-number vs. skip; parallelism benefit; how often `Unrepresentable` actually fires) | **These are the spikes real usage answers better than more research would, and they're the reason this plan's whole argument exists — see §6.** Not closed by a lab measurement in this plan; closed by asking M1's users.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| **6** (CP1252 repair false-positive rate)                                                                                           | Moot for this plan, same as for the full design — the subsystem doesn't exist (owner decision). Retained only as the spec for a post-1.0 measurement, unchanged from the proposal.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| **10, 12** (aggregate distro-packaging count; stage-13 iteration bound)                                                             | Informational/non-blocking, same as the full design's own gating table. Not touched by this plan.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| **15** (`nlink > 1` respell)                                                                                                        | Carried as an open assumption in M1 (§3's debt table), same posture the full design already has.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |

---

## 6. Feedback plan

The point of shipping thin is that §11's spikes 7, 8, 9, and 11 are explicitly not answerable from
a lab — they need real trees. This is not a plan afterthought; it is the actual argument for
building this way. Concretely:

1. **Ship M1 as a tagged `v0.1` GitHub release (and optionally a crates.io yank-if-broken
   pre-release) the moment its exit criteria pass — not bundled with M2.** The ask to early
   testers is deliberately zero-risk: _"run `detoxrs -r <some real messy directory>` with no `-x`
   and paste the output."_ Nothing changes on their disk to get this. That lowers the
   participation bar to roughly zero, which matters given `user_feedback_online.md`'s own
   headline finding — `detox` generates almost no public discussion, so getting any signal at all
   requires making it unusually cheap to give.
2. **Ask specifically, not generally**, because a generic "try it and tell me what you think" is
   exactly the kind of low-effort ask that produced doc 02's history of getting almost no external
   engagement outside the tracker itself. Specific asks, each mapped to a spike:
   - "How many conflict lines did `--on-collision number` produce on your tree, and did the
     renumbered names look right to you?" -> **spike 8.**
   - "If you're on a Mac, paste any lines where a name changed only in accent marks or looks
     visually identical before/after." -> **spike 7** (NFC-rewrite collision rate on real macOS
     trees — the exact experiment §11 spike 7 specifies, just crowdsourced instead of run on one
     researcher's Photos library).
   - "Did anything get skipped as `Unrepresentable`? Paste those lines." -> **spike 11** directly.
   - "How long did a run over your largest directory take?" -> informs **spike 9** (whether
     parallelism is worth the complexity) with real tree sizes instead of a synthetic 200k-entry
     benchmark alone.
   - "Did you hit a `%20` or `%25` in a filename and wish it had been decoded?" -> validates (or
     invalidates) M2's priority ordering before M2 is built, not after.
3. **Post where detox users actually are**, per `user_feedback_online.md`'s own evidence — the
   Lobste.rs thread, Arch BBS-adjacent venues, r/commandline — rather than assuming a GitHub
   README alone will surface anyone. The research's own evidence-gaps section (#4: "what replaced
   detox for people who moved on... would be settled by a direct ask on a venue like
   r/commandline, which this passive research pass could not manufacture") names exactly this as
   unmanufacturable by research and manufacturable by shipping and asking. That is this plan's
   thesis in one citation.
4. **`--json` output doubles as a structured feedback channel**: asking a tester to paste `--json`
   output (already in M1) rather than free-text terminal output gives parseable data on real
   collision rates, skip reasons, and truncation frequency with far less manual triage than prose
   reports — cheap to ask for, since the flag already exists for other reasons.

---

## 7. Why this sequencing over "build the safety architecture fully first"

The obvious alternative reads §3 (full pipeline), §4 (full config), and §5 (full safety
architecture) as one foundational unit — ship nothing until the whole design is built, on the
theory that a tool which renames files should be completely specified before a stranger points it
at their home directory. That is a coherent position and I expect at least one of the other two
plans to take something like it.

**The case against it, specifically:**

- **It spends effort resolving questions the research itself says can't be resolved without
  users.** Section 11 lists four spikes (7, 8, 9, 11) whose own "closes with" text says the
  answer comes from counting real trees or watching real usage, not from more design work. Building
  the full customization surface (config, rules, transliteration, case-folding) before anyone has
  used the tool once means guessing at the shape of `[[rule]]` ergonomics, `--ascii` demand, and
  collision-numbering acceptance for months, when M1 gets a first, honest signal on the collision
  question and the `Unrepresentable`-frequency question in the time it takes to ship a thirteen-
  story-point milestone.
- **The proposal's own v0.1 already drew this line**, and its stated reason — "§5 is hard to
  retrofit, §4 is trivial to add" — is a claim about _relative_ retrofit cost, which is exactly the
  axis a thin-slice mandate should sequence on. Building §4 before shipping anything front-loads
  the cheap-to-add part and delays the moment real feedback starts arriving, for no safety benefit,
  since §4 (config/rules/case/ascii) touches none of the no-clobber-rename or collision-engine
  code that actually carries the safety property.
- **What it risks, honestly:** the debt table in §3 is real debt, not a euphemism for skipping
  safety. Hardcoding the length limit, narrowing the invisible-character set, and collapsing
  per-mount warnings to one global warning are all decisions made under speed pressure that a
  slower, safety-first-then-features sequencing wouldn't have needed to make at all — it would
  have built the `statfs` detector and the UCD generator as part of the same first pass. If early
  feedback surfaces a real filesystem where 255/255 is wrong, or a real Unicode-hygiene complaint
  the narrowed invisible set misses, that's rework this plan chose to risk in exchange for shipping
  roughly 15-20 story-points earlier. I think that trade is right for a v0.1 aimed at a user base
  the research itself says is nearly invisible online and has never given feedback on anything —
  the fastest way to learn whether `detoxrs` matters to anyone is to put a safe, honest, narrower
  tool in front of them now, not a complete one later.
- **What I refuse to trade away regardless of sequencing** is listed in §3's second table, and it's
  short and specific on purpose: the two-phase plan/apply split, the no-clobber rename as the only
  rename primitive, `OsStr` discipline, and the crash-safe journal. None of those are "safety
  architecture we'll get to" — they ship in M1, unnarrowed, because the mandate is explicit that
  "thin slice" must never become a euphemism for skipping them, and because (per §0.2) they are
  what makes the mandate's own non-negotiable claim — never a clobbering rename — actually true
  rather than merely stated.

---

## Appendix: things I considered narrowing further and decided against

- **Dropping `undo`/the journal from M1 entirely**, on the argument that rename never destroys
  data so undo is convenience, not safety (§0.2 makes this distinction explicitly). Decided
  against: the psychological trust cost of "if this goes wrong I have to manually rename 200
  files back" is exactly the friction that would suppress the feedback this plan exists to get
  (§6). Cheap enough to keep (2 SP per §2.7) that cutting it would be optimizing an already-fast
  milestone at the cost of the plan's own stated goal.
- **Dropping recursion (`-r`) from M1**, shipping single-directory-only first. Decided against:
  `walkdir` makes recursion nearly free once non-recursive walking exists, and the best-evidenced
  real use case in `user_feedback_online.md` (bulk cleanup of imported trees) is recursive by
  nature. Shipping without it would mean M1 can't demonstrate the use case the whole mandate is
  trying to get feedback on fastest.
- **Skipping the huge-tree (`criterion`) benchmark in M1.** Decided against, for the reason stated
  in §2.6: it is the regression test for the single most reliably-reproduced defect class in
  detox's own tracker (§8.4's five independently-filed crash reports), and it costs one benchmark
  file against a pipeline that already exists by the time it would be written.
