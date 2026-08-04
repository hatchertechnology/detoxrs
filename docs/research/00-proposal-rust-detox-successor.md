# detoxrs: a Rust successor to detox

Status: design proposal. Audience: the person who starts writing code Monday.
Inputs: docs 01-04 (primary research), docs 05-07 (adversarial validation), docs 10-13 (source-derived
upstream reference), and `docs/owner-decisions.md`. Every claim traceable to research is cited inline
as (doc NN, section). Every dependency on something validation could not confirm is marked
**[UNVERIFIED]** and appears again in §11.

**Evidence precedence.** An earlier version of this document said only "where they disagree, 05/06/07
win", which is not the rule the research documents themselves state and which would rank doc 05 above
the source-derived docs. The real rule, stated identically at the top of docs 05, 06, and 07 and
adopted here verbatim in substance:

0. **`docs/owner-decisions.md` overrides everything below it.** An owner's call is a decision, not
   evidence, and it does not lose to a citation.
1. **Docs 10-13 win** on any claim about the upstream C tool's source or behavior, over both doc 01
   and doc 05. They are newer, source-derived, and independently validated.
2. **Doc 05 wins over doc 01** on C-tool claims that docs 10-13 do not cover — chiefly doc 02's
   issue-tracker provenance, and anything needing live binary reproduction rather than source reading.
3. **Docs 06 and 07 win over docs 03 and 04 unconditionally.** Docs 10-13 have no jurisdiction there:
   03/04 are about external tools, crates, and OS/filesystem constraints, not detox's own source.
4. **Doc 02's stage-3 revision supersedes doc 05** on the issue-tracker facts it re-swept.
5. Outranking all of the above: **a row or item marked `SUPERSEDED`, `CORRECTED`, or `CONTESTED` is
   never authority for its original claim.** Read the row, not the tier. This one is not decorative —
   it is what makes doc 06 row 4e's self-withdrawal (§5.4) and doc 03 constraint 10's `[CORRECTED]`
   marker bind, and following the tier instead of the row is precisely how this document came to
   budget an `unsafe` FFI shim it never needed.

Upstream status, stated once and relied on throughout: **`dharple/detox` is archived.** Verified
directly against the GitHub API for this document (`archived: true`, `open_issues_count: 0`, 446
stars, last push 2026-07-12), matching doc 02 line 3 and `user_feedback_online.md`. Archived means
permanently read-only: no new issues, no new PRs, no new releases, ever. The 34 issues closed in a
single ~50-minute window on 2026-07-12 all carry one templated wind-down comment, so "closed" on
those issues is an administrative sweep, **not** triage and **not** rejection: this document reads
a closed issue as evidence of demand, never as evidence of a decision, unless the maintainer said
something substantive on it (as on #124 and #130).

The mandate (detox README, quoted in doc 02, "Maintainer's Stated Future Direction"; the closing
lines are quoted here in full because the archival makes them operative rather than aspirational):

> The days of weighty configuration files are behind us, and users looking for help with their
> files shouldn't need to be well-versed in character encoding. detox needs to be easier to work
> with, using command-line options and a config file that lets you pre-select those options. It
> needs to just work. Period.
>
> ... So, `detox` is paused. I hope to pick it up again at some point and rebuild it from scratch,
> in a different language, with a friendlier UI.

(Second paragraph verified verbatim against the README in the pinned upstream clone `0a8e212`,
lines 25-26.)

Not a drop-in replacement. Same job: point it at a file or a tree, get sane unix-safe names,
with room for custom patterns.

---

## 0. The bet, in one paragraph

detox's core mistake was making the user assemble the transform (sequences of table-driven
filters, `.tbl` files, mutually exclusive encoding filters that silently corrupt data if you
pick wrong: doc 01 §2.3, §7; doc 02 theme 1 at **17 items, 12 of them externally filed** — the
verified recount, superseding the earlier "~15", and still the highest-weight theme in the whole
tracker on raw count, on external count, and after subtracting every item it shares with another
theme). `detoxrs` ships **one fixed, ordered pipeline** whose stages are individually
switchable but never reorderable, never user-defined, and never table-driven. Encoding is not a
user-facing concept: the pipeline reads valid UTF-8 and skips everything else, so the one operation
that produced detox's worst data corruption (re-interpreting bytes as Latin-1) is _structurally
impossible_ — there is no legacy decoder at all to misapply (§3.4, owner decision 2026-07-31).
Customization is a small set of flags, mirrored 1:1 into a TOML file with named profiles:
"pre-select options, not a DSL."

---

## 1. Design principles

Seven. Each is falsifiable: a PR either violates it or does not.

