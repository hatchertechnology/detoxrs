# detox: filters and translation tables

Date: 2026-07-31
Pinned commit: `0a8e2127e3c59cb419912d77c50f592b6460480a` (tag v3.0.1 + 4 commits), repo `dharple/detox`
Base URL for source links: `https://github.com/dharple/detox/blob/0a8e212/`

## What was read

`src/filter.c`, `src/filter.h`, `src/clean_string.c`, `src/clean_utf_8.c`, `src/table.c`,
`src/table.h`, `src/detox_struct.h`, `src/parse_table.c`, `src/sequence.c`,
`src/config_file_yacc.y`, `src/config_file_lex.l`, `src/check_table.c`,
`src/generate_builtin_table.c`, `src/escape_utf_8.c`, `src/builtin_table.h`,
`etc/detoxrc`, `man/detoxrc.5`, `man/detox.tbl.5`, all files under `table/` (sizes/samples),
`tests/unit/test_clean_iso8859_1.c`, `test_clean_safe.c`, `test_clean_uncgi.c`,
`test_clean_utf_8.c`, `test_clean_max_length.c`, `test_clean_wipeup.c`,
`tests/legacy/github-issue-0014/test.tbl`, `tests/legacy/github-issue-0019/test.tbl`,
`tests/legacy/man-page-sequence-with-language/safe-manpage.tbl`, `man-page-example/`.
Not read: `src/config_file.c` (config-file merge glue, out of scope), `src/detox.c`/`src/parse_options.c`
(CLI flags, out of scope — this doc is filters/tables only), `table/legacy/*` beyond `wc -l`/head/tail sampling.

---

## 0. Framing: this document is a record of upstream, not a specification

Everything below describes what upstream detox does at `0a8e212`. It is not a parity target.
One family in particular is deliberately **not being carried forward**: the legacy-encoding-repair
filters `iso8859_1`, its `cp1252`-table variant, and `utf_8` (§3.2, §3.3, §3.4) together with their
tables (§5). Per `docs/owner-decisions.md` ("Drop legacy encoding repair from v1.0", 2026-07-31),
`detoxrs` v1.0 **skips and reports** a non-UTF-8 filename and never repairs it — the CP1252/Latin-1
decode tables, the `--legacy-encoding` flag, and the `Repaired` decode outcome are all removed.
The reason is exactly the behavior documented here: these filters guess at an encoding byte-by-byte
with no validation and no detection, which silently mangles a correctly-encoded name (upstream's own
`café.txt` → `cafÃ©.txt` class of bug) and cannot be distinguished from a successful repair after
the fact. Repair may return post-1.0 as an opt-in flag once there is hardware to measure its
false-positive rate.

So this content is kept in full and at full detail on purpose: it is the primary evidence for that
decision, and the precise description of the bug class the successor avoids structurally (Rust's
strict UTF-8 validation makes the whole overlong/invalid-sequence/encoded-NUL surface unreachable
rather than defended against). Read §3.2-§3.4, §5, and §6 as history and rationale, not as a to-do
list.

## 1. Filter catalog (from `enum` in detox_struct.h)

| Config keyword | Enum                | Dispatch (filter.c) | Implementation   | Options                             | Table-backed |
| -------------- | ------------------- | ------------------- | ---------------- | ----------------------------------- | ------------ |
| `iso8859_1`    | `FILTER_ISO8859_1`  | `clean_iso8859_1`   | `clean_string.c` | `builtin "name"`, `filename "path"` | yes          |
| `lower`        | `FILTER_LOWER`      | `clean_lower`       | `clean_string.c` | none                                | no           |
| `max_length`   | `FILTER_MAX_LENGTH` | `clean_max_length`  | `clean_string.c` | `length N`                          | no           |
| `safe`         | `FILTER_SAFE`       | `clean_safe`        | `clean_string.c` | `builtin "name"`, `filename "path"` | yes          |
| `uncgi`        | `FILTER_UNCGI`      | `clean_uncgi`       | `clean_string.c` | none                                | no           |
| `utf_8`        | `FILTER_UTF_8`      | `clean_utf_8`       | `clean_utf_8.c`  | `builtin "name"`, `filename "path"` | yes          |
| `wipeup`       | `FILTER_WIPEUP`     | `clean_wipeup`      | `clean_string.c` | `remove_trailing`                   | no           |

