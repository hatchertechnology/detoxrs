# detoxrs: a Rust successor to detox

Status: design proposal. Audience: the person who starts writing code Monday.
Inputs: docs 01-04 (primary research), docs 05-07 (adversarial validation). Where they
disagree, 05/06/07 win and are cited as such. Every claim traceable to research is cited
inline as (doc NN, section). Every dependency on something validation could not confirm is
marked **[UNVERIFIED]** and appears again in §11.

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
pick wrong: doc 01 §2.3, §7; doc 02 theme 1 at ~15 issues, the highest-weight theme in the
whole tracker). `detoxrs` ships **one fixed, ordered pipeline** whose stages are individually
switchable but never reorderable, never user-defined, and never table-driven. Encoding is not a
user-facing concept: the pipeline decodes, and the one operation that produced detox's worst
data corruption (re-interpreting bytes as Latin-1) is _structurally impossible_ because legacy
decoding only ever runs on input that is already invalid UTF-8. Customization is a small set of
flags, mirrored 1:1 into a TOML file with named profiles: "pre-select options, not a DSL."

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
_Falsifier: a PR adding `--encoding <x>` for input interpretation. `--legacy-encoding` (which
only names the fallback for already-invalid-UTF-8 bytes) is the sole permitted exception and is
documented as a repair hint, not an input declaration._

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
Evidence: dry-run-by-default is the confirmed convention among both direct competitors, f2 and
rnr (doc 04 §1; re-confirmed independently in doc 07 rows 1a/1c), and detox's own README calls
`--dry-run` "the most important option to learn" (doc 02 theme 7). detox itself does _not_
default to dry-run (confirmed from the local man page, doc 07 row 8c).
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

Two competitors that do exactly this job both default to preview and require an explicit flag to
write: f2 (`-x/--exec`) and rnr (`-f/--force`), independently confirmed twice (doc 04 §1, doc 07
rows 1a/1c). detox does not, and its own README then spends its safety budget telling you to
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

Non-UTF-8 name repaired without the user thinking about encoding.

```
$ detoxrs ./from-a-2004-cdrom
./from-a-2004-cdrom
  Bj<f6>rk - Vespertine.mp3   ->  Björk_-_Vespertine.mp3   (repaired: cp1252 -> UTF-8)
  Bj<f6>rk - Homogenic.mp3    ->  Björk_-_Homogenic.mp3    (repaired: cp1252 -> UTF-8)

$ detoxrs --ascii ./from-a-2004-cdrom     # same names, transliteration opted into
./from-a-2004-cdrom
  Bj<f6>rk - Vespertine.mp3   ->  Bjork_-_Vespertine.mp3   (repaired: cp1252 -> UTF-8, then --ascii)
  Bj<f6>rk - Homogenic.mp3    ->  Bjork_-_Homogenic.mp3    (repaired: cp1252 -> UTF-8, then --ascii)
```

Repair happens by default; transliteration does not (§3.6), which is why the first invocation
keeps `ö`. Note also that `_-_` survives untouched: that is stage 9's same-character rule (§3.8),
not a rule the user had to write.

(`<f6>` is how a byte that is not valid UTF-8 is rendered in the preview; it is never printed
raw, because printing raw invalid bytes is how a terminal gets confused.)

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
    -r, --recursive          Descend into directories
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
        --legacy-encoding <E>  Fallback for bytes that are NOT valid UTF-8, only
                             [cp1252 (default) | latin1 | koi8-r | sjis | none]
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
                             and flags) as TOML, and exit without walking anything
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
    Utf8(String),                                  // input was valid UTF-8
    Repaired { text: String, from: LegacyEncoding },// input was not; decoded as legacy bytes
    Opaque,                                        // undecodable; we refuse to guess
}

pub struct Outcome {
    pub text: String,
    pub stages: Vec<StageDelta>,   // for -vv and for snapshot tests
    pub notes: Vec<Note>,          // Repaired, Truncated, Renumbered, ReservedName, Confusable...
}

pub fn decode(raw: &OsStr, p: &Policy) -> Decoded;
pub fn transform(d: &Decoded, p: &Policy) -> Outcome;   // pure, no I/O, no allocation of paths
```

`transform` is a pure function of `(name, Policy)`. It never sees a path, a directory, a
filesystem, or another file. **The `Policy` reaching `transform` is always fully resolved**: in
particular `max_len` is a concrete number, never the CLI's `0 = auto` sentinel. Resolving `auto`
into a number is a walk-time concern (§3.10) that produces one resolved `Policy` per directory, and
that is the only reason `transform`'s purity and stage 12's filesystem-derived limit can coexist.
`Policy` therefore has two shapes in practice, and only the resolved one is what `transform`,
the snapshot tests, and the §8.1 property tests ever see. Everything that involves other files (collisions, existence,
length limits of _this_ filesystem) lives in the planner (§5). That split is what makes the
property tests in §8 possible.

### 3.2 The default pipeline, in order

| #   | Stage             | Default   | What it does                                                                                                                                                                                                                                                                        |
| --- | ----------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `decode`          | on        | `OsStr` -> text. Valid UTF-8 passes through untouched. Otherwise decode the raw bytes as CP1252 (superset of Latin-1 in the 0x80-0x9F range); if that yields no C1 controls and no replacement chars, emit `Repaired`. Else `Opaque`.                                               |
| 2   | `url_decode`      | on        | `%XX` -> byte, only when every escape in the name is well-formed and the decoded result is valid UTF-8 and contains no `/`, no NUL, and no controls. All-or-nothing per name. `+` -> space is **off**.                                                                              |
| 3   | `normalize`       | NFC       | Unicode normalization of the output name. Comparison inside the planner is _always_ NFC regardless of this setting.                                                                                                                                                                 |
| 4   | `invisible_strip` | on        | Delete bidi controls (U+202A-202E, U+2066-2069, U+200E/200F), zero-width (U+200B/200C/200D/2060/FEFF), Unicode Tags (U+E0000-E007F), and all remaining `Cf`, `Cc`, `Cs`, `Co`.                                                                                                      |
| 5   | `rules`           | none      | User's ordered `[[rule]]` list: literal or regex find/replace, applied in file order, each seeing the previous one's output. The only customization slot.                                                                                                                           |
| 6   | `ascii`           | **off**   | Transliterate to ASCII (`deunicode`). Lossy, opt-in.                                                                                                                                                                                                                                |
| 7   | `safe_map`        | on        | Character classes, not a table: delete-class -> nothing; separator-class -> `--separator`; everything else kept. Sets defined in §3.7.                                                                                                                                              |
| 8   | `case`            | keep      | `lower`/`upper` use Unicode simple case mapping, not ASCII-only.                                                                                                                                                                                                                    |
| 9   | `collapse`        | on        | Collapse a run of the _same repeated character_ to one, for exactly this collapse set: `.`, `-`, `_`, and the configured `--separator`. Never merge runs of _different_ characters. Drop separators adjacent to `.`.                                                                |
| 10  | `trim`            | on        | Strip leading `-`; strip leading/trailing separators (including one that immediately follows a preserved leading `.`); strip trailing dots and spaces; preserve exactly one leading `.` if the original had one.                                                                    |
| 11  | `target`          | unix      | With `--target windows` or `portable`: reserved-stem check, Windows illegal-character check, MAX_PATH warning.                                                                                                                                                                      |
| 12  | `truncate`        | on (auto) | Grapheme-safe, extension-preserving truncation to the filesystem limit, or `--max-len N`.                                                                                                                                                                                           |
| 13  | `finalize`        | on        | Re-run 9/10/11 until fixed point (bounded to 3 iterations). Then: if the result is empty, `.`, or `..`, or if the loop did not converge, `transform` returns `Unrepresentable` and the planner **skips the entry unchanged** (§3.14). It does _not_ fall back to the original text. |

### 3.3 Why that order

- **1 before everything.** Nothing textual is meaningful before decode. Critically, stage 1 only
  attempts legacy decoding on input that _failed_ UTF-8 validation. This makes detox's worst bug
  class unreachable rather than merely discouraged: valid UTF-8 is never re-interpreted as
  Latin-1, so `café.txt -> cafÃ©.txt` (doc 01 §7, doc 05) cannot happen by any flag combination.
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
  (`report.tar.gz` truncated to `report.` ; `CONSOLE.txt` truncated to `CON.txt`). One bounded
  fixed-point pass beats trying to predict the interaction. What happens when the bound is not
  enough is specified, not left to the implementer: see §3.14.

### 3.4 Encoding repair without making the user think about encoding

The rule is one sentence: **if the name is valid UTF-8 we never touch its encoding; if it is
not, we repair it.** No flag, no detection heuristic on valid input, no chardet.

CP1252 is the fallback because it is the encoding behind the overwhelming majority of surviving
mis-encoded Western filenames and because it needs no dependency: it is Latin-1 plus 27 defined
code points in 0x80-0x9F, a ~40-line const table. That is a deliberate rejection of
`encoding_rs`/`chardetng` (doc 03 crate table) on P7 grounds. `--legacy-encoding <cp1252 |
latin1 | koi8-r | sjis | none>` exists for the person whose corpus is not Western;
`none` makes `Opaque` the outcome for any non-UTF-8 name.

`Opaque` names are **skipped, never renamed**, and reported:

```
  <ff><fe>0A9.dat   -   skipped (name is not valid UTF-8 and could not be repaired; --legacy-encoding to try another)