**P1. One pipeline, fixed order. Stages toggle; they never reorder, never come from a file.**
Evidence: config/sequence complexity is the single highest-weight theme in detox's tracker
(doc 02 theme 1: #7, #19, #29, #42, #50, #52, #84, #89, #94, #95, #102, #105, #111, #118, #121,
#122, #124), and the maintainer's own final comment on #124 confirms it ("I have had many
requests of this nature", verbatim-confirmed in doc 05, claim row #124).
_Falsifier: any PR that adds a `sequence`, `filter`, `pipeline = [...]`, or external character
table is rejected on sight._

**P2. Encoding is never a user decision.** No flag selects "what my filenames are encoded in."
Evidence: detox's `iso8859_1` vs `utf_8` filters are documented as mutually exclusive and
silently corrupt data when misapplied (`café.txt` -> `cafÃ©.txt`, doc 01 §7, re-reproduced
verbatim in doc 05), and the mandate names this explicitly.
_Falsifier: a PR adding `--encoding <x>`, `--legacy-encoding <x>`, or any other flag that names an
input encoding. There are no exceptions in v1.0: a name is valid UTF-8 or it is skipped (§3.4, owner
decision 2026-07-31). An opt-in `--repair-encoding` may return post-1.0, but only after its
false-positive rate is measured against a real corpus, and never on by default._

**P3. Never destroy data; a collision is a first-class outcome, not an error path.**
Evidence: the maintainer's rejection of PR #130 is the most technically substantive comment in
the tracker (N-files-collapse risk, `readdir()` ordering, `S_ISREG`/`S_ISDIR`, BSD/macOS syscall
differences, "I don't want to be responsible for destroying other people's data") and doc 05
confirms it verbatim; #124 ties "just works" directly to "without completely destroying
someone's files" (doc 02 theme 6, doc 05 rows #130/#124). Fidelity note for anyone following that
citation back: doc 05, Corrections Required item 4, flags that doc 02's longer #124 block quote is
a synthesis of two maintainer comments 37 minutes apart. Only the short fragments quoted in this
document are individually verbatim.
_Falsifier: any `--overwrite`/`--force-replace` flag, any code path that can call a
clobbering rename, any collision handled by `unwrap()`/silent skip._

**P4. Do less by default than detox v1 did, and not much more than v3 does.**
Evidence: the transliteration arc: early demand for richer tables (#47, #53), then user
backlash that it was destroying intentional non-ASCII (#99 German umlauts), then v2 making it
opt-in (#21), then v3 removing it from default tables (#112/#113 merged) (doc 02 theme 5).
Transliteration, case folding, and `+`-to-space are opt-in for exactly this reason.
_Falsifier: a PR that makes any lossy or taste-driven transform on by default._

**P5. Preview is the product. Applying is a second, explicit act.**
Evidence: dry-run-by-default is the confirmed convention among all three of the tools closest to this
job — f2, rnr, and **`convmv`** (doc 04 §1; re-confirmed independently in doc 07 rows 1a/1c; `convmv`
verified from its own man page, quoted in §2.1). detox's own README calls `--dry-run` "the most
important option to learn" (doc 02 theme 7), while detox itself does _not_ default to it (confirmed
from the local man page, doc 07 row 8c).
_Falsifier: any code path where a bare `detoxrs <path>` mutates the filesystem, unless the
user's own config set `exec = true`._

**P6. Names are bytes until the last possible moment.** Filenames are `OsStr` at the boundary;
text is a decoded view used for transformation and display only.
Evidence: `rnr` documents itself as UTF-8-only, and validation found it worse than documented:
`src/renamer.rs:70` does `path.file_name().unwrap().to_str().unwrap()`, which **panics** on a
non-UTF-8 name (doc 06 row 6a). detox itself skips or mangles such names (doc 01 §2.3).
_Falsifier: any `.to_str().unwrap()`, `to_string_lossy()` outside display/log code, or
`String`-typed filename in a non-display signature._

**P7. Every dependency costs distro packaging.** Debian requires each crate, including
transitive ones, to be its own Debian source package built with no network (doc 04 §5, citing the
Debian Rust Team's own book; doc 07 row 8a upholds that citation but explicitly did _not_ re-fetch
it live, calling it "medium-high confidence, not re-verified live", which matches well-documented
Debian practice and is not in real doubt). Validation also
found several of doc 03's recommended crates stale: `unicode_skeleton` last released 2017
(~9 years), `jwalk` Dec 2022, `figment` May 2024, `confusables` Aug 2023 (doc 06 rows 5c/5d,
doc 07 row 7b).
_Falsifier: a PR that adds a direct dependency without (a) a line in `docs/deps.md` saying why
50 lines of our own code will not do, and (b) a last-release date inside 12 months._

---

## 2. The UX

### 2.1 Dry-run default: yes. Here is the argument.

Three tools that do this job default to preview and require an explicit flag to write, and the third
is the most relevant of them.

- **f2** (`-x/--exec`) and **rnr** (`-f/--force`), independently confirmed twice (doc 04 §1, doc 07
  rows 1a/1c). Both are general-purpose batch renamers, so the analogy is to the _mechanism_, not to
  the problem.
- **`convmv`** — same problem domain (mass filename repair on a tree the user did not create),
  decades old, and shipped in the same distributions as detox, frequently in the same
  `convmv -> detox -r` pipeline (doc 03 table; `user_feedback_online.md` documents that pipeline from
  multiple independent sources). Its man page states the default in one sentence, verbatim:

  > `--notest` Needed to actually rename the files. By default convmv will just print what it wants
  > to do.

  That is the argument's strongest leg, and it was missing. The counter to preview-by-default has
  always been "no established tool in this space does that, so it will surprise people." The tool with
  the longest tenure in exactly this space, on exactly these distros, has done it for decades — and
  goes further than we do, requiring a flag whose name (`--notest`) exists only to make writing the
  deliberate act. detox is the outlier here, not `detoxrs`.

detox does not default to preview, and its own README then spends its safety budget telling you to
remember `-n` (doc 02 theme 7; doc 07 row 8c). The mandate's "it just works" is about not having
to understand encoding tables, not about mutating a home directory on the first guess.

The tension is real, and the resolution is the mandate's own second clause: **a config file that
lets you pre-select those options.** `exec = true` in the user's config makes `detoxrs ~/Downloads`
rename immediately, forever, for that user. The default ships safe; the user opts into speed once,
in writing, instead of per-invocation.

Strongest counterargument: two-step is two commands for the 90% case where the preview is
obviously fine, and users will alias it away anyway, at which point the safety was theatre.

### 2.2 Top use cases, verbatim

Clean one directory (the default: preview only).

```
$ detoxrs ~/Downloads
~/Downloads
  Screen Shot 2026-07-30 at 10.14.22 AM.png   ->  Screen_Shot_2026-07-30_at_10.14.22_AM.png
  Mario & Luigi (1985) [720p].MKV             ->  Mario_Luigi_1985_720p.MKV
  invoice%20final%282%29.pdf                  ->  invoice_final_2.pdf
  ..bad  name...txt                           ->  .bad_name.txt
  résumé.pdf                                  =   (unchanged)
  Icon\r                                       -   skipped (excluded)

4 to rename, 1 unchanged, 1 skipped, 0 conflicts.
Nothing was changed. Re-run with -x to apply.
```

> **⚠ This example contradicts §5.6, §2.4 and §9.2, and it is the example that is wrong.**
> `detoxrs ~/Downloads` without `-r` cleans **only the basename `Downloads` itself**; it does not
> list or touch that directory's contents. Three sections say so and carry the reasoning (§5.6's
> "one argument, one name" rule, §2.4's `--help` text, §9.2's migration note); this one example
> shows upstream `detox`'s behaviour instead, where `-r` gates descent only _past_ the first level.
> The owner ruled on 2026-08-01 that the three sections win and the implementation follows them
> (`walk.rs`, verified by `recursion_flag_decides_whether_children_are_processed`). The listing
> below is therefore what `detoxrs -r ~/Downloads` prints, and the flag is missing from the command
> line above. It is left in place, flagged rather than silently rewritten, because this is the
> passage a reader is most likely to copy and the discrepancy is worth seeing once.

Two things in that output that are not defaults, said plainly so nobody has to guess. `Icon\r` is
skipped because _this user's config_ lists it in `exclude` (§4.2). **There is no built-in default
exclude list**: the only unconditional skips are `.git`/`.hg`/`.svn` (§5.6) and dotfiles during
recursion. With no config at all, `Icon\r` would be renamed to `Icon` (the `\r` is delete-class).
And `..bad  name...txt -> .bad_name.txt` depends on stage 9 collapsing runs of repeated `.`, which
§3.2/§3.8 specify.

Apply it.

```
$ detoxrs -x ~/Downloads
~/Downloads
  Screen Shot 2026-07-30 at 10.14.22 AM.png   ->  Screen_Shot_2026-07-30_at_10.14.22_AM.png
  Mario & Luigi (1985) [720p].MKV             ->  Mario_Luigi_1985_720p.MKV
  invoice%20final%282%29.pdf                  ->  invoice_final_2.pdf
  ..bad  name...txt                           ->  .bad_name.txt

4 renamed, 1 unchanged, 1 skipped, 0 failed.
Undo with: detoxrs undo 20260731T142233Z-a91c
```

A collision, shown in preview before anything happens.

```
$ detoxrs -r ./photos
./photos/2019
  IMG 0042.JPG      ->  IMG_0042.JPG
  IMG_0042.JPG      =   (unchanged)
  IMG-0042.JPG      ->  IMG_0042.JPG   ! collides with IMG 0042.JPG -> IMG_0042-2.JPG

2 to rename (1 renumbered), 1 unchanged, 0 skipped, 1 conflict resolved.
Nothing was changed. Re-run with -x to apply, or --on-collision skip to leave conflicts alone.
```

A non-UTF-8 name: skipped and reported, never guessed at (§3.4, owner decision).

```
$ detoxrs ./from-a-2004-cdrom
./from-a-2004-cdrom
  Bj<f6>rk - Vespertine.mp3   -   skipped (name is not valid UTF-8; detoxrs does not guess encodings)
  Bj<f6>rk - Homogenic.mp3    -   skipped (name is not valid UTF-8; detoxrs does not guess encodings)
  Björk - Volta.mp3           ->  Björk_-_Volta.mp3

1 to rename, 0 unchanged, 2 skipped, 0 conflicts.
Nothing was changed. Re-run with -x to apply.
2 names were skipped as not-valid-UTF-8: fix the encoding with convmv, then re-run.
```

The third file is the same artist's name already stored as valid UTF-8, and it shows the split
cleanly: `detoxrs` cleans what it can read, and does not touch what it cannot. Note that `_-_`
survives untouched: that is stage 9's same-character rule (§3.8), not a rule the user had to write.
Transliteration is off by default (§3.6), which is why `ö` is kept rather than folded to `o`.

(`<f6>` is how a byte that is not valid UTF-8 is rendered in the preview; it is never printed
raw, because printing raw invalid bytes is how a terminal gets confused. §6.1's `OsStr` discipline is
what makes this safe, and dropping repair does not relax it one inch.)

Machine consumption, and the plan/apply split.

```
$ detoxrs --json -r ./tree | jq '.items[] | select(.resolution=="conflict")'
$ detoxrs -r ./tree --plan-out plan.json
$ detoxrs apply plan.json          # refuses if any inode/mtime moved since the plan
```

Undo.

```
$ detoxrs undo --last
Reverting 20260731T142233Z-a91c (4 renames, in reverse order)
  Screen_Shot_2026-07-30_at_10.14.22_AM.png   ->  Screen Shot 2026-07-30 at 10.14.22 AM.png
  ...
4 reverted, 0 failed.
```

### 2.3 Interactive mode: not in v1.0

No per-file `y/n/a/q` prompt. Reason: the preview plus `-x` already gives review-then-commit,
and a y/n loop is the _weakest_ of the interactive designs in the research: `qmv`'s
"open the file list in `$EDITOR`, diff your edits, apply them" is strictly better and is called
out in doc 03 as "arguably the best renaming UX yet designed." Shipping y/n now means shipping
it twice. v1.1 gets `--edit`, which writes the plan as a two-column text buffer, opens `$EDITOR`,
and applies the diff through the same collision engine. Where a prompt is unavoidable (`undo`
onto an occupied name), the tool errors with a suggested flag rather than blocking, and any
future prompt gets a `--yes` escape and a no-TTY error rather than a hang (doc 04 §1,
non-interactive behavior).

Strongest counterargument: `-i` is 30 lines and some users genuinely want to walk a messy
directory one file at a time rather than iterate on flags.

### 2.4 `--help`

```
detoxrs 1.0.0
Make filenames sane: unix-safe, portable, readable. Preview by default.

USAGE
    detoxrs [OPTIONS] <PATH>...
    detoxrs [OPTIONS] --stdin
    detoxrs apply <PLAN>
    detoxrs undo [--last | <BATCH-ID>] [--list]

    Nothing is renamed unless you pass -x (or set exec = true in your config).

ARGS
    <PATH>...                Files and/or directories to clean

MAIN
    -x, --exec               Perform the renames (default: preview only)
    -n, --dry-run            Preview only; explicit form of the default (conflicts with -x)
    -r, --recursive          Descend into directories. Without it, a directory
                             argument has only its own name cleaned and nothing
                             inside it is touched (detox differs: see §5.6)
    -p, --profile <NAME>     Apply a [profile.NAME] table from the config file
        --target <OS>        Also enforce another platform's naming rules
                             [unix (default) | windows | portable]
        --config <FILE>      Use only this config file
        --no-config          Ignore all config files; built-in defaults only
        --stdin              Clean names read one per line from stdin; no filesystem access

TRANSFORM  (every option here has an identically-named config key)
        --case <MODE>        keep (default) | lower | upper
        --separator <CHAR>   Replacement for spaces and unsafe punctuation [default: _]
        --spaces <MODE>      replace (default) | keep
        --normalize <FORM>   nfc (default) | nfd | none
        --url-decode         Decode %XX escapes [default: on; --no-url-decode to disable]
        --plus-to-space      Also treat '+' as an encoded space [default: off]
        --ascii              Transliterate non-ASCII to ASCII (lossy) [default: off]
        --max-len <N>        Max component length; 0 = detect filesystem limit [default: 0]
        --keep <CHARS>       Treat these characters as safe (adds to the safe set)
        --strip <CHARS>      Delete these characters (adds to the delete set)
        --no-invisible-strip Keep bidi/zero-width/tag characters (NOT recommended)

SAFETY
        --on-collision <P>   number (default) | skip | fail
        --exclude <GLOB>     Skip entries whose name matches (repeatable)
        --hidden             Also process dotfiles/dot-dirs when recursing [default: off]
        --files-only         Do not rename directories
        --dirs-only          Rename only directories
        --plan-out <FILE>    Write the plan as JSON and exit without renaming
        --no-journal         Do not record an undo journal (also disables `undo`)

OUTPUT
    -v, --verbose...         -v: list unchanged entries. -vv: per-stage transform trace
    -q, --quiet              Errors only
        --json               JSON on stdout, diagnostics on stderr
        --color <WHEN>       auto (default) | always | never   (respects NO_COLOR)
    -h, --help               Print help
        --help-transforms    What each pipeline stage does, with examples
        --print-config       Print the fully resolved policy (after config, profile,
                             and flags) as TOML, and exit without walking anything.
                             Validates rules and globs: exits 2 if any would fail
                             at run time. `max_len = 0` stays 0 -- the filesystem
                             limit is detected per directory, which needs a walk.
        --explain-detox <F>  Read a detoxrc read-only and print the closest flag set
    -V, --version            Print version

EXIT CODES
    0  success (or preview produced with no errors)
    1  one or more entries failed
    2  usage, config, or plan error
    3  nothing matched (only with --quiet)

Config:  $XDG_CONFIG_HOME/detoxrs/config.toml, or ./.detoxrs.toml (nearest wins)
Journal: $XDG_STATE_HOME/detoxrs/journal/
Docs:    detoxrs --help-transforms   (what each stage does, with examples)
```

Deliberately absent, and each absence is load-bearing: no `-f/--force` (means two different
things in rnr and git: force-write vs force-overwrite; we need the distinction to stay sharp),
no `--overwrite` in any spelling (P3), no `--special` (see §5.6), no `-s/--sequence`, no
`--table`, no `--encoding`.

Progress/diagnostics on stderr, data on stdout, `--json` is the only stable contract (doc 04 §3,
Heroku/gh convention). `NO_COLOR` wins over `FORCE_COLOR`/`CLICOLOR_FORCE`, which win over
`CLICOLOR` (doc 04 §3, precedence re-confirmed doc 07 row 5). Batch behavior is best-effort with
a per-item error line and a non-zero exit at the end, not abort-on-first-error (doc 04 §1/§3
notes this is industry convention, not a confirmed f2 behavior: we choose it on its merits).

---

## 3. The transform model

### 3.1 Shape

```rust
pub struct Policy { /* every field maps 1:1 to a flag and a config key */ }

pub enum Decoded {
    Utf8(String),   // input was valid UTF-8
    Opaque,         // input was not valid UTF-8; we do not guess, we skip and report
}

pub struct Outcome {
    pub text: String,
    pub stages: Vec<StageDelta>,   // for -vv and for snapshot tests
    pub notes: Vec<Note>,          // Truncated, Renumbered, ReservedName, Confusable...
}

pub fn decode(raw: &OsStr) -> Decoded;
pub fn transform(d: &Decoded, p: &Policy) -> TransformResult;   // pure, no I/O, no path allocation
```

Two notes on that sketch, so it does not read as contradicting later sections. **`transform` returns
`TransformResult`, not a bare `Outcome`** -- see §3.14: a name that reduces to nothing is
`Unrepresentable(reason)` and is skipped, which is what makes §8.1's Safety closure honest. And the
`Outcome` fields above are the eventual v1.0 shape; M1 carries only
`{ text, truncated }`, because `stages` exists for `-vv` (M2) and `notes` accumulates variants that
later milestones introduce.

`transform` is a pure function of `(name, Policy)`. It never sees a path, a directory, a
filesystem, or another file. **The `Policy` reaching `transform` is always fully resolved**: in
particular both `max_len_bytes` and `max_len_utf16` are concrete numbers, never the CLI's `0 = auto` sentinel. Resolving `auto`
into a number is a walk-time concern (§3.10) that produces one resolved `Policy` per directory, and
that is the only reason `transform`'s purity and stage 12's filesystem-derived limit can coexist.
`Policy` therefore has two shapes in practice, and only the resolved one is what `transform`,
the snapshot tests, and the §8.1 property tests ever see. Everything that involves other files (collisions, existence,
length limits of _this_ filesystem) lives in the planner (§5). That split is what makes the
property tests in §8 possible.

### 3.2 The default pipeline, in order

| #   | Stage             | Default   | What it does                                                                                                                                                                                                                                                                                                            |
| --- | ----------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `decode`          | on        | `OsStr` -> text. Valid UTF-8 passes through untouched, and that is the whole stage. Anything else is `Opaque`: **skipped and reported, never repaired, never guessed at, never lossily converted** (§3.4, owner decision 2026-07-31). No `Policy` parameter.                                                            |
| 2   | `url_decode`      | on        | `%XX` -> byte, only when every escape in the name is well-formed and the decoded result is valid UTF-8 and contains no `/`, no NUL, and no controls. All-or-nothing per name. `+` -> space is **off**.                                                                                                                  |
| 3   | `normalize`       | NFC       | Unicode normalization of the output name. Comparison inside the planner is _always_ NFC regardless of this setting.                                                                                                                                                                                                     |
| 4   | `invisible_strip` | on        | Delete bidi controls (U+202A-202E, U+2066-2069, U+200E/200F), zero-width (U+200B/200C/200D/2060/FEFF), Unicode Tags (U+E0000-E007F), and all remaining `Cf`, `Cc`, `Cs`, `Co`.                                                                                                                                          |
| 5   | `rules`           | none      | User's ordered `[[rule]]` list: literal or regex find/replace, applied in file order, each seeing the previous one's output. The only customization slot.                                                                                                                                                               |
| 6   | `ascii`           | **off**   | Transliterate to ASCII (`deunicode`). Lossy, opt-in.                                                                                                                                                                                                                                                                    |
| 7   | `safe_map`        | on        | Character classes, not a table: delete-class -> nothing; separator-class -> `--separator`; everything else kept. Sets defined in §3.7.                                                                                                                                                                                  |
| 8   | `case`            | keep      | `lower`/`upper` use Unicode simple case mapping, not ASCII-only.                                                                                                                                                                                                                                                        |
| 9   | `collapse`        | on        | Collapse a run of the _same repeated character_ to one, for exactly this collapse set: `.`, `-`, `_`, and the configured `--separator`. Never merge runs of _different_ characters. Drop separators adjacent to `.`.                                                                                                    |
| 10  | `trim`            | on        | Strip leading `-`; strip leading/trailing separators (including one that immediately follows a preserved leading `.`); strip trailing dots and spaces; preserve exactly one leading `.` if the original had one.                                                                                                        |
| 11  | `target`          | unix      | With `--target windows` or `portable`: reserved-stem check, Windows illegal-character check, MAX_PATH warning.                                                                                                                                                                                                          |
| 12  | `truncate`        | on (auto) | Grapheme-safe, extension-preserving truncation to the filesystem limit, or `--max-len N`.                                                                                                                                                                                                                               |
| 13  | `finalize`        | on        | Re-run 3/9/10/12 (and 11 where `--target` applies) until fixed point (bounded to 3 iterations). Then: if the result is empty, `.`, or `..`, or if the loop did not converge, `transform` returns `Unrepresentable` and the planner **skips the entry unchanged** (§3.14). It does _not_ fall back to the original text. |

### 3.3 Why that order

- **1 before everything.** Nothing textual is meaningful before decode, and nothing downstream ever
  sees a byte sequence that is not valid UTF-8. This makes detox's worst bug class unreachable
  rather than merely discouraged: there is no legacy decoder in the binary, so `café.txt ->
cafÃ©.txt` (doc 01 §7, doc 05) cannot happen by any flag combination — not because the flags are
  ordered carefully, but because the code does not exist.
- **2 before 3.** Percent-decoding produces new bytes, which then need normalizing. Reversing
  these leaves `%CC%81` undecoded-then-decoded into an unnormalized sequence.
- **3 before 4.** NFC first means the invisible-character scan sees a stable form; a decomposed
  sequence cannot hide a joiner from the scanner.
- **4 before 5.** User rules should match what a human sees, not a string with a zero-width
  joiner wedged into the middle of the word they wrote in their config.
- **5 before 6/7.** A user rule such as `ü -> ue` must be able to run _before_ transliteration
  and before the safe map, or it can never fire: `--ascii` would have already turned `ü` into
  `u`, and `safe_map` would have already fixed the surroundings. This is the fix for #117
  ("don't append underscore after deaccented characters") and #121 (`_-_` music separators):
  both, per doc 02 theme 2, unfixable in detox without hand-editing tables.
- **7 before 8.** Case mapping on a reduced alphabet is cheaper and total; `İ`-class oddities
  are down to one place.
- **9 after 7.** The safe map is what creates the runs (`" & " -> "___"`); collapsing before it
  would collapse nothing.
- **12 near the end** because stages 5-7 can lengthen a name (`&` -> `_and_` via a user rule) and
  truncating before that reintroduces overlong names.
- **13 last** because truncation can itself create a trailing dot or a reserved stem
  (`report.tar.gz` truncated to `report.` ; `CONSOLE.txt` truncated to `CON.txt`). Stages 4 and 7
  **delete** characters, and a deletion can join a base character to a combining mark that was
  previously separated. Worked example: for input `e\r\u{301}`, NFC cannot compose across the
  carriage return, then stage 7 deletes the CR, leaving `e\u{301}` — which is **not NFC**. So
  `transform` was not a fixed point, and stage 3 (normalization) must re-run. Stage 12 must also
  re-run because NFC is **not** byte-length-preserving, so re-normalizing after truncation can push
  a name back over the byte limit. The re-run set is stages 3/9/10/12 (and 11 where `--target`
  applies), not 9/10/11, bounded to 3 iterations. This fix was discovered by the Idempotence
  property test (§8.1), which is why that property is a release gate.

### 3.4 Encoding: valid UTF-8, or skip. No repair in v1.0.

The rule is one sentence, and it is shorter than the one an earlier draft had: **if the name is valid
UTF-8 we clean it and never touch its encoding; if it is not, we skip it and say so.** No flag, no
detection heuristic, no chardet, no CP1252 table, no `--legacy-encoding`.

This is an **owner decision, 2026-07-31** (`docs/owner-decisions.md`), and it overrides this
document's prior text. The reasoning is worth keeping because it is the reasoning, not a preference:
the repair path was the highest-risk untested subsystem in the design. §3.4 used to justify a CP1252
fallback as "the encoding behind the overwhelming majority of surviving mis-encoded Western
filenames" and reject `encoding_rs`/`chardetng` on P7 grounds — but the premise that mattered was
never measured. Doc 05's Load-Bearing Uncertainties records that a genuinely mis-encoded non-UTF-8
filename could **never be materialized on APFS in any research pass**, because APFS rejects invalid
UTF-8 at the syscall level, and doc 01 §7 hit the same wall. Shipping a default-on transform whose
false-positive rate nobody has ever seen is exactly the mistake P4 exists to prevent: a clever guess,
on by default, that rewrites a name the user may have meant. Deleting it costs a ~40-line table and
removes an entire class of silent corruption.

`Opaque` names are **skipped, never renamed**, and reported:

```
  <ff><fe>0A9.dat   -   skipped (name is not valid UTF-8; detoxrs does not guess encodings — see convmv, §9.3)
```

What refusing to repair does **not** license, stated because the temptation runs the other way: it is
not permission to panic, to `to_string_lossy()` the name into U+FFFD and rename _that_, or to print
raw invalid bytes at a terminal. `OsStr`-at-the-boundary discipline (§6.1, P6) is **retained in full**
and is if anything more load-bearing now, because `Opaque` is the only non-UTF-8 outcome there is. An
undecodable name is displayed with `<hh>` escapes and left exactly as it is on disk.

Three things we deliberately do not do. (a) Legacy decoding of any kind, per the above. (b) Mojibake
repair of _valid_ UTF-8 (ftfy-style `cafÃ©` -> `café`): it requires guessing that a legitimate name is
wrong, and it is the same class of clever transform. (c) Any implication that the user is stuck: the
division of labor with `convmv` is spelled out in §9.3, and it is a real answer, not a deflection.

Repair can return **post-1.0 as an opt-in `--repair-encoding`**, once there is Linux hardware to
measure its false-positive rate against a real corpus. That is the owner's framing and it is the right
shape: opt-in, after measurement, not default-on before it.

### 3.5 Normalization

NFC output, on by default; NFC always for internal comparison regardless of the flag. Rationale
for the split: doc 03's lessons from git `core.precomposeUnicode` and rclone
`--local-unicode-normalization` say the bookkeeping form must be pinned or the tool sees one file
as two, but also that _rewriting_ names is a policy choice with real cost (rclone #1472, #4228:
normalizing can erase a legitimate distinction). Validation confirmed APFS is
normalization-preserving but not normalization-sensitive: an NFC name stays NFC on disk and is
still found by its NFD spelling, verified four ways on two volume formats (doc 06, Test 2). So
an NFC rewrite on macOS is a real change to the directory entry, and it is the change that makes
the tree behave the same on Linux and in git. We make it, and any duplicate it creates is caught
by the collision engine rather than silently merged.

Counterargument: two files that legitimately differ only in normalization become a collision
where before they coexisted, and `--normalize none` is a flag the affected user has to discover.

We use NFC, never NFKC. NFKC would fold full-width forms (the #140 ask, doc 02) but also fold
`²` to `2` and ligatures apart, which is a content decision disguised as normalization. Full-width
folding, if wanted, is a v1.1 opt-in stage, not a normalization mode.

### 3.6 Transliteration policy

**Off by default. `--ascii` opt-in.** This is the one policy the research settles outright:
detox went from aggressive transliteration (v1/v2, `é->e`, `ß->ss`, `Þ->TH`, `default _`
catch-all: confirmed present in legacy `unicode.tbl`, doc 05) to removing it from default
tables in v3 (#112/#113), because users reported it destroying intentional non-ASCII (#99
umlauts, resolved by the v3 change; #117 diacritics-with-trailing-underscore) (doc 02 theme 5,
doc 01 §4 HISTORY). Note the correction in doc 05: there is no legacy `safe.tbl`: the
transliteration lived only in `unicode.tbl`. The arc is unambiguous: doing less by default was
the fix, and re-litigating it would repeat a mistake the upstream project already paid for.

`--ascii` uses `deunicode` (1.6.2, 2025-04, confirmed active in doc 06 row 5b), behind a
default-enabled Cargo feature so a minimal/distro build can drop the tables.

### 3.7 The safe-character policy

Three classes, defined by rule, not by a shipped table. Doc 03 constraint 6 is the source: it
separates genuinely shell-dangerous characters from merely non-portable ones and warns that a
single "safe" set over-mangles legitimate Unicode.

**Delete class** (removed, no replacement): control characters only, i.e. Unicode `Cc` (C0
controls including newline and tab, C1 controls) plus DEL and NUL. Deleted rather than substituted
because a control character carries no information a human wanted and substituting it leaves
visible litter. The delete class deliberately does **not** include stage 4's invisibles (`Cf`,
bidi, zero-width, Tags, `Cs`, `Co`): those are stage 4's business alone, and if the delete class
duplicated them, `--no-invisible-strip` would be a dead flag and the Stage Independence property
(§8.1) would be false. Controls are the one thing no flag can keep.

Worth naming what this class does **not** have to handle, because upstream needed hardcoded C for it
and we get it from the type system. Doc 12 §6 records three encoding hazards detox's `utf_8` filter
deals with by hand: a UTF-8-encoded NUL is caught by a hardcoded `_hidden_null_` string
(`src/clean_utf_8.c:164-167`) so that no table misconfiguration can ever let a NUL reach the output;
overlong encodings are accepted and normalized rather than rejected, which doc 12 flags as a
security-relevant divergence from strict validators since overlong forms are a classic filter-bypass
vector; and codepoints beyond `0x10FFFF`, reachable only via legacy 5/6-byte forms, are forced to `_`.
**Strict UTF-8 validation at stage 1 subsumes all three.** An overlong encoding, a 5-byte form, an
encoded surrogate, and an encoded NUL are each simply _not valid UTF-8_, so such a name never becomes
text at all: it is `Opaque` and skipped (§3.4). There is no bypass surface to defend because there is
no lenient decoder to bypass, and the delete class handles only the NULs and controls that arrive as
legitimately-encoded `Cc`. This is the clearest case in the document of a whole hardcoded-C safety
mechanism becoming unnecessary rather than being reimplemented.

**Separator class** (each run becomes one `--separator`, default `_`): ASCII space, and the
shell-metacharacter and path set: `" ' ` $ ! * ? [ ] < > | ; & : \ / ( )`.

**Keep class** (everything else, including all Unicode letters, marks, digits, and punctuation
not listed above): letters, digits, and `. , - _ + = ~ # % @ ^ { }`.

Defenses for the contested members:

- `( )` are in the separator class because they are unquoted-shell metacharacters, even though
  media filenames use them constantly (`Movie (1985).mkv`). The stage-9 rule that drops
  separators adjacent to `.` is what keeps the result readable: `Movie_1985.mkv`, not
  `Movie_1985_.mkv`. Counterargument: `_1985_` is uglier than `(1985)` and this is the single
  most likely default to draw complaints.
- `[ ]` are in the **separator** class and `{ }` are in the **keep** class, which looks arbitrary
  side by side and is not. `[` and `]` are glob metacharacters: doc 03 constraint 6 names `*?[]`
  specifically as dangerous "when a filename is later globbed unquoted", and a bracket expression is
  the one glob construct that can silently _match a different file_ rather than merely failing to
  match. `{ }` are not glob metacharacters in POSIX `sh` at all — brace expansion is a bash/zsh
  extension, it is not filename globbing, and it cannot make a name match something it is not. So
  `[720p].MKV` becomes `_720p_.MKV` (then `_720p` after stage 9's dot rule) while `{draft}.txt`
  survives as `{draft}.txt`. Counterargument, and it is the same one `( )` draws: media and
  screenshot filenames use `[...]` heavily (`[1080p]`, `[SubsPlease]`), so this default will be felt
  by exactly the users with the messiest directories. The escape is one flag, `--keep '[]'`, and the
  asymmetry is documented rather than smoothed over — `--strip '{}'` is there for anyone who wants the
  reverse.
- `%` and `#` are kept. detox v3 also leaves them (doc 01 §8 item 4). Once `url_decode` has run,
  a surviving `%` is a literal the user meant.
- `=` is kept, despite #109 asking for it to be tamed (doc 02 theme 2): it is not shell-dangerous
  and `--strip '='` or a config `rule` covers the ask.
- `&` becomes a separator, not the string `_and_` that detox emits (doc 01 §4). Word injection is
  a language-specific content decision; a config `rule` with `find = "&", replace = "and"` does it
  for the people who want it, and does it before the safe map, in the right order.
- Non-ASCII letters are kept, always, unless `--ascii`. This is #99's resolution and P4.

`--keep <CHARS>` and `--strip <CHARS>` move individual characters between classes. That is the
entire "custom character set" surface, and it is exactly what the Debian maintainer asked detox
for in #7: the request the upstream maintainer himself called well-aligned with his vision
(doc 02 theme 10).

### 3.8 Whitespace and separator collapsing

Collapse a run of the _same repeated character_ only, and only for characters in the collapse set:
`.`, `-`, `_`, and the configured `--separator` (which is `_` by default, so by default the set is
three characters). Stating the set matters, because `-` and `.` are Keep-class (§3.7) and would
otherwise be untouchable: Keep-class means "not deleted and not substituted", not "exempt from
collapsing". `a__b` -> `a_b`. `a--b` -> `a-b`. `a...b` -> `a.b`. `a_-_b` -> `a_-_b`,
**unchanged**, because no run here is longer than one character. Nothing outside the collapse set
collapses, so `aaa` stays `aaa`; but a run produced _by_ stage 7 is a run of the separator and does
collapse, which is the whole point (`" & " -> "___" -> "_"`).
detox collapses mixed runs by positional precedence in a
hardcoded search string (`.` beats `-` beats `_`, an artifact of `strchr`, confirmed in source
by both doc 01 §2.3/§8.7 and doc 05), and that behavior is precisely what #121 filed a bug
about: the `_-_` convention in music filenames being destroyed. Same-character-only collapsing
is simpler to explain, simpler to implement, and fixes the complaint.

Trailing/leading handling: strip a leading `-` because a filename starting with `-` is parsed as
a flag by every command that receives it unquoted (doc 03 constraint 6, the one unambiguous
shell-danger). Strip trailing dots and spaces always, on every platform, because Windows and the
Windows shell do not agree with POSIX about whether they exist (doc 03 constraint 4, whose wording now
carries doc 06 row 7c's refinement: Microsoft's own text is the softer "the Windows shell and user
interface does not [support such names]", i.e. a shell/UI-layer inconsistency rather than a guaranteed
hard filesystem strip — which does not change the decision, since a name that changes on arrival is a
collision risk either way). Preserve exactly one leading `.` so dotfiles stay dotfiles.

"Leading" means leading _after_ the preserved dot, and that needs a worked example because two
implementations would otherwise differ. `.!file.txt`: stage 7 turns `!` into `_`, giving `._file.txt`;
stage 10 preserves the one leading `.` and then strips the separator that immediately follows it,
giving `.file.txt`. Not `._file.txt`. The dot is preserved as a dotfile marker, not as a character
that shields whatever comes after it.

**A leading dot RUN is preserved verbatim** (§8.1, Dotfile preservation). `..weird..name..` becomes
`..weird.name`, not `.weird.name`. The input already has multiple leading dots, making it a dotfile
or an edge case of one — collapsing the leading run would manufacture a dotfile out of a name that
was not one.

### 3.9 Case handling

`keep` by default. Lowercasing is taste, not safety: #102 asked for a flag, not a default (doc 02
theme 10), and on a case-insensitive volume a mass lowercase is a collision generator. When
requested, use Unicode simple case mapping, not detox's ASCII-only `tolower()` per byte (doc 01
§2.3). Case-only renames are safe to perform in a single syscall, and safe to perform through the
ordinary no-clobber path: see §5.4, where doc 06 refuted both doc 03's temp-name dance and this
document's own later claim that the no-clobber flag rejects a same-inode respell.

### 3.10 Length truncation

On by default, but it only fires on names that are actually too long, so in practice it is
invisible. detox's default sequence has no length filter at all and a 254-byte name passes
through untouched (doc 01 §8 item 5).

Limits, using the **refined** numbers from doc 06, not doc 03's originals:

| Filesystem      | Limit                     | Source                                                                                                                                                                                                                                                        |
| --------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| ext4 and family | 255 **bytes**             | doc 03 constraint 7                                                                                                                                                                                                                                           |
| APFS / HFS+     | 255 **UTF-16 code units** | doc 06 Test 1, four-way discriminated against byte and codepoint hypotheses using ASCII (255), precomposed é (255 at 510 bytes), CJK (255 at 765 bytes), and astral emoji (127, i.e. 254 UTF-16 units), identical on case-sensitive and case-insensitive APFS |
| NTFS / exFAT    | assume 255 UTF-16 units   | **[UNVERIFIED]**: doc 06, Load-Bearing Uncertainties: no such volume was available in any research pass                                                                                                                                                       |

Field mapping: the resolved `Policy` carries `max_len_bytes: usize` and `max_len_utf16: usize`, both always concrete, both always checked, `truncate` shrinking until both are satisfied. Per §3.10's detection:

- **ext4**: `bytes = 255, utf16 = usize::MAX`
- **APFS**: `utf16 = 255, bytes = usize::MAX`
- **unknown volume and NTFS/exFAT under their standing assumption**: `both = 255`

We satisfy **both** metrics simultaneously (bytes <= byte limit AND UTF-16 units <= unit limit)
whenever the volume is unknown, which is the conservative intersection and costs nothing. The
limit is detected per-directory, once, from a small probe or a `statfs` type mapping, and is
overridable with `--max-len N`, which sets the field corresponding to the detected volume's own unit and leaves the other at its detected value; on an unknown volume it sets both. Note that APFS's
255-UTF-16-unit figure is empirically solid but not documented by Apple (doc 03/06 both flag
this), which is why it is a runtime constant, not a compile-time assumption.

Algorithm:

1. Split extension. Treat as an extension the last `.`-suffix and, if the segment before it is
   <= 4 **bytes of UTF-8** and itself preceded by a `.`, the pair (`.tar.gz`). This deliberately
   reproduces detox's behavior, including the 5-character lookback (doc 01 §2.3, §8 item 8;
   confirmed against source in doc 05), because it is well-understood and the failure mode is
   benign. The unit is stated because everything else in this section is careful about
   bytes-vs-codepoints-vs-UTF-16-units and this comparison must not be the one place a reader
   guesses. Bytes, specifically, and verified against primary source rather than inferred:
   `src/clean_string.c:284-294` in the pinned clone `0a8e212` does `while (--input_walk > filename)
{ if (extension - input_walk > 5) break; ... }` on a `char *`, i.e. pointer arithmetic over
   bytes. For the ASCII inner segments this rule exists to catch (`.tar`, `.tar.gz`, `.tar.bz2`)
   bytes and codepoints agree, so the choice only shows up on inputs the rule was never aimed at.
2. Truncate the stem on a **grapheme cluster** boundary via `unicode-segmentation`, not a
   codepoint boundary. `sanitize-filename` 0.6.0 truncates at `is_char_boundary`, i.e. it will
   split a base+combining-mark pair or a ZWJ emoji sequence (doc 06 row 5a, read from source:
   worse than doc 03 implied). That is why we do not use it.
3. If the extension alone does not fit, truncate the whole name as one unit rather than detox's
   "print a warning and give up unchanged" (doc 01 §2.3). Same grapheme-cluster boundary algorithm
   as step 2, just with no extension split: the "No grapheme splitting" property (§8.1) is not
   waived on the fallback path, which is exactly where an implementer would be tempted to reach for
   `is_char_boundary`. **This fallback also applies when the stem would otherwise become empty**
   (§8.1, Dotfile preservation): `abcdef.txt` under a 4-byte limit would otherwise become `.txt`,
   which is also a manufactured dotfile. Truncate the whole name instead.

Both step 2 and step 3 call one shared grapheme-truncation helper, not two loops.

4. If truncation makes the name collide, append a disambiguator (see §5.3). Truncation alone
   generates collisions among similarly-prefixed files; Samba's mangling scheme hashes precisely
   to avoid unrelated files merging (doc 03, "Samba mangled names").

### 3.11 URL decoding