Enum values and keywords: [src/detox_struct.h:17-25](https://github.com/dharple/detox/blob/0a8e212/src/detox_struct.h#L17-L25), lexer keywords [src/config_file_lex.l:28-40](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L28-L40), dispatcher switch [src/filter.c:207-239](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L207-L239).

Grammar for each filter's option block is defined in the yacc file
[src/config_file_yacc.y:96-150](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L96-L150):
every filter accepts bare form (`safe;`), empty-brace form (`safe {};`), and (for table filters)
exactly one of `filename "..."` or `builtin "..."` inside braces — never both, and the grammar has
no rule for specifying both in one block.

Four builtin tables ship compiled into the binary: `iso8859_1`, `unicode`, `safe`, `cp1252`
[src/builtin_table.h:17-20](https://github.com/dharple/detox/blob/0a8e212/src/builtin_table.h#L17-L20).
`iso8859_1`/`utf_8`/`safe` filters silently fall back to their respective compiled-in builtin table
when no `.tbl` file is found on disk and no explicit `builtin`/`filename` was given
[src/filter.c:131-184](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L131-L184).
`filename` is a _hard_ reference: if given, no search/fallback happens — a parse failure there is
fatal (`exit(EXIT_FAILURE)`) [src/filter.c:174-181](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L174-L181).

## 2. Default sequence and named sequences (from `etc/detoxrc`)

Verbatim from [etc/detoxrc](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc):

```
sequence default {
    safe { builtin "safe"; };
    wipeup { remove_trailing; };
};
```

Other shipped sequences (all in the same file):

| Sequence           | Filters, in order                                                                                                                                                |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `default`          | `safe(builtin=safe)` → `wipeup(remove_trailing)`                                                                                                                 |
| `iso8859_1`        | `iso8859_1(builtin=iso8859_1)` → `safe(builtin=safe)` → `wipeup(remove_trailing)`                                                                                |
| `iso8859_1-legacy` | `iso8859_1(builtin=cp1252)` → `iso8859_1(builtin=iso8859_1)` → `safe(builtin=safe)` → `wipeup(remove_trailing)`                                                  |
| `utf_8`            | `utf_8(builtin=unicode)` → `safe(builtin=safe)` → `wipeup(remove_trailing)`                                                                                      |
| `uncgi`            | `uncgi` → `safe(builtin=safe)` → `wipeup(remove_trailing)`                                                                                                       |
| `lower`            | `safe(builtin=safe)` → `lower` → `wipeup(remove_trailing)`                                                                                                       |
| `iso8859_1-only`   | `iso8859_1(builtin=iso8859_1)` only                                                                                                                              |
| `cp1252-only`      | `iso8859_1(builtin=cp1252)` only (note: this is the `iso8859_1` _filter_, just pointed at the `cp1252` builtin table — there is no separate CP-1252 filter type) |
| `utf_8-only`       | `utf_8(builtin=unicode)` only                                                                                                                                    |
| `uncgi-only`       | `uncgi` only                                                                                                                                                     |
| `lower-only`       | `lower` only                                                                                                                                                     |

`ignore { filename "{arch}"; };` is also defined here — unrelated to filters, listed for completeness
[etc/detoxrc:131-133](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L131-L133).

Sequence selection: `sequence_choose_default` looks up by name, defaulting to `"default"`; if no
name was requested and no sequence named `default` exists, it silently uses the _first_ sequence
in the merged list instead of erroring [src/sequence.c:28-54](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L28-L54).
Sequences run as a straight linked-list pipeline, output of one filter piped as input to the next,
short-circuiting (stops filtering, returns NULL) the moment any filter returns NULL
[src/sequence.c:98-124](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L98-L124).

`iso8859_1-legacy` is the officially sanctioned way to chain CP-1252 in front of ISO-8859-1: the
man page explicitly says you can chain multiple `iso8859_1` filters "as long as the default value
of all but the last one [is] empty" [man/detoxrc.5:109-113](https://github.com/dharple/detox/blob/0a8e212/man/detoxrc.5#L109-L113).
The `cp1252.tbl` table indeed ships with `default` commented out
[table/cp1252.tbl:22](https://github.com/dharple/detox/blob/0a8e212/table/cp1252.tbl#L22), so
unmapped bytes fall through to the next filter in the chain instead of being replaced/dropped.

## 3. Per-filter detail

### 3.1 `safe` — `clean_safe`

[src/clean_string.c:90-133](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L90-L133)

Byte-at-a-time (not UTF-8 aware): for every single byte in the input, look it up as a table key.
If found, splice in the replacement string (which may be multi-char or empty); if not found, use
`table->default_translation` if set, else pass the byte through unchanged. `table==NULL` is a fatal
internal error (`exit(EXIT_FAILURE)`), not a NULL return.

Output buffer sizing: `strlen(filename) * table->max_data_length + 1` — i.e. it assumes every input
byte could expand to the table's single longest replacement string; this is always big enough
because expansion happens per-byte and never per-multi-byte-cluster.

Builtin `safe` table has **no `default`** directive
[table/safe.tbl:11-14](https://github.com/dharple/detox/blob/0a8e212/table/safe.tbl#L11-L14) — bytes
not explicitly listed pass through unchanged, confirmed by test:
`,comma` → `,comma`, `%percent` → `%percent`, `+plus` → `+plus`
[tests/unit/test_clean_safe.c:46,59-60](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_safe.c#L46).

Worked example (builtin safe table): `&ampersand` → `_and_ampersand` (0x26 maps to the literal
string `_and_`, not a single char) [table/safe.tbl:94](https://github.com/dharple/detox/blob/0a8e212/table/safe.tbl#L94),
verified by test [tests/unit/test_clean_safe.c:37](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_safe.c#L37).
High bytes (0x80-0xFF, i.e. Latin-1/UTF-8 continuation bytes) are _not_ in the safe table at all, so
they pass through raw — `clean_safe` runs _after_ `iso8859_1`/`utf_8` in every shipped sequence for
exactly this reason.

### 3.2 `iso8859_1` — `clean_iso8859_1`

[src/clean_string.c:29-80](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L29-L80)

For each byte: if the high bit (`0x80`) is clear, copy through unchanged. If set, treat the byte
value itself (0x80-0xFF) as the table key. This means it operates on raw bytes ≥0x80 whether or not
the input is validly encoded anything — it does not attempt to detect or skip multi-byte UTF-8
sequences, it treats every high-bit byte independently.

Builtin `iso8859_1.tbl` has `default _` [table/iso8859_1.tbl:18](https://github.com/dharple/detox/blob/0a8e212/table/iso8859_1.tbl#L18)
— any 0x80-0xFF byte not individually listed collapses to `_`. Table only defines rows for
0xA0-0xFF [table/iso8859_1.tbl:28 onward](https://github.com/dharple/detox/blob/0a8e212/table/iso8859_1.tbl#L28), so 0x80-0x9F (the C1
control range, undefined in ISO-8859-1) always hit the `default` and become `_`.

Worked example: byte `0xA9` (©, ISO-8859-1) → table row emits the 2-byte UTF-8 encoding of U+00A9
(`©`, i.e. bytes `0xC2 0xA9`) [table/iso8859_1.tbl:37](https://github.com/dharple/detox/blob/0a8e212/table/iso8859_1.tbl#L37). Note this
builtin row's _source_ text (in `table/iso8859_1.tbl` and, after generation, in `src/builtin_table.c`)
is the C-escape literal spelled out as six ASCII characters (backslash, `u`, `0`, `0`, `A`, `9`), not a raw © glyph — see §4's `\uXXXX` discussion for why that's
only decoded at compile time, yet still ends up as real UTF-8 bytes in the shipped binary.

### 3.3 `cp1252` — same filter, different table

There is no separate CP-1252 filter/cleaner; `cp1252` is only a builtin _table_ selected via
`iso8859_1 { builtin "cp1252"; };` [src/filter.c:119-121](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L119-L121).
It reuses `clean_iso8859_1` verbatim.

Worked example, dedicated CP-1252 test fixture: byte `0x81` (undefined in both CP-1252 and
Latin-1) → `-` (an explicit table row, not the table's `default`, since `cp1252.tbl`'s `default`
is commented out — see §5)
[tests/unit/test_clean_iso8859_1_cp1252.c:50-52](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_iso8859_1_cp1252.c#L50-L52).
Same fixture also confirms `0x80` (EURO SIGN, CP-1252-specific — undefined in Latin-1) → `€ euro`
and `0x97` (EM DASH, also CP-1252-specific) → `— em dash`, both mapped to their multi-byte UTF-8
equivalents rather than collapsed to `_`
[tests/unit/test_clean_iso8859_1_cp1252.c:40-46](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_iso8859_1_cp1252.c#L40-L46).

### 3.4 `utf_8` — `clean_utf_8`

[src/clean_utf_8.c:60-206](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L60-L206)

This is the most algorithmically involved filter. Per character:

1. `get_utf_8_width()` classifies the lead byte into a 1-6 byte UTF-8 sequence length (supports the
   old pre-RFC3629 5/6-byte extended forms) or returns `-1` for an invalid lead byte
   [src/clean_utf_8.c:217-232](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L217-L232).
   `-1` → emit `_` (the module-level `invalid_replacement`), advance one byte, continue
   [src/clean_utf_8.c:111-116](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L111-L116).
2. Unpack the lead byte's payload bits, then consume `width-1` continuation bytes, shifting 6 bits
   per byte. If a continuation byte is missing (string ends, or the byte isn't `10xxxxxx`), abort
   the sequence, emit `_`, and resume scanning from the _next_ byte after the point of failure
   (not from the start of the failed sequence) [src/clean_utf_8.c:123-148](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L123-L148).
3. Table lookup by decoded Unicode code point (not by byte length or byte sequence).
4. Special-cased results, checked in this order, before falling to `table->default_translation`:
   - decoded value `0` (a UTF-8-encoded NUL, e.g. `0xC0 0x80`) and no table entry → replaced with
     the literal string `_hidden_null_` [src/clean_utf_8.c:164-167](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L164-L167).
   - decoded value `> 0x10FFFF` (outside valid Unicode) and no table entry → replaced with `_`
     [src/clean_utf_8.c:174-177](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L174-L177).
   - otherwise fall to `table->default_translation`, and if that's also NULL, **re-emit the
     original encoded bytes unchanged** (walks `input_walk` back by `characters_eaten` and copies
     them through byte-for-byte) [src/clean_utf_8.c:183-196](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L183-L196).

This "leave it alone" fallback preserves overlong/legacy encodings — e.g. a 2-byte overlong encoding
of ASCII `0` (`0xC0 0xB0`, decoded value 0x30) is _not_ rejected as invalid; it's looked up in the
table like any other code point and — if unmapped and no default — even the overlong _encoding_ survives untouched. The builtin `unicode.tbl` maps this class of overlong-ASCII code points directly
to their single-byte ASCII form (e.g. `0x0030 → "0"`), collapsing overlong encodings of printable
ASCII into plain ASCII. **The normalization comes from the explicit identity row, not from any
`default` setting**: `0x0030 0` is a real row in `unicode.tbl`, so the collapse happens whether or
not the table has a `default`. (`default` only matters for code points with _no_ row at all — see
§3.4's fallback ladder, where an unmapped code point with no default is re-emitted in its original
encoding, overlong included.) The cited test happens to run with a `default` set, which is
incidental to the mechanism: test `\xC0\xB0` (2-byte-encoded `0`) → `0`
[tests/unit/test_clean_utf_8.c:111-115](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_utf_8.c#L111-L115) (adjacent cases at lines 117-127 cover `\xC0\xA0`→space and the
`0xC1`-lead-byte overlong forms `0x7E`/`0x7F`). This is intentional per the file's own comment: "detox should convert
multibyte versions of them to single-byte versions" [table/unicode.tbl:20-25](https://github.com/dharple/detox/blob/0a8e212/table/unicode.tbl#L20-L25).

Builtin `unicode.tbl` has **no `default`** directive (grep of the file confirms no top-level
`default` line) — unmapped code points pass through unchanged by default, matching the man page's
"any unknown character should fall through to the next filter" semantics
[man/detox.tbl.5:27-30](https://github.com/dharple/detox/blob/0a8e212/man/detox.tbl.5#L27-L30). Tests
explicitly install a temporary `default = "_"` (table_a) to exercise that path
[tests/unit/test_clean_utf_8.c:186-190](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_utf_8.c#L186-L190).

**Table-authoring hazard — `default` applies to ASCII too, and `unicode.tbl` only survives it
because it identity-maps all of printable ASCII.** `clean_utf_8` calls `table_get()` for _every_
decoded code point, unconditionally, with no ASCII short-circuit
[src/clean_utf_8.c:154](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L154). So a
table with a `default` set and no row for `0x0041` replaces a literal `A` with the default. The
shipped `unicode.tbl` never exhibits this because it carries **96 explicit rows for `0x0020`-`0x007F`**:
all 95 printable ASCII code points mapped to themselves (`0x0041 A`, `0x0061 a`, `0x0030 0`,
`0x0020 " "`, …) plus `0x007F` → `"_"`
[table/unicode.tbl:63-164](https://github.com/dharple/detox/blob/0a8e212/table/unicode.tbl#L63-L164).
Those rows exist for the overlong-collapse reason above, but their _second_ effect is to make the
table safe to give a `default`. **Empirically confirmed**: a synthetic table with `default "_"` and
no ASCII rows, loaded via `filename` on `utf_8`, turned `"® reg"` into `"_____"` — every character
replaced, including the space and the letters — where the builtin table leaves `"® reg"` untouched.
Anyone designing a table format (or a default-replacement policy) from this document must not assume
ASCII passes through a `default`-bearing table unscathed; upstream's default translation is uniform
over the entire code-point space.

Worked example: `"® reg"` (® , U+00AE, 2-byte UTF-8) run against the unicode table with a
forced `default="_"` → `"_ reg"` — the whole 2-byte sequence collapses to one underscore
[tests/unit/test_clean_utf_8.c:47-49](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_utf_8.c#L47-L49).
Worked example (default=NULL, no builtin table entry): same input passes through byte-for-byte
unchanged [tests/unit/test_clean_utf_8.c:48-49](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_utf_8.c#L48-L49).

Output buffer sizing uses the same `strlen(input) * max_data_length + 1` formula as `safe`/`iso8859_1`
— sized per raw input _byte_, not per decoded character, which is always sufficient since a
multi-byte input sequence produces at most one replacement string.

### 3.5 `uncgi` — `clean_uncgi`

[src/clean_string.c:143-175](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L143-L175)

Not table-driven. Two rules, checked left to right per position:

- `%XX` where both `X` are hex digits (`isxdigit`) → decoded byte value (`strtol` base 16), consumes
  3 input chars. If either following char isn't a hex digit, the `%` is left as a literal `%` and
  only 1 char is consumed (no error, no warning).
- `+` → space (classic CGI form-encoding), 1-for-1.
- Everything else copied verbatim.

Worked example: `%3Dequals` → `=equals`; `here+and+there` → `here and there`
[tests/unit/test_clean_uncgi.c:30-42](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_uncgi.c#L30-L42).
No lowercase/uppercase constraint on the hex digits (`isxdigit` accepts both).

### 3.6 `lower` — `clean_lower`

[src/clean_string.c:318-342](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L318-L342)

`isupper`/`tolower` per byte, C-locale ASCII only ("only works on ASCII characters" per man page
[man/detoxrc.5:174-176](https://github.com/dharple/detox/blob/0a8e212/man/detoxrc.5#L174-L176)) — no
option, no table, always runs to completion, never fails.

Worked examples: `L0W3R` → `l0w3r`, `UPPER` → `upper`, `UPPer_2` → `upper_2` (digits/underscores
untouched, only ASCII letters affected)
[tests/unit/test_clean_lower.c:33-36](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_lower.c#L33-L36).

### 3.7 `wipeup` — `clean_wipeup`

[src/clean_string.c:194-245](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L194-L245)

Algorithm:

1. Strip any run of `-`, `_`, `#` from the very start of the string first, unconditionally, before
   any other processing (loop, not table lookup) [src/clean_string.c:204-206](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L204-L206).
2. Build a "search" set: `"-_"` normally, or `".-_"` if `remove_trailing` is set
   [src/clean_string.c:210](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L210) —
   note the _order_ of this string is the precedence order used in step 3.
3. Walk the (already-stripped) string. Any char in the search set is _not_ copied immediately;
   instead its position in the search string is compared (`seek < current`) against a
   previously-held candidate for the current run, and the _earliest-appearing-in-`search`_ one wins
   — i.e. with `remove_trailing` unset, `-` (search[0]) always beats `_` (search[1]); with it set,
   `.` (search[0]) beats `-` beats `_`. This is why the man page says "dash takes precedence" (resp.
   "period ... then dash") [man/detoxrc.5:154-161](https://github.com/dharple/detox/blob/0a8e212/man/detoxrc.5#L154-L161).
4. On the first non-search-set char after a run, flush the winning representative char, then copy
   the current char. A run at the very end of the string also gets flushed after the loop
   [src/clean_string.c:236-238](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L236-L238) — i.e. trailing separator runs are _not_ deleted, they are collapsed to one char, same as internal runs.

Worked example (from man page): not literally wipeup, but wipeup's own doc example: default mode,
`_-_-_-_-_-dotted-_-_-_-_line.....part......two.......` → leading `_-...` run stripped by step 1 down
to `dotted-_-_-_-_line.....part......two.......`, then internal mixed `-`/`_` runs collapse to `-`
(dash wins) and `.` runs are _not_ touched (not in search set without remove_trailing) →
`dotted-line.....part......two.......`
[tests/unit/test_clean_wipeup.c:76-78](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_wipeup.c#L76-L78).
With `remove_trailing=1` the same input → `dotted-line.part.two.` — periods now collapse too, and
period wins over dash within any run that mixes them
[tests/unit/test_clean_wipeup.c:76-78](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_wipeup.c#L76-L78) (`expected_b`).

### 3.8 `max_length` — `clean_max_length`

[src/clean_string.c:250-309](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L250-L309)

- `max_length <= 0` is treated as "unset", coerced to `256` [src/clean_string.c:262-264](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L262-L264)
  — confirmed by test: `max_length = 0` on a 21-char filename is a no-op
  [tests/unit/test_clean_max_length.c:136-140](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_max_length.c#L136-L140). There is no way to configure a true "no limit"; 0 silently becomes 256, not infinite.
- If `strlen(filename) <= max_length`, return an unmodified copy — no-op, never even looks at the
  extension logic [src/clean_string.c:266-269](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L266-L269).
- Otherwise: truncate to exactly `max_length` bytes first (`snprintf`), then find the extension via
  `strrchr(filename, '.')` **on the original filename**, not the truncated copy
  [src/clean_string.c:271-276](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L276).
  If there's no `.` at all, or the "extension" is just a lone trailing `.` (`strlen(extension)==1`),
  the plain truncation stands, no extension preservation [src/clean_string.c:277-283](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L277-L283).
- **Lookback-for-double-extension**: starting just before the last `.`, walk backward up to 5
  characters looking for _another_ `.` (e.g. to catch `.tar.gz`); if found within that 5-char
  window, that earlier `.` becomes the new "extension" start instead
  [src/clean_string.c:285-295](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L285-L295). 5 is a hardcoded constant, not configurable, and it counts characters _between_ the
  dot-before-last-dot and the last dot (i.e. covers extensions like `.tar.gz` — 4 chars between —
  but not longer compound extensions).
- If the (possibly extended) extension's length is `>= max_length`, give up entirely and return the
  **original, untruncated** filename with a stderr warning
  [src/clean_string.c:297-302](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L297-L302) — this is a case where the filter can produce output _longer_ than `max_length`.
- Otherwise: splice `extension` onto the truncated output at `max_length - extension_length`.

Worked examples straight from the test fixture / man page:

- `this_is_my_file.txt`, max 12 → `this_is_.txt` (man-page example)
  [tests/unit/test_clean_max_length.c:56-60](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_max_length.c#L56-L60).
- `safe.tar.gz`, max 32 → unchanged (shorter than max, no-op).
- `safe and stuff.tar.gz`, max 20 → `safe and stuf.tar.gz` — 5-char lookback finds the `.tar` dot
  before `.gz`, so `.tar.gz` (7 chars) is preserved as the extension
  [tests/unit/test_clean_max_length.c:106-110](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_max_length.c#L106-L110).
- `safe and stuff.tar.gz`, max 7 → unchanged, because required extension length (7, `.tar.gz`) is
  not `< max_length` (7) → give-up path, warning printed, original returned
  [tests/unit/test_clean_max_length.c:130-134](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_clean_max_length.c#L130-L134).

---

## 4. `.tbl` file format — grammar from `parse_table.c`

Parser: [src/parse_table.c:27-202](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L27-L202). Line-oriented, read with `fgets` into a 1024-byte buffer (**a table-file line longer than
1023 bytes will be silently truncated/split across the next `fgets` call** — no protection against
this) [src/parse_table.c:80,86](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L86).

Two states: `BASE_STATE` (outside any `start`/`end` block) and `INSIDE_STATE` (between them)
[src/parse_table.c:22-25](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L22-L25).

Grammar, line by line:

- `#` as the _first character of the line_ → whole line ignored (comment). Note: this is a
  first-character check (`*work == '#'`), not "contains a `#`" — a line like `  # comment` (leading
  whitespace) is **not** treated as a comment by this check and instead falls into normal parsing
  (in practice it fails `sscanf`'s token match harmlessly, so it's still effectively ignored, but
  via a different path than true full-line comments). Trailing `# comment` after real content on a
  data line is _not_ stripped either — the value token is captured via `%s`/`%[^"]` which stops at
  whitespace/quote-close, so trailing comments after unquoted or quoted values are naturally excluded from the captured value, but there's no explicit comment-strip pass for the rest of the line.
  [src/parse_table.c:87-93](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L87-L93)
- `start` (BASE_STATE only, case-insensitive `strncasecmp(...,5)`): with nothing after it → enter
  `INSIDE_STATE` unconditionally (applies to "all languages"). With a following token → that token
  is compared case-insensitively against the current `LC_CTYPE` locale string (`setlocale(LC_CTYPE, "")`); if it's a case-insensitive _prefix match_ (`strncasecmp(parsed, system_ctype, strlen(parsed))`)
  of the locale string, enter `INSIDE_STATE`, else the whole block is skipped (state stays
  `BASE_STATE`, and note there's no explicit "skip to matching end" — it just doesn't transition, so all lines are re-parsed as BASE_STATE tokens until an `end` is seen, which is harmless because non-start/default keywords in BASE_STATE are silently ignored)
  [src/parse_table.c:104-127](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L104-L127).
  This is a **prefix match with the match direction arguably backwards**: `strncasecmp` is bounded
  by `strlen(parsed)` (the language token's length), so `start en` matches locales `en`, `en_US`,
  `english`, etc. — any locale string that merely _starts with_ `en`.
- `default` (BASE_STATE only): with nothing after it → `default_translation = NULL` (explicit no-op,
  same as never writing the field, since `table_t` is zero-initialized)
  [src/parse_table.c:131-134](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L131-L134). With a value → parsed the same way as a data-row value (see below) and stored as
  `default_translation`.
- Inside `INSIDE_STATE`, a line starting with a value scanned by `%i` (accepts decimal, `0x`-hex, or
  `0`-octal, per `sscanf`/`strtol` rules — man page confirms "decimal (1), hex (0x01) or octal (01)"
  [man/detox.tbl.5:40-42](https://github.com/dharple/detox/blob/0a8e212/man/detox.tbl.5#L40-L42)) is a
  data row: `<code> <value>` where `<value>` is either a bare whitespace-delimited token (`%s`) or,
  if the first non-space char after the code is `"` or `'`, everything up to the matching closing
  quote (`%[^"]` / `%[^']` — **no escape mechanism for embedding the same quote character inside a
  quoted value**, and no requirement that the closing quote actually be present: if absent, the scan
  just captures to end of the 1024-byte buffer). If `<value>` is entirely absent (EOL right after the
  code, `work[offset]=='\0'`), the line is silently skipped — **no way to map a code to the empty
  string via a bare (unquoted) line**; empty-string mappings require `0x2060 ""` (an explicitly
  quoted empty string, as used in `unicode.tbl`) [src/parse_table.c:174-184](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L174-L184), [table/unicode.tbl (WORD JOINER row)].
- A line inside `INSIDE_STATE` that is _not_ a valid `%i`-scannable code is checked for the literal
  token `end` (case-insensitive, 5-char compare — meaning `endless` would also match since
  `strncasecmp(...,5)` only compares 5 chars, not full-token) → returns to `BASE_STATE`. Any other
  unparseable line inside a block is silently ignored, not an error
  [src/parse_table.c:160-172](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L160-L172).
- `code` must be `>= 0`; `table_put` further requires `key != 0` — **code `0` can never be stored**
  (silently dropped, `table_put` returns -1 but the caller checked `ret == -1` after the fact only
  reports an error for genuinely full/duplicate-slot tables, not for `key==0`... actually re-reading: `table_put` returns -1 for key==0, and `parse_table.c` treats any `-1` from `table_put` as fatal,
  freeing the table and returning NULL.) [src/table.c:122-124](https://github.com/dharple/detox/blob/0a8e212/src/table.c#L122-L124), [src/parse_table.c:186-194](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L186-L194) — **so a `0x0000` row in a `.tbl` file makes the whole table fail to load**, silently disabling that filter's config-file table and forcing fallback to the builtin.
- **No `\uXXXX`/`\xXX` escape interpretation happens at table-parse time at all.** The value string
  captured by `sscanf` (whether quoted or bare) is stored _verbatim_ as bytes — `"¡"` in a
  `.tbl` file is stored as the **literal 6 ASCII characters** `\`, `u`, `0`, `0`, `A`, `1`, not as the
  Unicode character U+00A1. This is directly checked: `parse_table` has no `\u`/`\x` handling
  anywhere in [src/parse_table.c](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c), and `generate_builtin_table.c`'s own `escape_string()` explicitly says "leave `\u` and `\U` alone"
  when re-escaping a parsed value for embedding as a C string literal
  [src/generate_builtin_table.c:43-49](https://github.com/dharple/detox/blob/0a8e212/src/generate_builtin_table.c#L43-L49) — i.e. the `\uXXXX` escape is only ever interpreted by the **C
  compiler**, at build time, when the generated `builtin_table.c` is compiled. **Correction:** the
  generated file's _source text_ still spells out the six-character escape sequence itself — it is
  NOT pre-decoded into a UTF-8 glyph by any detox tooling. Confirmed by inspecting the actual generated
  line, [src/builtin_table.c:102](https://github.com/dharple/detox/blob/0a8e212/src/builtin_table.c#L102):
  `{ .key = 0x00a1, .data = "¡" }` — that `.data` value, as C source text, is literally
  `\`, `u`, `0`, `0`, `A`, `1` (six ASCII characters), identical in form to what a raw `.tbl` file
  would contain. The only thing that differs at runtime is that this text sits inside a _C string
  literal_ that gets fed to a C compiler (producing the real 2-byte UTF-8 encoding of U+00A1 in the
  compiled `.o`/binary), whereas the same six characters typed into a user's `.tbl` file loaded via
  `filename "..."` are handed to `parse_table`'s `sscanf`, which performs no such decoding and
  stores them as literal bytes. **Practical consequence: if a user writes `¡` in their own `.tbl`
  file loaded via `filename "..."` at runtime, it will NOT be decoded — the literal backslash-u-etc.
  text will be spliced into the output filename.** Users must write the raw UTF-8 bytes directly in
  their table file (in the appropriate locale/encoding) to get a real Unicode character out.
  **Empirically confirmed**: loading a table containing a `0x00A1 "¡"` row via `filename` and running
  detox against a file whose name contains the actual UTF-8 byte pair `0xC2 0xA1` (¡) produces the
  literal six-character text `¡` in the output filename, not the original ¡ glyph and not any other
  decoded form.
- The file itself is read with plain `fgets`/`sscanf`, so it is encoding-agnostic at the syntax level
  — only the numeric code and the literal replacement bytes matter; the `.tbl` file's own encoding
  must match whatever encoding the _filter_ is decoding filenames as (e.g. UTF-8 for `unicode.tbl`,
  raw Latin-1/output-UTF-8 bytes for `iso8859_1.tbl`).

`end` without a matching `start` is a no-op (state is already `BASE_STATE`, the `end` check only
runs from `INSIDE_STATE`). Multiple `start`/`end` blocks (e.g. one default + one `start en`) are
each independently evaluated: [tests/legacy/man-page-sequence-with-language/safe-manpage.tbl] and
`man/detox.tbl.5`'s own worked example demonstrate exactly this — a generic `start`/`end` block
followed by a `start en`/`end` block that overrides `0x24` (`$`) if the locale is English
[man/detox.tbl.5:66-84](https://github.com/dharple/detox/blob/0a8e212/man/detox.tbl.5#L66-L84).

Table sizing: `table_init` picks `max(500, file_size/6)` initial hash-table rows
[src/parse_table.c:54-69](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L54-L69),
[src/table.c:21-44](https://github.com/dharple/detox/blob/0a8e212/src/table.c#L21-L44). Lookup uses a
modulo hash by code point with linear-scan fallback on collision or when `use_hash` is off
[src/table.c:100-221](https://github.com/dharple/detox/blob/0a8e212/src/table.c#L100-L221). **If the
table fills up (`table->length == table->used`) before all rows are inserted, `table_put` fails
and the _entire_ `.tbl` file fails to parse** (returns NULL, triggering builtin fallback or fatal
exit depending on caller) [src/table.c:126-128](https://github.com/dharple/detox/blob/0a8e212/src/table.c#L126-L128) — this is generally avoided in practice by the `file_size/6` sizing heuristic (each `code value` line is rarely shorter than 6 bytes), but a pathological very-dense/very-short-line table could hit this.
`table.c` does have a `table_resize()` helper that can grow a table by copying its rows into a
larger freshly-`table_init`'d one [src/table.c:46-72](https://github.com/dharple/detox/blob/0a8e212/src/table.c#L46-L72), but `parse_table.c`'s runtime `.tbl`-loading path **never calls it** —
every caller of `table_resize` is either the `generate_builtin_table`/`check-table` build-time
tooling or the builtin-table loaders in `builtin_table.c` re-sizing a static compiled-in array at
startup. So the "table fills up → whole file fails" failure mode above is a real, unmitigated risk
for user-supplied `.tbl` files, not something silently absorbed by an auto-grow path.

**Empirically confirmed, and this is a defect `detoxrs` should deliberately not reproduce.** A
601-row table whose lines are short enough to keep `table_init`'s `max(500, file_size/6)` estimate
below the row count (601 rows in a 3508-byte file → capacity 584) fails at exactly the 585th row:

```
$ printf 'sequence b { utf_8 { filename ".../big.tbl"; }; };' > rc
$ printf 'hello\n' | inline-detox -f rc -s b
Failed to add row 0x0249 "a" to translation table
detox: unable to parse file: ".../big.tbl"
$ echo $?
1
```

`0x0249` is 585 decimal — capacity plus one. Both `detox` and `inline-detox` exit 1 and process no
input at all. The user-visible failure is a valid, well-formed table file being rejected for being
_dense_ rather than wrong, with an error message that names a row rather than the size limit that
was hit, and no way to raise the limit. A successor should either grow the structure on demand (the
obvious choice in Rust — a `HashMap` has no such ceiling) or, if a bound is wanted for
resource-safety reasons, state the limit up front and report exceeding it as "table too large:
N rows, limit M", never as a per-row insert failure. Note also that upstream _has_ the resize
primitive and simply doesn't wire it into this path, so this is an oversight in upstream, not a
considered design constraint.

## 5. Shipped table files (`table/`)

| File                         | Lines | `default`                          | Purpose                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| ---------------------------- | ----- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `table/safe.tbl`             | 96    | none (unmapped bytes pass through) | Builtin `safe` table. Maps ASCII control chars 0x01-0x1F, DEL (0x7F), and shell-special punctuation to `_` or `-`; `&` (0x26) specially maps to `_and_` (multi-char). [table/safe.tbl](https://github.com/dharple/detox/blob/0a8e212/table/safe.tbl)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| `table/iso8859_1.tbl`        | 130   | `_`                                | Builtin `iso8859_1` table. Maps 0xA0-0xFF (Latin-1 Supplement) to `\uXXXX`-escaped UTF-8 (compiled) equivalents; a handful (e.g. NBSP 0xA0 → `" "`) map to plain ASCII. 0x80-0x9F implicitly hit `default` → `_`. [table/iso8859_1.tbl](https://github.com/dharple/detox/blob/0a8e212/table/iso8859_1.tbl)                                                                                                                                                                                                                                                                                                                                                                                                                         |
| `table/cp1252.tbl`           | 64    | commented out (none)               | Only covers 0x80-0x9F (the CP-1252-specific block that differs from Latin-1) — meant to run _before_ `iso8859_1(builtin=iso8859_1)` in a chain so 0xA0-0xFF still gets handled by the latter. [table/cp1252.tbl](https://github.com/dharple/detox/blob/0a8e212/table/cp1252.tbl)                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| `table/unicode.tbl`          | 221   | none                               | Builtin `utf_8` table. Covers ASCII control range 0x01-0x1F → `_` and `0x7F` (DEL) → `_`; **identity-maps the entire printable ASCII range — 95 rows for `0x0020`-`0x007E`, each code point mapped to itself** (96 rows total for `0x0020`-`0x007F`), which is what makes multi-byte/overlong encodings of ASCII collapse to single bytes _and_ what would make the table safe to give a `default` (see the hazard note in §3.4 — a `default` applies to ASCII too); plus a broad swath of Unicode punctuation/dashes/space variants (e.g. EM DASH U+2014 → `-`, several space-like chars → `" "`), and WORD JOINER U+2060 → `""` (deletion). [table/unicode.tbl](https://github.com/dharple/detox/blob/0a8e212/table/unicode.tbl) |
| `table/c_escape.tbl`         | 57    | n/a (not a filter table)           | Not consumed by any filter at runtime — escapes 0x01-0x7F to C-style `\xNN`/`\a`/`\t` etc. Used only as reference data for tooling ([src/generate_builtin_table.c], [src/table_dump.c] — dump/debug utilities), out of this doc's filter scope. [table/c_escape.tbl](https://github.com/dharple/detox/blob/0a8e212/table/c_escape.tbl)                                                                                                                                                                                                                                                                                                                                                                                             |
| `table/legacy/cp1252.tbl`    | 57    | —                                  | Pre-refactor version of `cp1252.tbl`, kept for compatibility/reference; not loaded by any current builtin path.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| `table/legacy/iso8859_1.tbl` | 137   | —                                  | Pre-refactor version of `iso8859_1.tbl`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| `table/legacy/unicode.tbl`   | 801   | —                                  | Older, much larger unicode table (801 vs 221 lines) — earlier detox versions transliterated far more of Unicode (accented Latin → base letter, etc.) than the current slimmer `unicode.tbl`, which only targets control/space/dash/punctuation classes. **[UNVERIFIED]** exact diff of what was dropped between legacy and current — would need a line-by-line diff to characterize precisely.                                                                                                                                                                                                                                                                                                                                     |
| `table/legacy/unidecode.tbl` | 9089  | —                                  | A full Text::Unidecode-derived transliteration table (credited in `unicode.tbl`'s header comment) — not wired into any current filter; historical/reference only.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |

## 6. Encoding/invalid-byte edge cases worth flagging for `detoxrs`

- **`iso8859_1`/`safe` never validate encoding** — they process raw bytes; there is no concept of
  "invalid" input for these two filters, every byte 0x00-0xFF has defined behavior (pass-through,
  table hit, or default).
- **`utf_8` is the only filter with a true "invalid" path** — invalid lead bytes, truncated
  sequences, and missing continuation bytes are replaced 1-for-1 with `_` and scanning resumes
  immediately after the failure point (not after a full sequence-width skip), so a stream of
  contiguous invalid bytes produces one `_` per byte, not one `_` per attempted-sequence
  [src/clean_utf_8.c:111-148](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L111-L148).
- **Overlong encodings are accepted and normalized** (see §3.4) rather than rejected — a deliberate
  design choice per the `unicode.tbl` header comment, not a bug, but a security-relevant divergence
  from strict UTF-8 validators (overlong encodings are a classic bypass vector for other filters
  downstream; detox's own `safe`/`wipeup` still run after normalization in every shipped sequence,
  which mitigates this in practice).
- **Codepoints beyond `0x10FFFF`** (only reachable via the legacy 5/6-byte forms) are forced to `_`
  unconditionally, table lookup notwithstanding, if untabled [src/clean_utf_8.c:174-177](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L174-L177).
- **A UTF-8-encoded NUL is specifically caught and defanged** even when a table's default is unset,
  via the hardcoded `_hidden_null_` string, to prevent a NUL from ever entering the output filename
  [src/clean_utf_8.c:164-167](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L164-L167) — this is a case where table configuration is deliberately overridden by hardcoded C logic for safety, worth preserving in `detoxrs`.
- **`.tbl` file `\u`/`\x` escapes do not work at runtime**, only at compile-time for the four
  builtins (see §4) — a `detoxrs` implementation that wants user-supplied `.tbl` files to support
  `\uXXXX` would be a deliberate _behavioral improvement/divergence_ from upstream detox, not parity;
  worth calling out explicitly if adopted, since it changes what a hand-written table file means.

## 7. Open items marked [UNVERIFIED]

- Precise semantic diff between `table/legacy/unicode.tbl` (801 lines) and `table/unicode.tbl` (221
  lines) — would require a full line-by-line diff against the legacy file's git history to state
  authoritatively "what got removed and why" rather than the general characterization above.
- Behavior when a `.tbl` file has a data line whose value is a quoted string with no closing quote
  before EOF: traced from `sscanf("%[^\"]", ...)` semantics (captures to end of 1024-byte `work`
  buffer) but not exercised by any test in `tests/`; would need a constructed fixture + `check-table`
  run to confirm no crash/UB.

## Validation log (stage 2)

Adversarial re-check against the same pinned clone/commit, with a compiled `src/detox`/`src/check-table`.
Every file+line citation in the document was opened and diffed against the cited line range; the
five load-bearing algorithmic claims were re-derived independently from the code (not from this
document's own description) and, where practical, exercised empirically against the compiled binary.

| Claim                                                                                                                                                                 | Verdict                                    | Evidence                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| (a) `\uXXXX` in `.tbl` files interpreted only at build time by the code generator, never by the runtime parser                                                        | CONFIRMED                                  | Read-only: no `\u`/`\x` handling anywhere in `src/parse_table.c` (full file); `generate_builtin_table.c:43-49` `escape_string()` explicitly leaves `\u`/`\U` alone. **Empirically confirmed**: a `.tbl` file with a `0x00A1 "¡"` row, loaded via `filename`, left the literal 6-character escape text in the output filename when run against a real UTF-8 `¡` input byte pair — see §4. |
| (b) UTF-8 decoder accepts overlong/non-minimal sequences and maps them to the same code point rather than rejecting them                                              | CONFIRMED, with a corrected worked example | Read-only: `get_utf_8_width`/continuation-unpacking in `clean_utf_8.c` never checks minimality. The document's original worked example cited the wrong bytes (`0xC1 0xB0`, which actually decodes to `0x70`/'p'); corrected to `0xC0 0xB0` (decodes to `0x30`/'0'), matching `tests/unit/test_clean_utf_8.c:111-115`.                                                                    |
| (c) `clean_max_length` uses a hardcoded 5-char backward window from the last `.`, can bail out and return the original unmodified string                              | CONFIRMED                                  | Read-only: `src/clean_string.c:250-309`, hardcoded `5` at the `extension - input_walk > 5` check (line 287), give-up path at lines 297-302 returns `wrapped_strdup(filename)` (the original, untruncated string). **Empirically confirmed**: `safe and stuff.tar.gz` at `max_length 7` produced a stderr warning and an unchanged filename.                                              |
| (d) `-`/`_`/`.` run-collapsing precedence comes from index order inside a literal search string, not an explicit priority table                                       | CONFIRMED                                  | Read-only: `src/clean_string.c:194-245`, `search = wrapped_strdup(remove_trailing ? ".-_" : "-_")` (line 210) then `strchr`/pointer-position comparison (lines 217-220) — no lookup table anywhere. **Empirically confirmed**: default and `remove_trailing` runs against the man-page example produced the exact strings the document claims.                                           |
| (e) UTF-8-encoded NUL replaced with literal `_hidden_null_` via hardcoded C logic, not the translation table                                                          | CONFIRMED                                  | Read-only: `static char *null_replacement = "_hidden_null_";` at `src/clean_utf_8.c:22`, applied at lines 164-167 only when no table entry and no default exist. String spelling and case are exact.                                                                                                                                                                                     |
| Filter catalog / enum / dispatch switch (§1)                                                                                                                          | CONFIRMED                                  | Read-only: `src/detox_struct.h:17-25`, `src/config_file_lex.l:28-40`, `src/filter.c:207-239` all match cited ranges exactly; all 7 filters dispatch correctly.                                                                                                                                                                                                                           |
| Builtin-table fallback / hard `filename` reference (§1)                                                                                                               | CONFIRMED                                  | Read-only: `filter_load_table` is exactly `src/filter.c:131-184`; explicit `filename` takes the `do_search=0` branch (156-181), fatal `exit(EXIT_FAILURE)` on parse failure confirmed at 174-181.                                                                                                                                                                                        |
| Default sequence and all 11 named sequences (§2)                                                                                                                      | CONFIRMED                                  | Read-only: every sequence block in `etc/detoxrc` transcribed verbatim and verified filter-by-filter; no omissions or wrong orderings found.                                                                                                                                                                                                                                              |
| `sequence_choose_default` fallback-to-first-sequence / linked-list short-circuit-on-NULL (§2)                                                                         | CONFIRMED                                  | Read-only: `src/sequence.c:28-54` and `:98-124` match as described.                                                                                                                                                                                                                                                                                                                      |
| `wipeup` worked examples (default and `remove_trailing`)                                                                                                              | CONFIRMED, empirically                     | Both `dotted-line.....part......two.......` and `dotted-line.part.two.` reproduced exactly by running the compiled binary against the man-page example string.                                                                                                                                                                                                                           |
| `uncgi` worked examples (`%3Dequals`, `here+and+there`)                                                                                                               | CONFIRMED, empirically                     | Reproduced exactly by the compiled binary.                                                                                                                                                                                                                                                                                                                                               |
| `max_length` worked examples (max 12/20/7/0)                                                                                                                          | CONFIRMED, empirically                     | All four reproduced exactly, including the max-0-is-a-no-op case and the max-7 give-up-with-warning case.                                                                                                                                                                                                                                                                                |
| `table/safe.tbl` `&` → `_and_` example                                                                                                                                | CORRECTED (line drift)                     | Cited as `:91-93`; actual data row is at `:94` (91-93 is the preceding comment block).                                                                                                                                                                                                                                                                                                   |
| `table/iso8859_1.tbl` `default _` line and data-row start                                                                                                             | CORRECTED (line drift)                     | `default _` is at `:18`, not `:19`; first data row (`0x00A0`) is at `:28`, not `:29-30`.                                                                                                                                                                                                                                                                                                 |
| `table/cp1252.tbl` commented-out `default`                                                                                                                            | CORRECTED (line drift)                     | Actual `# default` line is at `:22`, not in the `:16-19` explanatory-comment range originally cited.                                                                                                                                                                                                                                                                                     |
| Generated `src/builtin_table.c` entry for `0x00a1`                                                                                                                    | CORRECTED (illustrative detail)            | Line number (102) and the fact that the source text still spells out the escape sequence (not a pre-decoded glyph) were clarified; the underlying "compiler decodes it" mechanism was already correct.                                                                                                                                                                                   |
| `cp1252` filter has no dedicated worked example (§3.3)                                                                                                                | CORRECTED (omission)                       | Added citation to `tests/unit/test_clean_iso8859_1_cp1252.c:40-52` (0x80 EURO SIGN, 0x81 undefined→`-`, 0x97 EM DASH).                                                                                                                                                                                                                                                                   |
| `lower` filter has no test citation (§3.6)                                                                                                                            | CORRECTED (omission)                       | Added citation to `tests/unit/test_clean_lower.c:33-36`.                                                                                                                                                                                                                                                                                                                                 |
| `table_resize()` exists but is never called from the runtime `.tbl`-parsing path (§4)                                                                                 | ADDED (omission)                           | New paragraph in §4: `table_resize` (`src/table.c:46-72`) is used only by build-time tooling and builtin-table startup loaders, never by `parse_table.c` — so the "table fills up → whole file fails" risk is real and unmitigated for user tables.                                                                                                                                      |
| Parser grammar details in §4 (comments, `start`/`end` states, locale prefix-match direction, quoted-value handling, `code==0` rejection, `table_init` sizing formula) | CONFIRMED                                  | All read-only-verified against `src/parse_table.c`/`src/table.c`; locale prefix-match is bounded by the _parsed token's_ length (confirmed asymmetric as claimed), not the locale string's.                                                                                                                                                                                              |
| Shipped table-file line counts and `default` presence (§5)                                                                                                            | CONFIRMED                                  | All 9 files' `wc -l` and `default`-presence checks match exactly.                                                                                                                                                                                                                                                                                                                        |
| §7 open items (legacy-table diff, unterminated-quote behavior)                                                                                                        | UNVERIFIABLE (as originally flagged)       | Confirmed no test fixture exercises the unterminated-quote case; a full legacy-vs-current `unicode.tbl` diff was out of scope for this pass too.                                                                                                                                                                                                                                         |

Empirical testing method: compiled `src/detox`/`src/check-table` from the pinned clone, ran with
`-n` (dry-run) and small ad hoc config files (`sequence wipeup-only { wipeup { ... }; };`, etc.) against
throwaway filenames in a scratch directory. All four empirical checks above (wipeup, uncgi, max_length,
`.tbl` `\u`-escape) reproduced the document's claims exactly, with one correction (the overlong-UTF-8
worked example's byte values) surfaced by cross-checking the cited test file rather than by the binary
run itself.

## Review record (stage 3)

Three independent reviewers (L1 source fidelity + empirical, L2 completeness/pipeline mapping, L3
link and structure integrity). Every load-bearing finding below was re-verified by this adjudicator
against the pinned clone `0a8e212` before being accepted; empirical checks were rerun from scratch in
a scratch directory rather than trusted from the reviewer's write-up.

| Finding (reviewer)                                                                                                                                                                                                                                           | Verdict                                | Action or reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **MAJOR (L1).** §5's `table/unicode.tbl` row omits that the table identity-maps the full printable ASCII range; the §3.4 worked example only works because of those rows, so a reader would wrongly assume ASCII passes a `default`-bearing table unscathed. | **ACCEPTED**                           | Independently confirmed: 96 rows for `0x0020`-`0x007F` in `table/unicode.tbl` (all 95 of `0x0020`-`0x007E` mapped to themselves — verified programmatically, zero non-identity rows in that range — plus `0x007F` → `_`), and `table_get()` is called unconditionally per code point at `src/clean_utf_8.c:154`. **Reproduced empirically**: a table with `default "_"` and no ASCII rows turned `"® reg"` into `"_____"`; the builtin table leaves it untouched. Fixed in two places — the §5 row now states the identity mapping and its row count, and §3.4 gains a "Table-authoring hazard" paragraph with the empirical result.                                                      |
| **Defect (L1).** `table_resize()` is never called on the runtime `.tbl` load path; a dense 601-row table fails fatally with exit 1 instead of growing.                                                                                                       | **ACCEPTED**                           | Reproduced from scratch, not taken on trust: 601 rows in a 3508-byte file → `table_init` capacity `max(500, 3508/6)` = 584, and insertion fails at row `0x0249` (585 decimal) with `Failed to add row ... to translation table`, `unable to parse file`, exit 1 for both `detox` and `inline-detox`. §4's `table_resize` paragraph now carries the reproduction verbatim and frames it explicitly as a defect **not** to reproduce: grow on demand, or state a limit up front and report exceeding it as a size error, never as a per-row insert failure. Noted that upstream owns the resize primitive and simply doesn't wire it in — an oversight, not a design constraint.            |
| **MINOR (L1).** `0xA9` → `0xC2 0xA9` (©) and the three `cp1252` byte examples were test-citation-verified in stage 2 but never run against the compiled binary.                                                                                              | **ACCEPTED** (no text change)          | L1's runs (`inline-detox -s iso8859_1-only`, `-s cp1252-only`) are recorded here as the empirical confirmation stage 2 lacked. The document's claims were already correct, so nothing in the body changes; this row is the record.                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| **L1's prescriptive-vs-descriptive check**: doc 12 nowhere reads as a spec for `detoxrs`.                                                                                                                                                                    | **ACCEPTED, and extended**             | L1 concluded "no change needed". Adjudicator disagrees on sufficiency, not on the finding: the document was neutral by accident rather than by statement, and a reader arriving at §3.2-§3.4 has no way to know the whole filter family is dropped. New **§0 "Framing"** states it explicitly — the legacy-encoding family is deliberately not carried forward per `owner-decisions.md`, with the reason (byte-by-byte encoding guessing that silently mangles correctly-encoded names and cannot be distinguished from a successful repair) and an explicit instruction that the content is kept in full because it is the evidence for that decision. Nothing was deleted or condensed. |
| **L3.** 140 source permalinks verified locally against the pinned clone; 0 failures.                                                                                                                                                                         | **ACCEPTED**                           | Re-run locally (not over the network) as a spot audit of the audit: 145 base-URL matches across both documents, of which 2 are the documents' own base-URL/template prose (`.../blob/0a8e212/` and `<path>#L<n>`), not links. All real permalinks resolve — file present, commit prefix correct, line ranges within bounds.                                                                                                                                                                                                                                                                                                                                                               |
| **L3 MINOR.** §3.4 lines ~189-190: ambiguous whether overlong-ASCII collapse comes from the table's identity row or from a `default` being set.                                                                                                              | **ACCEPTED**                           | Rewritten. The normalization is the identity row; `default` is irrelevant to it and only governs code points with no row at all. The cited test's `default` is called out as incidental.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| **L3.** §4's self-corrected `code == 0` claim ("silently dropped … actually re-reading: fatal") reads as an unresolved mid-claim correction.                                                                                                                 | **REJECTED**                           | L3 itself recommended keeping it, and that is the right call: the self-correction lands on the accurate behavior (`table_put` returns -1 for `key == 0`, `parse_table` treats -1 as fatal) and showing the reasoning is worth more than a tidier sentence in a document whose purpose is evidentiary. Rewriting it would only hide that the obvious first reading is wrong.                                                                                                                                                                                                                                                                                                               |
| **L2.** Every filter in §1/§2 has a traceable disposition in doc 00; the `.tbl` mechanism's wholesale rejection is explicitly argued, not silently dropped.                                                                                                  | **ACCEPTED** (no change)               | Spot-checked and agreed. Recorded as a clean result.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| **L2 CRITICAL (F1).** Doc 00 still specifies an active CP1252 legacy-encoding-repair stage that `owner-decisions.md` drops from v1.0.                                                                                                                        | **OUT OF SCOPE — verified and queued** | Confirmed real, read-only: doc 00 still carries `Decoded::Repaired`/`LegacyEncoding` (§3.1), the `decode` pipeline row describing CP1252 repair as on-by-default (§3.2), the CP1252-fallback rationale (§3.4), `--legacy-encoding <cp1252\|latin1\|koi8-r\|sjis\|none>` in `--help` (§2), `repaired: cp1252 -> UTF-8` in the worked examples, a `decode.rs` "cp1252/latin1 tables" module listing, the `Repaired` invariant and test-matrix rows (Appendix A), and gating spike 6. **Not edited here** — doc 00 is outside this pass's edit scope and a separate propagation pass owns it. Precise requirements listed under "What doc 00 must change" below.                             |
| **L2 MINOR (F5/F6).** Doc 00 silently reassigns `[ ]`/`{ }` relative to `safe.tbl`; doc 00 never cites doc 12 §6 for the `_hidden_null_`/overlong subsumption.                                                                                               | **OUT OF SCOPE**                       | Both are defects in doc 00, not doc 12 — doc 12 documents upstream's table correctly in both cases. Forwarded to the doc 00 pass, no edit here.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| **L2 MINOR (F3).** §7's `[UNVERIFIED]` legacy-vs-current `unicode.tbl` diff underpins doc 00 §3.6's transliteration narrative.                                                                                                                               | **MODIFIED**                           | Rejected as a doc 12 change: the marker is doing exactly its job — the diff was not run, and §7 says so plainly. Running an 801-vs-221-line semantic diff to firm up a narrative that is independently evidenced by upstream's own README and man-page HISTORY is not worth the pass. The actionable half belongs to doc 00 (hedge the "arc" claim or cite the qualitative source), and is forwarded there.                                                                                                                                                                                                                                                                               |
| **L2.** §3.5 doesn't state that upstream never validates the post-`uncgi` decoded byte sequence.                                                                                                                                                             | **REJECTED**                           | Adding a sentence enumerating a check upstream does not perform invites the same treatment for every other absent check. §3.5's rule list is exhaustive and closed ("Two rules … Everything else copied verbatim"), and §6's first bullet already establishes that no filter but `utf_8` has any concept of invalid input. The successor's own all-or-nothing validity rule is a doc 00 design decision with no upstream counterpart to cite.                                                                                                                                                                                                                                             |

**What doc 00 must change** (not done here; recorded for the propagation pass):

1. §3.1 — drop the `Repaired { text, from: LegacyEncoding }` variant from `Decoded`, leaving `Utf8`
   and `Opaque`; drop `Repaired` from the `Note` enum's listed variants.
2. §3.2 — rewrite the stage-1 `decode` row to "valid UTF-8, or `Opaque` → skip with a report",
   removing the CP1252-decode-and-emit-`Repaired` description.
3. §3.4 — the CP1252-as-fallback rationale subsection goes, replaced by the skip-and-report rule plus
   a pointer to `owner-decisions.md` and the post-1.0 opt-in `--repair-encoding` path.
4. §2 `--help` block — remove `--legacy-encoding <E>` and its value list.
5. Worked examples — the `Björk` lines currently annotated `(repaired: cp1252 -> UTF-8)` must show
   the name being skipped and reported instead.
6. §3.6 and §6.x — remove "Non-UTF-8 repair (CP1252)" from the behavior/feature tables.
7. Module layout — `decode.rs`'s "cp1252/latin1 tables (ours, ~40 lines)" comment, and `LegacyEncoding`
   in `policy.rs`.
8. Appendix A — the `Repaired`-path invariant and the "Repaired if CP1252-plausible" test-matrix row
   become "always `Opaque`, always skipped, never panics".
9. §11 spike 6 — mark **moot/withdrawn** per `owner-decisions.md`, not "open".
10. Roadmap v0.3 — remove the "Legacy-encoding decode … with the Linux spike closed" milestone.
11. Add the F5/F6 fixes: one defense line each for `[ ]` and `{ }` in §3.7's contested-members list,
    and one sentence citing doc 12 §6 stating that strict UTF-8 validation subsumes `_hidden_null_`
    and overlong normalization by construction.

Empirical method (stage 3): all runs used the prebuilt `detox`/`inline-detox` from the pinned clone,
invoked with ad hoc `-f <rc>` config files and `-n`/dry-run or throwaway files inside a scratch
directory outside both the repo and the clone. Newly run this pass: the default-only-table ASCII
replacement test, the builtin-unicode-table control, and the 601-row table-overflow reproduction.