```

Two things we deliberately do not do. (a) Mojibake repair of _valid_ UTF-8 (ftfy-style
`cafÃ©` -> `café`): it requires guessing that a legitimate name is wrong, and it is exactly the
class of "clever" transform that produced the v1 backlash (P4). Out of v1.0. (b) Any claim that
the forward repair case is behaviorally verified: **[UNVERIFIED]**: doc 05, Load-Bearing
Uncertainties, records that a genuinely mis-encoded non-UTF-8 filename could never be
materialized on APFS in any research pass, so this whole path needs a Linux/tmpfs spike (§11).

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
Windows shell do not agree with POSIX about whether they exist (doc 03 constraint 4; doc 06 row
7c refines the mechanism to a shell/UI-layer inconsistency rather than a hard filesystem strip,
which does not change the decision). Preserve exactly one leading `.` so dotfiles stay dotfiles.

"Leading" means leading _after_ the preserved dot, and that needs a worked example because two
implementations would otherwise differ. `.!file.txt`: stage 7 turns `!` into `_`, giving `._file.txt`;
stage 10 preserves the one leading `.` and then strips the separator that immediately follows it,
giving `.file.txt`. Not `._file.txt`. The dot is preserved as a dotfile marker, not as a character
that shields whatever comes after it.

### 3.9 Case handling

`keep` by default. Lowercasing is taste, not safety: #102 asked for a flag, not a default (doc 02
theme 10), and on a case-insensitive volume a mass lowercase is a collision generator. When
requested, use Unicode simple case mapping, not detox's ASCII-only `tolower()` per byte (doc 01
§2.3). Case-only renames are safe to perform in a single `rename(2)`: see §5.4, and note that
doc 06 refuted doc 03's claim that a temp-name dance is needed.

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

We satisfy **both** metrics simultaneously (bytes <= byte limit AND UTF-16 units <= unit limit)
whenever the volume is unknown, which is the conservative intersection and costs nothing. The
limit is detected per-directory, once, from a small probe or a `statfs` type mapping, and is
overridable with `--max-len N` (interpreted in the filesystem's own unit). Note that APFS's
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
   `is_char_boundary`.
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

| On by default                              | Opt-in                                 |
| ------------------------------------------ | -------------------------------------- |
| Non-UTF-8 repair (CP1252)                  | `--ascii` transliteration              |
| Percent-decoding (`%XX`, safe-only)        | `+` -> space                           |
| NFC normalization                          | `--case lower/upper`                   |
| Invisible/bidi/tag stripping               | `--target windows/portable`            |
| Control-character deletion                 | `[[rule]]` custom patterns             |
| Space and shell-metacharacter -> separator | `--keep` / `--strip`                   |
| Same-separator run collapsing              | `--hidden` (dotfiles during recursion) |
| Leading `-`, trailing dot/space trimming   | `--files-only` / `--dirs-only`         |
| Grapheme-safe length truncation            | Full-width folding (v1.1)              |
| Collision detection + renumbering          | Confusable-pair warnings (v1.1)        |
| Undo journal                               | `--edit` (v1.1)                        |

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
list", doc 04 §2, confirmed doc 07 rows 2a/2b) with named profiles as TOML sub-tables, which is
the Cargo `[profile.*]` shape a Rust-CLI audience already knows. Doc 04 §2 is honest that the
named-profile precedent is the weakest-evidenced part of that research and doc 07 row 9b did not
strengthen it; we take it anyway because the mandate names it directly ("a config file that lets
you pre-select those options") and because the alternative (multiple config files, one per
preset) is worse.

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
renames, doc 02 theme 6). Doc 03 constraint 11 states the requirement directly: snapshot the
list, do not rename while iterating.

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
  `IMG_0042.JPG` -> `IMG_0042-2.JPG`. Numbering is allocated deterministically in a stable sort
  order (NFC bytes of the source name), not `readdir()` order, so two runs over the same tree
  produce the same result. If numbering would exceed the length limit, truncate the stem further
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
item that is _also_ a `Rename` in the same directory, that is an internal-consistency bug, and the
planner refuses the entire batch with an internal error rather than renaming anything. Cheaper than a
cycle breaker, and it catches the real failure (a stage that is not idempotent) instead of papering
over it.

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
    fn rename_noreplace(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
    /// Same source inode, different spelling of the same name: a case change (§5.4) OR a
    /// normalization change such as NFD -> NFC (§6.2). Both route here, which is why the
    /// no-clobber flag must NOT be used: the destination "exists" and is the same inode.
    /// Plain rename(2) is correct here. The name says "case_only" for historical reasons;
    /// read it as "same inode, respelled".
    fn rename_case_only(&self, dir: &Path, from: &OsStr, to: &OsStr) -> Result<(), RenameErr>;
}
```

- **Linux**: `renameat2(..., RENAME_NOREPLACE)`. Doc 03 constraint 10 gives the filesystem
  support matrix (ext4 3.15+, btrfs/tmpfs/cifs 3.17+, xfs 4.0+, most by 4.9) but doc 06 row 4c
  marks it **[UNVERIFIED]**: no Linux machine existed in any research pass. So support is
  detected at runtime: `EINVAL`/`ENOSYS`/`EOPNOTSUPP` on first use demotes that mount to the
  fallback path and prints one warning naming the mount.
- **macOS**: `renamex_np(..., RENAME_EXCL)`, gated on the `getattrlist` volume capability
  `VOL_CAP_INT_RENAME_EXCL`. Doc 06 row 4b confirmed the semantics from the local `man
renamex_np` on Darwin 25.5. Critically, doc 06 row 4e **refutes** doc 03's implication that
  `rustix` or `nix` wrap this: neither exposes any macOS `renamex_np` flag today. **Budget real
  time for a hand-written `libc` FFI shim** (~60 lines plus a capability probe plus tests). This
  is a named, validated gap, not a guess.