On by default, all-or-nothing per name, and only when the decode is provably safe: every `%`
in the name starts a well-formed triplet, the result is valid UTF-8, and the result contains no
`/`, NUL, or control characters. A name like `100%25 done.txt` decodes to `100% done.txt` and
then the safe map runs (matching detox's `uncgi`, doc 01 §2.3); a name like `50%-70% off.txt`
has a malformed escape and so is left entirely alone rather than half-decoded. detox's `uncgi`
decodes unconditionally and is not in the default sequence; ours is safe enough to default on,
because downloaded files with `%20` in the name are the single most common real-world case of an
ugly filename.

`+` -> space is **off by default**, and this is not a close call: `libstdc++`, `g++-13`, `C++
notes.txt` are all real filenames and `plus_to_space` mangles every one of them.

### 3.12 Unicode security

Stripping invisibles is **on by default and is the one transform with no taste component**.
CVE-2021-42574 (Trojan Source) showed bidi controls make displayed order diverge from byte order,
and the same trick works on a filename in a file manager or terminal; zero-width characters make
visually identical names byte-distinct (doc 03 constraint 8). detox has none of this awareness
(doc 03, positioning anchor), and the tracker shows it surfacing as user-visible confusion rather
than as a security report: #120 hidden Unicode Tags, #116 narrow no-break space behaving
differently on macOS and Linux (doc 02 theme 3).

Confusable/homoglyph handling is **detection only, never rewriting**, and is **out of v1.0**.
Two reasons. First, rewriting a confusable is a guess about intent and violates P4. Second, the
crates doc 03 recommended for it do not hold up: `unicode_skeleton` last released 2017-10-08,
`confusables` 0.1.0 from 2023 (doc 06 row 5c). v1.1 gets a UTS #39 skeleton table generated from
UCD data checked into the repo (P7: we own the table, no dependency), and emits a warning when a
batch would produce two names that are skeleton-identical but byte-different. v1.0 ships the
cheap subset: a warning when a single name mixes scripts in a way UTS #39 calls out.

### 3.13 Summary: on by default vs opt-in

Non-UTF-8 names appear in neither column: they are skipped, and skipping is not a transform the user
can turn on or off (§3.4).

| On by default                              | Opt-in                                 |
| ------------------------------------------ | -------------------------------------- |
| Percent-decoding (`%XX`, safe-only)        | `--ascii` transliteration              |
| NFC normalization                          | `+` -> space                           |
| Invisible/bidi/tag stripping               | `--case lower/upper`                   |
| Control-character deletion                 | `--target windows/portable`            |
| Space and shell-metacharacter -> separator | `[[rule]]` custom patterns             |
| Same-separator run collapsing              | `--keep` / `--strip`                   |
| Leading `-`, trailing dot/space trimming   | `--hidden` (dotfiles during recursion) |
| Grapheme-safe length truncation            | `--files-only` / `--dirs-only`         |
| Collision detection + renumbering          | Full-width folding (v1.1)              |
| Undo journal                               | Confusable-pair warnings (v1.1)        |
|                                            | `--edit` (v1.1)                        |
|                                            | `--repair-encoding` (post-1.0, §3.4)   |

### 3.14 When there is no representable output: `Unrepresentable`

This subsection exists because the earlier draft had a real contradiction, and the resolution is a
design decision, not a wording fix. Stage 13's original rule ("if the result is empty, `.`, or
`..`, keep the original name") reintroduces exactly the characters the pipeline exists to remove.
Worked counterexample: `***`. All three characters are separator-class, so stage 7 gives `___`,
stage 9 collapses to `_`, stage 10 strips the lone leading/trailing separator, and the result is
empty; falling back to `***` yields an output containing three separator-class characters. That
falsifies the **Safety closure** property in §8.1, which is a non-negotiable release gate. A
release gate that the specified default pipeline violates on a three-character input is not a
gate.

The resolution: `transform` has a third outcome. It either produces a name that satisfies safety
closure, or it produces **`Unrepresentable(reason)`** and no name at all.

```rust
pub enum TransformResult { Name(Outcome), Unrepresentable(Unrepresentable) }
pub enum Unrepresentable { ReducesToEmpty, ReducesToDotOrDotDot, NotConverged }
```

- The planner turns `Unrepresentable` into `Resolution::Skipped`, reported like an `Opaque` name
  (§3.4) and counted in the `skipped` tally, with the entry left **exactly** as it is on disk:

  ```
    ***   -   skipped (nothing safe remains after cleaning; use --keep, a [[rule]], or rename it yourself)
  ```

- `NotConverged` (the stage-13 loop still moving after 3 iterations) takes the same path. That is
  the whole answer to "what happens when the bound is not enough": no silent non-idempotent
  output, no runtime-raised bound, no panic. A `NotConverged` occurrence is also a bug report
  against us, so it is logged at `-v` with the intermediate states.
- Skipping rather than substituting a placeholder is P3 and P4 in combination: we never destroy
  data, and inventing a name (`_`, `file`, a hash stub) is a taste-driven guess that would also
  collide with every other unrepresentable name in the directory. Doing nothing is the honest
  outcome, and `--strip`/`[[rule]]`/`--keep` give the user a way to say what they wanted.

Consequences elsewhere, all of them now stated: §8.1's Safety closure and Non-empty properties are
quantified over `Name(_)` results only, and a new property covers the other branch (`transform`
returns either a safety-closed name or `Unrepresentable`, never an unsafe name). §11 question 11
records the one thing this does _not_ settle: whether real trees contain enough all-punctuation
names that a placeholder policy would earn its keep.

---

## 4. Configuration

### 4.1 Format and shape

TOML, flat, keys named identically to the CLI flags. Not a DSL: no conditionals, no expressions,
no includes, no character tables. The model is ripgrep's ("the config file is a saved argument
list", doc 04 §2, confirmed doc 07 rows 2a/2b), with named profiles as TOML sub-tables.

**The named-profile hedge is withdrawn.** An earlier draft called this "the weakest-evidenced part of
that research" and justified profiles by the Cargo `[profile.*]` analogy plus the mandate. The Cargo
analogy was never the real precedent — it is a shape a Rust audience recognizes, which is a
familiarity argument, not an evidence one. The actual precedent is **AWS CLI named profiles**
(`~/.aws/config`, `[profile web]`, selected with `--profile web` or `AWS_PROFILE`): the same feature,
same file, same selection mechanism, in a tool with an enormous user base and a decade of stability,
where a named profile is a saved set of options for one context rather than a build variant.
**kubectl contexts** and **gcloud configurations** corroborate it: three independent, widely-used CLIs
that solved "same command, different preset" the same way, none of them a build system. That is not a
weak precedent; it is the dominant convention for exactly this problem, and finding it strengthens the
design rather than merely excusing it.

The mandate still names the feature directly ("a config file that lets you pre-select those options"),
and the alternative (multiple config files, one per preset) is still worse — but those are now
supporting arguments rather than the whole case. One deliberate divergence from AWS: `detoxrs` reads
no environment variable to select a profile, only `-p/--profile` (§4.3), because an ambient
`$DETOXRS_PROFILE` that silently changes what a rename does is the same class of hazard as the
locale-conditional tables §9.2 rejects.

### 4.2 A realistic config file

```toml
# $XDG_CONFIG_HOME/detoxrs/config.toml

# I have read the preview enough times. Just rename.
exec       = true

separator  = "_"
case       = "keep"
normalize  = "nfc"
url_decode = true
on_collision = "number"

exclude = [
  "*.crdownload",
  "*.part",
  "Icon\r",          # macOS custom-folder-icon file; issue #94 asked detox for exactly this
  ".DS_Store",
]

# Custom patterns. Ordered. Literal unless regex = true.
# Applied after invisible-stripping and before transliteration and the safe map.
[[rule]]
find    = " - "
replace = "-"

[[rule]]
find    = "&"
replace = "and"

[[rule]]
find    = '^(\d{4})-(\d{2})-(\d{2}) '
replace = '$1$2$3_'
regex   = true

# Keep the _-_ convention my music library uses (detox issue #121 could not).
[[rule]]
find    = "_-_"
replace = "�KEEP�"     # no. see note below.

[profile.web]
case      = "lower"
separator = "-"
ascii     = true
max_len   = 100

[profile.sdcard]
target    = "windows"
ascii     = true
case      = "lower"
max_len   = 64

[profile.paranoid]
exec         = false
on_collision = "fail"
ascii        = true
```

That fourth `[[rule]]` is deliberately shown as a mistake and is **not** how you preserve `_-_`:
because stage 9 only collapses runs of identical characters, `_-_` survives with no rule at all.
The docs will show it as the worked example of "you do not need a rule for this."

Usage: `detoxrs -p web ./site-assets`, `detoxrs -p sdcard -x /Volumes/CARD`.

### 4.3 Discovery and precedence

Resolution order, first match wins, **no merging between files**:

1. `--config <FILE>`: that file only. Error if unreadable (ripgrep's behavior: never silently
   ignore an explicitly requested config, doc 04 §2).
2. `$DETOXRS_CONFIG`: same.
3. Nearest `.detoxrs.toml` walking up from the current directory to the filesystem root.
4. `$XDG_CONFIG_HOME/detoxrs/config.toml` (fallback `$HOME/.config/...`, doc 04 §2 table,
   confirmed verbatim doc 07 row 4).
5. Built-in defaults.

Within the chosen file: built-in defaults < top-level keys < `[profile.NAME]` (only if `-p NAME`)
< CLI flags. Environment variables set no transform options; only `DETOXRS_CONFIG`, `NO_COLOR`,
and friends are read.

Because that stack is four layers deep, there is one command that answers "which of these set this
value": `detoxrs --print-config` dumps the fully resolved policy as TOML and exits. This is the one
capability detox's `-L -v` had (list sequences / dump the active config, doc 10) that P1's fixed
pipeline does not make redundant — it makes it _more_ useful, since a user combining `--config`, `-p`,
and flags has more ways to be surprised than a user picking one sequence. It is read-only, walks
nothing, and its output is the same shape as a config file, so it doubles as "write my current
invocation down."

**`--print-config` must resolve everything it prints, and must say so about the one thing it cannot.**
Upstream's `-L` is the worked example of why this is a hard requirement rather than a nicety: doc 10
records that `-L` **exits 0** while dumping a configuration that the same binary then **fatally exits
1** on at run time. A dump that succeeds on a config that cannot run is worse than no dump, because it
converts a discoverable error into false confidence. So:

- **Resolve, do not echo.** The chosen config file and profile are resolved and named in a comment
  header (`# from: ./.detoxrs.toml, profile "web"`), the four precedence layers are collapsed into one
  value per key, and no key is printed as an unevaluated reference to anything.
- **Validate everything compilable.** Every `[[rule]]` regex and every `--exclude` glob is compiled
  during the dump, and an invalid one makes `--print-config` fail with **exit 2 and the same error
  message a real run would give** — not exit 0 with the broken pattern faithfully echoed. If a config
  cannot run, `--print-config` must not succeed on it. This is the whole of the `-L` lesson.
- **Name the one unresolvable key rather than faking it.** `max_len = 0` means "detect the
  filesystem limit", and detection is per-directory at walk time (§3.1, §3.10); `--print-config`
  deliberately walks nothing, so it genuinely cannot resolve it. It prints the sentinel with a comment
  saying so — `max_len = 0  # auto: resolved per directory at walk time, not by --print-config` —
  rather than printing 255 and being wrong on some volume. The `--help` text for the flag carries the
  same one-line caveat, because the alternative to resolving a value honestly is disclosing that you
  did not, and the alternative to both is upstream's `-L`.

First-match-wins over cascading is the ruff/Prettier model (doc 04 §2; doc 07 row 3 confirms
doc 04 got the ruff `extend` nuance right, and row 3b confirms Prettier's nearest-file-fully-
governs behavior). detox merges a system file, `$HOME/.detoxrc`, and the XDG file with same-named
sequences replacing earlier ones (doc 01 §2.1, source-confirmed doc 05, though doc 05 flags the
merge as source-verified only, never behaviorally exercised). Five merged files with silent
name-shadowing is exactly the "which of these set this value" confusion doc 04 §2 warns about.
There is deliberately **no system-wide `/etc/detoxrs/config.toml`** in v1.0: a tool that renames
a user's files should not have its defaults changed by a package.

### 4.4 What is deliberately not configurable

| Not configurable                                | Why                                                                                                                                                               |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Pipeline order                                  | P1. The order is a correctness property (§3.3), and reorderable stages are how detox got mutually-exclusive filters that corrupt data.                            |
| Character _tables_ as external files            | P1/P2. `.tbl` files are the config-complexity theme's root cause (doc 02 theme 1). `--keep`/`--strip`/`[[rule]]` cover the real asks (#7, #84, #117, #121, #124). |
| Which encoding to decode _valid_ UTF-8 as       | P2. There is no such option; this is the whole point.                                                                                                             |
| Overwrite-on-collision                          | P3, and the maintainer's #130 rejection. Not a default to change, an absent capability.                                                                           |
| Whether truncation is grapheme-safe             | Correctness, not preference.                                                                                                                                      |
| Journal format                                  | Undo has to be readable by a future version.                                                                                                                      |
| Whether recursion follows symlinked directories | §5.6. There is no flag. Issue #23 documents the blast radius (upstream fixed its own instance in 2.0.0-beta1; the hazard is inherent to the operation).           |
| VCS-metadata skip (`.git`/`.hg`/`.svn`)         | §5.6. Never configurable, not even with `--hidden`. Renaming inside `.git` corrupts a repository; #110's `--git` rejection says this tool has no business there.  |
| `[profile.*]` inheritance / `extend`            | One level of indirection is enough; ruff added `extend` under pressure and it is the part of its config model people misread.                                     |

---

## 5. Safety architecture

This section is the conservative one. Everything here defaults to "do less."

### 5.1 Two phases, never interleaved

```
walk  ->  snapshot: Vec<Entry>  ->  plan(entries, policy) -> Plan  ->  apply(plan) -> Report
                                                      (no I/O)          (all I/O)
```

The walk completes and the entry list is frozen **before** any rename happens. detox renames a
directory and then recurses into its new path (doc 01 §6, `parse_dir`), which is the hazard the
maintainer himself named in rejecting #130 (`readdir()` ordering interacting with in-place
renames, doc 02 theme 6). Doc 03 constraint **11c** states the requirement directly: snapshot the
list, do not rename while iterating. (Doc 03's constraint 11 was split into 11a/11b/11c at its own
stage-3 review, with content unchanged, so citations here name the sub-point rather than the group.)

`apply` processes items **deepest-first** so that a parent directory's rename never invalidates a
child's recorded path. This is the opposite of detox's top-down order and it is the reason the
plan file in §5.7 can be trusted.

```rust
pub struct Plan { pub root: PathBuf, pub created: SystemTime, pub policy_digest: [u8;32],
                  pub items: Vec<PlanItem> }

pub struct PlanItem {
    pub dir: PathBuf, pub from: OsString, pub to: OsString,
    pub kind: EntryKind,          // File | Dir | Symlink | Other
    pub ident: Ident,             // (dev, ino, nlink, mtime) captured at walk time
    pub depth: u32,
    pub resolution: Resolution,   // Rename | Unchanged | Skipped(Reason) | Conflict(Conflict)
    pub notes: Vec<Note>,
}
```

### 5.2 Files are never moved between directories

`detoxrs` only ever changes a basename. It has no syntax for a destination directory. That is a
scope decision with an outsized safety payoff: `EXDEV` (doc 03 constraint 11a) becomes
structurally impossible, so there is no copy+unlink fallback, no non-atomic path, and no
"lost hardlink identity" case to reason about. If you want to move files, use `mv`.

Because only the directory entry changes, everything attached to the inode is untouched by
construction: extended attributes (including macOS resource forks and Finder metadata), ACLs,
SELinux contexts, owner, group, mode, and the inode number itself all survive a `detoxrs` rename
unchanged. That is a `rename(2)`-level guarantee, not something we implement, and it is stated here
so a reader auditing the safety story does not have to derive it from POSIX.

### 5.3 Collisions

Three layers, in order:

1. **Intra-batch, before any I/O.** The planner builds a map of `(dir, NFC(to))` -> sources. Any
   destination with more than one source is a conflict. This is the "N files collapse into 1"
   risk the maintainer raised on #130 (doc 02 theme 6, verbatim-confirmed doc 05) and detox has
   no equivalent check: its collision logic is per-rename at rename time, in `readdir()` order,
   so which file loses is filesystem-dependent (doc 01 §5, scope note).
2. **Pre-existing destination**, from the walk snapshot plus a fresh `symlink_metadata` at apply
   time.
3. **Kernel-level no-clobber** at the syscall (§5.4). Belt and braces: layers 1 and 2 have a
   TOCTOU window; layer 3 does not.

Policies, `--on-collision`:

- `number` (**default**): insert ` -N` before the extension, smallest free N >= 2:
  `IMG_0042.JPG` -> `IMG_0042-2.JPG`. Numbering is allocated deterministically: sources with colliding
  NFC hashes are sorted by their **raw name bytes** (which are distinct by definition for two distinct
  directory entries), not by `readdir()` order. This was discovered by the Determinism property test
  (§8.2), which surfaces the exact upstream defect this design exists to fix: a pair like `café.txt`
  (NFC composed) and `cafe\u{301}.txt` (NFD decomposed) share one NFC key and have nothing to sort by except
  input order, so a collation that stops at NFC re-introduces the filesystem-order dependency. A named
  regression test pins this. If numbering would exceed the length limit, truncate the stem further
  rather than exceed it.
- `skip`: leave the conflicting entries alone, report them, exit 1.
- `fail`: refuse the entire batch before renaming anything.

**Renumbering terminates, and the bound is stated rather than assumed.** Further truncation can
itself collide with a third, unrelated entry, which needs another candidate; and at a small enough
limit (`--max-len 2`) no numbered name fits at all. So the search is bounded exactly like stage 13
is: for a given source, try N = 2..999, each candidate truncated as needed to fit the limit, against
the set of names already taken in that directory (existing entries plus destinations already
allocated in this plan). The first candidate that fits and is free wins. If none does, the item is
an **unresolvable conflict**: it becomes `Conflict` and is routed by `--on-collision` exactly as if
the user had asked for that policy, i.e. reported and left alone under `number`/`skip`, and fatal
to the batch under `fail`. We never guess a name, never drop the numbering to fit, and never exceed
the limit. 999 is a stated ceiling, not a computed one: a directory with a thousand names colliding
on one destination is a case where the honest output is a report, not a rename.

**Rename cycles and sibling swaps: structurally impossible, and here is the proof rather than the
assurance.** A reviewer asked for cycle detection, a temp-name dance, and topological ordering of
sibling renames, on the grounds that deepest-first ordering (§5.1) only protects parents against
children and says nothing about siblings. The observation is right; the remedy is unnecessary, and
building it would add the one thing §5.4 works hardest to avoid — a rename to a name the user never
asked for.

`transform` is a pure function of `(name, Policy)` (§3.1) and one resolved `Policy` governs the whole
directory (§3.10). Suppose a two-item swap: `f(a) = b` and `f(b) = a`. Idempotence (§8.1, a
non-negotiable release gate) requires `f(f(a)) = f(a)`, i.e. `f(b) = b`, so `a = b`. Contradiction.
The same argument kills chains: `f(a) = b` and `f(b) = c` forces `f(b) = b`, i.e. `c = b`, which means
the second entry was never a `Rename` at all — it is `Unchanged`, because it is already a fixed point.
Renumbering cannot manufacture one either, since it only ever allocates a name that is free of both
existing entries and already-allocated destinations.

So the only way one entry's destination equals another entry's current name is the case layers 1 and
2 already handle: the other entry is `Unchanged`, and this is an ordinary pre-existing-destination
conflict, renumbered or refused per `--on-collision`. There is no ordering problem to solve.

What we do add is the assertion, because a proof that rests on Idempotence should fail loudly if
Idempotence ever breaks: at plan time, if any `Rename` item's destination equals the `from` of another
item that is _also_ a `Rename` in the same directory (excluding renames whose destination equals their
own NFC source, which is an NFD respell and not a chain), that is an internal-consistency bug, and the
planner refuses the entire batch with an internal error rather than renaming anything. This was found
by checking against near-swap pairs during property-test generation (§8.2), to exclude false positives
like an NFD entry `cafe\u{301}.txt` respelling to its NFC form `café.txt` sitting beside that same NFC form
as an unrelated intra-batch collision. Cheaper than a cycle breaker, and the assertion catches the real
failure (a stage that is not idempotent) instead of papering over it.

**Implementation deviations from the collision model.** The following were discovered during engine
implementation and are recorded here so the code and spec agree:

1. The `Conflict` result carries three distinct variants rather than a single `Collision` case: one
   for unresolvable candidates (renumbering exhausted, §5.3), one for pre-existing paths, and one for
   intra-batch collisions where the policy is `fail`. Reporting "probe limit exhausted" for a plain
   two-file collision under `skip` or `number` would be false. Each variant conveys the failure reason.
2. `--on-collision fail` returns an error rather than an appliable `Plan`. The refusal is structural:
   the engine refuses the entire batch at plan time, not after allocating a destination. The error
   carries the conflicting items so the caller can still report them (e.g. to console or a logging system).
3. Item ordering in the plan derives depth from the directory path (computed during walk) rather than
   from a depth field supplied by the walker. This means the Order-safety property (§8.2) cannot be
   broken by a miscounting walk. Similarly, `plan()` takes the volume's case-sensitivity as an explicit
   enum parameter (§6.2) rather than inferring it from the platform, so the callee has no ambient state
   to trust.

