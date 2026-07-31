# detoxrs: a Rust successor to detox

Status: design proposal. Audience: the person who starts writing code Monday.
Inputs: docs 01-04 (primary research), docs 05-07 (adversarial validation). Where they
disagree, 05/06/07 win and are cited as such. Every claim traceable to research is cited
inline as (doc NN, section). Every dependency on something validation could not confirm is
marked **[UNVERIFIED]** and appears again in §11.

The mandate (detox README, quoted in doc 02, "Maintainer's Stated Future Direction"):

> The days of weighty configuration files are behind us, and users looking for help with their
> files shouldn't need to be well-versed in character encoding. detox needs to be easier to work
> with, using command-line options and a config file that lets you pre-select those options. It
> needs to just work. Period.

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
someone's files" (doc 02 theme 6, doc 05 rows #130/#124).
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
transitive ones, to be its own Debian source package built with no network (doc 04 §5,
confirmed against the Debian Rust Team's own book; re-affirmed doc 07 row 8a). Validation also
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
  Bj<f6>rk - Vespertine.mp3   ->  Bjork_-_Vespertine.mp3   (repaired: cp1252 -> UTF-8, then --ascii)
  Bj<f6>rk - Homogenic.mp3    ->  Björk_-_Homogenic.mp3    (repaired: cp1252 -> UTF-8)
```

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
filesystem, or another file. Everything that involves other files (collisions, existence,
length limits of _this_ filesystem) lives in the planner (§5). That split is what makes the
property tests in §8 possible.

### 3.2 The default pipeline, in order

| #   | Stage             | Default   | What it does                                                                                                                                                                                                                          |
| --- | ----------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | `decode`          | on        | `OsStr` -> text. Valid UTF-8 passes through untouched. Otherwise decode the raw bytes as CP1252 (superset of Latin-1 in the 0x80-0x9F range); if that yields no C1 controls and no replacement chars, emit `Repaired`. Else `Opaque`. |
| 2   | `url_decode`      | on        | `%XX` -> byte, only when every escape in the name is well-formed and the decoded result is valid UTF-8 and contains no `/`, no NUL, and no controls. All-or-nothing per name. `+` -> space is **off**.                                |
| 3   | `normalize`       | NFC       | Unicode normalization of the output name. Comparison inside the planner is _always_ NFC regardless of this setting.                                                                                                                   |
| 4   | `invisible_strip` | on        | Delete bidi controls (U+202A-202E, U+2066-2069, U+200E/200F), zero-width (U+200B/200C/200D/2060/FEFF), Unicode Tags (U+E0000-E007F), and all remaining `Cf`, `Cc`, `Cs`, `Co`.                                                        |
| 5   | `rules`           | none      | User's ordered `[[rule]]` list: literal or regex find/replace, applied in file order, each seeing the previous one's output. The only customization slot.                                                                             |
| 6   | `ascii`           | **off**   | Transliterate to ASCII (`deunicode`). Lossy, opt-in.                                                                                                                                                                                  |
| 7   | `safe_map`        | on        | Character classes, not a table: delete-class -> nothing; separator-class -> `--separator`; everything else kept. Sets defined in §3.7.                                                                                                |
| 8   | `case`            | keep      | `lower`/`upper` use Unicode simple case mapping, not ASCII-only.                                                                                                                                                                      |
| 9   | `collapse`        | on        | Collapse runs of the _same_ separator character to one. Do **not** merge runs of different separators. Drop separators adjacent to `.`.                                                                                               |
| 10  | `trim`            | on        | Strip leading `-`; strip leading/trailing separators; strip trailing dots and spaces; preserve exactly one leading `.` if the original had one.                                                                                       |
| 11  | `target`          | unix      | With `--target windows` or `portable`: reserved-stem check, Windows illegal-character check, MAX_PATH warning.                                                                                                                        |
| 12  | `truncate`        | on (auto) | Grapheme-safe, extension-preserving truncation to the filesystem limit, or `--max-len N`.                                                                                                                                             |
| 13  | `finalize`        | on        | Re-run 9/10/11 until fixed point (bounded to 3 iterations), then the non-empty guard: if the result is empty, `.`, or `..`, keep the original name and emit a note.                                                                   |

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
  fixed-point pass beats trying to predict the interaction.

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

**Delete class** (removed, no replacement): C0 controls including newline and tab, C1 controls,
DEL, NUL, and the invisibles from stage 4. Deleted rather than substituted because a control
character carries no information a human wanted and substituting it leaves visible litter.

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

Collapse runs of the _same_ separator character only. `a__b` -> `a_b`. `a--b` -> `a-b`.
`a_-_b` -> `a_-_b`, **unchanged**. detox collapses mixed runs by positional precedence in a
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
   <= 4 characters and itself preceded by a `.`, the pair (`.tar.gz`). This deliberately
   reproduces detox's behavior, including the 5-character lookback (doc 01 §2.3, §8 item 8;
   confirmed against source in doc 05), because it is well-understood and the failure mode is
   benign.
2. Truncate the stem on a **grapheme cluster** boundary via `unicode-segmentation`, not a
   codepoint boundary. `sanitize-filename` 0.6.0 truncates at `is_char_boundary`, i.e. it will
   split a base+combining-mark pair or a ZWJ emoji sequence (doc 06 row 5a, read from source:
   worse than doc 03 implied). That is why we do not use it.
3. If the extension alone does not fit, truncate the whole name as one unit rather than detox's
   "print a warning and give up unchanged" (doc 01 §2.3).
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
| Whether recursion follows symlinked directories | §5.6. There is no flag. Issue #23 is a real user incident.                                                                                                        |
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
    /// Same source inode, different spelling of the same name. Plain rename(2) is correct here.
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
  Doc 05 correction #2 is emphatic here and overturns doc 02's dismissal of symlink handling as
  a weak theme: issue **#23** is a first-person incident report where `detox -r --special`
  followed a relative symlink pointing at `../..` and recursed across the reporter's entire
  projects directory, and **#20** flags symlink loops and `.`/`..` symlinks as an untested gap.
  Unbounded blast radius from a single symlink is not a feature that earns a flag.
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

Target: **<= 10 direct dependencies, <= 45 crates in `cargo tree` for a default build.**
Enforced by a CI check that fails on regression. The reason is not aesthetics: Debian requires
every transitive crate to become its own Debian source package, built with no network (doc 04 §5,
confirmed; doc 07 row 8a). Dependency count is the packaging cost.

| Direct dep                                                                 | Why not our own code                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `clap` (derive)                                                            | 4.6.5, 2026-07-31 (doc 07 row 7a). Arg parsing plus help plus completions plus man is not worth hand-rolling.                                                                                                                        |
| `serde` + `toml`                                                           | Config.                                                                                                                                                                                                                              |
| `serde_json`                                                               | `--json`, plan files, journal.                                                                                                                                                                                                       |
| `unicode-normalization`                                                    | 0.1.25, 2025-10-30 (doc 06 row 5b). UAX #15 is not hand-rollable.                                                                                                                                                                    |
| `unicode-segmentation`                                                     | 1.13.3, 2026-06-01 (doc 06 row 5b). Grapheme clusters, mandatory for truncation.                                                                                                                                                     |
| `regex`                                                                    | `[[rule]] regex = true` and `--exclude` globs compiled to regex. Already in every distro. RE2-derived, so no backreferences or lookaround: a documented ceiling, same as f2's (doc 03, f2 row), not a bug to fix with `fancy-regex`. |
| `walkdir`                                                                  | Recursive walk.                                                                                                                                                                                                                      |
| `libc`                                                                     | `renameat2`, `renamex_np`, `getattrlist`, `statfs`.                                                                                                                                                                                  |
| `deunicode` (feature `ascii`, default on)                                  | 1.6.2, 2025-04-27 (doc 06 row 5b). Transliteration tables.                                                                                                                                                                           |
| `terminal_size` or equivalent, only if needed for preview column alignment | Candidate for deletion; a fixed two-column layout may not need it.                                                                                                                                                                   |

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

CP1252/Latin-1 decode tables; percent-decoding; the character classifier; the invisible/bidi
table (generated from UCD at build time from data files in-tree, never fetched); grapheme-safe
extension-aware truncation; the Windows reserved-name check; config discovery and three-source
merge; the collision engine; the journal; the rename FFI shims. Total estimate: 1200-1800 lines
of non-test Rust. All of it is either logic we must be able to test exhaustively (P7's real
motivation) or a stale-crate replacement validation told us to avoid.

---

## 8. Testing strategy

Non-negotiable means: missing or failing blocks a release.

### 8.1 Property tests (`proptest`), against `transform`

`transform` is pure, so all of these are cheap and hold over arbitrary input strings including
astral planes, combining marks, bidi controls, and long runs.

| Property                            | Statement                                                                                                                                                                                                                                                                                 |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Idempotence**                     | `transform(transform(x)) == transform(x)` for every policy. The stage-13 fixed-point loop exists to make this true; if it does not hold, the loop bound is wrong.                                                                                                                         |
| **Safety closure**                  | `transform(x)` contains no delete-class character, no separator-class character, no leading `-`, no trailing dot or space, and (unless `--case keep`) is entirely in the requested case.                                                                                                  |
| **Length bound**                    | `transform(x)` satisfies both the byte and UTF-16-unit limit for the target policy, for every input, including inputs made of astral emoji only.                                                                                                                                          |
| **No grapheme splitting**           | The grapheme cluster count of `transform(x)` is not greater than that of `x`, and every cluster in the output is a complete cluster from the input or a replacement character we chose.                                                                                                   |
| **Non-empty**                       | `transform(x)` is never `""`, `"."`, or `".."`.                                                                                                                                                                                                                                           |
| **Dotfile preservation**            | `x` starts with exactly one `.` implies `transform(x)` starts with exactly one `.`, and vice versa.                                                                                                                                                                                       |
| **Valid UTF-8 is never re-decoded** | For every valid-UTF-8 `x`, the `Decoded` variant is `Utf8` and the `Repaired` path is never entered. This is P2 as an executable assertion, and it is the regression test for detox's `café.txt -> cafÃ©.txt` (doc 01 §7, doc 05).                                                        |
| **Stage independence**              | Disabling stage N changes only what stage N is documented to change: the output with stage N off equals the output of the pipeline with stage N replaced by identity. Catches the scope-creep bug detox had, where the UTF-8 filter also did safe-filter work (#40, #86, doc 02 theme 2). |

### 8.2 Property tests against `plan`

| Property                    | Statement                                                                                                                                                                            |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **No collision**            | For any set of entries and any policy, the plan's `Rename` items have pairwise-distinct `(dir, NFC(casefold?(to)))`. This is the executable form of the maintainer's #130 objection. |
| **No pre-existing clobber** | No `Rename` item's `to` equals an entry that exists and is not that item's own `from`.                                                                                               |
| **Order safety**            | Applying the plan in the plan's own order never renames a directory before an item inside it.                                                                                        |
| **Determinism**             | Shuffling the input entry list produces an identical plan, including collision numbering. Directly targets the `readdir()`-order dependence in detox (doc 01 §5, scope note).        |
| **Undo round-trip**         | Apply plan then undo journal, against an in-memory filesystem model, restores the exact original name set.                                                                           |

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
| Symlink to `../..` under `-r`  | Linux, macOS                                                                                | Recursion does not escape the tree. Regression test named for detox #23 (doc 05 correction #2).                                                                                                                                                                                                                              |
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

A successor, not a fork, not a drop-in. Positioned as: "detox is unmaintained and being wound
down; here is what to use instead."

- No `detoxrc` parsing, ever, in any form. This is option 3 from doc 04 §6A. Reason: the config
  grammar _is_ the thing the mandate rejects, and doc 05 records that even detox's own merge
  semantics were never behaviorally verified. Instead: a `MIGRATING-FROM-DETOX.md` table mapping
  every detox filter to its `detoxrs` equivalent, plus a `detoxrs --explain-detox <sequence>`
  helper that reads a `detoxrc` **read-only** and prints the closest flag set, refusing to write
  anything. That is a docs feature with a shell, not a parser we have to maintain.
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

Order matters, because doc 07 row 8b found the bar is eroding rather than static: detox is in
Debian 11-14, Fedora 38-44 + Rawhide, Arch, and nixpkgs (Repology, though **[UNVERIFIED]**: the
direct fetch failed in both research passes and rests on a search-summarized snapshot), _and_ its
Homebrew formula is deprecated with a hard disable date of 2027-07-28. There is a dated window
here.

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
   separate Debian source package built offline (doc 04 §5, confirmed). Ten direct dependencies
   is a tractable debcargo job; forty is not.

MSRV: rolling, "stable at least 6 months old," declared via `rust-version`, checked with
`cargo-msrv` in CI and re-checked after every dependency bump (doc 04 §5).

---

## 10. Roadmap

### v0.1: MVP (the walking skeleton, Linux + macOS)

Scope: `detoxrs [-r] [-x] <paths>` with the default pipeline (stages 1, 3, 4, 7, 9, 10, 12, 13),
the snapshot walk, the collision engine with `number`/`skip`/`fail`, `renameat2`/`renamex_np`
no-clobber plus fallback, the JSONL journal and `undo`, human preview, `--json`, exit codes.
No config file. No profiles. No rules. No transliteration.

The MVP boundary is drawn at "safety architecture complete, customization absent," because
§5 is the part that is hard to retrofit and §4 is the part that is trivial to add.

### v0.2

Config file (§4.2), discovery and precedence (§4.3), `[profile.*]`, `[[rule]]`, `--keep`/
`--strip`, `--case`, `--ascii`, stage 2 (`url_decode`), stage 6.

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
it. Spikes 1-4 gate v1.0.

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

**2. `renameat2(RENAME_NOREPLACE)` behavior in the wild.** (blocks v1.0)
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

**6. Does the CP1252 repair path work on a real mis-encoded name?**
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

---

## Appendix A: traceability of the biggest calls

| Decision                                                             | Primary evidence                                     | Validation effect                                                                                                                                                               |
| -------------------------------------------------------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fixed pipeline, no sequences or `.tbl`                               | doc 02 theme 1 (~15 issues, highest weight); mandate | doc 05 confirms the #124 "many requests of this nature" quote verbatim                                                                                                          |
| Legacy decode only on invalid UTF-8                                  | doc 01 §7 (`café.txt -> cafÃ©.txt`)                  | doc 05 reproduced it verbatim                                                                                                                                                   |
| Transliteration off by default                                       | doc 02 theme 5 (#47/#53 -> #99 -> #21 -> #112/#113)  | doc 05 corrects doc 01: no legacy `safe.tbl`; transliteration lived in `unicode.tbl` only                                                                                       |
| Dry-run default                                                      | doc 04 §1 (f2, rnr)                                  | doc 07 rows 1a/1c confirm both; row 8c confirms detox does _not_                                                                                                                |
| No overwrite, ever                                                   | doc 02 theme 6 (#130, #122, #124)                    | doc 05 confirms the #130 rejection verbatim, all four technical points                                                                                                          |
| Snapshot walk, apply deepest-first                                   | doc 03 constraint 11; doc 01 §6                      | n/a                                                                                                                                                                             |
| No temp-name dance for case-only renames                             | doc 03 constraint 2 said otherwise                   | doc 06 Test 3 **refutes** doc 03; we follow doc 06                                                                                                                              |
| Hand-written macOS FFI shim budgeted                                 | doc 03 constraint 10 implied crate support           | doc 06 row 4e **refutes** it: neither `rustix` nor `nix` exposes `renamex_np`                                                                                                   |
| Grapheme-safe truncation, own implementation                         | doc 03 constraint 7                                  | doc 06 row 5a: `sanitize-filename` splits clusters (from source)                                                                                                                |
| APFS limit = 255 UTF-16 units                                        | doc 03 constraint 7 (2-point test)                   | doc 06 Test 1 confirms with a 4-way discriminated test; we use the refined numbers                                                                                              |
| Journal in XDG_STATE_HOME, not temp                                  | doc 04 §2                                            | doc 07 row 1b: f2 uses `os.TempDir()`; explicitly a cautionary example, not a precedent                                                                                         |
| No `figment`, no `jwalk`, no `unicode_skeleton`                      | doc 03/04 recommended all three                      | doc 06 rows 5c/5d and doc 07 row 7b found all three stale                                                                                                                       |
| Symlink recursion has no flag                                        | doc 02 called symlinks a weak theme                  | doc 05 correction #2 **refutes** that: #23 is a real blast-radius incident                                                                                                      |
| Name is `detoxrs` (binary `detoxrs` + `dtx`), never the bare `detox` | doc 04 §5, §6C; user direction                       | doc 07 rows 9a/9b: availability unverifiable (no candidate existed to check), `detoxpy` collision, squatting precedent; doc 07 row 8b: `detox` binary name is live in 4 distros |