- **Windows** (best-effort tier): `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, which is
  no-clobber by default. Matches what `rust-lang/libs-team#131` proposes; doc 06 row 4a confirms
  that issue is still open, so `std::fs::rename` gives us nothing here and will not soon.
- **Fallback** (unsupported flag or filesystem): `symlink_metadata(dest)` then `rename`, with the
  TOCTOU window documented in the man page and reported in `--json` as
  `"atomicity": "check-then-rename"`. We do **not** use the `link()`+`unlink()` trick: it fails
  on directories, changes `st_nlink` observably, and is a surprise on filesystems without
  hardlinks.

**Case-only renames get their own method, and this is a trap worth naming.** Doc 06 Test 3
**refutes** doc 03 constraint 2: `rename(2)` renames `CaseTest.txt` to `casetest.txt` directly on
case-insensitive APFS, verified at both the `os.rename` and raw C syscall level, on the boot
volume and on fresh case-sensitive and case-insensitive images. No temp-name dance. But
`RENAME_NOREPLACE`/`RENAME_EXCL` would return `EEXIST` for that same rename, because the
destination "exists": it is the same inode. So: when the planner sees that `to` differs from
`from` only by case, and `symlink_metadata(to)` reports the same `(dev, ino)` as `from`, it
routes to `rename_case_only`, which uses plain `rename(2)`. This mirrors detox's same-inode
escape hatch (doc 01 §5, `st_dev`/`st_ino` match with `st_nlink == 1`, confirmed doc 05) minus
the `nlink == 1` condition, which detox needs only because it has no batch-level plan.
**[UNVERIFIED]**: whether direct case-only rename also works on SMB/NFS mounts. Doc 06 correction
#1 explicitly asks for a citation for any filesystem where it fails. Spike in §11; until then the
fallback on `EEXIST` from `rename_case_only` is to skip and report, never to unlink anything.

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
  so the behavior is not a surprise (doc 03 constraint 11). Doc 02 and doc 05 both confirm zero
  hardlink-related issues were ever filed against detox, so this is a documentation problem, not
  a feature request.
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

- **SIGINT/SIGTERM.** We install a handler that sets a flag; the apply loop checks it between items
  and stops cleanly, writing the summary and the journal's closing state. A rename already in flight
  is not interrupted — `rename(2)` is a single syscall and either happened or did not. Ctrl-C
  therefore leaves a prefix of the plan applied and fully recorded, and `detoxrs undo --last` reverts
  exactly that prefix. An abrupt `SIGKILL` is also safe, but only via the `intent`/fsync/rename/`done`
  protocol: it can leave one item whose outcome is unknown, which replay reports as such rather than
  guessing. The handler exists to turn the common interrupt into the clean case, not because the
  abrupt case is unsafe.
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
  Test 3 and is not implemented.