There is **no** `overwrite`. Not off-by-default, absent. Doc 02 theme 6 records two independent
asks (#122, #130) and a considered rejection; #124 ties "just works" directly to not destroying
files. Renumbering is what "just works" means here: detox's refuse-and-report is why users asked
for `-F` in the first place.

Counterargument: auto-numbering silently implies a relationship between unrelated files
(`Report-2.pdf` looks like a second version of `Report.pdf`), and a user who wanted `skip` gets
a tidy directory full of misleading names.

### 5.4 Atomicity and the platform rename API

Per-item atomic, no-clobber, with a graceful ladder. There is no batch atomicity and we will not
pretend otherwise: a POSIX filesystem gives no way to make 400 renames one transaction. `--atomic`
does not exist (compare Helm's `--atomic`, doc 04 §1: it can roll back because it owns the
resources; we cannot). What we give instead is: nothing mutates before the whole plan is computed
and shown, per-item renames are individually atomic, and the journal makes the batch reversible.

```rust
pub trait RenameOps {
    /// Fails with AlreadyExists rather than clobbering. Never falls back to a clobbering call.
    /// This is the only rename entry point. A "same inode, respelled" rename (a case change,
    /// or NFD -> NFC: §6.2) goes through it too — measurement says the kernel resolves the
    /// same-inode variant rather than reporting the destination as occupied, and where it does
    /// not, the EEXIST is handled as a narrow observed-error fallback below, not predicted.
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}
```

**One safe API covers both tier-1 platforms: `rustix::fs::renameat_with` with
`rustix::fs::RenameFlags::NOREPLACE`.** This is a correction to an earlier draft of this section,
and it removes a whole subsystem rather than adjusting a sentence. `rustix` 1.1.4 maps `NOREPLACE`
to `RENAME_NOREPLACE` (`renameat2`) under `#[cfg(linux_kernel)]` and to `RENAME_EXCL`
(`renameatx_np`, weak-linked with a documented plain-`rename` fallback for macOS < 10.12) under
`#[cfg(apple)]`; `EXCHANGE` maps to `RENAME_EXCHANGE`/`RENAME_SWAP` the same way (doc 06 row 4e,
which **withdraws its own earlier refutation**, and doc 03 constraint 10 `[CORRECTED]`). So there
is **no hand-written `libc` FFI shim**, no ~60 lines of `unsafe`, no shim tests, and no unsafe-audit
budget: both crates carry `#![forbid(unsafe_code)]`. docs.rs is what hid this — its default render
target for `rustix` is `x86_64-unknown-linux-gnu`, so every `#[cfg(apple)]` item is invisible in the
default view. `nix` genuinely does not expose the macOS flags (Linux-glibc only), which is an
argument for `rustix` over `nix`, not for a shim.

- **Linux**: `RenameFlags::NOREPLACE` -> `renameat2(..., RENAME_NOREPLACE)`. Doc 03 constraint 10
  gives the filesystem support matrix (ext4 3.15+, btrfs/tmpfs/cifs 3.17+, xfs 4.0+, most by 4.9)
  but doc 06 row 4c marks it **[UNVERIFIED]**: no Linux machine existed in any research pass. So
  support is detected at runtime: `EINVAL`/`ENOSYS`/`EOPNOTSUPP` on first use demotes that mount to
  the fallback path and prints one warning naming the mount.
- **macOS**: the same call, `RenameFlags::NOREPLACE` -> `renameatx_np(..., RENAME_EXCL)`. Doc 06
  row 4b confirmed the semantics from the local `man renamex_np` on Darwin 25.5, and doc 06 rows
  4e/4f then confirmed them by running the safe wrapper on APFS: `EEXIST` (errno 17) onto an
  occupied name, `Ok(())` onto a free one, and `EXCHANGE` swapping two files' contents. On
  2026-07-31, `rustix` 1.1.4 `renameat_with(NOREPLACE)` under `#![forbid(unsafe_code)]` on APFS
  returned `Ok(())` on a same-inode case-only respell and `Err(EEXIST/17)` onto a distinct occupied
  destination. The observed-`EEXIST` same-inode fallback is defensive-only rather than a normal path.
  **There is deliberately no `getattrlist`/`VOL_CAP_INT_RENAME_EXCL` probe.** `rustix` does not wrap
  `getattrlist`, and a probe would be an `unsafe` dependency whose only job is to predict, at open
  time, what the rename call reports anyway — the design already demotes on the error for Linux, so
  macOS uses the identical path and needs no second mechanism. **[UNVERIFIED]**: which errno a
  macOS volume lacking `VOL_CAP_INT_RENAME_EXCL` actually returns (the demotion set is assumed to be
  the same `EINVAL`/`ENOTSUP` family); §11 spike 13.
- **Windows** (best-effort tier): `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, which is
  no-clobber by default. Matches what `rust-lang/libs-team#131` proposes; doc 06 row 4a confirms
  that issue is still open, so `std::fs::rename` gives us nothing here and will not soon.
- **Fallback** (unsupported flag or filesystem): `symlink_metadata(dest)` then `rename`, with the
  TOCTOU window documented in the man page and reported in `--json` as
  `"atomicity": "check-then-rename"`. We do **not** use the `link()`+`unlink()` trick: it fails
  on directories, changes `st_nlink` observably, and is a surprise on filesystems without
  hardlinks.

**Case-only renames do _not_ need their own syscall path, and the reason an earlier draft gave for
one was measurably false.** Two separate refutations stack here, so both are stated.

First, doc 06 Test 3 **refutes** doc 03 constraint 2: plain `rename(2)` renames `CaseTest.txt` to
`casetest.txt` directly on case-insensitive APFS, verified at both the `os.rename` and raw C syscall
level, on the boot volume and on fresh case-sensitive and case-insensitive images. No temp-name
dance. That much this document already said.

Second, and new: this section used to claim that `RENAME_NOREPLACE`/`RENAME_EXCL` "would return
`EEXIST` for that same rename, because the destination 'exists': it is the same inode", and made
that claim the entire justification for a second trait method. Doc 06 row 4f measured it and it is
**false** on APFS: `Case.txt` -> `case.txt` with `RenameFlags::NOREPLACE` on a case-insensitive APFS
volume returns **`Ok(())`**, with a control in the same run proving the flag was honored rather than
dropped (two distinct existing files, `p` -> `q`, returned `EEXIST`). The kernel resolves the
same-inode case variant instead of treating it as an occupied distinct destination. Re-run
independently for this pass on the same hardware, including a `foo.txt` -> `FOO.txt` case-only rename
under `#![forbid(unsafe_code)]`: `Ok(())`.

So `rename_case_only` is **deleted**. Its stated reason is gone, and a rename to a name the user
never asked for is exactly what §5.4 works hardest to avoid; a whole method existing to route around
an error that does not occur is worse than no method. What survives is the part that was never about
the syscall: **the planner still detects the same-inode respell** — `to` differs from `from` only by
case or only by normalization form, and `symlink_metadata(to)` reports the same `(dev, ino)` — because
otherwise collision layers 1 and 2, which compare names, would report a respell as a conflict with
itself. That detection now feeds reporting, not routing. (It mirrors detox's same-inode escape hatch,
doc 01 §5, `st_dev`/`st_ino` match with `st_nlink == 1`, confirmed doc 05, minus the `nlink == 1`
condition detox needs only because it has no batch-level plan.)

Where the old claim might still be true, it is handled as an **observed error, not a prediction**: if
`rename_noreplace` returns `EEXIST` and the destination is the same inode as the source, fall back to
plain `rename(2)` for that one item and warn once naming the mount. Three lines, the same
demotion-on-error shape as the Linux flag support check, and no behavior asserted about a filesystem
nobody has measured. **[UNVERIFIED]**, and now two distinct gaps rather than one: whether direct
case-only rename works on SMB/NFS mounts (§11 spike 5, unchanged; doc 06 correction #1 asks for a
citation for any filesystem where it fails), and whether Linux `RENAME_NOREPLACE` behaves like APFS
on a case-only rename over a case-insensitive mount such as ext4-casefold, vfat, or exFAT (§11 spike
14, new — doc 06 row 4f explicitly leaves the Linux side unmeasured). Until both close, the fallback
never unlinks anything.

### 5.5 Undo

Append-only JSONL, one file per batch, in `$XDG_STATE_HOME/detoxrs/journal/<UTC-timestamp>-<id>.jsonl`
(`$HOME/.local/state/...` fallback). XDG's own spec names state as the home for "actions history"
(doc 04 §2; confirmed verbatim doc 07 row 4).

We are explicitly **not** copying f2 here. Doc 07 row 1b resolved doc 04's open question by
reading f2's source: f2 writes `os.TempDir()/f2/backups/<md5(cwd)>.json`, i.e. the OS temp
directory, keyed only by a hash of the working directory. That undo does not survive a reboot on
most systems, and it cannot be found by a human. Ours is durable, timestamped, and greppable.

Protocol per item: write an `intent` record, fsync, rename, write a `done` or `failed` record.
A crash therefore leaves a journal from which the exact interrupted item is knowable.

```jsonl
{"v":1,"batch":"20260731T142233Z-a91c","policy_digest":"9f2c…","cwd":"/Users/k/Downloads"}
{"op":"intent","dir":"/Users/k/Downloads","from":"Mario & Luigi (1985).MKV","to":"Mario_Luigi_1985.MKV","dev":16777232,"ino":8419,"mtime":1785000000}
{"op":"done","ino":8419}
```

`detoxrs undo <BATCH-ID>` replays in reverse order, and for each item verifies that the current
`to` name still resolves to the recorded `(dev, ino)`. If it does not, that item is refused, not
forced: something else touched the file. Undo runs through the same no-clobber rename path and
the same collision engine as a forward run, so undo can also report conflicts and can itself be
undone. `--no-journal` exists for scripted use on huge trees and disables `undo` for that batch,
loudly.

Non-goals: no trash-can integration (nothing is deleted, so `trash` from doc 03/04 has no role),
no content backups, no undo across a `--no-journal` run, no undo of a batch whose files have
since moved to another directory.

### 5.6 Symlinks, special files, hardlinks

- We always `lstat`/`symlink_metadata`, never `stat`. A symlink's own name is what gets cleaned;
  the target is never touched, never followed, never resolved.
- **Recursion never descends into a symlinked directory, and there is no flag to make it.**
  The evidence here needs stating precisely, because the obvious citation is stale. Issue **#23**
  is a first-person incident report where `detox -r --special` followed a relative symlink pointing
  at `../..` and recursed across the reporter's entire projects directory (doc 05 correction #2,
  which rightly overturns doc 02's dismissal of symlinks as a weak theme). But **upstream fixed
  that bug in 2.0.0-beta1 (2021-03-05)** — verified directly in the pinned clone `0a8e212`:
  `CHANGELOG.md` line 144, Security section, "Symlinks that point at directories are no longer
  followed when `--special` and `-r` are specified together. [#23]", and structurally confirmed at
  `src/file.c:218-223`, where `lstat` plus `S_ISDIR` means a symlink is never classified as a
  directory and `parse_dir` never descends through one (doc 10, `--special`; doc 13 §4.3). So #23
  is **not** a live flaw in detox v3.0.1 and this document does not claim it is.
  What #23 is still good for is the only thing we need it for: a documented, first-person account of
  the blast radius when a renamer does follow a directory symlink — one relative symlink turning a
  scoped run into a whole-home-directory run. That is evidence about the _hazard_, and the hazard is
  a property of the operation, not of detox's 2021 code. **#20** (symlink loops and `.`/`..`
  symlinks flagged as an untested gap) is unchanged by the #23 fix and remains open evidence that
  nobody has characterized the edge cases.
  The decision therefore rests on its own merits, not on upstream's bug: unbounded blast radius from
  a single symlink is not a feature that earns a flag, and following one buys the user nothing they
  cannot get by naming the target directly. We reach the same place upstream did, by construction
  rather than by patch — the difference being that in `detoxrs` there is no `--special` to combine it
  with and no flag to turn it back on.
- **No `--special`.** detox silently skips symlinks, FIFOs, sockets, and device nodes unless
  `--special` is passed: including ones named explicitly on the command line, which doc 01 §8
  item 3 calls "a very easy trap" and doc 05 re-confirmed (only `Scanning:` is printed, no error).
  We rename any entry we are pointed at, including symlinks and FIFOs, because renaming a
  directory entry is safe regardless of what it points at. The information the `--special` flag
  was protecting is instead surfaced in the preview: entry kind is shown for anything that is
  not a regular file or directory.
- **Hardlinks**: renaming one link renames one directory entry; the other links keep the old
  name. That is `rename(2)`, not a choice we make. We report `nlink > 1` in the preview as a note
  so the behavior is not a surprise (doc 03 constraint **11b**). Doc 02 and doc 05 both confirm zero
  hardlink-related issues were ever filed against detox, so this is a documentation problem, not
  a feature request. Note the limit of that argument: zero filed issues is evidence of low demand,
  not evidence that a hardlinked file's respell is safe, and §5.4 drops detox's `nlink == 1` guard on
  an untested argument. That gap is §11 spike 15, not a settled point.
- **`--recursive` does not replicate upstream's first-level-always-processed behavior, and this is
  stated here because it is where three reviewers looked for it.** detox's `-r` gates descent only
  _past_ the first level: the immediate children of a directory named on the command line are processed
  with or without `-r` (doc 10, `-r`). `detoxrs` deliberately does not copy that. Without `-r`, a
  directory argument has **only its own basename cleaned** and nothing inside it is touched; with `-r`,
  the whole subtree is. The rule is "one argument, one name" versus "one argument, its whole tree",
  with no third middle behavior — and the reason is §2.1: a flag whose scope is one level deeper than
  it reads is a preview the user will misjudge, and it is a poor bargain to inherit a quirk in exchange
  for compatibility this tool has already declined elsewhere (§9.2, which carries the same statement as
  a migration note for someone coming from detox rather than someone reading the safety section).
- Dotfiles are skipped during recursion (matching detox, doc 01 §6) but processed when named
  explicitly on the command line. `--hidden` opts into recursing over them. `.git`, `.hg`, `.svn`
  are skipped unconditionally, even with `--hidden`: renaming a file inside `.git` corrupts a
  repository, and detox's `--git` rejection (#110, doc 02 theme 10) shows this tool has no
  business near VCS metadata.

### 5.7 Plan files

`--plan-out plan.json` writes the plan and exits without renaming; `detoxrs apply plan.json`
executes exactly that plan. Before applying, every item's `(dev, ino, mtime)` is re-checked
against the snapshot; any drift aborts the whole batch with a stale-plan error. This is
Terraform's saved-plan model (doc 04 §1): and doc 07 row 6 is the reason we describe it as the
"Saved plan is stale" behavior sourced to HashiCorp's issue tracker rather than citing the
`plan`/`apply` doc pages, which do not actually state it.

Why bother when preview-then-`-x` already exists: a preview and a subsequent `-x` recompute the
plan and can differ if the tree changed in between. `apply` cannot. For a 200k-file media
library, that difference matters.

### 5.8 Interrupts, I/O failure, and concurrent runs

§5.5's crash story is only as good as its answer for the ways a run actually gets cut short, so
those answers are here rather than left inferable.

Everything in this section used to read as invention, because there was no upstream baseline to
compare against. There is now, and it is a set of **verified negatives** from the source-derived
reference docs, which is what makes the design choices below arguments rather than preferences:
upstream has **no signal handling at all** (no `signal`/`sigaction` anywhere), **no locking
primitives** (no lock file, no `flock`, so concurrent runs were never a considered case), and
**`EROFS`/`ENOSPC` are not special-cased** — a read-only or full filesystem produces one failure line
per entry, for every remaining entry. Each of those is a gap we are filling deliberately, not a
feature we are inventing a need for.

One thing this must **not** claim, and an earlier framing of it would have: upstream is **not** free of
errno branching. It branches on exactly one, and it is the one that matters most for a tree walk —
`EMFILE` after a failed `opendir`, at `src/file.c:197-200` in the pinned clone `0a8e212`
(`if (errno == EMFILE) { exit(EXIT_FAILURE); }`), verified by reading the file. That is a hard exit on
descriptor exhaustion, and it is the same call the last bullet below makes for the same reason. Saying
"upstream does no errno branching" would be both false and gratuitous: on this specific point upstream
got it right first, and the design agrees with it.

- **SIGINT/SIGTERM.** v1.0 ships no signal handler. SIGINT terminates the process; `rename(2)` is a single
  syscall and is not interrupted mid-flight; the journal writes and fsyncs its `intent` record before the rename, so an interrupted batch leaves at most one item whose outcome is unknown, which is
  precisely what the `kill -9` protocol already reports and what `undo --last` already reverts. A clean
  between-items summary line on Ctrl-C is cosmetic. The upgrade path, named with its cost: spend the
  11th budget slot on `signal-hook` if a real user asks. The safety claim is unaffected, and the test
  that proves it — `kill -9` mid-batch — is strictly harsher than the case a handler would cover. (Note:
  std exposes no signal-handler API at all, and `rustix::kernel_sigaction` is `pub unsafe fn` and
  Linux-only, so any handler costs a crate dependency.)
- **I/O failure taxonomy.** The one hand-written error enum (§7.2) distinguishes at least
  `AlreadyExists`, `PermissionDenied` (`EACCES`/`EPERM`), `ReadOnlyFilesystem` (`EROFS`),
  `NoSpace` (`ENOSPC`/`EDQUOT`), `NameTooLong` (`ENAMETOOLONG`, which after §3.10 means our detected
  limit was wrong and is worth a loud report), `NotFound` (raced away since the walk), and
  `Unsupported` (the flag demotion in §5.4). Every one is a per-item error line naming the errno and
  the path, best-effort continuation, exit 1 at the end (§2.4). `EROFS` and `ENOSPC` are the two that
  will fail every remaining item, so the first occurrence of either aborts the rest of the batch with
  one message instead of printing 200k identical lines.
- **The journal is itself I/O and can itself fail.** If the `intent` record cannot be written or
  fsynced, the rename does **not** happen: an unjournaled rename is worse than a skipped one, because
  it is the one thing `undo` cannot reverse. A journal write failure therefore aborts the batch
  immediately, before the rename it was describing. `--no-journal` is the supported way to rename
  without that dependency, and it says so loudly.
- **Walk errors.** An unreadable directory during recursion is non-fatal: it is reported and skipped,
  and the walk continues, matching detox (doc 13 §4.4). `EMFILE`/`ENFILE` aborts the run before any
  rename, exit 1 — detox hard-exits here too, and continuing with an exhausted descriptor table
  produces a silently incomplete snapshot, which is the one thing the two-phase design in §5.1 cannot
  tolerate.
- **Concurrent invocations are an explicit non-goal, not a guarded case.** Two `detoxrs -x` runs over
  overlapping trees are not serialized: there is no lock file. What bounds the damage is already in
  the design — no-clobber renames mean neither run can destroy the other's file, `apply`'s
  `(dev, ino, mtime)` recheck means a saved plan refuses to run against a tree the other run moved,
  and each batch gets its own journal file, so neither undo history is corrupted. What you can get is
  a confusing report and a partially-cleaned tree, which a second run fixes. A lock is deliberately
  not added: it would need to be advisory, on a path we do not own, with a stale-lock story, to
  prevent an outcome that is already non-destructive.

---

## 6. Correctness boundaries

### 6.1 Non-UTF-8 names and `OsStr` discipline

`OsString`/`OsStr` at every boundary. `decode()` (§3.1) is the single place a name becomes text,
and its result carries how that happened. A name that cannot be decoded is `Opaque` and is
skipped with a report, never renamed, never lossily converted, never panicked on. Display of an
undecodable name uses `<hh>` escapes for the invalid bytes so a terminal cannot be driven by a
filename. `rnr` panics on such a name (doc 06 row 6a, from source); repeating that is a
non-negotiable bug in this project.

On Unix, `OsStr` is bytes (`std::os::unix::ffi::OsStrExt`). On Windows, `OsStr` is WTF-8 over
UTF-16 (`OsStrExt::encode_wide` / `OsStringExt::from_wide`); unpaired surrogates are handled as
`Opaque`. There is no `Vec<u8>` filename type in the codebase: the platform types are the
contract.

### 6.2 NFC/NFD and case-insensitive filesystems

- Internal comparison is always NFC. Without this, an NFD name and its NFC spelling look like
  two different destinations and the collision engine misses a real conflict (doc 03 constraint
  1, the git `core.precomposeUnicode` lesson).
- On a case-insensitive volume, comparison for collision purposes is also case-folded, but
  **only on volumes detected as case-insensitive**, and the detection is empirical (create a
  probe entry, or `getattrlist`/`pathconf`), not a per-OS assumption. macOS ships
  case-insensitive APFS by default but case-sensitive APFS exists and doc 06 tested both.
- Case-only renames: §5.4. Doc 03's two-step temp-name requirement is **refuted** by doc 06
  Test 3 and is not implemented; neither is the separate `rename_case_only` method an earlier draft
  of §5.4 called for, because doc 06 row 4f refuted its stated reason.
- Verified APFS behavior we rely on (doc 06 Test 2): an NFC name stays NFC on disk, is findable
  by its NFD spelling, and `O_CREAT|O_EXCL` with the NFD spelling returns `EEXIST`. So on APFS,
  a rename from NFD to NFC is not a no-op (the entry bytes change) but is also not a collision
  with itself. It is the planner's same-inode-respell case (§5.4) for exactly the same reason a case
  change is: detected so it is not _reported_ as a conflict, then renamed through the one ordinary
  no-clobber path. **[UNVERIFIED]**: row 4f measured the case-only variant, not the NFD -> NFC
  variant, so "the kernel resolves the same-inode variant" is measured for case and inferred for
  normalization. Same observed-`EEXIST` fallback covers it either way.

### 6.3 Length limits

See §3.10. Summary of what we assume and what we do not: ext4 255 bytes (assumed, standard),
APFS 255 UTF-16 units (**verified**, doc 06 Test 1, four-way discriminated), NTFS/exFAT
**[UNVERIFIED]** so treated as 255 UTF-16 units with the byte limit also enforced. Windows
MAX_PATH 260 produces a **warning, never a silent truncation**, because we cannot know the
destination's long-path opt-in state (doc 03 constraint 5; confirmed doc 06 row 7d).

### 6.4 Platform tiers

**Tier 1: CI-tested every commit, bugs block release.**

- Linux x86_64 and aarch64, gnu and musl. ext4, tmpfs, and btrfs in the matrix.
- macOS aarch64 and x86_64. Both case-sensitive and case-insensitive APFS, via ephemeral
  `hdiutil` images exactly as doc 06 built them (that transcript is the test plan).

**Best-effort: built and unit-tested in CI, filesystem behavior not guaranteed.**

- Windows x86_64. `--target windows` rules apply automatically. `MoveFileExW` no-clobber path.
  Not tier 1 because the reserved-name behavior we would need to assert is contested (§6.5) and
  no verified NTFS/exFAT length data exists.
- FreeBSD/NetBSD/OpenBSD: compile-checked, community-supported, fallback rename path.

Note a structural win over detox worth stating: detox's Windows story is permanently blocked by
MSYS2 lacking `lstat()`, which the maintainer researched and concluded will likely never change
(#77, doc 02 theme 4, recounted at doc 02's stage 3 as **10 items, 3 external**, superseding the
earlier "~9 portability issues"). Rust's `std::fs::symlink_metadata` exists on
Windows. The single hardest portability constraint in detox's tracker evaporates with the
language change; it is not a thing we have to solve.

Also worth stating: doc 02 theme 4 records that the detox maintainer could not run his own unit
tests on macOS (#69, #116), leaving macOS bugs permanently unverified. A CI matrix that includes
macOS with both APFS variants is therefore not a nice-to-have; it is the fix for a named,
years-long structural weakness of the predecessor.

### 6.5 The Windows reserved-name mess

Doc 06 row 7b corrected doc 03 constraint 3's direction of travel, and **doc 03 has since absorbed
that correction**, so the two now agree and neither is the refuted party: per a CPython core-dev
discussion (`python/cpython#95486`), Windows 11 path normalization **no longer** special-cases a
DOS device name that has an extension, so `con.txt`/`nul.txt` are generally no longer reserved
as a leaf; only the bare name still is. Two other secondary sources assert the old universal
rule. Both documents' conclusion is the same: this is genuinely contested among people who study
Windows internals professionally, **no Windows 11 machine was available in any pass, so neither side
was tested**, and neither behavior may be hard-coded on the strength of the sources alone.

Our decision: **default to the conservative, pre-Windows-11 rule** (treat `CON`, `PRN`, `AUX`,
`NUL`, `COM1-9`, `LPT1-9`, and the superscript-digit variants as reserved with or without an
extension), because a file created on Linux today can be read from a 2012 Windows box or an SMB
share tomorrow, and the cost of being conservative is one underscore. This is logged as an
assumption, not a fact, and §11 lists the live-Windows-11 spike that would settle it.
`--target windows` also applies the illegal-character set `< > : " / \ | ? *` plus controls 0-31
(confirmed against current MS Learn, doc 06 row 7e) and the trailing dot/space strip, which we do
on all platforms anyway (§3.8).

---

## 7. Architecture

### 7.1 Layout

Library plus binary, in one workspace. The library exists because `transform` must be testable
with `proptest` without touching a filesystem, and because that is the split `pathvalidate` gets
right (validate vs sanitize as separate APIs, doc 03 crate table). It is not published as a
general-purpose crate in v1.0; the binary is the product.

```
detoxrs/
  Cargo.toml                     # workspace
  crates/
    detoxrs-core/               # no I/O, no clap, no std::fs
      src/
        lib.rs
        policy.rs                # Policy, Target, CaseMode; serde derives
        decode.rs                # OsStr -> Decoded (Utf8 | Opaque). No encoding tables:
                                 # valid UTF-8 or skip (§3.4). A few lines, not ~40.
        percent.rs               # safe all-or-nothing %XX decode (ours, ~50 lines)
        classes.rs               # delete/separator/keep classification (ours)
        invisible.rs             # generated from UCD; build-time script, data checked in
        scripts.rs               # UCD Script property, same generator; for §3.12's mixed-script warning
        rules.rs                 # [[rule]] application, literal + regex
        pipeline.rs              # the 13 stages, in order, and only here. Each linear stage
                                 # is its own named function; one internal `run_with(input,
                                 # policy, disabled)` composes them — `pipeline::transform` is
                                 # the all-stages-on case. Testability requirement, not style.
        truncate.rs              # grapheme-safe, extension-aware, limit-aware
        reserved.rs              # Windows reserved stems + illegal chars
        plan.rs                  # Plan, PlanItem, Resolution, collision engine
    detoxrs/                    # the binary
      src/
        main.rs
        cli.rs                   # clap derive; one struct, serde-Serialize into Policy
        config.rs                # TOML load + discovery + profile selection (~150 lines, ours)
        walk.rs                  # snapshot walk; skip rules; entry kinds
        fsops.rs                 # RenameOps impls; rustix::fs::renameat_with covers
                                 # Linux + macOS in one cfg-free call. No FFI shim,
                                 # and therefore no fsops/linux.rs or fsops/macos.rs.
        fsops/windows.rs         # MoveFileExW
        fsops/fallback.rs        # check-then-rename
        apply.rs                 # apply loop, fresh symlink_metadata recheck, written against
                                 # &dyn RenameOps and journal-writer trait for Undo round-trip
                                 # property and TOCTOU test, testable from in-memory double
        limits.rs                # per-directory length-limit detection
        journal.rs               # JSONL write, read, replay
        report.rs                # human preview, --json, exit codes
      tests/
        cli/                     # trycmd .toml cases
        snapshots/               # insta
```

### 7.2 Dependency budget

Target: **<= 11 direct dependencies** for a default build, enforced by a CI check that fails on
regression. The reason is not aesthetics: Debian requires every transitive crate to become its own
Debian source package, built with no network (doc 04 §5; doc 07 row 8a). Dependency count is the
packaging cost.

Eleven, counted honestly, because the table below names eleven distinct crates: `serde` and `toml`
are one row but two packages, and `terminal_size` is a maybe. An earlier draft said "<= 10" while
listing eleven, which is the kind of budget a CI check turns into a lie on day one. If
`terminal_size` proves unnecessary (see its row), the real number is ten and the cap comes down with
it. The `libc` -> `rustix` swap in §5.4 is budget-neutral: one direct dependency out, one in, and the
~60-line `unsafe` shim `libc` existed to carry is gone.

There is deliberately **no total-transitive-crate cap in this document.** A draft target of "<= 45
crates in `cargo tree`" was struck because nobody has run `cargo tree` against this exact set:
`clap`'s derive feature alone pulls the `syn`/`quote`/`proc-macro2` chain plus its own ecosystem, and
a number asserted without measuring is not a budget. **[UNVERIFIED]**: the transitive count for the
set below. The first `cargo add` commit measures it and writes the real ceiling into CI; until then
the direct-dependency cap is the only enforced number.

The enforced `just dep-budget` recipe counts `[dependencies]`, `[build-dependencies]` and every `[target.*.dependencies]` table, in the workspace root and each crate, excluding dev-dependencies — because a build- or target-gated crate still becomes a Debian source package, which is what the budget exists to bound.

| Direct dep                                                                 | Why not our own code                                                                                                                                                                                                                                                  |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `clap` (derive)                                                            | 4.6.5, 2026-07-31 (doc 07 row 7a). Arg parsing plus help plus completions plus man is not worth hand-rolling.                                                                                                                                                         |
| `serde` + `toml` (two crates, two budget lines)                            | Config.                                                                                                                                                                                                                                                               |
| `serde_json`                                                               | `--json`, plan files, journal.                                                                                                                                                                                                                                        |
| `unicode-normalization`                                                    | 0.1.25, 2025-10-30 (doc 06 row 5b). UAX #15 is not hand-rollable.                                                                                                                                                                                                     |
| `unicode-segmentation`                                                     | 1.13.3, 2026-06-01 (doc 06 row 5b). Grapheme clusters, mandatory for truncation.                                                                                                                                                                                      |
| `regex`                                                                    | `[[rule]] regex = true` and `--exclude` globs compiled to regex. Already in every distro. RE2-derived, so no backreferences or lookaround: a documented ceiling, same as f2's (doc 03, f2 row), not a bug to fix with `fancy-regex`.                                  |
| `walkdir`                                                                  | Recursive walk.                                                                                                                                                                                                                                                       |
| `rustix` (feature `fs`)                                                    | 1.1.4. `renameat_with` + `RenameFlags::NOREPLACE` is `renameat2` on Linux and `renameatx_np` on macOS from **safe** code (§5.4), plus `statfs` for the §3.10 limit mapping. Replaces `libc` outright: one crate instead of a crate plus a hand-written `unsafe` shim. |
| `deunicode` (feature `ascii`, default on)                                  | 1.6.2, 2025-04-27 (doc 06 row 5b). Transliteration tables.                                                                                                                                                                                                            |
| `terminal_size` or equivalent, only if needed for preview column alignment | Candidate for deletion, and the 11th budget line until deleted; a fixed two-column layout may not need it. Resolve before the first release, not after.                                                                                                               |

Dev-only: `insta`, `trycmd`, `assert_cmd`, `proptest`, `criterion`, `clap_complete`,
`clap_mangen`. All confirmed active (doc 07 row 7a).

**Explicitly rejected, with the validated reason:**

| Rejected                                   | Reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `figment`                                  | Last publish 2024-05-17; a `figment2` fork exists, itself a staleness signal (doc 07 row 7b **refutes** doc 04's "actively maintained"). We need ~150 lines of three-source merge, not a provider framework.                                                                                                                                                                                                                                                                    |
| `config`-rs                                | Active, but pulls parsers for formats we do not support.                                                                                                                                                                                                                                                                                                                                                                                                                        |
| `ignore`                                   | Excellent crate, but `.gitignore` awareness is wrong for this tool: a gitignored file is exactly the kind of junk-named file a user wants cleaned. We want a hardcoded VCS-metadata skip list plus `--exclude`, which is less code and no `crossbeam`/`globset` tree.                                                                                                                                                                                                           |
| `jwalk`                                    | Last release 2022-12-15, over 3.5 years stale (doc 06 row 5d **refines** doc 03's unqualified recommendation). Also unneeded: see parallelism below.                                                                                                                                                                                                                                                                                                                            |
| `sanitize-filename` / `sanitise-file-name` | 0.6.0 truncates at codepoint boundaries, not grapheme clusters, and will split base+combining-mark and ZWJ sequences (doc 06 row 5a, read from source). Its whole job is the part we must get right.                                                                                                                                                                                                                                                                            |
| `unicode_skeleton`, `confusables`          | 2017 and 2023 respectively (doc 06 row 5c). We generate a UTS #39 table from UCD data checked into the repo instead.                                                                                                                                                                                                                                                                                                                                                            |
| `unicode-security`                         | 0.1.2, 2024. Reconsider for the v1.1 confusable work; not needed for v1.0's mixed-script warning.                                                                                                                                                                                                                                                                                                                                                                               |
| `encoding_rs` / `chardetng`                | Both active, both now moot rather than merely overkill: v1.0 does no legacy decoding and no detection at all (§3.4, owner decision). Reconsider only if a measured `--repair-encoding` lands post-1.0, at which point "40 lines of our own table" is no longer the obvious answer and this row should be re-argued rather than reused.                                                                                                                                          |
| `nix`                                      | Does **not** expose the macOS rename flags: a grep of the 0.31.3 tarball for `renamex_np`/`renameatx_np`/`RENAME_EXCL`/`RENAME_SWAP` returns zero hits, and its `renameat2`/`RenameFlags` are gated `#[cfg(all(target_os = "linux", target_env = "gnu"))]` (doc 06 row 4e). `rustix` does expose them, so it is the syscall crate we take (§7.2 table above). Not both.                                                                                                         |
| `libc`                                     | **Struck as a direct dependency.** An earlier draft needed it for a hand-written `renamex_np` shim and a `getattrlist`/`VOL_CAP_INT_RENAME_EXCL` probe; `rustix` covers the rename call from safe code and the probe is dropped in favor of demotion-on-error (§5.4), so nothing is left for `libc` to do that `rustix::fs::statfs` does not. It will still appear transitively under `rustix`, which is a packaging cost we pay either way, not an `unsafe`-audit cost we own. |
| `rayon`, `tokio`                           | Renaming is syscall-bound; doc 04 §4 flags rayon as a questionable fit and explicitly says the claim needs a project-specific benchmark. v1.0 is single-threaded. If a benchmark later shows a win, it will be a small capped worker pool, not a work-stealing pool.                                                                                                                                                                                                            |
| `indicatif`                                | v1.0 prints a plain counting line to stderr for large trees. A progress bar is a dependency for a cosmetic.                                                                                                                                                                                                                                                                                                                                                                     |
| `anyhow` / `thiserror`                     | One hand-written error enum. This is a leaf binary with maybe 15 error variants.                                                                                                                                                                                                                                                                                                                                                                                                |
| `trash`                                    | Nothing is ever deleted.                                                                                                                                                                                                                                                                                                                                                                                                                                                        |

### 7.3 What we implement ourselves, deliberately

Percent-decoding; the character classifier; the invisible/bidi and
UCD Script tables (generated at build time from data files in-tree, never fetched); grapheme-safe
extension-aware truncation; the Windows reserved-name check; config discovery and first-match
selection; the collision engine including chain ordering and cycle refusal (§5.3); the journal. All
of it is either logic we must be able to test exhaustively (P7's real motivation) or a stale-crate
replacement validation told us to avoid. **The rename FFI shims are no longer on this list**: §5.4's
correction means the no-clobber rename is a `rustix` call, not code we write, audit, or own the
`unsafe` for.

Size estimate, split so it is checkable rather than reassuring. **v0.1 (§10): 1200-1800 lines of
non-test Rust**, which is where the earlier single figure came from. **v1.0 including config,
profiles, rules, `--target`, per-directory limit detection, `--stdin`, plan files, and the two
generated tables: 2200-3000 lines.** The parts that historically blow such estimates are named
rather than averaged away: the crash-safe journal with reverse replay and the three-layer collision
engine with deterministic renumbering, chain ordering, and cycle detection each run 300-500 lines on
their own, and `report.rs` (human preview plus `--json` plus verbosity plus color) is another 250-400.
The already-itemized small pieces (`percent.rs` ~50, `config.rs` ~150) are a rounding error against
those. Two line items an earlier draft carried are now **zero**: the macOS FFI shim (~60 lines plus a
capability probe), struck by §5.4's `rustix` correction, and `decode.rs`'s CP1252/Latin-1 tables
(~40 lines), struck by the owner's decision to drop encoding repair. Treat the range as a budget to
be checked at v0.1, not as a prediction — and note it is now a slightly generous one.

---

## 8. Testing strategy

Non-negotiable means: missing or failing blocks a release.

### 8.1 Property tests (`proptest`), against `transform`

`transform` is pure, so all of these are cheap and hold over arbitrary input strings including
astral planes, combining marks, bidi controls, and long runs.

Two scoping rules apply to the whole table and are not negotiable, because without them two of the
properties are false on three-character inputs (§3.14). First, every property whose subject is "the
output name" is quantified over the `Name(_)` branch of `TransformResult` only; the
`Unrepresentable` branch produces no name and so cannot violate a property about names. It is
covered instead by **Totality** below, which is what stops that scoping from being a loophole.
Second, every property is quantified over **resolved** policies (both length fields concrete numbers, never
the CLI's `0 = auto` sentinel: §3.1), because a `proptest` harness has no directory to probe.

| Property                                    | Statement                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Totality**                                | For every input and every resolved policy, `transform` returns either `Name(o)` where `o` satisfies Safety closure and Non-empty, or `Unrepresentable(r)`. It never returns an unsafe name and never panics. This is the property that makes the `Name(_)` scoping above honest rather than a hole.                                                                                                                                                                                                                                                                        |
| **Idempotence**                             | For `Name(o)`, `transform(o) == Name(o)` for every resolved policy. The stage-13 fixed-point loop exists to make this true; non-convergence within the bound is `Unrepresentable(NotConverged)` (§3.14), not a silently non-idempotent output.                                                                                                                                                                                                                                                                                                                             |
| **Safety closure**                          | For `Name(o)`, `o` contains no delete-class character, no separator-class character, no leading `-`, no trailing dot or space, and (unless `--case keep`) is entirely in the requested case.                                                                                                                                                                                                                                                                                                                                                                               |
| **Length bound**                            | For `Name(o)`, `o` satisfies both `max_len_bytes` and `max_len_utf16` for the resolved policy, for every input, including inputs made of astral emoji only.                                                                                                                                                                                                                                                                                                                                                                                                                |
| **No grapheme splitting**                   | Three forms: **(1) Truncation boundary.** `truncate_graphemes`'s output is always a grapheme-cluster prefix of its input, and satisfies both `max_len_bytes` and `max_len_utf16`. **(2) Pipeline level.** With stage 4 masked off, the grapheme cluster count of the output never exceeds that of the input. **(3) ZWJ family emoji.** Stage 4 strips U+200D (zero-width joiner) by design, so a ZWJ family emoji legitimately becomes three separate clusters — the count rises. This case is pinned by a named regression test rather than left as a property exception. |
| **Non-empty**                               | For `Name(o)`, `o` is never `""`, `"."`, or `".."`. The empty/dot cases are exactly what `Unrepresentable` exists to carry instead (§3.14).                                                                                                                                                                                                                                                                                                                                                                                                                                |
| **Dotfile preservation**                    | For `Name(o)`, `x` starts with exactly one `.` implies `o` starts with exactly one `.`, and vice versa.                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| **Decode is total and never re-interprets** | For every byte string `x`, `decode` returns `Utf8` if and only if `x` is valid UTF-8, and `Opaque` otherwise — no third outcome, no panic, and `Utf8(s)` always round-trips to exactly `x`. This is P2 as an executable assertion, and it is the regression test for detox's `café.txt -> cafÃ©.txt` (doc 01 §7, doc 05). With repair dropped (§3.4) the property is stronger than the version it replaces: there is no `Repaired` branch to assert the absence of.                                                                                                        |
| **Stage independence**                      | Disabling stage N changes only what stage N is documented to change: the output with stage N off equals the output of the pipeline with stage N replaced by identity. The substitution seam is an internal `pub(crate)` stage mask over the linear stages; stages 12 and 13 are tested directly rather than through the mask; the property lives in an in-crate `#[cfg(test)]` module so no public API is widened for it. Catches the scope-creep bug detox had, where the UTF-8 filter also did safe-filter work (#40, #86, doc 02 theme 2).                              |

### 8.2 Property tests against `plan`

| Property                    | Statement                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **No collision**            | For any set of entries and any policy, the plan's `Rename` items have pairwise-distinct `(dir, NFC(casefold?(to)))`. This is the executable form of the maintainer's #130 objection.                                                                                                                                                                                                                                   |
| **No pre-existing clobber** | No `Rename` item's `to` equals an entry that exists and is not that item's own `from`. This property covers the plan-time half only, because `plan()` has no I/O; the apply-time recheck and the kernel refusal are covered by the new §8.4 TOCTOU row.                                                                                                                                                                |
| **Order safety**            | Applying the plan in the plan's own order never renames a directory before an item inside it.                                                                                                                                                                                                                                                                                                                          |
| **No sibling chains**       | No `Rename` item's destination equals another `Rename` item's `from` in the same directory. This is the executable form of §5.3's argument that swaps and chains cannot arise from an idempotent `transform`; if it ever fails, Idempotence failed first. Generated inputs must include near-swap pairs (`a_b`/`a-b`, `A.txt`/`a.txt` under `--case lower`), which the Order-safety property does not exercise at all. |
| **Bounded renumbering**     | Renumbering either produces a name inside the length limit or yields `Conflict`, in at most 998 candidate probes per source, for every limit including limits too small for any suffix (§5.3).                                                                                                                                                                                                                         |
| **Determinism**             | Shuffling the input entry list produces an identical plan, including collision numbering. Directly targets the `readdir()`-order dependence in detox (doc 01 §5, scope note).                                                                                                                                                                                                                                          |
| **Undo round-trip**         | Apply plan then undo journal, against an in-memory filesystem model, restores the exact original name set.                                                                                                                                                                                                                                                                                                             |

### 8.3 Snapshot tests (`insta`)

`--help` and `--help-transforms` verbatim (help text is a contract; drift is a bug). The human
preview for a nasty-name corpus. The `--json` output shape. The `-vv` per-stage trace for ~30
canonical inputs, which doubles as the pipeline's documentation and makes any ordering change
show up as a reviewable diff.

The corpus is a checked-in fixture list and must include, at minimum: `café.txt` in NFC and NFD;
a name containing U+202E; one containing U+200B; a 300-byte ASCII name; 128 astral emoji;
`CON.txt`; `.hidden file`; `..weird..name..`; `100%20done.txt`; `100%25 done.txt`; `50%-70%.txt`;
`libstdc++.so`; `a_-_b.mp3`; `Icon\r`; a name that is a lone `-`; a name that is entirely
punctuation; a CP1252 `Bj\xf6rk` byte string; an invalid-UTF-8 lone `\xff`. The last two are retained
deliberately even though repair is gone: they are now the fixtures that assert `Opaque` and a
correctly `<hh>`-escaped display, which is the behavior §3.4 does promise.

**Corpus storage:** the fixture list lives as Rust `b"..."` constants in `crates/detoxrs-core/tests/support/corpus.rs` with a per-entry `disk_constructible_everywhere: bool` flag, never as a checked-in file whose name is the payload. Reason: APFS refuses invalid-UTF-8 names with EILSEQ (errno 92), verified on 2026-07-31 for both `b"bad\xffname.txt"` and `b"Bj\xf6rk - Vespertine.mp3"`.

### 8.4 Filesystem and platform matrix (`assert_cmd` + `trycmd`)

| Case                           | Where                                                                                                                       | Asserts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case-only rename               | macOS case-insensitive APFS image, macOS case-sensitive APFS image, Linux ext4, Linux tmpfs, a case-insensitive Linux mount | Succeeds in one syscall through the ordinary `rename_noreplace` path, and is _not_ misreported as a collision. This makes both refutations permanent: doc 06 Test 3 (no temp-name dance) and doc 06 row 4f (`NOREPLACE` returns `Ok(())`, not `EEXIST`, on a same-inode respell). Assert the syscall's return value, not just the end state, or the regression this guards is invisible.                                                                                                                                                                                                        |
| NFD -> NFC rename              | Both APFS images                                                                                                            | Entry bytes change; no duplicate entry; still one file. Doc 06 Test 2 as a test.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Length limit probe             | ext4, tmpfs, both APFS images                                                                                               | The detected limit matches the empirical binary search from doc 06 Test 1 (255 bytes on ext4; 255 ASCII / 127 astral-emoji on APFS).                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| `RENAME_NOREPLACE` unsupported | A mount where it fails (or an injected failure)                                                                             | Falls back, warns once, still never clobbers.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| Non-UTF-8 name                 | Linux tmpfs (APFS rejects them, per doc 01 §7 and doc 05)                                                                   | **Always** `Opaque`: skipped, reported, left byte-identical on disk, displayed with `<hh>` escapes, never panics, never lossily converted. No CP1252 branch exists to test (§3.4). This is the test rnr fails (doc 06 row 6a).                                                                                                                                                                                                                                                                                                                                                                  |
| Symlink to `../..` under `-r`  | Linux, macOS                                                                                                                | Recursion does not escape the tree. Named for the hazard #23 documented (doc 05 correction #2); not a regression test against upstream, which fixed its own instance in 2.0.0-beta1 (§5.6).                                                                                                                                                                                                                                                                                                                                                                                                     |
| Rename-during-walk             | 5000-entry tree                                                                                                             | Every entry is visited exactly once; no entry visited under both its old and new name.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| TOCTOU collision during apply  | Linux, macOS                                                                                                                | Compute a plan against a snapshot; create a file at one item's destination after the snapshot and before `apply` runs; run `apply`. Assert the affected item is reported as a fresh conflict, the pre-existing file is byte-identical afterwards, the process does not panic, and the exit code is 1.                                                                                                                                                                                                                                                                                           |
| Crash mid-batch                | Kill after N renames                                                                                                        | Journal replay identifies the exact interrupted item; `undo` restores the completed ones.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Huge tree                      | 200k entries                                                                                                                | Completes; memory stays bounded; `criterion` benchmark recorded so a regression is visible. Also the stability test detox never had: doc 02's stage-3 re-sweep of all 140 items found **five** independent external crash reports — **#11, #56, #85, #96, #137** — every one `author_association: NONE`, not one maintainer-filed, the only perfect-external-ratio cluster in the tracker. This supersedes doc 05's "at least three", which doc 02 itself records as an undercount of an undercount. In Rust most of that class is a bug we do not get to have; OOM on a large snapshot is not. |

### 8.5 Fuzzing

One `cargo-fuzz` target over `decode` + `transform` with an arbitrary byte string as the name.
The oracle is the §8.1 property set. This is the cheapest possible insurance against the
crash-bug class that dominated detox's tracker.

---

## 9. Migration and positioning

### 9.1 The name

**Project and crate: `detoxrs`. Installed binary: `detoxrs`, with `dtx` as a shipped short
alias.** Adopted, not merely tolerated: the burden of proof is on rejecting it, and nothing in the
research clears that bar.

What validation actually established about naming (doc 07 rows 9a/9b, doc 04 §5): (a) doc 07 row
9b marks candidate-name availability **UNVERIFIABLE AS WRITTEN**, because doc 04 proposed no
candidate to check, so there is no finding that `detoxrs` contradicts; (b) PyPI already ships
`detoxpy`, "A tool to rename directories/files that contain unsafe characters", a direct
functional and naming collision in this exact problem space, which tells us the `detox`-derived
namespace is _populated_, not that it is unusable, and that a same-family name will be read as
lineage rather than as squatting; (c) short generic English words are routinely pre-squatted on
crates.io, evidenced by the bare `dist` crate being an unrelated 2016 v0.0.0 placeholder that
blocked axo.dev's own rebrand, so a compound like `detoxrs` is _lower_ risk than the invented
alternatives; (d) no trademark search was ever run in any pass, and a web search is not clearance
(doc 04 §5) so the wellness-industry "detox" branding remains an unresolved search-visibility and
clearance question for any `detox`-derived name, `detoxrs` included.

Command-name collisions: `detoxrs` shadows nothing. Critically, the binary must **not** be named
`detox`, which is packaged in Debian 11-14, Fedora 38-44 + Rawhide, Arch, and nixpkgs (doc 07 row
8b) and is **Homebrew-deprecated as of `deprecation_date: 2026-07-28`, reason `unmaintained`, with
`disable_date: null`** — i.e. no disable date is published. An earlier draft asserted "a hard disable
date of 2027-07-28"; that figure is `brew info`'s _projection_ from Homebrew's usual one-year
deprecate-then-disable convention, not a date Homebrew has committed to, and it should never be cited
as one. Verified directly against the formula JSON. The argument does not need it: a same-named binary
with a preview-by-default posture would silently change behavior under existing scripts, the incident
class doc 04 §6C names, and that is true on any timeline. `dtx` collides with no binary known to this research **[UNVERIFIED]**,
which is part of spike 1.

On the `-rs` suffix: it is dated, and some maintainers now advise against it, but the lineage
signal is worth more here than fashion, because the entire distribution strategy is "the tool
`detox` users should move to" and the name has to survive being typed into a search box by someone
who only knows the old name. On typing cost: `detoxrs` is the package and the canonical name;
`dtx` is installed alongside it as the interactive short form, which is a distribution decision
(two names for one binary), not a second tool. Do not invent a third spelling.

Reject it only if spike 1 finds a hard blocker: the crates.io name taken by an active crate, a
Debian source-package conflict, or trademark exposure. In that order, the fallbacks are
`detoxr`, then `namewash`, then `sanename`, checked by the same criteria.

Counterargument in one sentence: `-rs` names age badly and read as "the Rust port of X" rather
than as a product, which is a ceiling on the project's identity if it outlives detox's relevance.

### 9.2 Relationship to detox

A successor, not a fork, not a drop-in. Positioned as: "detox is **archived** (upstream,
2026-07-12); here is what to use instead." Archived, not merely unmaintained: the repository is
permanently read-only, so there is no upstream to coordinate with, no PR that could ever be
accepted, and no issue that could be filed to settle a question. Every decision below that might
otherwise have deferred to upstream is ours to make and ours to own.

- No `detoxrc` parsing, ever, in any form. This is option 3 from doc 04 §6A. Reason: the config
  grammar _is_ the thing the mandate rejects, and doc 05 records that even detox's own merge
  semantics were never behaviorally verified. Instead: a `MIGRATING-FROM-DETOX.md` table mapping
  every detox filter to its `detoxrs` equivalent, plus a `detoxrs --explain-detox <sequence>`
  helper that reads a `detoxrc` **read-only** and prints the closest flag set, refusing to write
  anything. That is a docs feature with a shell, not a parser we have to maintain. Archival is what
  makes both finite: detox's filter set, config grammar, and CLI surface are frozen at v3.0.1 and
  will never grow another filter, so the mapping table is a one-time write, not a maintenance
  treadmill.
- Two migration notes the mapping table must carry, because they are behavior differences a
  detox user will otherwise hit blind. (a) detox's `-r` only gates descent _past_ the first level
  of a named directory: the immediate children of a directory argument are processed with or
  without `-r` (doc 10, `-r`). `detoxrs` does not copy that: without `-r` a directory argument has
  only its own basename cleaned, and nothing inside it is touched. (b) detox's `.tbl` grammar has
  locale-conditional filter blocks (`start "<lang>" ... end`, activated by `setlocale(LC_CTYPE,"")`,
  doc 11 §4, doc 12 §4, doc 13 §3). `detoxrs` has **no** locale-conditional behavior anywhere: the
  same input produces the same output under every locale. That is a deliberate drop, not an
  oversight — a rename whose result depends on the ambient environment cannot be previewed
  honestly, and P1 forbids the table mechanism that carried it.
- No flag aliasing (doc 04 §6B option 2). `-n` happens to mean the same thing in both, which is
  a coincidence we will accept but not extend. `-r` also matches. `-v` matches. Nothing else.
  Notably `-f` is _not_ accepted, because it means "config file" in detox and "force" in rnr, and
  a silent misinterpretation there is a data-loss shape.
- We never take the `detox` package name in any distro (doc 04 §6C option 2). A same-name
  replacement whose default is preview-instead-of-rename would silently change behavior under
  existing scripts, which is the incident class doc 04 §6C names. Whether to add
  `Provides:`/`Conflicts:` metadata is left to individual distro maintainers, who understand
  their own users' expectations better than we do.
- `inline-detox` has a genuine use (clean filenames from a text stream, and #90 confirms the
  demand). We cover it as `detoxrs --stdin` reading names one per line and writing cleaned names
  to stdout, with no filesystem access at all. Not a second binary.

### 9.3 Relationship to f2 and rnr

Not competitors on the same axis, and the README will say so plainly.

- **f2** (Go, v2.2.2 2025-11-10, ~2.4k stars, actively maintained, doc 06 row 6b, doc 07 row 1a)
  is a general-purpose batch renamer: you supply the find/replace, it supplies EXIF/ID3/hash
  variables, counters, CSV batch mode. `detoxrs` supplies the _policy_ and you supply nothing.
  If you know what you want the names to be, use f2. If you want the names to stop being a
  problem, use `detoxrs`. We copy f2's dry-run default (§2.1) and explicitly reject its undo
  storage location (§5.5, doc 07 row 1b).
- **rnr** (Rust, v0.5.1 2025-12-13, active, doc 07 row 1c) is a regex renamer with the same
  "you write the transform" model, and it panics on non-UTF-8 filenames (doc 06 row 6a). We beat
  it on exactly one axis that matters here and say which one.
- **convmv** is not a competitor at all: it is the **other half of the job**, and now that §3.4 drops
  encoding repair entirely it is load-bearing rather than incidental. The division of labor, which the
  README and `MIGRATING-FROM-DETOX.md` must both state plainly so a user with a mis-encoded tree is
  never left stuck:

  > **`convmv` fixes the encoding. `detoxrs` fixes the name.**

  `convmv -f cp1252 -t utf8 --notest -r ./tree` turns byte sequences that are not valid UTF-8 into
  names that are; `detoxrs -x -r ./tree` then cleans them. Run in that order, the tree that `detoxrs`
  reports as skipped becomes a tree it can clean. This is not a workaround for a missing feature — it is
  the correct factoring: `convmv` does encoding conversion with heuristic detection and interactive
  confirmation (doc 03 table), which is a genuinely different and genuinely harder problem, and one it
  has been doing since long before this project existed. Duplicating it badly, on by default, is what
  the owner's 2026-07-31 decision declined to do.

  Note also `convmv`'s own default, quoted in §2.1: it prints what it would do unless given
  `--notest`. So the two-command pipeline is preview-by-default end to end, which is a coherence
  argument for §2.1 as much as a migration note.

### 9.4 Packaging path

Order matters, because doc 07 row 8b found the bar is eroding rather than static, and archival
makes the erosion one-way: detox is in Debian 11-14, Fedora 38-44 + Rawhide, Arch, and nixpkgs
(now primary-confirmed per distro rather than resting on the search-summarized Repology snapshot; only
an _aggregate count_ would still need Repology, §11 spike 10), _and_ its Homebrew formula is
**deprecated (`deprecation_date: 2026-07-28`, reason `unmaintained`) with `disable_date: null`.**

**The urgency argument must not rest on a disable date, because there is not one.** The "2027-07-28
hard disable date" an earlier draft used is `brew info`'s projection from Homebrew's usual one-year
convention, not a published commitment — verified directly against the formula JSON, where
`disable_date` is `null`. Homebrew may disable it earlier, later, or leave it deprecated
indefinitely. Nothing in the packaging plan needs the date: because upstream is archived, the footprint
is frozen at v3.0.1 forever and can only be dropped by each distro, never refreshed. That is the
one-directional part, and it is a fact about archival rather than a countdown — every distro that
removes detox is a set of users with nowhere to land, and there will never be a competing upstream
release to displace. Deadline-shaped arguments built on a projected date are exactly the kind that turn
into a correction later; this one is deleted rather than rephrased.

1. **GitHub Releases with prebuilt static binaries.** This section chose `cargo-dist` (now branded
   `dist`, though `cargo-dist` remains the installable crates.io name because the bare `dist` name is
   squatted: doc 07 row 7c) plus `release-plz` (0.3.160, 2026-07-14, very active, doc 07 row 7d).
   `x86_64`/`aarch64` for linux-musl, linux-gnu, macOS, and Windows.

   **What actually shipped is `release-please`** (`release-please-config.json`,
   `.release-please-manifest.json`, and the release workflow in this repository), not
   `cargo-dist` + `release-plz`. That is a real divergence between this document and the repository,
   recorded here rather than left for someone to discover from a diff. It is not obviously wrong —
   `release-please` handles the changelog/tag/release-PR half competently and was already the
   governing guide's prescription — but it is a version-and-changelog tool, not a packaging one, so
   the prebuilt-binary half that was this item's whole point is a hand-assembled ~200-line `build`
   job in `release.yml` rather than something `cargo-dist` generates and maintains. `docs/research/rust-setup-release.md`
   §"The tooling conflict" lays out the trade in full and its recommendation is explicit:
   `release-please` is the **interim** mechanism for the first few releases, and the choice should be
   **revisited before the v1.0 packaging milestone** — either add `cargo-dist` alongside it for the
   artifacts, or migrate to `release-plz` + `cargo-dist` and accept a one-time re-wiring. Deciding now,
   before there is a binary to ship or a distro asking for one, would be deciding without the
   information that makes the choice.

2. **Homebrew tap**, formula pulling the prebuilt binary (no `depends_on "rust"`), targeting
   homebrew-core once the notability bar is met. The motivation is that detox's formula is already
   deprecated (2026-07-28) and `brew` users need somewhere to land — not a race against a disable
   date, which is unpublished (see above).
3. **Nix**: `rustPlatform.buildRustPackage` with `cargoLock.lockFile`, plus a flake.
4. **Arch AUR**: cheap, `makepkg` can build against network-fetched deps.
5. **Debian and Fedora last**, and this is where §7.2's dependency budget pays: every crate is a
   separate Debian source package built offline (doc 04 §5, confirmed). Eleven direct dependencies
   is a tractable debcargo job; forty is not.

**No Snap**, and it is worth declining explicitly because detox ships one. Doc 13 §5.1 records that
detox's snap uses `devmode` confinement and is pinned to a stale tarball — i.e. the precedent is a
package that gave up on the confinement model that is Snap's whole point, for a tool whose job is to
rename arbitrary files anywhere the user can reach. A strictly confined snap could not do that job,
and a `devmode` snap is a tarball with extra steps. Skipped, not forgotten.

MSRV: rolling, "stable at least 6 months old," declared via `rust-version`, checked with
`cargo-msrv` in CI and re-checked after every dependency bump (doc 04 §5).

Messages are **English-only**, like upstream (detox has no gettext layer and no localized strings at
all: doc 13 §3, §8). Not revisited for v1.0, and stated rather than left silent because §8.3 pins
`--help` as a snapshot-tested contract, which any future localization would have to plan around.

---

## 10. Roadmap

### v0.1: MVP (the walking skeleton, Linux + macOS)

Scope: `detoxrs [-r] [-x] <paths>` with the default pipeline (stages 1, 3, 4, 7, 9, 10, 12, 13),
the snapshot walk, the collision engine with `number`/`skip`/`fail`, `rustix` no-clobber rename
(`renameat2`/`renameatx_np`) plus fallback, the JSONL journal and `undo`, human preview, `--json`,
exit codes.
No config file. No profiles. No rules. No transliteration.

The MVP boundary is drawn at "safety architecture complete, customization absent," because
§5 is the part that is hard to retrofit and §4 is the part that is trivial to add.

That stage list is a **strict subset of the on-by-default pipeline**, and the two on-by-default
behaviors it defers are named rather than left for someone to discover from a diff:

- **Stage 2 (`url_decode`) is on by default in §3.2 but absent from v0.1.** So v0.1's output for
  `invoice%20final.pdf` is `invoice%20final.pdf`, not `invoice_final.pdf`, and the §2.2 worked
  example is a v0.2 output. Acceptable for a walking skeleton because a surviving `%20` is ugly, not
  unsafe, and because the all-or-nothing validation rule (§3.11) is the fiddly part and deserves its
  own release. Not acceptable silently, which is why it is here.
- **Stage 11 (`target`) is absent, and this one costs nothing observable**, because stage 11 is
  identity under the default `--target unix` (§3.2 row 11: the reserved-stem and illegal-character
  checks fire only under `windows`/`portable`). The globally-applied piece of Windows defensiveness
  is the trailing dot/space strip, and that lives in stage 10, which v0.1 ships. Correspondingly,
  **v0.1's stage 13 fixed-point loop re-runs 9/10 only**; stage 11 joins the loop in v0.3 when
  `--target` arrives. §3.2's "re-run 9/10/11" describes the v1.0 pipeline. This is the deliberate
  reading of §6.5's conservative reserved-name default: it is a `--target`-gated rule, not an
  always-on global one, and §6.5 should be read that way.
- **No signal handler.** v0.1 ships no SIGINT/SIGTERM handler; Ctrl-C terminates the process (§5.8).
- **Hardcoded length limits: `bytes = 255, utf16 = 255`.** Per-directory detection via `statfs` and
  the field mapping (§3.10) arrives in v0.3; v0.1 uses the conservative intersection for all volumes.

### v0.2

Config file (§4.2), discovery and precedence (§4.3), `--print-config`, `[profile.*]`, `[[rule]]`,
`--keep`/`--strip`, `--case`, `--ascii`, stage 2 (`url_decode`), stage 6.

### v0.3

`--target windows`/`portable`;
`--plan-out`/`apply`; `--stdin`; per-directory length-limit detection; `clap_complete` and
`clap_mangen` output.

### v1.0

Windows best-effort tier building and unit-tested in CI; the full test matrix in §8 green on
both APFS variants and three Linux filesystems; documentation including
`MIGRATING-FROM-DETOX.md` and `--help-transforms`; packaging items 1-4 from §9.4; fuzz target
running in CI.

### Deliberately out of v1.0

| Out                                                 | Why                                                                            |
| --------------------------------------------------- | ------------------------------------------------------------------------------ |
| `--edit` ($EDITOR plan buffer, qmv model)           | v1.1. Better than a y/n prompt and worth waiting for (§2.3).                   |
| Any interactive prompt                              | Same.                                                                          |
| Confusable/skeleton collision warnings              | v1.1, and only with our own UTS #39 table (§3.12, doc 06 row 5c).              |
| Full-width/halfwidth folding (#140)                 | v1.1 opt-in stage. Real demand, but NFKC is the wrong hammer (§3.5).           |
| Content-derived names (EXIF, ID3, hashes, dates)    | Never. That is f2's job and doing it would make this a general renamer (§9.3). |
| Moving files between directories                    | Never. §5.2 is a safety property, not a missing feature.                       |
| Parallelism                                         | Until a `criterion` benchmark justifies it (doc 04 §4).                        |
| `.gitignore` awareness                              | Wrong semantics for this tool (§7.2).                                          |
| Overwrite-on-collision                              | Never. P3.                                                                     |
| `detoxrc` parsing                                   | Never. §9.2.                                                                   |
| Debian/Fedora native packages                       | Post-1.0, once the dep tree is stable (§9.4).                                  |
| Windows as tier 1                                   | Blocked on §11 spikes 3 and 4.                                                 |
| Library API stability guarantees for `detoxrs-core` | Post-1.0. The binary is the product.                                           |

---

## 11. Open questions and required spikes

Each one is a decision the research could not responsibly settle. Each names what evidence closes
it.

Gating, labelled by what each actually blocks rather than by a blanket "1-4 gate v1.0" that the
spikes' own text contradicts:

| Gate                                                                                   | Spikes       |
| -------------------------------------------------------------------------------------- | ------------ |
| Any public commit (the name is in every path and manifest)                             | 1            |
| v0.1 Tier-1 correctness (the default pipeline and the rename path v0.1 actually ships) | 2, 13, 14    |
| v1.0, Windows best-effort tier only                                                    | 3, 4         |
| v1.0, everything else                                                                  | 5, 7, 8, 11  |
| Nothing; informational, moot, or post-1.0                                              | 6, 9, 10, 12 |

Spike 2 in particular is mislabelled if called v1.0-only: v0.1 ships no-clobber renaming plus runtime
demotion, so how often the fallback is the _normal_ path is a v0.1 correctness question. Spikes 13 and
14 join it there for the same reason: they are about the rename call v0.1 makes. **Spike 2 is now
closeable** — the owner has Linux hardware (`docs/owner-decisions.md`, 2026-07-31), so this is a
matrix to run, not a gap to live with. **Spike 6 is moot**: the subsystem it gated was dropped by owner
decision, and it is retained only as the specification for a post-1.0 measurement. **Spikes 3 and 4
stay open and are not closeable**: there is no Windows machine and no NTFS or exFAT volume, so Windows
remains a best-effort tier and **no verified Windows filesystem behavior may be claimed anywhere in
this project's documentation.**

**1. Is `detoxrs` actually available?** (blocks any public commit)
Doc 07 row 9b: availability was **UNVERIFIABLE** because no candidate existed; doc 07 row 9a:
`detoxpy` shows this problem space already has naming collisions; doc 04 §5: no trademark search
was ever run and short generic names are routinely squatted on crates.io.
_Closes with:_ `cargo search`/crates.io API for `detoxrs`, GitHub org+repo check, Debian
`apt-cache search` and the source-package namespace, a USPTO TESS (or jurisdiction-equivalent)
search, and a plain web search for brand collision. Also check `dtx` as a binary name against
Debian's `apt-file`/`command-not-found` index, Homebrew, and the `busybox`/coreutils name space.
Fallbacks, in order: `detoxr`, `namewash`, `sanename`. Note the trademark question applies with
extra force to a `detox`-derived name given the unrelated wellness-industry branding (doc 04 §5).

**2. `renameat2(RENAME_NOREPLACE)` behavior in the wild.** (blocks v0.1 Tier-1 correctness; **now
closeable** — the owner has Linux hardware, `docs/owner-decisions.md` 2026-07-31, so this is a matrix
to run rather than an open question to carry)
Doc 06 row 4c marks the Linux syscall and doc 03's filesystem support matrix **UNVERIFIED**: no
Linux machine existed in any research pass. Doc 06's Load-Bearing Uncertainties repeats this. Run it
through `rustix::fs::renameat_with` (§5.4) so the thing measured is the thing shipped.
_Closes with:_ a matrix run on real Linux: ext4, xfs, btrfs, tmpfs, overlayfs, NFSv4, CIFS, ZFS,
and an old-kernel container. Record the exact errno per filesystem. The design already assumes
runtime demotion on `EINVAL`/`ENOSYS`/`EOPNOTSUPP`; the spike tells us how often that path is
the _normal_ path, which decides whether the fallback deserves more engineering than a warning.

**3. Windows 11 reserved names.** (blocks the Windows tier and `--target windows` defaults)
Doc 06 row 7b **refutes** doc 03's claim and then states the question is contested between two
CPython core devs and two blog/Q&A sources. Doc 06 explicitly says do not hard-code either
behavior without a live test.
_Closes with:_ on real Windows 11 and real Windows 10, attempt to create `con.txt`, `nul.txt`,
`aux.c`, `CON`, `NUL`, and `com1.txt` on NTFS, exFAT, and an SMB share, via both Win32
`CreateFileW` and a `\\?\`-prefixed path. Until then the conservative default (§6.5) stands as a
logged assumption.

**4. NTFS and exFAT component length limits.** (blocks the Windows tier)
Doc 03's numbers are "general reputation"; doc 06's Load-Bearing Uncertainties confirms no such
volume was ever available.
_Closes with:_ the same four-character-class binary search doc 06 ran on APFS (ASCII, precomposed
é, CJK, astral emoji), on a real NTFS volume and a real exFAT SD card. Until then both limits are
enforced simultaneously (§3.10).

**5. Case-only rename on network filesystems.**
Doc 06 correction #1 refutes the temp-name requirement for APFS and asks for a citation for any
filesystem where a direct case-only `rename(2)` genuinely fails.
_Closes with:_ the case-only rename test against SMB (macOS and Linux clients), NFSv3/v4, and an
exFAT card. If any fails with `EEXIST`, the two-step temp-name dance comes back, but as a
per-filesystem fallback triggered by an observed error, never as an unconditional default.

**6. Does the CP1252 repair path work on a real mis-encoded name?** — **MOOT. Closed by the owner
decision of 2026-07-31, not by evidence.** The subsystem this spike gated does not exist: non-UTF-8
names are skipped and reported (§3.4). The spike is retained rather than deleted because it is exactly
the measurement that must be run **before** any post-1.0 `--repair-encoding` ships, and its original
text is the specification for that measurement. It gates nothing in v0.1 or v1.0. Original text
follows.

_(historical)_
Doc 05's Load-Bearing Uncertainties: a genuinely mis-encoded non-UTF-8 filename was **never
materialized or tested on any platform in any research pass**, because APFS rejects invalid UTF-8
at the syscall level. Doc 01 §7 records the same wall. Our entire §3.4 story rests on this.
_Closes with:_ on Linux tmpfs/ext4, create names from a real corpus of CP1252, Latin-1, KOI8-R,
and Shift-JIS bytes; measure how often CP1252 decoding produces the intended string versus
plausible-looking garbage. If the false-positive rate is meaningful, the repair path becomes
opt-in (`--repair-encoding`) rather than default, which would be a change to §3.13.

**7. Should NFC rewriting really be on by default?**
Doc 03 constraint 1 and its rclone lesson (#1472, #4228) frame normalization as a genuine policy
trade-off where normalizing can erase a legitimate distinction; doc 06 Test 2 confirms APFS
preserves whatever you give it, so this rewrite is a real change, not a no-op.
_Closes with:_ running the planner in report-only mode over large real macOS trees (a Photos
library, a git checkout, a Time Machine-adjacent tree) and counting how many NFD-only names exist
and how many NFC rewrites would create a collision. If collisions are non-trivial, the default
becomes "normalize for comparison only, do not rewrite."

**8. Is auto-numbering the right collision default?**
No research source settles this: doc 02 theme 6 establishes only that overwriting is rejected and
that refuse-and-report is what users complained about. Numbering is our inference.
_Closes with:_ user feedback on the v0.1 preview output, plus counting how many conflicts a real
messy tree actually produces. If most real conflicts are duplicate-ish files, `skip` may be the
better default.

**9. Does any parallelism help?**
Doc 04 §4 flags rayon as a questionable fit for I/O-bound work and says explicitly that the claim
needs a project-specific benchmark rather than being treated as settled.
_Closes with:_ a `criterion` benchmark of walk + plan + apply over 200k entries at 1, 2, 4, and 8
capped workers, on NVMe and on spinning rust, on ext4 and APFS. Default stays single-threaded
unless the win exceeds 2x.

**10. Distro packaging reality for detox's current footprint.** — **narrowed.**
The Debian/Fedora/Arch/nixpkgs presence and versions are **no longer** a search-summarized snapshot:
they have since been primary-confirmed through the individual distros' own APIs, which is stronger
evidence than Repology's aggregation would have been. Homebrew likewise, directly (see the note in
§9.4 on `deprecation_date` vs `disable_date`). What remains open is only the aggregate view: Repology's
own fetch still failed in the research passes, so any claim phrased as _"detox is packaged in N
distributions"_ — a count, rather than a list of the distros actually checked — is still unsourced.
_Closes with:_ a direct query to Repology's API from a machine with working DNS, and only if a
README ever wants the aggregate count. Citing the per-distro checks by name needs nothing further.

**11. Do unrepresentable names occur often enough to deserve a placeholder policy?** (blocks v1.0)
§3.14 resolves the design contradiction by skipping a name that reduces to nothing (`***`), which is
the conservative and P3/P4-consistent answer. What it does _not_ settle is whether skipping is the
_useful_ answer. No research source touches this: no tracker issue in doc 02 concerns an
all-punctuation filename, and nothing in docs 10-13 shows how detox behaves on one (detox's own
`safe` filter maps ASCII control characters to `_` rather than deleting, doc 12 §3.1, so it
structurally cannot produce an empty name and never had to answer this question).
_Closes with:_ counting, over large real trees (a Downloads directory, a media library, a scratch
directory, and an extracted archive corpus), how many entries `transform` returns `Unrepresentable`
for, broken down by reason. If the count is effectively zero, skipping is obviously right and the
question closes. If a real class of such names exists (emoji-only names and all-punctuation names are
the plausible candidates), the follow-up decision is a `--on-unrepresentable <skip|placeholder>`
flag, where the placeholder would have to be derived from the input (a hash stub) rather than
invented, so that two such names in one directory do not collide. Do not add the flag before the
count exists.

**12. Is three the right bound for the stage-13 fixed-point loop?**
§3.14 makes non-convergence a first-class outcome (`Unrepresentable(NotConverged)`, skip and report)
rather than an undefined behavior, which removes the safety question. It does not answer the
engineering one: nobody has shown either that three iterations always suffice or that any input
needs more than two. Three interacting fixups (truncation creating a trailing dot, truncation
creating a reserved stem, either re-triggering a collapse) is exactly the shape that oscillates, and
the bound was chosen by taste.
_Closes with:_ the §8.5 fuzz target instrumented to record the iteration count reached, run over the
`--target windows` policy (where stage 11 participates and so the interaction is richest) with tight
`--max-len` values, which is the corner most likely to oscillate. If nothing observed exceeds two,
raise the bound to a comfortable 8 and treat `NotConverged` as a genuine internal-bug signal. If
something legitimately needs more, the loop needs an invariant, not a bigger number. Cheap, and it
turns a taste-driven constant into a measured one.

**13. What does a macOS volume without `VOL_CAP_INT_RENAME_EXCL` actually return?** (blocks v0.1
release specifically, unlike spikes 2 and 14 — a silently-clobbering volume produces no error for
demotion-on-error to observe)
§5.4 drops the `getattrlist` capability probe and relies on demotion-on-error instead, which is the
right trade — a probe is an `unsafe` dependency predicting what the call reports anyway — but it makes
the demotion errno set load-bearing. On a capable APFS volume, `rustix::fs::renameat_with` with
`NOREPLACE` was measured returning `EEXIST`/`Ok(())` correctly (doc 06 rows 4e/4f). Nothing has ever
been measured on an **in**capable volume, so the assumption that it fails with something in the
`EINVAL`/`ENOTSUP`/`ENOSYS` family, rather than silently dropping the flag and clobbering, is
**[UNVERIFIED]**. Silently dropping the flag is the failure mode that matters, because it is
indistinguishable from success.
_Closes with:_ mount a volume format lacking the capability (an old HFS+ image, an SMB share, an
`msdos`/exFAT image via `hdiutil`), attempt a `NOREPLACE` rename onto an occupied name, and record
whether it returns an error or clobbers. If any format clobbers, the probe comes back — as a
narrow, per-format capability check with its own justification, not as the general design.

**14. Linux `RENAME_NOREPLACE` on a case-only rename over a case-insensitive mount.** (blocks v0.1
Tier-1 correctness)
Doc 06 row 4f measured this on APFS and got `Ok(())`, which is what let §5.4 delete
`rename_case_only`. Row 4f explicitly leaves the Linux side unmeasured, and Linux has case-insensitive
mounts too: `ext4` with the casefold feature, `vfat`, `exfat`, `ntfs3`, and CIFS. If any of them
returns `EEXIST` for a same-inode respell, the observed-error fallback in §5.4 is what catches it —
so this spike does not change the design, it tells us whether the fallback is a rare path or the
normal one on those mounts.
_Closes with:_ on the owner's Linux box, create `Case.txt` on each of ext4 (casefold on and off),
vfat, exfat, and tmpfs, rename to `case.txt` with `RenameFlags::NOREPLACE`, and record the return
value and the resulting inode. Run the same for an NFD -> NFC respell, which row 4f did not cover on
any platform.

**15. `nlink > 1` respell (M1 WP5 as an ordinary CI row — closeable and scheduled)**
Create a file with two hardlinks in the same directory and in different directories via `std::fs::hard_link`; run a case-only respell, an NFD -> NFC respell, and an ordinary rename against each; confirm the other link is untouched and the inode is stable, on ext4 and both APFS variants. No exotic hardware needed. This is no longer an open assumption: it is a schedulable test row.

---

## Appendix A: traceability of the biggest calls

| Decision                                                                                     | Primary evidence                                                                                                                                                                             | Validation effect                                                                                                                                                                                                                                                                                              |
| -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fixed pipeline, no sequences or `.tbl`                                                       | doc 02 theme 1 (**17 items, 12 external** — recounted at doc 02's stage 3, superseding "~15"; highest weight on every measure); mandate                                                      | doc 05 confirms the #124 "many requests of this nature" quote verbatim                                                                                                                                                                                                                                         |
| **No legacy decode at all**: valid UTF-8 or skip                                             | doc 01 §7 (`café.txt -> cafÃ©.txt`); doc 05 reproduced it verbatim; doc 05 Load-Bearing Uncertainties (the repair path was never once exercised, because APFS refuses to create such a name) | **Owner decision 2026-07-31**, overriding this document's prior "repair on invalid UTF-8" design. `Decoded` is now `Utf8 \| Opaque`; the `Repaired` invariant becomes "always `Opaque`". `OsStr` discipline (§6.1) retained in full                                                                            |
| Transliteration off by default                                                               | doc 02 theme 5 (#47/#53 -> #99 -> #21 -> #112/#113)                                                                                                                                          | doc 05 corrects doc 01: no legacy `safe.tbl`; transliteration lived in `unicode.tbl` only                                                                                                                                                                                                                      |
| Dry-run default                                                                              | doc 04 §1 (f2, rnr)                                                                                                                                                                          | doc 07 rows 1a/1c confirm both; row 8c confirms detox does _not_                                                                                                                                                                                                                                               |
| No overwrite, ever                                                                           | doc 02 theme 6 (#130, #122, #124)                                                                                                                                                            | doc 05 confirms the #130 rejection verbatim, all four technical points                                                                                                                                                                                                                                         |
| Snapshot walk, apply deepest-first                                                           | doc 03 constraint 11; doc 01 §6                                                                                                                                                              | n/a                                                                                                                                                                                                                                                                                                            |
| No temp-name dance for case-only renames, **and no separate `rename_case_only` path at all** | doc 03 constraint 2 said a temp-name dance was needed; this document then invented a same-inode `EEXIST` reason for a second method                                                          | doc 06 Test 3 **refutes** doc 03 (plain `rename(2)` works); doc 06 row 4f then **refutes this document**: `NOREPLACE` on a case-only rename returns `Ok(())`, control `EEXIST`. Both refutations applied; the method is deleted (§5.4)                                                                         |
| **No** hand-written macOS FFI shim; `rustix` for both platforms; `libc` struck               | doc 03 constraint 10 `[CORRECTED]` names `rustix::fs::renameat_with` explicitly                                                                                                              | doc 06 row 4e **withdraws its own earlier refutation**: `rustix` 1.1.4 _does_ expose `renameatx_np` via `renameat_with`/`RenameFlags` under `#[cfg(apple)]`, verified from the crate tarball and by running a `forbid(unsafe_code)` program on APFS. `nix` does not. Both crates now `#![forbid(unsafe_code)]` |
| Grapheme-safe truncation, own implementation                                                 | doc 03 constraint 7                                                                                                                                                                          | doc 06 row 5a: `sanitize-filename` splits clusters (from source)                                                                                                                                                                                                                                               |
| APFS limit = 255 UTF-16 units                                                                | doc 03 constraint 7 (2-point test)                                                                                                                                                           | doc 06 Test 1 confirms with a 4-way discriminated test; we use the refined numbers                                                                                                                                                                                                                             |
| Journal in XDG_STATE_HOME, not temp                                                          | doc 04 §2                                                                                                                                                                                    | doc 07 row 1b: f2 uses `os.TempDir()`; explicitly a cautionary example, not a precedent                                                                                                                                                                                                                        |
| No `figment`, no `jwalk`, no `unicode_skeleton`                                              | doc 03/04 recommended all three                                                                                                                                                              | doc 06 rows 5c/5d and doc 07 row 7b found all three stale                                                                                                                                                                                                                                                      |
| Symlink recursion has no flag                                                                | doc 02 called symlinks a weak theme                                                                                                                                                          | doc 05 correction #2 **refutes** the "weak theme" reading: #23 is a real blast-radius incident. But upstream fixed #23 in 2.0.0-beta1 (verified in clone `0a8e212`), so the argument rests on the hazard, not on a live upstream flaw (§5.6)                                                                   |
| Name is `detoxrs` (binary `detoxrs` + `dtx`), never the bare `detox`                         | doc 04 §5, §6C; user direction                                                                                                                                                               | doc 07 rows 9a/9b: availability unverifiable (no candidate existed to check), `detoxpy` collision, squatting precedent; doc 07 row 8b: `detox` binary name is live in 4 distros                                                                                                                                |

---

## Review record (stage 3)

Three independent reviewers examined this document under different lenses: **L1** source fidelity and
citation audit, **L2** completeness and internal/cross-document consistency, **L3** implementer
reliability. Every finding is adjudicated below. Rejections are included on purpose: two reviewer
recommendations would have made the document worse, and one rested on a misreading of §6.5.

Findings marked **[verified here]** were checked against a primary source during adjudication rather
than taken from a reviewer's summary: the GitHub API for upstream status, the pinned upstream clone
`0a8e212` for `CHANGELOG.md`, `README.md`, `src/file.c`, and `src/clean_string.c`.

| Finding (reviewer)                                                                                                                                                                                                                                               | Verdict                       | Action taken, or reason for rejection                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream is **archived**, never stated; softened to "unmaintained and being wound down" (L1 CRITICAL, L2 CRITICAL)                                                                                                                                               | **ACCEPTED**                  | **[verified here]** GitHub API: `archived: true`, `open_issues_count: 0`, 446 stars, `pushed_at 2026-07-12`. Stated once at the top of the document as the fact everything else relies on, with the "34 issues closed in one ~50-minute administrative sweep, so closed means demand and not rejection" reading made explicit. §9.2 rewritten to "archived (upstream, 2026-07-12)" with the consequence spelled out: no upstream to coordinate with, no PR that could be accepted, no issue that could be filed. §9.4's packaging argument re-derived: the distro footprint is frozen at v3.0.1 and can only be dropped, never refreshed, so the window is one-directional rather than merely dated. §9.2 also now notes archival is what makes `MIGRATING-FROM-DETOX.md` and `--explain-detox` finite deliverables. Re-examined §9 and §11 for arguments needing a live upstream: none found — §11's spikes are all our own measurements, and spike 8's "user feedback" means our users.                                                                                                                                                                                                           |
| Mandate quote truncated, omitting the README's "So, `detox` is paused" (L1, inside CRITICAL)                                                                                                                                                                     | **ACCEPTED**                  | **[verified here]** README `0a8e212` lines 25-26. Both closing sentences quoted, with the line citation.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| §5.6's symlink policy justified by #23 as a live architectural flaw; doc 10 shows it was fixed in 2.0.0-beta1 (L2 CRITICAL)                                                                                                                                      | **MODIFIED**                  | **[verified here]** `CHANGELOG.md` line 144 under `## [2.0.0-beta1] - 2021-03-05`, Security: "Symlinks that point at directories are no longer followed when `--special` and `-r` are specified together. [#23]"; structurally confirmed at `src/file.c:218-223`, where `lstat` + `S_ISDIR` means `parse_dir` never descends through a symlink; `man/detox.1:109` documents it. The reviewer is right that the citation was stale. **The policy is unchanged** — it is right on its own merits — but the argument is rebuilt: #23 is now cited as a first-person account of the _hazard_ (one relative symlink turning a scoped run into a whole-home-directory run), explicitly not as a live flaw, with the fix version named; #20 (symlink loops, untested) is unaffected by the #23 fix and carries the "nobody has characterized this" weight. §4.4, §8.4, and Appendix A corrected in the same direction; the §8.4 case is relabelled as asserting our construction rather than as a regression test against upstream.                                                                                                                                                                        |
| Stage 13's empty-name fallback ("keep the original name") falsifies §8.1 Safety closure; `***` is a counterexample (L3 CRITICAL)                                                                                                                                 | **ACCEPTED**                  | Traced by hand and confirmed: `***` -> `___` (stage 7) -> `_` (stage 9) -> `` (stage 10) -> fallback `***`, which contains three separator-class characters. A release gate the default pipeline violates on a three-character input is not a gate. Resolved as a design decision in new **§3.14**: `transform` returns `TransformResult::Unrepresentable(ReducesToEmpty                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | ReducesToDotOrDotDot | NotConverged)`and the planner skips the entry unchanged, reported like`Opaque`. No invented placeholder — a placeholder is a taste-driven guess (P4) that would also collide with every other unrepresentable name in the directory. Stage 13's row in §3.2 rewritten accordingly. §8.1 re-scoped: name-properties quantified over `Name(_)`, plus a new **Totality** property so the scoping is not a loophole. The residual design question (do such names occur often enough to earn a placeholder policy?) is §11 question 11, not a decision invented here. |
| §3.7 delete class re-includes stage 4's invisibles, making `--no-invisible-strip` a dead flag and falsifying Stage independence (L3 CRITICAL)                                                                                                                    | **ACCEPTED**                  | Delete class narrowed to control characters only (`Cc` plus DEL and NUL), with the reason stated inline: if it duplicated stage 4's set the flag would be dead and Stage independence false.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Stage 13's 3-iteration bound has no defined non-convergence behavior (L3 CRITICAL)                                                                                                                                                                               | **ACCEPTED**                  | `Unrepresentable(NotConverged)` in §3.14: same skip path, logged at `-v` with intermediate states, treated as a bug report against us. No silent non-idempotent output, no runtime-raised bound, no panic. Whether 3 is the right number is now §11 question 12 with a cheap closing experiment (instrument the fuzz target's iteration count under `--target windows` with tight `--max-len`), rather than a taste-driven constant defended in prose.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `transform` purity vs stage 12's filesystem-detected limit; Length Bound proptest not implementable as specified (L3 CRITICAL)                                                                                                                                   | **ACCEPTED**                  | §3.1 now states that the `Policy` reaching `transform` is always fully resolved (`max_len` concrete, never the CLI's `0 = auto` sentinel) and that resolution is a walk-time concern producing one resolved `Policy` per directory. §8.1 gains a blanket scoping rule: every property is quantified over resolved policies.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| §5.3 renumber-then-truncate has no termination bound and no failure mode (L3 CRITICAL)                                                                                                                                                                           | **ACCEPTED**                  | Bounded exactly like stage 13 and stated: N = 2..999, each candidate truncated to fit, against existing names plus already-allocated destinations; if none fits, the item is an unresolvable `Conflict` routed by `--on-collision`. Never drop the numbering, never exceed the limit, never guess. New §8.2 **Bounded renumbering** property.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §5.1/§8.2 do not address sibling rename cycles or swaps (L3 CRITICAL)                                                                                                                                                                                            | **MODIFIED**                  | The gap was real — the document said nothing — but the reviewer's remedy (cycle detection, temp-name routing, topological ordering) is unnecessary and would have added the one thing §5.4 works hardest to avoid: a rename to a name the user never asked for. Cycles and chains are **structurally impossible** given the document's own non-negotiable Idempotence property: if `f(a) = b` and `f(b) = a` then `f(f(a)) = f(a)` forces `f(b) = b`, so `a = b`; the same argument collapses `f(a) = b, f(b) = c` to `c = b`, meaning the second entry is `Unchanged`, which is an ordinary pre-existing-destination conflict layers 1-2 already handle. Renumbering cannot manufacture one because it only allocates free names. §5.3 now carries the proof, plus the cheap guard the proof deserves: a plan-time assertion that refuses the batch as an internal error if a `Rename` destination ever equals another `Rename`'s `from`. §8.2's property is **No sibling chains** (with near-swap generators), not cycle handling. Rejected the temp-name dance outright: an invented intermediate name is a P3 surprise and would put a state in the journal that no forward plan ever produced. |
| Canonical `--help` omits `--legacy-encoding`, `--stdin`, `--explain-detox`, `--help-transforms` while §8.3 calls help a snapshot-tested contract (L2 CRITICAL)                                                                                                   | **ACCEPTED**                  | All four added to the appropriate `--help` sections, plus the `detoxrs [OPTIONS] --stdin` usage line.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| §2.2's `..bad  name...txt -> .bad_name.txt` is not producible: `.` is Keep-class so stage 9 never collapses the interior dots (L3 MAJOR)                                                                                                                         | **MODIFIED**                  | The reviewer traced correctly, but the defect was in the spec, not the example — collapsing repeated `.` is behavior the document wants (and detox had, by a worse mechanism). Fixed at the source: stage 9's collapse set is now stated explicitly as `.`, `-`, `_`, and the configured `--separator`, with the clarification that Keep-class means "not deleted and not substituted", not "exempt from collapsing". Example left as written because it is now correct.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| §3.7 vs §3.8: `-` is Keep-class but `a--b -> a-b` requires collapsing it; the actual collapse rule is never stated (L3 MAJOR)                                                                                                                                    | **ACCEPTED**                  | Same §3.8 rewrite. Also states what does _not_ collapse (`aaa` stays `aaa`) and why a run produced by stage 7 does (`" & " -> "___" -> "_"`), so `_-_` surviving and `a--b` collapsing follow from one rule instead of two examples.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| §2.2's Björk example shows `--ascii` applied to one sibling and not the other, with no flag shown (L3 MAJOR)                                                                                                                                                     | **ACCEPTED**                  | Split into two invocations: the default keeps `ö` for both files, and a second `detoxrs --ascii` invocation shows the opted-in transliteration. Reinforced with one line naming §3.6 and pointing out that `_-_` survives by stage 9's rule.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §2.2's `Icon\r` "skipped (excluded)" implies a built-in default exclude list that is never specified (L3 MAJOR)                                                                                                                                                  | **ACCEPTED**                  | Annotated inline: the example assumes the §4.2 user config, **there is no built-in default exclude list**, the only unconditional skips are `.git`/`.hg`/`.svn` and dotfiles during recursion, and with no config `Icon\r` would become `Icon`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| §7.2's dependency table names 11-12 crates against a CI-enforced "<= 10 direct" cap; the "<= 45 crates in `cargo tree`" figure was never measured (L2 MAJOR, L3 MAJOR)                                                                                           | **ACCEPTED**                  | Cap restated as "<= 11 direct dependencies" with the count done honestly (`serde` and `toml` are one row but two packages; `terminal_size` is the 11th line until deleted, and must be resolved before first release). The transitive-crate figure is **struck** rather than adjusted: nobody has run `cargo tree` against this set, so it is marked **[UNVERIFIED]** with the real ceiling to be written into CI by the first `cargo add` commit. §9.4's "Ten direct dependencies" updated to eleven. A budget a CI check turns into a lie on day one is worse than no budget.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| §10's v0.1 stage list omits on-by-default stage 2 and stage 11 with no note (L2 MAJOR); v0.1 cannot implement stage 13's "re-run 9/10/11" without stage 11 (L3 MAJOR)                                                                                            | **ACCEPTED**                  | §10 now states that v0.1's stage list is a strict subset of the on-by-default pipeline and names both deferrals with their consequences: stage 2's absence means v0.1 leaves `%20` alone (ugly, not unsafe, and the §2.2 example is a v0.2 output); stage 11 is identity under the default `--target unix`, so its absence costs nothing observable, and **v0.1's stage 13 re-runs 9/10 only**, with 11 joining the loop in v0.3.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| §6.5's conservative reserved-name rule is presented as an always-on global safety property that v0.1 does not actually deliver (L2 MAJOR)                                                                                                                        | **REJECTED**                  | Misreads §6.5. The reserved-stem and illegal-character checks are `--target windows`/`portable`-gated in §3.2 row 11, in §6.5's own sentence ("`--target windows` also applies the illegal-character set..."), and in §3.13's opt-in column. The only piece applied on all platforms is the trailing dot/space strip, which lives in stage 10, ships in v0.1, and §6.5 says so. There is no contradiction to fix. The genuine wrinkle the reviewer was circling — stage 13's fixed-point loop naming stage 11 — is L3's finding and is addressed above; §10 now states the `--target`-gated reading explicitly so this misreading is harder to repeat.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| No SIGINT/SIGTERM handling anywhere, despite heavy emphasis on crash resilience (L2 MAJOR)                                                                                                                                                                       | **SUPERSEDED**                | The finding that the gap existed was correct; the remedy is not implementable under the constraints. Amended §5.8 with the v1.0 decision — no handler — and its reason: std exposes no signal API, `rustix::kernel_sigaction` is `pub unsafe fn` and Linux-only, and the `intent`/fsync/rename/`done` protocol already covers the strictly harsher `SIGKILL` case.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| No error taxonomy for `EROFS`/`ENOSPC`/`EACCES`; journal write is itself I/O that can fail (L2 MAJOR)                                                                                                                                                            | **ACCEPTED**                  | §5.8 names the enum variants §7.2 promised but never showed, and answers the sharp version of the question: if the `intent` record cannot be written or fsynced, **the rename does not happen**, because an unjournaled rename is the one thing `undo` cannot reverse. `EROFS`/`ENOSPC` abort the remainder after the first occurrence instead of printing 200k identical lines. `ENAMETOOLONG` is called out as evidence our detected limit was wrong.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Unreadable directory / `EMFILE` during the walk not addressed (L2 coverage gap)                                                                                                                                                                                  | **ACCEPTED**                  | §5.8: unreadable directory is reported and skipped, walk continues (matching detox, doc 13 §4.4); `EMFILE`/`ENFILE` aborts before any rename, because an incomplete snapshot is the one thing the two-phase design in §5.1 cannot tolerate.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| No behavior stated for concurrent `detoxrs -x` runs; journal names are not a lock (L2 MAJOR)                                                                                                                                                                     | **MODIFIED**                  | Recorded as an **explicit non-goal**, and the lock file rejected with a reason. What already bounds the damage is in the design: no-clobber renames, `apply`'s `(dev, ino, mtime)` recheck, one journal file per batch. A lock would have to be advisory, on a path we do not own, with a stale-lock story, to prevent an outcome that is already non-destructive. Stating the non-goal is the fix; building the lock is not.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §7.3's 1200-1800 LOC estimate is thin against its own itemized scope; reviewer proposed 2500-4000+ (L3 MAJOR)                                                                                                                                                    | **MODIFIED**                  | The criticism holds — the estimate read as whole-project while its itemized pieces already consumed a fifth of it — but 4000 is high for a single-binary tool with eleven dependencies. Split into **v0.1: 1200-1800** and **v1.0: 2200-3000**, with the three parts that historically blow such estimates named individually (journal, collision engine, `report.rs`) rather than averaged away, and the range labelled a budget to be checked at v0.1 rather than a prediction.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| §11's "spikes 1-4 gate v1.0" contradicted by the spikes' own annotations (L3 MAJOR)                                                                                                                                                                              | **ACCEPTED**                  | §11 opens with a gating table keyed to what each spike actually blocks: 1 blocks any public commit; 2 and 6 block v0.1 Tier-1 correctness (v0.1 ships the `renameat2` path and stage 1 is on by default); 3 and 4 block only the Windows tier; 5, 7, 8, 11 block v1.0; 9, 10, 12 gate nothing. Spike 2's and spike 6's inline annotations corrected to match.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §3.10 step 1's "<= 4 characters" extension lookback has no unit (L3 MAJOR)                                                                                                                                                                                       | **MODIFIED**                  | **[verified here]** The reviewer guessed codepoints. It is **bytes**: `src/clean_string.c:284-294` in `0a8e212` does `while (--input_walk > filename) { if (extension - input_walk > 5) break; ... }` — pointer arithmetic over `char *`. Stated as "<= 4 bytes of UTF-8" with the source line and the note that for the ASCII segments this rule targets (`.tar`, `.tar.gz`) bytes and codepoints agree, so the choice only shows up on inputs the rule was never aimed at.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §3.10 step 3's whole-name fallback does not restate grapheme safety (L3 MAJOR)                                                                                                                                                                                   | **ACCEPTED**                  | Step 3 now says "same grapheme-cluster boundary algorithm as step 2, just with no extension split", and names the temptation it is closing off (`is_char_boundary`). §8.1's No-grapheme-splitting row says the property is not waived on that path.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `rename_case_only` is misleadingly named; §6.2 also routes NFD->NFC through it (L3 MINOR)                                                                                                                                                                        | **MODIFIED, then SUPERSEDED** | Kept the name and fixed the doc-comment to name both cases and state why the no-clobber flag must not be used on this path. **That "why" was false** — doc 06 row 4f measured `NOREPLACE` returning `Ok(())` on a same-inode respell — so the propagation pass below deleted the method rather than renaming it. This row is not authority for the method's existence.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| §3.12's mixed-script warning needs a UCD Script table not listed in §7.1/§7.3 (L3 MINOR)                                                                                                                                                                         | **ACCEPTED**                  | `scripts.rs` added to §7.1's layout (same build-time generator as `invisible.rs`) and to §7.3's list.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| §3.2 stage 10: "leading separator" ambiguous when a preserved leading `.` comes first (L3 MINOR)                                                                                                                                                                 | **ACCEPTED**                  | Stage 10's row says "including one that immediately follows a preserved leading `.`", and §3.8 gains the worked example: `.!file.txt` -> `.file.txt`, not `._file.txt`. The dot is a dotfile marker, not a shield for what follows it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| §7.2's Debian claim cited doc 07 row 8a as a live re-verification it explicitly was not (L1 MINOR)                                                                                                                                                               | **ACCEPTED**                  | Hedged to doc 07's own confidence level: row 8a upholds the citation but did not re-fetch it, "medium-high confidence, not re-verified live", consistent with well-documented Debian practice and not in real doubt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| P3 leans on #124, whose longer quote doc 05 flags as a synthesis of two comments 37 minutes apart (L1 MINOR)                                                                                                                                                     | **ACCEPTED**                  | Fidelity note added to P3 pointing at doc 05 Corrections Required item 4, stating that only the short fragments quoted here are individually verbatim. The document never block-quoted the synthesized text, so this is a citation-chain courtesy, not a fabrication fix.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| §4.4's not-configurable table omits the `.git`/`.hg`/`.svn` skip (L2 MINOR)                                                                                                                                                                                      | **ACCEPTED**                  | Row added, with the #110 `--git`-rejection reason.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| No statement that xattrs, ACLs, ownership, and mode survive a rename (L2 MINOR)                                                                                                                                                                                  | **ACCEPTED**                  | One paragraph in §5.2: everything attached to the inode is untouched by construction, and it is a `rename(2)`-level guarantee rather than something we implement. Stated so an auditor need not derive it from POSIX.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| No statement on i18n of detoxrs's own messages (L2 MINOR)                                                                                                                                                                                                        | **ACCEPTED**                  | §9.4: English-only like upstream (doc 13 §3, §8), not revisited for v1.0, with the reason it is worth stating (§8.3 pins `--help` as a snapshot contract).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Snap neither adopted nor declined, though detox ships one (L2 MINOR)                                                                                                                                                                                             | **ACCEPTED**                  | Declined explicitly in §9.4 with the doc 13 §5.1 precedent as the reason: a `devmode` snap is a tarball with extra steps, and a strictly confined one could not do this tool's job.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `.tbl` locale-conditional filter blocks never discussed as a dropped capability (L2 coverage gap)                                                                                                                                                                | **ACCEPTED**                  | §9.2 migration bullet: `detoxrs` has no locale-conditional behavior anywhere, same output under every locale, and the reason it is a deliberate drop — a rename whose result depends on the ambient environment cannot be previewed honestly.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| No `detoxrs` analogue for detox's `-L -v` (dump the resolved config) (L2 coverage gap)                                                                                                                                                                           | **ACCEPTED**                  | `--print-config` added to `--help`, justified in §4.3 (four-layer precedence makes "which of these set this value" a real question, and profiles plus rules make the dump _more_ useful than for detox's sequences, not less), and slotted into v0.2 in §10.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| detox's `-r` quirk (first-level children processed with or without `-r`) not noted for migrators (L2 coverage gap)                                                                                                                                               | **ACCEPTED**                  | §9.2 migration bullet states detox's behavior (doc 10) and that `detoxrs` deliberately does not copy it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| L2's remaining coverage rows judged "moot", "adequate", or "acceptable omission" by the reviewer itself (duplicate-sequence merge semantics, `\uXXXX` escapes, `configure.ac` options, `DETOX_SEQUENCE`, `max_length <= 0` coercion, overlong UTF-8, `-?` alias) | **REJECTED** as findings      | No action needed and none taken. Each is either mooted by a rejected mechanism (sequences, `.tbl` files) or already covered by an existing statement (§4.3's environment-variable sentence, §3.7's delete class, Rust's own UTF-8 validation). Documenting a non-difference would add length without adding information.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |

Counts: **31 accepted, 7 modified, 2 rejected** (plus one block of reviewer-acknowledged non-findings
rejected as a group).

The three findings flagged as most serious were each verified against primary source before acting,
and each produced a different kind of change: the archival was a **fact** to correct, #23 was an
**argument** to rebuild while keeping the policy, and stage 13's fallback was a genuine **design
contradiction** that needed a new decision (§3.14) plus one honest open question (§11.11) about the
part the decision does not settle.

---

## Propagation record (stage 3)

The review effort that produced the record above forbade per-document reviewers from editing this
proposal, so corrections accumulated in the research documents and in `docs/owner-decisions.md`
instead. This section is the single pass that applied them, so that no stale claim survives in one
file to be regenerated later from a surviving copy. Authority order used throughout:
`docs/owner-decisions.md` > the stage-3 review records inside each research doc > this document's own
prior text (the full rule is now stated at the top of the document).

**"Verified by me" means I reproduced the finding from a primary source on this machine**, not that I
found a research document asserting it. Where I could not, the row says so.

| Correction                                                                                                                                             | Source                                                                                                                              | Action taken                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | Verified by me                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `rustix` **does** expose macOS `renameatx_np` (`RenameFlags::NOREPLACE` -> `RENAME_EXCL`, `EXCHANGE` -> `RENAME_SWAP`) from safe code; `nix` does not  | doc 06 row 4e (withdrawing its own earlier refutation); doc 03 constraint 10 `[CORRECTED]`                                          | §5.4 rewritten: one `rustix::fs::renameat_with` call for Linux and macOS. Hand-written `libc` FFI shim, its ~60 lines, its tests and its unsafe-audit budget all struck — from §5.4, the §7.1 tree (`fsops/linux.rs` and `fsops/macos.rs` deleted), §7.2's rejection row, §7.3, the §7.3 LOC estimate, §10's v0.1 scope line, and Appendix A                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       | **Yes.** Built a scratch crate with `#![forbid(unsafe_code)]` and `rustix` 1.1.4, compiled and ran it on this APFS volume: `NOREPLACE` onto an occupied name -> `Err(EEXIST, errno 17)`; onto a free name -> `Ok(())`; `EXCHANGE` -> contents swapped. Read the `#[cfg(apple)]` `bitflags!` block at `rustix-1.1.4/src/backend/libc/fs/types.rs:527-544` and the `weak!`-linked `renameatx_np` call at `syscalls.rs:584-624` in the vendored registry copy |
| `libc` is no longer a direct dependency                                                                                                                | Follows from the above; doc 03 constraint 10's implication ("keep `libc` only for the `getattrlist` capability probe and `statfs`") | **Decided: `libc` is struck entirely, and I am saying which.** The `getattrlist`/`VOL_CAP_INT_RENAME_EXCL` probe is dropped, because an unsupported flag is already detected from the error `renameat_with` returns — that is the design's existing Linux demotion path, and a probe would be an `unsafe` dependency predicting what the call reports anyway. `rustix::fs::statfs` covers the remaining `statfs` need. Budget-neutral: one direct dep out, one in, cap unchanged at <= 11                                                                                                                                                                                                                                                                                                                                                                                                                                                          | **Partly.** The `rustix` side is verified as above. The residual risk — that an incapable macOS volume might silently drop the flag instead of erroring — is **not** verified and is now **§11 spike 13**, written as the failure mode that matters because it is indistinguishable from success                                                                                                                                                           |
| Both crates can use `#![forbid(unsafe_code)]`                                                                                                          | Follows from the above; `docs/research/rust-setup-notes.md` stage-3 record, which explicitly deferred the code change to this pass  | `crates/detoxrs/src/main.rs` changed from `deny` to `forbid` and the doc comment rewritten (its stated reason was false). `CONTRIBUTING.md` Code standards and `SECURITY.md` corrected. `docs/research/rust-setup-notes.md` was already right about the facts but described a state that this pass changed, so its header line and its two "still declares `deny`" bullets were updated                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | **Yes.** `just gate` passes with `forbid` in place — real output below. `forbid` broke nothing, which is unsurprising: there is no `unsafe` in the codebase                                                                                                                                                                                                                                                                                                |
| **Drop legacy encoding repair from v1.0.** Non-UTF-8 names are skipped and reported, never repaired                                                    | **Owner decision, 2026-07-31** (`docs/owner-decisions.md`)                                                                          | All 11 enumerated locations applied: `Decoded::Repaired` and `LegacyEncoding` dropped from §3.1; stage-1 `decode` row rewritten to valid-UTF-8-or-skip (§3.2); §3.4 rewritten (title included) with the CP1252 rationale replaced by the reason the subsystem was the highest-risk untested one; `--legacy-encoding` removed from §2.4; the §2.2 `Björk` examples changed from repaired to skipped, with a valid-UTF-8 sibling added so the split reads clearly; "Non-UTF-8 repair (CP1252)" dropped from §3.13's on-by-default column; `decode.rs`'s table comment and `policy.rs`'s `LegacyEncoding` dropped from the §7.1 tree; §8.4's test-matrix row and the Appendix A invariant become always-`Opaque`; §11 spike 6 marked moot; the v0.3 legacy-decode milestone dropped from §10. Also swept: §0, P2's falsifier, §3.3's "1 before everything", §8.1's decode property, §8.3's corpus note, §7.2's `encoding_rs` row, §7.3's LOC estimate | **Yes** for the enumeration — I checked each location rather than trusting the list, and found four further sites the list did not name (§0, P2, §3.3, §8.1's property). The owner's premise is also independently attested: doc 05's Load-Bearing Uncertainties and doc 01 §7 both record that a mis-encoded non-UTF-8 name could never be materialized on APFS                                                                                           |
| `OsStr`-at-the-boundary discipline is **retained**; refusing to repair is not permission to panic, lossily convert, or print raw bytes                 | Owner decision, explicit retention clause                                                                                           | Stated in §3.4 as its own paragraph rather than left implied, cross-referenced from §6.1 and P6, and reinforced in §2.2's `<f6>` note and §8.4's row                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | n/a — a retention, nothing to measure                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Add the `convmv` division of labor to §9.3                                                                                                             | Owner decision's corollary; `convmv` man page                                                                                       | §9.3's `convmv` bullet rewritten from "not a competitor" to "the other half of the job", with the rule in one line — **`convmv` fixes the encoding, `detoxrs` fixes the name** — a worked two-command pipeline, and the note that both halves are preview-by-default                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | **Yes.** Fetched the `convmv` man page and matched the quoted sentence verbatim: "`--notest` Needed to actually rename the files. By default convmv will just print what it wants to do."                                                                                                                                                                                                                                                                  |
| §2.1/P5: `convmv` is a third and more relevant dry-run-by-default precedent                                                                            | Same man page; doc 03 table; `user_feedback_online.md` on the `convmv -> detox -r` pipeline                                         | §2.1 restructured into three bulleted precedents with `convmv` given the weight it earns: same problem domain, same distros, decades older, and its flag is _named_ `--notest`. The counterargument it answers ("nothing established here does this") is named and rebutted rather than left standing. P5 updated to three tools                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | **Yes**, same fetch                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §4.1: drop the named-profile hedge; the precedent is AWS CLI named profiles                                                                            | Review finding                                                                                                                      | §4.1 rewritten: the "weakest-evidenced part of that research" hedge is **withdrawn**, the Cargo `[profile.*]` analogy demoted to a familiarity argument, and AWS CLI named profiles named as the primary precedent with kubectl contexts and gcloud configurations corroborating. Added one deliberate divergence from AWS: no `$DETOXRS_PROFILE`, because an ambient profile is the same hazard class as §9.2's locale-conditional tables                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | **No** — taken on trust as general knowledge about three widely-used CLIs. I did not install or exercise `aws`, `kubectl`, or `gcloud`, and the document does not cite a source for it. Cheap to close if it ever matters                                                                                                                                                                                                                                  |
| §5.4: `RENAME_EXCL` on a case-only rename returns `Ok(())`, so the stated reason for `rename_case_only` is false                                       | doc 06 row 4f `[MEASURED]`                                                                                                          | **Re-derived, and the path is deleted, not merely re-justified.** `rename_case_only` is removed from the `RenameOps` trait; the planner still detects a same-inode respell, but for _reporting_ (so collision layers 1-2 do not flag a respell as a conflict with itself), not for routing. Where the old claim might hold on an unmeasured filesystem it is handled as an **observed** `EEXIST` + same-inode fallback to plain `rename(2)`, which is the same demotion-on-error shape as the Linux flag check. §3.9, §6.2, §8.4 and Appendix A updated; the stage-3 review row that kept the method is marked SUPERSEDED                                                                                                                                                                                                                                                                                                                          | **Yes.** Reproduced independently in the same scratch crate: `foo.txt` -> `FOO.txt` with `NOREPLACE` on this case-insensitive APFS volume returned `Ok(())`, with a control (two distinct files) returning `EEXIST` in the same run, and case-insensitivity confirmed by inode comparison rather than assumed                                                                                                                                              |
| Cite doc 02's **five** external crash reports (#11, #56, #85, #96, #137), not doc 05's "at least three"                                                | doc 02 stage-3 re-sweep, which supersedes doc 05 on re-swept tracker facts                                                          | §8.4's huge-tree row updated to five, with the note that all five are `author_association: NONE` and that this is the only perfect-external-ratio cluster in the tracker                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | **Yes.** Read doc 02 lines 533-543 and 713: the five IDs, the `NONE` association, and doc 02's own statement that L2's "at least three" was an undercount                                                                                                                                                                                                                                                                                                  |
| Homebrew publishes `deprecation_date: 2026-07-28` with `disable_date: null`; "2027-07-28 hard disable date" is `brew info`'s projection                | Review finding                                                                                                                      | Corrected in **both** places that asserted it (§9.1 and §9.4) and in §9.4's packaging item 2. The urgency argument is **re-derived** rather than rephrased: it now rests on archival freezing the footprint at v3.0.1, which is a fact rather than a countdown. Deadline-shaped arguments built on projected dates are the kind that turn into corrections later                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | **Yes.** `brew info --json=v2 detox` on this machine: `deprecated: True`, `deprecation_date: 2026-07-28`, `deprecation_reason: unmaintained`, `disable_date: None`, `disabled: False`                                                                                                                                                                                                                                                                      |
| §9.4: `release-please` is what shipped, not `cargo-dist` + `release-plz`                                                                               | The repository itself; `docs/research/rust-setup-release.md`                                                                        | §9.4 item 1 now records the divergence, why it is not obviously wrong, what it does **not** cover (the prebuilt-binary half is a hand-assembled ~200-line `build` job rather than something `cargo-dist` generates), and the release notes' explicit recommendation to revisit before the v1.0 packaging milestone                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 | **Yes.** `release-please-config.json` and `.release-please-manifest.json` exist; `.github/workflows/release.yml` runs `googleapis/release-please-action`; `docs/research/rust-setup-release.md` §"The tooling conflict" carries the recommendation                                                                                                                                                                                                         |
| Replace the blanket "docs 05/06/07 win on conflict" with the real precedence rule                                                                      | Stated identically at the top of docs 05, 06, and 07                                                                                | The document header now carries the full six-level rule, including `owner-decisions.md` at level 0 and the caveat that a `SUPERSEDED`/`CORRECTED`/`CONTESTED` row is never authority for its original claim — with the note that following the tier instead of the row is exactly how this document came to budget an FFI shim it never needed                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | **Yes.** Read the "Evidence precedence" section at the head of docs 05, 06, and 07 and reproduced it in substance rather than paraphrasing from the brief                                                                                                                                                                                                                                                                                                  |
| doc 03's constraints 2, 3, 4, 7 and 10 were corrected; constraint 11 split into 11a/11b/11c                                                            | doc 03 stage-3 review record                                                                                                        | Audited **every** citation of doc 03 in this document (14 of them). Four were stale in a way that mattered and are fixed: the bare "constraint 11" for the snapshot-walk requirement is now **11c**; the bare "constraint 11" for hardlinks is now **11b**; constraint 3 no longer reads as refuted by doc 06 (doc 03 has since absorbed that correction, so the two agree); constraint 4's citation now reflects that doc 03's own wording already carries doc 06 row 7c's softer mechanism. Constraints 2, 7 and 10's citations were re-read and are correct as they stand                                                                                                                                                                                                                                                                                                                                                                       | **Yes.** Grepped every `doc 03 constraint` citation and read doc 03 lines 150-300 and its review record to check each one, rather than checking only the five the brief named                                                                                                                                                                                                                                                                              |
| §5.8 may cite verified upstream negatives — but must **not** claim "no errno branching"                                                                | docs 10-13; the pinned clone `0a8e212`                                                                                              | §5.8 gains an opening paragraph citing the three verified negatives (no signal handling, no locking primitives, `EROFS`/`ENOSPC` not special-cased) so the section reads as filling named gaps rather than inventing needs — followed by an explicit paragraph stating that upstream **does** branch on `EMFILE` at `src/file.c:197-200`, that this is the one errno that matters most for a tree walk, and that the design agrees with upstream on it                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | **Yes.** In the pinned clone: `if (errno == EMFILE) { exit(EXIT_FAILURE); }` is at `src/file.c:197-200` exactly; `grep` for `signal`/`sigaction`/`flock`/`lockf`/`LOCK_EX` over `src/*.c` and `src/*.h` returns **zero** hits, as does `grep` for `EROFS`/`ENOSPC`/`EDQUOT` over `src/`                                                                                                                                                                    |
| `--print-config` must resolve references or disclose that it does not                                                                                  | Review finding; doc 10 on upstream's `-L`                                                                                           | §4.3 gains three explicit requirements: resolve rather than echo (naming the chosen file and profile in a comment header); **validate everything compilable** — every `[[rule]]` regex and `--exclude` glob is compiled, and an invalid one makes `--print-config` fail with exit 2 rather than exit 0 on a config that cannot run; and name the one genuinely unresolvable key (`max_len = 0`, since detection is per-directory and this command deliberately walks nothing) instead of printing a plausible number. Upstream's `-L` is cited as the worked example of false confidence: it exits 0 on a config the same binary fatally exits 1 on. `--help` carries the caveat                                                                                                                                                                                                                                                                   | **No, and it did not need measuring** — this is a design requirement derived from doc 10's recorded `-L` behavior, not a claim about a system. The `-L` behavior itself I took from doc 10 rather than reproducing, since it needs a deliberately-broken `detoxrc`                                                                                                                                                                                         |
| State explicitly whether `--recursive` replicates upstream's first-level-always-processed behavior                                                     | Three reviewers                                                                                                                     | **Partly rejected as stated, and fixed anyway.** The premise "silence is currently the only answer" is **not right**: §9.2's migration bullet already stated it. But three reviewers missing it is a finding about discoverability, and they were not looking in the migration section. So the statement is now also in **§5.6**, where recursion behavior lives, and compressed into the `-r` line of `--help`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | **Yes.** Located the existing statement in §9.2 before adding the others, which is why this row says "partly rejected" instead of "added"                                                                                                                                                                                                                                                                                                                  |
| §3.7: add defense lines for `[ ]` and `{ }`; cite doc 12 §6 for strict UTF-8 subsuming `_hidden_null_` and overlong handling                           | Review finding; doc 03 constraint 6; doc 12 §6                                                                                      | §3.7 gains a defense for the `[ ]`-separator / `{ }`-keep asymmetry, which turns on `[ ]` being glob metacharacters (a bracket expression can silently match a _different_ file) while `{ }` are a bash/zsh expansion that is not filename globbing at all — with the counterargument named, since media filenames use `[1080p]` constantly. A second paragraph records that strict UTF-8 validation subsumes all three of doc 12 §6's hand-coded hazards (the `_hidden_null_` guard, accepted-and-normalized overlong encodings, and >`0x10FFFF` legacy forms), because none of those inputs is valid UTF-8 and so none ever becomes text                                                                                                                                                                                                                                                                                                         | **Yes** for doc 12 §6 — read it directly, including the `src/clean_utf_8.c:164-167` citation for `_hidden_null_`. The glob-semantics argument is standard POSIX shell behavior, not measured here                                                                                                                                                                                                                                                          |
| Theme 1's verified figure is 17 items / 12 external, not "~15"                                                                                         | doc 02 stage-3 recount                                                                                                              | Corrected in both places (§0 and Appendix A). Also swept the same class of stale count: theme 4's "~9 portability issues" is now doc 02's recounted **10 items, 3 external** (§6.4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                | **Yes.** doc 02 lines 217-220 and 699-703, including doc 02's own note that its downstream-citation fix was left as a flagged action for exactly this pass                                                                                                                                                                                                                                                                                                 |
| If detox's config format is described as YAML anywhere, that is wrong — it is a custom `detoxrc` grammar                                               | Review finding                                                                                                                      | **Nothing to fix.** Grepped the proposal, `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, and every `docs/*.md` for "YAML"/"yaml": every hit is about this project's own `.yml` workflow files or `prettier`'s file globs. No document describes detox's config as YAML. The proposal consistently says `detoxrc`/`.tbl` grammar                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | **Yes** — the check was the work; the result is that the finding does not apply here                                                                                                                                                                                                                                                                                                                                                                       |
| §11 spikes: narrow 10, close 2, moot 6, keep 3 and 4 open, add two                                                                                     | `docs/owner-decisions.md` (test hardware); doc 06 row 4f; doc 07                                                                    | The gating table is rebuilt. Spike **2** marked **closeable** (owner has Linux) and re-pointed at `rustix` so the thing measured is the thing shipped. Spike **6** marked **moot** by owner decision, retained only as the specification for a post-1.0 `--repair-encoding` measurement. Spike **10 narrowed**: per-distro presence is now primary-confirmed, so only an _aggregate count_ still needs Repology. Spikes **3 and 4 stay open and are not closeable**, with the explicit rule that **no verified Windows filesystem behavior may be claimed anywhere**. Three spikes added, not two: **13** (what an incapable macOS volume returns, which the dropped `getattrlist` probe made load-bearing), **14** (Linux `RENAME_NOREPLACE` on a case-only rename over ext4-casefold/vfat/exfat/CIFS), **15** (whether a same-inode respell should refuse when `nlink > 1`)                                                                      | **Yes** for the hardware constraints (read `docs/owner-decisions.md` in full first, as instructed) and for row 4f. Spike 13 is my own addition, not on the brief's list: dropping the probe is what created it, and shipping that drop without recording the gap would have been the same mistake as the original shim                                                                                                                                     |
| §5.6's hardlink argument leaned on "zero issues ever filed"                                                                                            | My own finding while writing spike 15                                                                                               | Added the limit of that argument in place: zero filed issues is evidence of low demand, not evidence of safety, and §5.4 drops detox's `nlink == 1` guard on an untested argument. Pointed at spike 15                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             | **Yes** — doc 02 and doc 05 do both report zero hardlink issues; what I added is that this does not establish what the sentence was being used to establish                                                                                                                                                                                                                                                                                                |
| `just geiger` documented as `cargo geiger -p detoxrs`                                                                                                  | Review finding                                                                                                                      | `CONTRIBUTING.md`'s recipe table now shows the absolute `--manifest-path` form, matching the `justfile`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | **Yes.** Ran `cargo geiger -p detoxrs` in this repo and reproduced the failure: "manifest path `/Users/kerry.hatcher/projects/detoxrs/Cargo.toml` is a virtual manifest, but this command requires running against an actual package in this workspace"                                                                                                                                                                                                    |
| `SECURITY.md` says dependency scanning is not wired up                                                                                                 | Review finding                                                                                                                      | Corrected, and named specifically rather than vaguely: `cargo audit` and `cargo deny check` (policy in `deny.toml`) plus `trivy` and `cargo geiger` in `.github/workflows/security.yml`, and `cargo vet` against `supply-chain/` in `.github/workflows/supply-chain.yml`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | **Yes.** `security.yml` runs `cargo audit` (line 49) and `cargo deny check` (line 51); `supply-chain.yml` runs `cargo vet check` (line 41); `deny.toml` and `supply-chain/{audits,config}.toml` all exist                                                                                                                                                                                                                                                  |
| `release.yml`'s checkout pin is labelled `v4.2.2` but its SHA is `v4.4.0`                                                                              | Review finding                                                                                                                      | **The label is corrected to match the SHA**, in both occurrences (lines 130 and 212). The SHA was **not** changed to match the label, which would have silently downgraded a pinned action                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | **Yes.** `git ls-remote --tags https://github.com/actions/checkout`: `11d5960a326750d5838078e36cf38b85af677262` is `refs/tags/v4.4.0` (and `refs/tags/v4`); `v4.2.2` is `11bd71901bbe5b1630ceea73d27597364c9af683`, a different commit. The five other workflow files already labelled this SHA `v4.4.0`, so `release.yml` was the outlier                                                                                                                 |
| §5.3 collision-numbering tie-break: NFC key alone is not a total order; tie-breaking on raw name bytes was discovered by the Determinism property test | Implementation finding                                                                                                              | The `number` policy bullet rewritten: ties on the NFC key are resolved by sorting on **raw name bytes** (which are distinct by definition for two distinct directory entries), not `readdir()` order. This directly addresses the upstream defect this design exists to fix: `café.txt` (NFC composed) and `cafe\u{301}.txt` (NFD decomposed) share one NFC key and have nothing to order by except input order, so stopping at NFC re-introduces filesystem-order dependence. A named regression test now pins this. Cross-referenced from the Determinism property row in §8.2                                                                                                                                                                                                                                                                                                                                                                   | **Yes.** Constructed a test case with NFC and NFD variants; verified that ties are sorted by raw bytes and produce deterministic, input-order-independent numbering across two runs with shuffled input                                                                                                                                                                                                                                                    |
| §5.3 sibling-chain assertion false-positives on NFD respells; clarify it excludes renames whose destination equals their own NFC source                | Implementation finding                                                                                                              | The assertion logic rewritten: the check excludes renames whose destination equals their own NFC source (an NFD respell case that must not trigger on what is an intra-batch collision in the other entry, not a chain). This was found by checking generated near-swap pairs against the assertion in property-test harness (§8.2). Clarified that genuine sibling swaps remain unconstructible (every changed name's destination is a `transform` fixed point, so the partner comes out `Unchanged`), and that the assertion is retained as a loud error rather than removed                                                                                                                                                                                                                                                                                                                                                                     | **Yes.** Fed a hand-built swap pair and a near-swap pair (NFD respell beside NFC) to the assertion; the genuine swap fired, the respell did not                                                                                                                                                                                                                                                                                                            |
| Three implementation deviations recorded: `Conflict` variants, `--on-collision fail` behavior, and item ordering depth derivation                      | Implementation findings                                                                                                             | **Added to §5.3 as a deliberate decisions block.** (1) The `Conflict` result carries three distinct variants rather than one, because reporting "probe limit exhausted" for a plain two-file collision under `skip` or `number` would be false. (2) `--on-collision fail` returns an error rather than an appliable `Plan`, because refusal is structural; the error carries conflicting items. (3) Item ordering derives depth from directory path (computed during walk) rather than a depth field from the walker, so the Order-safety property cannot be broken by miscounting. Also noted: `plan()` takes volume case-sensitivity as an explicit enum parameter (§6.2), not inferred from the platform                                                                                                                                                                                                                                        | **Yes.** Verified all three in the engine code: `Conflict` has three match arms (line counts and error messages distinct); `fail` returns `Err` at plan time with conflicting items attached; depth derived from path depth and used for sort order; `plan()` signature shows case-sensitivity as `enum` argument                                                                                                                                          |

### Rejected, and why

- **"§11 spike 2 is closeable" is recorded as closeable, not closed.** The owner has Linux hardware,
  which removes the obstacle; it does not run the ext4/xfs/btrfs/tmpfs/overlayfs matrix. Marking a
  spike closed because it _became possible_ to close is the same error as citing a projected disable
  date as a published one. It stays open, relabelled.
- **The `--recursive` finding's premise, partly.** "Silence is currently the only answer" was not
  accurate — §9.2 already stated the behavior. Recorded as a discoverability fix rather than accepting
  a characterization of the document that is not true, while still making the change three reviewers'
  confusion justified.
- **The AWS-CLI/kubectl/gcloud precedent is asserted, not verified.** I strengthened §4.1 as directed
  because the claim is uncontroversial, but this pass added no primary citation for it, and the row
  above says so rather than letting a strengthened argument look better-evidenced than it is. If §4.1
  is ever load-bearing in a public document, one `aws configure list-profiles` and one link to each
  tool's docs closes it.
- **Nothing was done about detox's config format being called YAML**, because nothing in this
  repository calls it that. Reporting the check as a non-finding is more useful than a defensive
  clarification nobody needed.
- **Spike 6 was not deleted, only mooted.** Deleting it would lose the one thing that has to happen
  before a post-1.0 `--repair-encoding` can ship honestly — a false-positive measurement against a real
  corpus — and the owner's decision explicitly leaves that door open.