- Verified APFS behavior we rely on (doc 06 Test 2): an NFC name stays NFC on disk, is findable
  by its NFD spelling, and `O_CREAT|O_EXCL` with the NFD spelling returns `EEXIST`. So on APFS,
  a rename from NFD to NFC is not a no-op (the entry bytes change) but is also not a collision
  with itself, and it goes through `rename_case_only`'s same-inode path for exactly the same
  reason a case change does.

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
(#77, doc 02 theme 4, ~9 portability issues). Rust's `std::fs::symlink_metadata` exists on
Windows. The single hardest portability constraint in detox's tracker evaporates with the
language change; it is not a thing we have to solve.

Also worth stating: doc 02 theme 4 records that the detox maintainer could not run his own unit
tests on macOS (#69, #116), leaving macOS bugs permanently unverified. A CI matrix that includes
macOS with both APFS variants is therefore not a nice-to-have; it is the fix for a named,
years-long structural weakness of the predecessor.

### 6.5 The Windows reserved-name mess

Doc 06 row 7b **refutes** doc 03 constraint 3's direction of travel: per a CPython core-dev
discussion (`python/cpython#95486`), Windows 11 path normalization **no longer** special-cases a
DOS device name that has an extension, so `con.txt`/`nul.txt` are generally no longer reserved
as a leaf; only the bare name still is. Two other secondary sources assert the old universal
rule. Doc 06's own conclusion is that this is genuinely contested among people who study Windows
internals professionally.

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
        policy.rs                # Policy, LegacyEncoding, Target, CaseMode; serde derives
        decode.rs                # OsStr -> Decoded; cp1252/latin1 tables (ours, ~40 lines)
        percent.rs               # safe all-or-nothing %XX decode (ours, ~50 lines)
        classes.rs               # delete/separator/keep classification (ours)
        invisible.rs             # generated from UCD; build-time script, data checked in
        scripts.rs               # UCD Script property, same generator; for §3.12's mixed-script warning
        rules.rs                 # [[rule]] application, literal + regex
        pipeline.rs              # the 13 stages, in order, and only here
        truncate.rs              # grapheme-safe, extension-aware, limit-aware
        reserved.rs              # Windows reserved stems + illegal chars
        plan.rs                  # Plan, PlanItem, Resolution, collision engine
    detoxrs/                    # the binary
      src/
        main.rs
        cli.rs                   # clap derive; one struct, serde-Serialize into Policy
        config.rs                # TOML load + discovery + profile selection (~150 lines, ours)
        walk.rs                  # snapshot walk; skip rules; entry kinds
        fsops.rs                 # RenameOps impls
        fsops/linux.rs           # renameat2 via libc
        fsops/macos.rs           # renamex_np via libc + VOL_CAP probe  <-- the FFI shim
        fsops/windows.rs         # MoveFileExW
        fsops/fallback.rs        # check-then-rename
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
it.

There is deliberately **no total-transitive-crate cap in this document.** A draft target of "<= 45
crates in `cargo tree`" was struck because nobody has run `cargo tree` against this exact set:
`clap`'s derive feature alone pulls the `syn`/`quote`/`proc-macro2` chain plus its own ecosystem, and
a number asserted without measuring is not a budget. **[UNVERIFIED]**: the transitive count for the
set below. The first `cargo add` commit measures it and writes the real ceiling into CI; until then
the direct-dependency cap is the only enforced number.

| Direct dep                                                                 | Why not our own code                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `clap` (derive)                                                            | 4.6.5, 2026-07-31 (doc 07 row 7a). Arg parsing plus help plus completions plus man is not worth hand-rolling.                                                                                                                        |
| `serde` + `toml` (two crates, two budget lines)                            | Config.                                                                                                                                                                                                                              |
| `serde_json`                                                               | `--json`, plan files, journal.                                                                                                                                                                                                       |
| `unicode-normalization`                                                    | 0.1.25, 2025-10-30 (doc 06 row 5b). UAX #15 is not hand-rollable.                                                                                                                                                                    |
| `unicode-segmentation`                                                     | 1.13.3, 2026-06-01 (doc 06 row 5b). Grapheme clusters, mandatory for truncation.                                                                                                                                                     |
| `regex`                                                                    | `[[rule]] regex = true` and `--exclude` globs compiled to regex. Already in every distro. RE2-derived, so no backreferences or lookaround: a documented ceiling, same as f2's (doc 03, f2 row), not a bug to fix with `fancy-regex`. |
| `walkdir`                                                                  | Recursive walk.                                                                                                                                                                                                                      |
| `libc`                                                                     | `renameat2`, `renamex_np`, `getattrlist`, `statfs`.                                                                                                                                                                                  |
| `deunicode` (feature `ascii`, default on)                                  | 1.6.2, 2025-04-27 (doc 06 row 5b). Transliteration tables.                                                                                                                                                                           |
| `terminal_size` or equivalent, only if needed for preview column alignment | Candidate for deletion, and the 11th budget line until deleted; a fixed two-column layout may not need it. Resolve before the first release, not after.                                                                              |

Dev-only: `insta`, `trycmd`, `assert_cmd`, `proptest`, `criterion`, `clap_complete`,
`clap_mangen`. All confirmed active (doc 07 row 7a).

**Explicitly rejected, with the validated reason:**

| Rejected                                   | Reason                                                                                                                                                                                                                                                                |
| ------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `figment`                                  | Last publish 2024-05-17; a `figment2` fork exists, itself a staleness signal (doc 07 row 7b **refutes** doc 04's "actively maintained"). We need ~150 lines of three-source merge, not a provider framework.                                                          |
| `config`-rs                                | Active, but pulls parsers for formats we do not support.                                                                                                                                                                                                              |
| `ignore`                                   | Excellent crate, but `.gitignore` awareness is wrong for this tool: a gitignored file is exactly the kind of junk-named file a user wants cleaned. We want a hardcoded VCS-metadata skip list plus `--exclude`, which is less code and no `crossbeam`/`globset` tree. |
| `jwalk`                                    | Last release 2022-12-15, over 3.5 years stale (doc 06 row 5d **refines** doc 03's unqualified recommendation). Also unneeded: see parallelism below.                                                                                                                  |
| `sanitize-filename` / `sanitise-file-name` | 0.6.0 truncates at codepoint boundaries, not grapheme clusters, and will split base+combining-mark and ZWJ sequences (doc 06 row 5a, read from source). Its whole job is the part we must get right.                                                                  |
| `unicode_skeleton`, `confusables`          | 2017 and 2023 respectively (doc 06 row 5c). We generate a UTS #39 table from UCD data checked into the repo instead.                                                                                                                                                  |
| `unicode-security`                         | 0.1.2, 2024. Reconsider for the v1.1 confusable work; not needed for v1.0's mixed-script warning.                                                                                                                                                                     |
| `encoding_rs` / `chardetng`                | Both active, both overkill: our legacy decode is CP1252/Latin-1 only, a ~40-line const table, and we deliberately do not do detection (§3.4).                                                                                                                         |
| `rustix` / `nix`                           | Neither exposes macOS `renamex_np`/`RENAME_EXCL`/`RENAME_SWAP` (doc 06 row 4e, **refuting** doc 03's implication). Since a raw `libc` shim is required for macOS anyway, adding a second syscall crate for Linux only buys asymmetry.                                 |
| `rayon`, `tokio`                           | Renaming is syscall-bound; doc 04 §4 flags rayon as a questionable fit and explicitly says the claim needs a project-specific benchmark. v1.0 is single-threaded. If a benchmark later shows a win, it will be a small capped worker pool, not a work-stealing pool.  |
| `indicatif`                                | v1.0 prints a plain counting line to stderr for large trees. A progress bar is a dependency for a cosmetic.                                                                                                                                                           |
| `anyhow` / `thiserror`                     | One hand-written error enum. This is a leaf binary with maybe 15 error variants.                                                                                                                                                                                      |
| `trash`                                    | Nothing is ever deleted.                                                                                                                                                                                                                                              |

### 7.3 What we implement ourselves, deliberately

CP1252/Latin-1 decode tables; percent-decoding; the character classifier; the invisible/bidi and
UCD Script tables (generated at build time from data files in-tree, never fetched); grapheme-safe
extension-aware truncation; the Windows reserved-name check; config discovery and first-match
selection; the collision engine including chain ordering and cycle refusal (§5.3); the journal; the
rename FFI shims. All of it is either logic we must be able to test exhaustively (P7's real
motivation) or a stale-crate replacement validation told us to avoid.

Size estimate, split so it is checkable rather than reassuring. **v0.1 (§10): 1200-1800 lines of
non-test Rust**, which is where the earlier single figure came from. **v1.0 including config,
profiles, rules, `--target`, per-directory limit detection, `--stdin`, plan files, and the two
generated tables: 2200-3000 lines.** The parts that historically blow such estimates are named
rather than averaged away: the crash-safe journal with reverse replay and the three-layer collision
engine with deterministic renumbering, chain ordering, and cycle detection each run 300-500 lines on
their own, and `report.rs` (human preview plus `--json` plus verbosity plus color) is another 250-400.
The already-itemized small pieces (`decode.rs` ~40, `percent.rs` ~50, `config.rs` ~150, the macOS FFI
shim ~60 plus a capability probe) are a rounding error against those. Treat the range as a budget to
be checked at v0.1, not as a prediction.

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
Second, every property is quantified over **resolved** policies (`max_len` a concrete number, never
the CLI's `0 = auto` sentinel: §3.1), because a `proptest` harness has no directory to probe.

| Property                            | Statement                                                                                                                                                                                                                                                                                           |
| ----------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Totality**                        | For every input and every resolved policy, `transform` returns either `Name(o)` where `o` satisfies Safety closure and Non-empty, or `Unrepresentable(r)`. It never returns an unsafe name and never panics. This is the property that makes the `Name(_)` scoping above honest rather than a hole. |
| **Idempotence**                     | For `Name(o)`, `transform(o) == Name(o)` for every resolved policy. The stage-13 fixed-point loop exists to make this true; non-convergence within the bound is `Unrepresentable(NotConverged)` (§3.14), not a silently non-idempotent output.                                                      |
| **Safety closure**                  | For `Name(o)`, `o` contains no delete-class character, no separator-class character, no leading `-`, no trailing dot or space, and (unless `--case keep`) is entirely in the requested case.                                                                                                        |
| **Length bound**                    | For `Name(o)`, `o` satisfies both the byte and UTF-16-unit limit for the resolved policy, for every input, including inputs made of astral emoji only.                                                                                                                                              |
| **No grapheme splitting**           | For `Name(o)`, the grapheme cluster count of `o` is not greater than that of `x`, and every cluster in the output is a complete cluster from the input or a replacement character we chose. Holds on the whole-name fallback of §3.10 step 3 as well as the stem path of step 2.                    |
| **Non-empty**                       | For `Name(o)`, `o` is never `""`, `"."`, or `".."`. The empty/dot cases are exactly what `Unrepresentable` exists to carry instead (§3.14).                                                                                                                                                         |
| **Dotfile preservation**            | For `Name(o)`, `x` starts with exactly one `.` implies `o` starts with exactly one `.`, and vice versa.                                                                                                                                                                                             |
| **Valid UTF-8 is never re-decoded** | For every valid-UTF-8 `x`, the `Decoded` variant is `Utf8` and the `Repaired` path is never entered. This is P2 as an executable assertion, and it is the regression test for detox's `café.txt -> cafÃ©.txt` (doc 01 §7, doc 05).                                                                  |
| **Stage independence**              | Disabling stage N changes only what stage N is documented to change: the output with stage N off equals the output of the pipeline with stage N replaced by identity. Catches the scope-creep bug detox had, where the UTF-8 filter also did safe-filter work (#40, #86, doc 02 theme 2).           |

### 8.2 Property tests against `plan`

| Property                    | Statement                                                                                                                                                                                                                                                                                                                                                                                                              |
| --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **No collision**            | For any set of entries and any policy, the plan's `Rename` items have pairwise-distinct `(dir, NFC(casefold?(to)))`. This is the executable form of the maintainer's #130 objection.                                                                                                                                                                                                                                   |
| **No pre-existing clobber** | No `Rename` item's `to` equals an entry that exists and is not that item's own `from`.                                                                                                                                                                                                                                                                                                                                 |
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
punctuation; a CP1252 `Bj\xf6rk` byte string; an invalid-UTF-8 lone `\xff`.

### 8.4 Filesystem and platform matrix (`assert_cmd` + `trycmd`)

| Case                           | Where                                                                                       | Asserts                                                                                                                                                                                                                                                                                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Case-only rename               | macOS case-insensitive APFS image, macOS case-sensitive APFS image, Linux ext4, Linux tmpfs | Succeeds in one syscall via `rename_case_only`, and is _not_ misreported as a collision. This is the doc 06 Test 3 refutation, made permanent.                                                                                                                                                                               |
| NFD -> NFC rename              | Both APFS images                                                                            | Entry bytes change; no duplicate entry; still one file. Doc 06 Test 2 as a test.                                                                                                                                                                                                                                             |
| Length limit probe             | ext4, tmpfs, both APFS images                                                               | The detected limit matches the empirical binary search from doc 06 Test 1 (255 bytes on ext4; 255 ASCII / 127 astral-emoji on APFS).                                                                                                                                                                                         |
| `RENAME_NOREPLACE` unsupported | A mount where it fails (or an injected failure)                                             | Falls back, warns once, still never clobbers.                                                                                                                                                                                                                                                                                |
| Non-UTF-8 name                 | Linux tmpfs (APFS rejects them, per doc 01 §7 and doc 05)                                   | Repaired if CP1252-plausible, `Opaque`-skipped otherwise, never panics. This is the test rnr fails.                                                                                                                                                                                                                          |
| Symlink to `../..` under `-r`  | Linux, macOS                                                                                | Recursion does not escape the tree. Named for the hazard #23 documented (doc 05 correction #2); not a regression test against upstream, which fixed its own instance in 2.0.0-beta1 (§5.6).                                                                                                                                  |
| Rename-during-walk             | 5000-entry tree                                                                             | Every entry is visited exactly once; no entry visited under both its old and new name.                                                                                                                                                                                                                                       |
| Crash mid-batch                | Kill after N renames                                                                        | Journal replay identifies the exact interrupted item; `undo` restores the completed ones.                                                                                                                                                                                                                                    |
| Huge tree                      | 200k entries                                                                                | Completes; memory stays bounded; `criterion` benchmark recorded so a regression is visible. Also the stability test detox never had: doc 05 corrects doc 02 upward to at least three independent crash bugs (#11, #96, #137), which in Rust are mostly a class of bug we do not get to have, but OOM on a large snapshot is. |

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
8b) and is Homebrew-deprecated with a hard disable date of 2027-07-28: a same-named binary with a
preview-by-default posture would silently change behavior under existing scripts, the incident
class doc 04 §6C names. `dtx` collides with no binary known to this research **[UNVERIFIED]**,
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
- **convmv** does encoding conversion only and does it with heuristic detection (doc 03 table);
  our §3.4 policy is narrower and safer by construction.

### 9.4 Packaging path

Order matters, because doc 07 row 8b found the bar is eroding rather than static, and archival
makes the erosion one-way: detox is in Debian 11-14, Fedora 38-44 + Rawhide, Arch, and nixpkgs
(Repology, though **[UNVERIFIED]**: the direct fetch failed in both research passes and rests on a
search-summarized snapshot), _and_ its Homebrew formula is deprecated with a hard disable date of
2027-07-28. Because upstream is archived, that footprint is frozen at v3.0.1 forever: it can only
be dropped by each distro, never refreshed. So the window is not merely dated, it is one-directional
— every distro that removes detox is a set of users with nowhere to land, and there will never be a
competing upstream release to displace.

1. **GitHub Releases with prebuilt static binaries.** `cargo-dist` (now branded `dist`, though
   `cargo-dist` remains the installable crates.io name because the bare `dist` name is squatted:
   doc 07 row 7c) plus `release-plz` (0.3.160, 2026-07-14, very active, doc 07 row 7d).
   `x86_64`/`aarch64` for linux-musl, linux-gnu, macOS, and Windows.
2. **Homebrew tap**, formula pulling the prebuilt binary (no `depends_on "rust"`), targeting
   homebrew-core once the notability bar is met, and specifically before detox's 2027-07-28
   disable date, so `brew` users have somewhere to land.
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
the snapshot walk, the collision engine with `number`/`skip`/`fail`, `renameat2`/`renamex_np`
no-clobber plus fallback, the JSONL journal and `undo`, human preview, `--json`, exit codes.
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

### v0.2

Config file (§4.2), discovery and precedence (§4.3), `--print-config`, `[profile.*]`, `[[rule]]`,
`--keep`/`--strip`, `--case`, `--ascii`, stage 2 (`url_decode`), stage 6.

### v0.3

Legacy-encoding decode (§3.4) with the Linux spike closed; `--target windows`/`portable`;
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

| Gate                                                                                   | Spikes      |
| -------------------------------------------------------------------------------------- | ----------- |
| Any public commit (the name is in every path and manifest)                             | 1           |
| v0.1 Tier-1 correctness (the default pipeline and the rename path v0.1 actually ships) | 2, 6        |
| v1.0, Windows best-effort tier only                                                    | 3, 4        |
| v1.0, everything else                                                                  | 5, 7, 8, 11 |
| Nothing; informational or post-1.0                                                     | 9, 10, 12   |

Spike 2 in particular is mislabelled if called v1.0-only: v0.1 ships `renameat2` no-clobber plus
runtime demotion, so how often the fallback is the _normal_ path is a v0.1 correctness question.
Spike 6 governs stage 1, which is on by default from v0.1.

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

**2. `renameat2(RENAME_NOREPLACE)` behavior in the wild.** (blocks v0.1 Tier-1 correctness)
Doc 06 row 4c marks the Linux syscall and doc 03's filesystem support matrix **UNVERIFIED**: no
Linux machine existed in any research pass. Doc 06's Load-Bearing Uncertainties repeats this.
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

**6. Does the CP1252 repair path work on a real mis-encoded name?** (blocks v0.1 Tier-1 correctness: stage 1 is on by default)
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

**10. Distro packaging reality for detox's current footprint.**
Doc 07's Load-Bearing Uncertainties: the direct Repology fetch failed in both research passes,
so the Debian/Fedora/Arch/nixpkgs version list is a search-summarized snapshot, not primary data.
_Closes with:_ a direct query to Repology's API from a machine with working DNS, before any
"distribution parity" claim appears in a README.

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

---

## Appendix A: traceability of the biggest calls

| Decision                                                             | Primary evidence                                     | Validation effect                                                                                                                                                                                                                            |
| -------------------------------------------------------------------- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fixed pipeline, no sequences or `.tbl`                               | doc 02 theme 1 (~15 issues, highest weight); mandate | doc 05 confirms the #124 "many requests of this nature" quote verbatim                                                                                                                                                                       |
| Legacy decode only on invalid UTF-8                                  | doc 01 §7 (`café.txt -> cafÃ©.txt`)                  | doc 05 reproduced it verbatim                                                                                                                                                                                                                |
| Transliteration off by default                                       | doc 02 theme 5 (#47/#53 -> #99 -> #21 -> #112/#113)  | doc 05 corrects doc 01: no legacy `safe.tbl`; transliteration lived in `unicode.tbl` only                                                                                                                                                    |
| Dry-run default                                                      | doc 04 §1 (f2, rnr)                                  | doc 07 rows 1a/1c confirm both; row 8c confirms detox does _not_                                                                                                                                                                             |
| No overwrite, ever                                                   | doc 02 theme 6 (#130, #122, #124)                    | doc 05 confirms the #130 rejection verbatim, all four technical points                                                                                                                                                                       |
| Snapshot walk, apply deepest-first                                   | doc 03 constraint 11; doc 01 §6                      | n/a                                                                                                                                                                                                                                          |
| No temp-name dance for case-only renames                             | doc 03 constraint 2 said otherwise                   | doc 06 Test 3 **refutes** doc 03; we follow doc 06                                                                                                                                                                                           |
| Hand-written macOS FFI shim budgeted                                 | doc 03 constraint 10 implied crate support           | doc 06 row 4e **refutes** it: neither `rustix` nor `nix` exposes `renamex_np`                                                                                                                                                                |
| Grapheme-safe truncation, own implementation                         | doc 03 constraint 7                                  | doc 06 row 5a: `sanitize-filename` splits clusters (from source)                                                                                                                                                                             |
| APFS limit = 255 UTF-16 units                                        | doc 03 constraint 7 (2-point test)                   | doc 06 Test 1 confirms with a 4-way discriminated test; we use the refined numbers                                                                                                                                                           |
| Journal in XDG_STATE_HOME, not temp                                  | doc 04 §2                                            | doc 07 row 1b: f2 uses `os.TempDir()`; explicitly a cautionary example, not a precedent                                                                                                                                                      |
| No `figment`, no `jwalk`, no `unicode_skeleton`                      | doc 03/04 recommended all three                      | doc 06 rows 5c/5d and doc 07 row 7b found all three stale                                                                                                                                                                                    |
| Symlink recursion has no flag                                        | doc 02 called symlinks a weak theme                  | doc 05 correction #2 **refutes** the "weak theme" reading: #23 is a real blast-radius incident. But upstream fixed #23 in 2.0.0-beta1 (verified in clone `0a8e212`), so the argument rests on the hazard, not on a live upstream flaw (§5.6) |
| Name is `detoxrs` (binary `detoxrs` + `dtx`), never the bare `detox` | doc 04 §5, §6C; user direction                       | doc 07 rows 9a/9b: availability unverifiable (no candidate existed to check), `detoxpy` collision, squatting precedent; doc 07 row 8b: `detox` binary name is live in 4 distros                                                              |

---

## Review record (stage 3)

Three independent reviewers examined this document under different lenses: **L1** source fidelity and
citation audit, **L2** completeness and internal/cross-document consistency, **L3** implementer
reliability. Every finding is adjudicated below. Rejections are included on purpose: two reviewer
recommendations would have made the document worse, and one rested on a misreading of §6.5.

Findings marked **[verified here]** were checked against a primary source during adjudication rather
than taken from a reviewer's summary: the GitHub API for upstream status, the pinned upstream clone
`0a8e212` for `CHANGELOG.md`, `README.md`, `src/file.c`, and `src/clean_string.c`.

| Finding (reviewer)                                                                                                                                                                                                                                               | Verdict                  | Action taken, or reason for rejection                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Upstream is **archived**, never stated; softened to "unmaintained and being wound down" (L1 CRITICAL, L2 CRITICAL)                                                                                                                                               | **ACCEPTED**             | **[verified here]** GitHub API: `archived: true`, `open_issues_count: 0`, 446 stars, `pushed_at 2026-07-12`. Stated once at the top of the document as the fact everything else relies on, with the "34 issues closed in one ~50-minute administrative sweep, so closed means demand and not rejection" reading made explicit. §9.2 rewritten to "archived (upstream, 2026-07-12)" with the consequence spelled out: no upstream to coordinate with, no PR that could be accepted, no issue that could be filed. §9.4's packaging argument re-derived: the distro footprint is frozen at v3.0.1 and can only be dropped, never refreshed, so the window is one-directional rather than merely dated. §9.2 also now notes archival is what makes `MIGRATING-FROM-DETOX.md` and `--explain-detox` finite deliverables. Re-examined §9 and §11 for arguments needing a live upstream: none found — §11's spikes are all our own measurements, and spike 8's "user feedback" means our users.                                                                                                                                                                                                           |
| Mandate quote truncated, omitting the README's "So, `detox` is paused" (L1, inside CRITICAL)                                                                                                                                                                     | **ACCEPTED**             | **[verified here]** README `0a8e212` lines 25-26. Both closing sentences quoted, with the line citation.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| §5.6's symlink policy justified by #23 as a live architectural flaw; doc 10 shows it was fixed in 2.0.0-beta1 (L2 CRITICAL)                                                                                                                                      | **MODIFIED**             | **[verified here]** `CHANGELOG.md` line 144 under `## [2.0.0-beta1] - 2021-03-05`, Security: "Symlinks that point at directories are no longer followed when `--special` and `-r` are specified together. [#23]"; structurally confirmed at `src/file.c:218-223`, where `lstat` + `S_ISDIR` means `parse_dir` never descends through a symlink; `man/detox.1:109` documents it. The reviewer is right that the citation was stale. **The policy is unchanged** — it is right on its own merits — but the argument is rebuilt: #23 is now cited as a first-person account of the _hazard_ (one relative symlink turning a scoped run into a whole-home-directory run), explicitly not as a live flaw, with the fix version named; #20 (symlink loops, untested) is unaffected by the #23 fix and carries the "nobody has characterized this" weight. §4.4, §8.4, and Appendix A corrected in the same direction; the §8.4 case is relabelled as asserting our construction rather than as a regression test against upstream.                                                                                                                                                                        |
| Stage 13's empty-name fallback ("keep the original name") falsifies §8.1 Safety closure; `***` is a counterexample (L3 CRITICAL)                                                                                                                                 | **ACCEPTED**             | Traced by hand and confirmed: `***` -> `___` (stage 7) -> `_` (stage 9) -> `` (stage 10) -> fallback `***`, which contains three separator-class characters. A release gate the default pipeline violates on a three-character input is not a gate. Resolved as a design decision in new **§3.14**: `transform` returns `TransformResult::Unrepresentable(ReducesToEmpty                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | ReducesToDotOrDotDot | NotConverged)`and the planner skips the entry unchanged, reported like`Opaque`. No invented placeholder — a placeholder is a taste-driven guess (P4) that would also collide with every other unrepresentable name in the directory. Stage 13's row in §3.2 rewritten accordingly. §8.1 re-scoped: name-properties quantified over `Name(_)`, plus a new **Totality** property so the scoping is not a loophole. The residual design question (do such names occur often enough to earn a placeholder policy?) is §11 question 11, not a decision invented here. |
| §3.7 delete class re-includes stage 4's invisibles, making `--no-invisible-strip` a dead flag and falsifying Stage independence (L3 CRITICAL)                                                                                                                    | **ACCEPTED**             | Delete class narrowed to control characters only (`Cc` plus DEL and NUL), with the reason stated inline: if it duplicated stage 4's set the flag would be dead and Stage independence false.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| Stage 13's 3-iteration bound has no defined non-convergence behavior (L3 CRITICAL)                                                                                                                                                                               | **ACCEPTED**             | `Unrepresentable(NotConverged)` in §3.14: same skip path, logged at `-v` with intermediate states, treated as a bug report against us. No silent non-idempotent output, no runtime-raised bound, no panic. Whether 3 is the right number is now §11 question 12 with a cheap closing experiment (instrument the fuzz target's iteration count under `--target windows` with tight `--max-len`), rather than a taste-driven constant defended in prose.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| `transform` purity vs stage 12's filesystem-detected limit; Length Bound proptest not implementable as specified (L3 CRITICAL)                                                                                                                                   | **ACCEPTED**             | §3.1 now states that the `Policy` reaching `transform` is always fully resolved (`max_len` concrete, never the CLI's `0 = auto` sentinel) and that resolution is a walk-time concern producing one resolved `Policy` per directory. §8.1 gains a blanket scoping rule: every property is quantified over resolved policies.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| §5.3 renumber-then-truncate has no termination bound and no failure mode (L3 CRITICAL)                                                                                                                                                                           | **ACCEPTED**             | Bounded exactly like stage 13 and stated: N = 2..999, each candidate truncated to fit, against existing names plus already-allocated destinations; if none fits, the item is an unresolvable `Conflict` routed by `--on-collision`. Never drop the numbering, never exceed the limit, never guess. New §8.2 **Bounded renumbering** property.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §5.1/§8.2 do not address sibling rename cycles or swaps (L3 CRITICAL)                                                                                                                                                                                            | **MODIFIED**             | The gap was real — the document said nothing — but the reviewer's remedy (cycle detection, temp-name routing, topological ordering) is unnecessary and would have added the one thing §5.4 works hardest to avoid: a rename to a name the user never asked for. Cycles and chains are **structurally impossible** given the document's own non-negotiable Idempotence property: if `f(a) = b` and `f(b) = a` then `f(f(a)) = f(a)` forces `f(b) = b`, so `a = b`; the same argument collapses `f(a) = b, f(b) = c` to `c = b`, meaning the second entry is `Unchanged`, which is an ordinary pre-existing-destination conflict layers 1-2 already handle. Renumbering cannot manufacture one because it only allocates free names. §5.3 now carries the proof, plus the cheap guard the proof deserves: a plan-time assertion that refuses the batch as an internal error if a `Rename` destination ever equals another `Rename`'s `from`. §8.2's property is **No sibling chains** (with near-swap generators), not cycle handling. Rejected the temp-name dance outright: an invented intermediate name is a P3 surprise and would put a state in the journal that no forward plan ever produced. |
| Canonical `--help` omits `--legacy-encoding`, `--stdin`, `--explain-detox`, `--help-transforms` while §8.3 calls help a snapshot-tested contract (L2 CRITICAL)                                                                                                   | **ACCEPTED**             | All four added to the appropriate `--help` sections, plus the `detoxrs [OPTIONS] --stdin` usage line.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| §2.2's `..bad  name...txt -> .bad_name.txt` is not producible: `.` is Keep-class so stage 9 never collapses the interior dots (L3 MAJOR)                                                                                                                         | **MODIFIED**             | The reviewer traced correctly, but the defect was in the spec, not the example — collapsing repeated `.` is behavior the document wants (and detox had, by a worse mechanism). Fixed at the source: stage 9's collapse set is now stated explicitly as `.`, `-`, `_`, and the configured `--separator`, with the clarification that Keep-class means "not deleted and not substituted", not "exempt from collapsing". Example left as written because it is now correct.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| §3.7 vs §3.8: `-` is Keep-class but `a--b -> a-b` requires collapsing it; the actual collapse rule is never stated (L3 MAJOR)                                                                                                                                    | **ACCEPTED**             | Same §3.8 rewrite. Also states what does _not_ collapse (`aaa` stays `aaa`) and why a run produced by stage 7 does (`" & " -> "___" -> "_"`), so `_-_` surviving and `a--b` collapsing follow from one rule instead of two examples.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| §2.2's Björk example shows `--ascii` applied to one sibling and not the other, with no flag shown (L3 MAJOR)                                                                                                                                                     | **ACCEPTED**             | Split into two invocations: the default keeps `ö` for both files, and a second `detoxrs --ascii` invocation shows the opted-in transliteration. Reinforced with one line naming §3.6 and pointing out that `_-_` survives by stage 9's rule.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §2.2's `Icon\r` "skipped (excluded)" implies a built-in default exclude list that is never specified (L3 MAJOR)                                                                                                                                                  | **ACCEPTED**             | Annotated inline: the example assumes the §4.2 user config, **there is no built-in default exclude list**, the only unconditional skips are `.git`/`.hg`/`.svn` and dotfiles during recursion, and with no config `Icon\r` would become `Icon`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| §7.2's dependency table names 11-12 crates against a CI-enforced "<= 10 direct" cap; the "<= 45 crates in `cargo tree`" figure was never measured (L2 MAJOR, L3 MAJOR)                                                                                           | **ACCEPTED**             | Cap restated as "<= 11 direct dependencies" with the count done honestly (`serde` and `toml` are one row but two packages; `terminal_size` is the 11th line until deleted, and must be resolved before first release). The transitive-crate figure is **struck** rather than adjusted: nobody has run `cargo tree` against this set, so it is marked **[UNVERIFIED]** with the real ceiling to be written into CI by the first `cargo add` commit. §9.4's "Ten direct dependencies" updated to eleven. A budget a CI check turns into a lie on day one is worse than no budget.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| §10's v0.1 stage list omits on-by-default stage 2 and stage 11 with no note (L2 MAJOR); v0.1 cannot implement stage 13's "re-run 9/10/11" without stage 11 (L3 MAJOR)                                                                                            | **ACCEPTED**             | §10 now states that v0.1's stage list is a strict subset of the on-by-default pipeline and names both deferrals with their consequences: stage 2's absence means v0.1 leaves `%20` alone (ugly, not unsafe, and the §2.2 example is a v0.2 output); stage 11 is identity under the default `--target unix`, so its absence costs nothing observable, and **v0.1's stage 13 re-runs 9/10 only**, with 11 joining the loop in v0.3.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| §6.5's conservative reserved-name rule is presented as an always-on global safety property that v0.1 does not actually deliver (L2 MAJOR)                                                                                                                        | **REJECTED**             | Misreads §6.5. The reserved-stem and illegal-character checks are `--target windows`/`portable`-gated in §3.2 row 11, in §6.5's own sentence ("`--target windows` also applies the illegal-character set..."), and in §3.13's opt-in column. The only piece applied on all platforms is the trailing dot/space strip, which lives in stage 10, ships in v0.1, and §6.5 says so. There is no contradiction to fix. The genuine wrinkle the reviewer was circling — stage 13's fixed-point loop naming stage 11 — is L3's finding and is addressed above; §10 now states the `--target`-gated reading explicitly so this misreading is harder to repeat.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| No SIGINT/SIGTERM handling anywhere, despite heavy emphasis on crash resilience (L2 MAJOR)                                                                                                                                                                       | **ACCEPTED**             | New §5.8: a flag-setting handler, checked between items, stops cleanly and writes the summary and closing journal state; an in-flight `rename(2)` is a single syscall and is not interrupted; Ctrl-C leaves a fully recorded prefix that `undo --last` reverts. `SIGKILL` remains safe via the `intent`/fsync/rename/`done` protocol, which reports the one unknown item rather than guessing.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| No error taxonomy for `EROFS`/`ENOSPC`/`EACCES`; journal write is itself I/O that can fail (L2 MAJOR)                                                                                                                                                            | **ACCEPTED**             | §5.8 names the enum variants §7.2 promised but never showed, and answers the sharp version of the question: if the `intent` record cannot be written or fsynced, **the rename does not happen**, because an unjournaled rename is the one thing `undo` cannot reverse. `EROFS`/`ENOSPC` abort the remainder after the first occurrence instead of printing 200k identical lines. `ENAMETOOLONG` is called out as evidence our detected limit was wrong.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| Unreadable directory / `EMFILE` during the walk not addressed (L2 coverage gap)                                                                                                                                                                                  | **ACCEPTED**             | §5.8: unreadable directory is reported and skipped, walk continues (matching detox, doc 13 §4.4); `EMFILE`/`ENFILE` aborts before any rename, because an incomplete snapshot is the one thing the two-phase design in §5.1 cannot tolerate.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| No behavior stated for concurrent `detoxrs -x` runs; journal names are not a lock (L2 MAJOR)                                                                                                                                                                     | **MODIFIED**             | Recorded as an **explicit non-goal**, and the lock file rejected with a reason. What already bounds the damage is in the design: no-clobber renames, `apply`'s `(dev, ino, mtime)` recheck, one journal file per batch. A lock would have to be advisory, on a path we do not own, with a stale-lock story, to prevent an outcome that is already non-destructive. Stating the non-goal is the fix; building the lock is not.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §7.3's 1200-1800 LOC estimate is thin against its own itemized scope; reviewer proposed 2500-4000+ (L3 MAJOR)                                                                                                                                                    | **MODIFIED**             | The criticism holds — the estimate read as whole-project while its itemized pieces already consumed a fifth of it — but 4000 is high for a single-binary tool with eleven dependencies. Split into **v0.1: 1200-1800** and **v1.0: 2200-3000**, with the three parts that historically blow such estimates named individually (journal, collision engine, `report.rs`) rather than averaged away, and the range labelled a budget to be checked at v0.1 rather than a prediction.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| §11's "spikes 1-4 gate v1.0" contradicted by the spikes' own annotations (L3 MAJOR)                                                                                                                                                                              | **ACCEPTED**             | §11 opens with a gating table keyed to what each spike actually blocks: 1 blocks any public commit; 2 and 6 block v0.1 Tier-1 correctness (v0.1 ships the `renameat2` path and stage 1 is on by default); 3 and 4 block only the Windows tier; 5, 7, 8, 11 block v1.0; 9, 10, 12 gate nothing. Spike 2's and spike 6's inline annotations corrected to match.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| §3.10 step 1's "<= 4 characters" extension lookback has no unit (L3 MAJOR)                                                                                                                                                                                       | **MODIFIED**             | **[verified here]** The reviewer guessed codepoints. It is **bytes**: `src/clean_string.c:284-294` in `0a8e212` does `while (--input_walk > filename) { if (extension - input_walk > 5) break; ... }` — pointer arithmetic over `char *`. Stated as "<= 4 bytes of UTF-8" with the source line and the note that for the ASCII segments this rule targets (`.tar`, `.tar.gz`) bytes and codepoints agree, so the choice only shows up on inputs the rule was never aimed at.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §3.10 step 3's whole-name fallback does not restate grapheme safety (L3 MAJOR)                                                                                                                                                                                   | **ACCEPTED**             | Step 3 now says "same grapheme-cluster boundary algorithm as step 2, just with no extension split", and names the temptation it is closing off (`is_char_boundary`). §8.1's No-grapheme-splitting row says the property is not waived on that path.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `rename_case_only` is misleadingly named; §6.2 also routes NFD->NFC through it (L3 MINOR)                                                                                                                                                                        | **MODIFIED**             | Kept the name (it is referenced from §5.4, §6.2, and §8.4; renaming buys nothing a comment does not) and fixed the doc-comment at the trait definition to name both cases and, more importantly, to state _why_ the no-clobber flag must not be used on this path.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| §3.12's mixed-script warning needs a UCD Script table not listed in §7.1/§7.3 (L3 MINOR)                                                                                                                                                                         | **ACCEPTED**             | `scripts.rs` added to §7.1's layout (same build-time generator as `invisible.rs`) and to §7.3's list.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| §3.2 stage 10: "leading separator" ambiguous when a preserved leading `.` comes first (L3 MINOR)                                                                                                                                                                 | **ACCEPTED**             | Stage 10's row says "including one that immediately follows a preserved leading `.`", and §3.8 gains the worked example: `.!file.txt` -> `.file.txt`, not `._file.txt`. The dot is a dotfile marker, not a shield for what follows it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| §7.2's Debian claim cited doc 07 row 8a as a live re-verification it explicitly was not (L1 MINOR)                                                                                                                                                               | **ACCEPTED**             | Hedged to doc 07's own confidence level: row 8a upholds the citation but did not re-fetch it, "medium-high confidence, not re-verified live", consistent with well-documented Debian practice and not in real doubt.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| P3 leans on #124, whose longer quote doc 05 flags as a synthesis of two comments 37 minutes apart (L1 MINOR)                                                                                                                                                     | **ACCEPTED**             | Fidelity note added to P3 pointing at doc 05 Corrections Required item 4, stating that only the short fragments quoted here are individually verbatim. The document never block-quoted the synthesized text, so this is a citation-chain courtesy, not a fabrication fix.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| §4.4's not-configurable table omits the `.git`/`.hg`/`.svn` skip (L2 MINOR)                                                                                                                                                                                      | **ACCEPTED**             | Row added, with the #110 `--git`-rejection reason.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| No statement that xattrs, ACLs, ownership, and mode survive a rename (L2 MINOR)                                                                                                                                                                                  | **ACCEPTED**             | One paragraph in §5.2: everything attached to the inode is untouched by construction, and it is a `rename(2)`-level guarantee rather than something we implement. Stated so an auditor need not derive it from POSIX.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| No statement on i18n of detoxrs's own messages (L2 MINOR)                                                                                                                                                                                                        | **ACCEPTED**             | §9.4: English-only like upstream (doc 13 §3, §8), not revisited for v1.0, with the reason it is worth stating (§8.3 pins `--help` as a snapshot contract).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Snap neither adopted nor declined, though detox ships one (L2 MINOR)                                                                                                                                                                                             | **ACCEPTED**             | Declined explicitly in §9.4 with the doc 13 §5.1 precedent as the reason: a `devmode` snap is a tarball with extra steps, and a strictly confined one could not do this tool's job.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| `.tbl` locale-conditional filter blocks never discussed as a dropped capability (L2 coverage gap)                                                                                                                                                                | **ACCEPTED**             | §9.2 migration bullet: `detoxrs` has no locale-conditional behavior anywhere, same output under every locale, and the reason it is a deliberate drop — a rename whose result depends on the ambient environment cannot be previewed honestly.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| No `detoxrs` analogue for detox's `-L -v` (dump the resolved config) (L2 coverage gap)                                                                                                                                                                           | **ACCEPTED**             | `--print-config` added to `--help`, justified in §4.3 (four-layer precedence makes "which of these set this value" a real question, and profiles plus rules make the dump _more_ useful than for detox's sequences, not less), and slotted into v0.2 in §10.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| detox's `-r` quirk (first-level children processed with or without `-r`) not noted for migrators (L2 coverage gap)                                                                                                                                               | **ACCEPTED**             | §9.2 migration bullet states detox's behavior (doc 10) and that `detoxrs` deliberately does not copy it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| L2's remaining coverage rows judged "moot", "adequate", or "acceptable omission" by the reviewer itself (duplicate-sequence merge semantics, `\uXXXX` escapes, `configure.ac` options, `DETOX_SEQUENCE`, `max_length <= 0` coercion, overlong UTF-8, `-?` alias) | **REJECTED** as findings | No action needed and none taken. Each is either mooted by a rejected mechanism (sequences, `.tbl` files) or already covered by an existing statement (§4.3's environment-variable sentence, §3.7's delete class, Rust's own UTF-8 validation). Documenting a non-difference would add length without adding information.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |

Counts: **31 accepted, 7 modified, 2 rejected** (plus one block of reviewer-acknowledged non-findings
rejected as a group).

The three findings flagged as most serious were each verified against primary source before acting,
and each produced a different kind of change: the archival was a **fact** to correct, #23 was an
**argument** to rebuild while keeping the policy, and stage 13's fallback was a genuine **design
contradiction** that needed a new decision (§3.14) plus one honest open question (§11.11) about the
part the decision does not settle.
