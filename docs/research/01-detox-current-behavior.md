# detox — Behavior Narrative and Empirical Bug Overview (v3.0.1)

**Front matter**

- Subject: `detox`/`inline-detox` by Doug Harple, `github.com/dharple/detox`. The
  upstream project is **archived as of 2026-07-12**; nothing here describes work in
  progress.
- Pinned commit for all source citations and line numbers below:
  `0a8e2127e3c59cb419912d77c50f592b6460480a` (short `0a8e212`, tag `v3.0.1` + 4
  commits), repo `dharple/detox`.
- Date of this research: original capture undated; **corrected, re-pinned, and
  adversarially re-verified 2026-07-31** (see "Review record (stage 3)" at the end).
- Original capture was made against a Homebrew-installed v3.0.1 binary and an
  unpinned `tip-of-main` tarball. Every load-bearing claim has since been re-checked
  against `0a8e212`; corrections are recorded in the review record.
- Link format used below: `[src/file.c:123](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L123)`.

## Scope of this document — read this first

This document is a **narrative overview plus a catalog of empirically reproduced
bugs**. It is deliberately not the exhaustive enumeration of any subsystem. The
authoritative, line-cited, independently stage-2-validated enumerations live in the
sibling documents:

| Topic                                             | Authoritative document                     |
| ------------------------------------------------- | ------------------------------------------ |
| Complete CLI surface, every flag, exit codes      | `10-detox-cli-surface.md`                  |
| `detoxrc` grammar, search order, merge semantics  | `11-detox-config-file.md`                  |
| Every filter and translation table, per-character | `12-detox-filters-and-tables.md`           |
| Build/env inputs, `DATADIR`/`SYSCONFDIR`, runtime | `13-detox-build-env-and-runtime-inputs.md` |

**Where this document and docs 10-13 disagree, docs 10-13 win.** What this document
uniquely contributes is §7 and §8: reproducible commands against a real binary that
demonstrate detox's failure modes as one coherent story, rather than as facts
scattered across four artifact-organized catalogs.

Local paths from the original capture (Homebrew v3.0.1):

- Binary: `/opt/homebrew/bin/detox`, `/opt/homebrew/bin/inline-detox`
- Shipped config: `/opt/homebrew/etc/detoxrc`
- Shipped tables: `/opt/homebrew/share/detox/{safe,iso8859_1,cp1252,unicode}.tbl`
- Legacy tables: `/opt/homebrew/Cellar/detox/3.0.1/share/detox/legacy/*.tbl`
- Experiments: `/private/tmp/.../scratchpad/detox-probe/`

---

## 1. CLI flags

> Summary only. `10-detox-cli-surface.md` has the complete option table with
> per-option source lines, the `-?` undocumented option, and the exit-code analysis.

From `detox -h` (v3.0.1) and [src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140):

```
usage: detox [-hLnrvV] [-f configfile] [-s sequence] [--dry-run] [--help]
             [--inline] [--recursive] [--special] [--verbose] file [file ...]
```

| Flag            | Long form     | Effect (verified in source / behavior)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| --------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `-f configfile` | —             | Use exactly this config file for sequence definitions; **no other config file is parsed** (man: "No other config file will be parsed").                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `-h`            | `--help`      | Print usage + option summary, exit 0.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `-L`            | —             | List available sequences and exit. Plain `-L` lists sequence **names only** (marking the active one with `(*)`), plus a `files to ignore:` section if any are configured — it prints no `source file:` line. `-L -v` additionally prints, per sequence, a `source file:` line and every filter with its options (builtin table name, `remove_trailing`, etc.) — [src/config_file_dump.c:24-41](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L24-L41). Does not touch any files.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| `-n`            | `--dry-run`   | Dry run: no `rename()` is performed. It implies verbose output **only for the rename-preview line** (`old -> new`), which is gated on `verbose \|\| dry_run` — [src/file.c:142-144](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L142-L144). The per-file `Scanning:` banner still requires an explicit `-v`, because `-n` sets only `dry_run` and never touches the `verbose` field — [src/detox.c:93-95](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L93-L95). Collision/clobber detection still runs during a dry run (see §5) — a `-n` run can print "Cannot rename ... file already exists" without changing anything.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| `-r`            | `--recursive` | Descend into subdirectories **beyond the first level**. Naming a directory on the command line always renames its immediate children — both files and subdirectories — even _without_ `-r`, because `main()` calls `parse_dir()` unconditionally for a directory argument: [src/detox.c:101-104](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L104). Inside `parse_dir`, `options->recurse` gates only whether it calls _itself_ again on subdirectories it found: [src/file.c:218-227](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L218-L227). `-r` is therefore needed only to reach content two or more levels deep. Dotfiles/dot-directories are skipped during the walk unless _explicitly_ named on the command line (see §6). Sequence: a directory is itself renamed first via `parse_file()`, then its (possibly new) path is passed to [`parse_dir` at src/file.c:166](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L166).                                                                                                                                                                                                                                                                             |
| `-s sequence`   | —             | Select a sequence by name defined in the config file(s). Overridden precedence: CLI `-s` > `DETOX_SEQUENCE` env var > `default` sequence > (if no sequence literally named `default` exists) the first sequence defined — [src/parse_options.c:133](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L133), [src/sequence.c:28-54](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L28-L54).                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| —               | `--special`   | Operate on symlinks and other non-regular files (device nodes, FIFOs, sockets). **Without this flag, detox silently skips any file argument that is not a regular file or a directory — including symlinks passed directly on the command line**, not just during recursion (verified: the `main()` dispatch only calls `parse_file`/`parse_dir` for `S_ISDIR`/`S_ISREG`, or falls through to `parse_file` if `options->special` is set — [src/detox.c:101-110](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L110); the same gate applies inside the walk at [src/file.c:224](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L224)). Even with `--special`, detox will **not recurse into a symlink that points at a directory** (man page, and `parse_dir` only recurses paths for which `lstat` says `S_ISDIR`, which a symlink-to-dir is not under `lstat`).                                                                                                                                                                                                                                                                                                                                                                 |
| `-v`            | `--verbose`   | Print `old -> new` for every rename that happens (or would happen, under `-n`). Repeatable — the field is a counter, not a bool ([src/parse_options.c:179-181](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L179-L181)) — but there is **no behavior difference between `-v` and `-vv`**: no code anywhere in `detox`/`inline-detox` compares `verbose` against anything but zero. Re-verified for this pass by exhaustive `grep -rn verbose src/*.c` at `0a8e212`: the only reads of the field are the three truthiness tests in [src/config_file_dump.c](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L24) (lines 24, 31, 40), one in [src/detox.c:93](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L93), and one in [src/file.c:142](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L142). The only other `verbose` in the tree is a local in the separate `check_table` utility, passed to [`table_dump()`](https://github.com/dharple/detox/blob/0a8e212/src/table_dump.c#L23) — also a truthiness test, and unreachable from either `detox` binary. `10-detox-cli-surface.md` leaves this `[UNVERIFIED]`; this document's stronger claim is the one to prefer, on the grep above. |
| `-V`            | —             | Print `PACKAGE_STRING` (e.g. `detox 3.0.1`) and exit 0.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| —               | `--inline`    | Switch to inline mode even when invoked as `detox` (normally inline mode is auto-selected when the binary's `basename` is exactly `inline-detox`). See §"inline mode" below.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |

Environment variable: **`DETOX_SEQUENCE`** — sets the default sequence name, same precedence as above (read once at startup via `getenv`, overridden by `-s`). Verified:

```
$ DETOX_SEQUENCE=lower detox -n -v "UPPER_CASE_FILE.TXT"
UPPER_CASE_FILE.TXT -> upper_case_file.txt
```

**Argument handling**: if no files are given, prints usage and exits with failure (non-inline mode only — inline mode with no filenames reads stdin).

**Exit codes**: detox has **no non-zero exit code for partial failure**. A run in which
some arguments could not be processed still exits 0 as long as the program itself
completes. Verified empirically at `0a8e212`: `detox -n "nonexistent one.txt" "needsclean file.txt"`
prints `nonexistent one.txt: No such file or directory` to stderr, renames the other
file, and exits **0**. `10-detox-cli-surface.md` has the full exit-code table,
including the discrepancy between the `'?'`-unknown-option path (stdout, exit 0) and
the `default:` path (stderr, exit 1).

**Inline mode** (`--inline`, or binary named `inline-detox`): reads text (a filename per line, or stdin) and rewrites just the _filename text_, not the actual filesystem entry — no `rename()`, no filesystem stat calls for existence, no collision detection. Useful for piping filenames through the same cleaning logic. `src/file.c: parse_inline()` handles UTF-8-aware line-buffering so a multi-byte UTF-8 character isn't split across an internal buffer boundary. `inline-detox` acts on `-f`, `-h`, `-L`, `-s`, `-v`, `-V`. It also **accepts `-r`, `--recursive`, `--special`, and `-n` and silently ignores them** — they are not rejected, because both binaries share one `getopt`/`getopt_long` call with an identical option string and `longopts[]` table ([src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140)); recursion, dry-run, and special-file handling are simply meaningless for a text stream.

Verified:

```
$ echo "some file name.txt and another (one).doc" | inline-detox
some_file_name.txt_and_another-one.doc
```

---

## 2. Config file format (`detoxrc`)

> Summary only. `11-detox-config-file.md` is authoritative for the grammar;
> `12-detox-filters-and-tables.md` is authoritative for per-filter semantics.

Grammar is C-/named-like: semicolon-terminated statements, brace-delimited blocks, `#` line comments.

### 2.1 Search order for config files ([src/config_file.c:38-93](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L38-L93), unless `-f` given)

1. `$SYSCONFDIR/detoxrc` (build-time configured sysconfdir; on Homebrew this resolves to `/opt/homebrew/Cellar/detox/3.0.1/etc/detoxrc`, symlinked from `/opt/homebrew/etc/detoxrc`)
2. `/etc/detoxrc` (only tried if step 1 didn't produce a file)
3. `/usr/local/etc/detoxrc` (only tried if steps 1–2 didn't produce a file)
4. `$HOME/.detoxrc` — **parsed and merged on top of** whatever was found in 1–3 (later sequences with the **same name replace** earlier ones — man page: "if a system-wide file defines normal_seq and a user has a sequence with the same name in their .detoxrc, the users' normal_seq will replace the system-wide version")
5. `$XDG_CONFIG_HOME/detox/detoxrc` — parsed/merged on top of that

If `-f configfile` is given, **only** that file is parsed; none of the above.

### 2.2 Top-level statements

```
sequence "name" { ...filters...; };
ignore { filename "name"; ... };
# comment
```

- `sequence default { ... };` — the sequence named exactly `default` is used when no `-s`/`DETOX_SEQUENCE` is given. If none is named `default`, the _first_ sequence defined in the loaded config wins as the fallback (only when the user also didn't request a sequence by name).
- Sequence names are case-sensitive and must be globally unique across all merged config files; duplicates replace earlier definitions of the same name (this is how a user overrides a shipped sequence).
- `ignore { filename "x"; ... };` — filenames to skip during `-r` recursion (exact basename match, `strcmp`). Independent of the "dotfiles are always skipped during recursion" rule.

### 2.3 Filter statements (each occurs _inside_ a `sequence { }` block, and filters run **in the order listed**, each one's output feeding the next)

```
iso8859_1;
iso8859_1 { builtin "iso8859_1"; };     # or "cp1252"
iso8859_1 { filename "/path/to/x.tbl"; };

utf_8;
utf_8 { builtin "unicode"; };
utf_8 { filename "/path/to/x.tbl"; };

uncgi;                                   # no options

safe;
safe { builtin "safe"; };
safe { filename "/path/to/x.tbl"; };

wipeup;
wipeup { remove_trailing; };             # only option

max_length;
max_length { length 128; };              # only option, default 256 if length<=0

lower;                                   # no options
```

Filter semantics (all confirmed against [src/clean_string.c](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c), [src/clean_utf_8.c](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c)):

- **`iso8859_1`** — [src/clean_string.c:30-79](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L30-L79). Walks the filename **byte by byte**. Any byte with the high bit set (`byte & 0x80`, i.e. 0x80–0xFF) is looked up in the table by its raw byte value; bytes below 0x80 pass through untouched. If the table has no entry for that byte: if the table defines a `default` translation, that is substituted; otherwise the byte passes through unchanged (this is what allows chaining, e.g. `cp1252` table with no `default` followed by `iso8859_1` table). **This filter is a naive single-byte transcoder — it does not decode multi-byte UTF-8 and will corrupt genuine UTF-8 input** (see §7).
- **`utf_8`** — [src/clean_utf_8.c:60-200](https://github.com/dharple/detox/blob/0a8e212/src/clean_utf_8.c#L60-L200). Properly decodes UTF-8 (1–6 byte forms per the original UTF-8 spec, including the technically-obsolete 5/6-byte lead bytes), computes the Unicode code point, and looks that code point up in the table (keyed by code point, not by raw byte). Handles malformed sequences: a truncated/invalid continuation byte prints a warning to stderr and emits a literal `_` for the bad lead byte; an embedded UTF-8-encoded NUL is forcibly replaced with the literal string `_hidden_null_` (never allowed to pass through, even with no matching table default) with a stderr warning; any decoded code point above `0x10FFFF` is forced to `_` with a warning. If a code point simply isn't in the table and the table has no `default`, it's copied through **as its original UTF-8 bytes**, unmodified.
- **`uncgi`** — decodes `%XX` (two hex digits, case-insensitive) to the corresponding byte, and `+` to a literal space. No other characters are touched. Verified: `100%20done.txt` → `100 done.txt` (pre-safe/wipeup stage); `100%25 done.txt` → `100% done.txt`.
- **`safe`** — Walks the filename **byte by byte** (0x00–0xFF, no UTF-8 awareness at all) and replaces any byte found in the table with its replacement string; bytes not found pass through unchanged if the table has no `default`, or are replaced by the `default` string otherwise. Because it operates on raw bytes, it does **not** touch valid multi-byte UTF-8 sequences (each byte of a UTF-8-encoded accented letter or emoji has the high bit set but the shipped `safe.tbl` has no entries above 0x7F and no `default`, so such bytes pass straight through — verified with `café.txt` and an emoji filename under the default sequence: unchanged).
- **`wipeup`** — [src/clean_string.c:194-245](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L194-L245) (search string set at [line 210](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L210)). Two independent behaviors:
  1. Strips any run of leading `-`, `_`, or `#` characters from the start of the filename (checked in that unordered set, simple `while` loop, no precedence among the three at the leading-edge).
  2. Collapses consecutive runs of a configurable character set down to one character. The set (`search`) is `-_` normally, or `.-_` when `remove_trailing` is set. **Precedence when a run mixes different chars from the set** is by _position in the search string_: within a contiguous run, the filter tracks the earliest-indexed search-string character seen so far and emits that single character once the run ends. So with `remove_trailing` the order is `.` (index 0) > `-` (index 1) > `_` (index 2); without it, `-` > `_`.

  Worked example, verified against the binary at `0a8e212` with the shipped `default` sequence (which _does_ set `remove_trailing`): `this__--.txt` → `this.txt`. Note that the contiguous run here is `__--.` — five characters, because the `.` is adjacent and is itself in the search set once `remove_trailing` is on — and `.` wins on index. A run of `_`/`-` alone, with no `.` in it, would collapse to `-`, not `.`.

- **`max_length { length N; }`** — [src/clean_string.c:250-309](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L250-L309). Trims the filename to at most `N` bytes (default 256 if `N<=0` or omitted). Extension-aware: it looks at the substring after the **last** `.`; if that's the whole filename (no dot) or the "extension" is a single character (i.e., just a trailing bare dot), it truncates dumbly to `N` bytes and returns. Otherwise it looks **backward up to 5 characters** from that last dot for an _earlier_ dot, and if found, treats everything from that earlier dot onward as "the extension" — this is exactly the mechanism that preserves `.tar.gz` as a unit when the two dots are ≤5 characters apart (`.tar` is 4 chars). The body (everything before the extension) is then truncated so body+extension == N bytes. If the extension alone is `>= N` bytes, it prints a warning to stderr (`max_length %d is less than required file length for '%s'. giving up.`) and returns the filename **unchanged** (not truncated at all).
- **`lower`** — ASCII-only `isupper()`/`tolower()` per byte; does not affect non-ASCII bytes (including any accented Latin-1/UTF-8 byte with the high bit set, since C's `isupper()` behavior on values >127 is at minimum locale-dependent and detox does not attempt Unicode case folding here).

Verified `max_length` behavior with a custom sequence (`length 20`):

```
this_is_my_file_name.txt        -> this_is_my_file_.txt      (20 bytes; no double-ext)
this_is_my_archive_name.tar.gz  -> this_is_my_ar.tar.gz       (20 bytes; .tar.gz kept as unit — "ar" + ".tar.gz" = 20)
```

---

## 3. Translation table format (`detox.tbl(5)`) and default/fallback rules

```
default _              # optional; empty/absent => unknown chars fall through unchanged
                        # (enables chaining multiple tables in one sequence)
start
0x09    _tab_          # value: decimal, 0x-hex, or 0-leading octal (sscanf rules); translation: bare word or quoted string
end
start lang              # optional language-specific block, e.g. "start en"
0x24    _money_          # overrides the plain-block translation for that char, ONLY if
end                       # the process locale's language portion matches `lang`
```

- `value` may repeat across `start`/`end` blocks; a later `start`/`end` block's value for the same key **overwrites** an earlier one (`table.c: table_put` increments an `overwrites` counter on key collision and replaces the data — no error, silent).
- A `start lang` block is only loaded if the current process locale's language code matches `lang` (e.g. `en`); otherwise its translations are ignored and the base block's values apply. This lets one table have a per-language override (man example: `$` → `_dollar_` in general, but `_money_` under English locale).
- Internally, tables are open-addressed hash tables (`table_hash = key % table_length`) with linear-scan fallback; irrelevant to on-disk semantics but explains that key lookup is by exact Unicode code point (for `utf_8`/table-keyed-by-codepoint filters) or exact byte value (for `safe`/`iso8859_1`).
- **Builtin tables** are compiled-in copies of the same 4 tables (`safe`, `iso8859_1`, `cp1252`, `unicode`) used when no on-disk `.tbl` file is found, or explicitly requested via `builtin "name"`. In this build the on-disk versions at `/opt/homebrew/share/detox/*.tbl` and the builtins are the same content (per `bin/generate-builtin.sh` in source, builtins are generated _from_ the shipped `.tbl` files at build time).
- **Table file search path** when _not_ using `builtin`/`filename` explicitly and a filter just says e.g. `safe;` with no block: detox searches (in order) `$DATADIR/detox/<name>.tbl`, `/usr/share/detox/<name>.tbl`, `/usr/local/share/detox/<name>.tbl`, falling back to the compiled-in builtin if none of those parse — [src/filter.c:30-70](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L30-L70), [src/filter.c:131-165](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L131-L165). The `$DATADIR` step is wrapped in `#ifdef DATADIR` ([src/filter.c:42-43](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L42-L43)), but automake always defines it, so in practice it is unconditional — see `13-detox-build-env-and-runtime-inputs.md` §1.2. Note: **`/opt/homebrew/...` is not in this search list** — on this Homebrew install, the on-disk tables are found via `$DATADIR` (Homebrew sets `DATADIR` to its own share dir at build time), not via a hardcoded `/opt/homebrew` path.

---

## 4. Default "safe" character set (`/opt/homebrew/share/detox/safe.tbl`, current v3.x)

No `default` line (commented out in the shipped file) → **anything not explicitly listed passes through unchanged**, including all non-ASCII/multi-byte UTF-8 bytes.

Mapped to `_` (underscore):

- All C0 control characters `0x01`–`0x1F` and `0x7F` (DEL)
- Space (`0x20`), `! " $ ' * / : ; < > ? @ \ ` |`

Mapped to `-` (dash):

- `( ) [ ] { }`

Special case:

- `&` → the literal string `_and_`

Everything else — letters, digits, `. , + - = ^ ~ # % _`, and all bytes ≥ 0x80 — is **not** in the table and passes through unchanged by the current (v3.0.1) `safe.tbl`. (Note: `#` is not translated by the `safe` filter itself; the leading `#` stripping is done by `wipeup`, not `safe`.)

This is a deliberate v3.0 design shift documented in the man page HISTORY section: "Version 3.0 further shifted this, by removing most of the transliteration from the tables ... many modern Unix-like OSs use UTF-8 ... Transliterating from UTF-8 to ASCII in this scenario is lossy and pointless."

A **legacy** table set with much more aggressive Latin transliteration (e.g. `é`→`e`,
`ß`→`ss`, `Þ`→`TH`, currency symbols → `_cent_`/`_pound_`/`_yen_`, and a `default _`
catch-all) ships separately in `share/detox/legacy/` for users who want v1/v2-style
behavior; it is not used unless explicitly pointed to via `filename`.

**Correction (2026-07-31): there is no legacy `safe.tbl`, and an earlier revision of this
document implied there was.** The legacy directory contains exactly four files —
`cp1252.tbl`, `iso8859_1.tbl`, `unicode.tbl`, `unidecode.tbl` — verified for this pass by
directory listing at `0a8e212` and by `git log --all -- 'table/legacy/safe.tbl'`, which
returns nothing: no such file has ever existed in the repo's history. The aggressive
transliteration above lives only in the legacy `unicode.tbl` (and `unidecode.tbl`), keyed
by **Unicode code point**, so it is reachable only through a `utf_8` filter pointed at
that file — never through the `safe` filter, which is keyed by raw byte. See
`12-detox-filters-and-tables.md` §5 for the authoritative legacy-table inventory.

---

## 5. Collision / clobber behavior

Verified against `parse_file` — [src/file.c:69-160](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L69-L160), the
existence/inode check specifically at [src/file.c:124-140](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L124-L140).
`13-detox-build-env-and-runtime-inputs.md` §4.1 independently re-derives the same check.

Exact algorithm:

1. Compute `new_filename` by running the chosen sequence's filters over the basename.
2. If the cleaned name is byte-identical to the original, no rename is attempted (silent no-op, nothing printed even with `-v`, because the "if nothing changed" check short-circuits before any print).
3. `lstat()` the **old** path. If that fails, abort (return original name, print nothing).
4. `lstat()` the **new** path.
   - If the new path does **not** exist: proceed to rename.
   - If the new path **does** exist: allow the rename to proceed **only if** the old and new paths refer to the exact same inode on the same device (`st_dev`/`st_ino` match) **and** that inode has exactly one hard link (`st_nlink == 1`).

     _Interpretation (this document's reading, not a source comment):_ this is in
     effect a case-only-rename escape hatch on case-insensitive filesystems such as
     macOS's default APFS. Renaming `Foo.txt` → `foo.txt` sees the "new" name already
     existing because it is the _same file_ under case-insensitive lookup, and detox
     lets that through. The code does not name this intent.

   - Otherwise (different file, or same file but hard-linked elsewhere) it refuses and prints to **stderr**: `Cannot rename <old> to <new>: file already exists`, and the original file is left untouched.
5. This whole check runs even under `-n`/`--dry-run` — a dry run **will** report `Cannot rename ...: file already exists` for a genuine collision, without needing `-v`, since the "already exists" message is printed via `fprintf(stderr, ...)` unconditionally, not gated on verbose.
6. On a real (non-dry-run) rename, `rename(2)` is called; if that itself fails (e.g. permissions), the error and `strerror()` text are printed and the original name is retained.

Verified:

```
$ ls
COLLIDE FILE.txt   collide_file.txt
$ detox -n -v *
Cannot rename COLLIDE FILE.txt to COLLIDE_FILE.txt: file already exists
```

(`COLLIDE_FILE.txt` collides case-insensitively with the pre-existing `collide_file.txt` on this APFS volume; they are different inodes, so detox refuses.)

Caveat also stated verbatim in `man detox`: _"If, after the translation of a filename is finished, a file already exists with that same name, detox will not rename the file."_ — this documents the design (never overwrite), but the source shows the one same-inode/single-link exception above that the man page prose doesn't call out explicitly.

**Important scope note**: this collision check is purely local — it only guards against the _specific pairing_ being renamed right now. If sequential renames in the same run (e.g. `A.txt`→`X.txt` then later `B.txt`→`X.txt`) collide, the check still applies per-rename at the time each is processed (files are visited via `readdir()` order, which is filesystem-dependent/unspecified), so the second one to be processed loses and is reported "already exists"; there is no batch-level "would multiple sources map to one destination" pre-check across the whole run.

---

## 6. Recursion, symlink, and special-file behavior

- **`-r`/`--recursive` gates only the _second_ level and deeper.** This is the single
  most consequential correction to this document, and it contradicts the man page.
  Naming a directory on the command line always processes its immediate children —
  files _and_ subdirectories are renamed — whether or not `-r` is given, because
  `main()` calls `parse_file()` on the directory and then `parse_dir()` on it
  unconditionally: [src/detox.c:101-104](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L104).
  Inside `parse_dir`'s `readdir()` walk, `options->recurse` guards only the recursive
  self-call on subdirectories it discovers: [src/file.c:218-227](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L218-L227).
  `10-detox-cli-surface.md` and `13-detox-build-env-and-runtime-inputs.md` §4.3 flag
  the same behavior; **a successor's `--recursive` cannot be modeled as "gates all
  subdirectory content" without diverging from upstream.**

  Reproduced against the binary built at `0a8e212`, on a tree
  `dir one/{file one.txt, sub one/nested one.txt}`:

  ```
  $ detox -n -v "dir one"            # no -r
  Scanning: dir one
  dir one -> dir_one
  dir one/file one.txt -> dir one/file_one.txt
  dir one/sub one -> dir one/sub_one
  # note: "dir one/sub one/nested one.txt" is NOT touched

  $ detox -n -v -r "dir one"         # with -r
  Scanning: dir one
  dir one -> dir_one
  dir one/file one.txt -> dir one/file_one.txt
  dir one/sub one -> dir one/sub_one
  dir one/sub one/nested one.txt -> dir one/sub one/nested_one.txt
  ```

- A directory entry is renamed (via the normal filter sequence, same as a file)
  **before** being descended into, and the descent uses the _possibly-renamed_ path —
  [src/file.c:218-222](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L218-L222):
  `work = parse_file(new_file, options); if (options->recurse) parse_dir(work, options);`.
  The same rename-then-descend order applies at the top level in `main()`.
- **Dotfile/dot-directory skipping during the walk**: any directory entry whose name starts with `.` is unconditionally skipped by [`ignore_file()` at src/file.c:33](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L33) during the `readdir()` walk — note this skip is a property of `parse_dir`, not of `-r`, so it applies to first-level children too — this applies even to `.` itself being filtered out implicitly, and to any user dotfile/dotdir, _except_ entries explicitly named on the command line (the dotfile skip only triggers inside the `readdir` loop in `parse_dir`, not in the top-level `main()` loop over `argv`). Verified:
  ```
  $ detox -n -v -r .            # recursion into cwd
  # (no lines for .hiddendir or its contents, and no line for ".hidden file.txt")
  $ detox -n -v ".hidden file.txt"   # named explicitly
  .hidden file.txt -> .hidden_file.txt
  ```
- **`ignore { filename "x"; }`** in the config supplements the dotfile rule with exact-basename ignores, again inside the `parse_dir` walk (so first-level children as well, `-r` or not) and never for paths named directly on the command line. The shipped `/opt/homebrew/etc/detoxrc` ignores exactly one name: `{arch}` (a CVS/Arch VCS metadata directory convention).
- **Symlinks / special files, top level**: without `--special`, a symlink (or FIFO, device node, socket) passed directly as a command-line argument is **silently skipped entirely** — not even scanned/considered — because the top-level dispatch in `main()` (`src/detox.c`) only calls `parse_file`/`parse_dir` when `lstat` says `S_ISDIR` or `S_ISREG`, or when `options->special` is true. Verified:
  ```
  $ detox -n -v "link to hello.txt"     # (symlink)
  Scanning: link to hello.txt
  # (no rename line — silently skipped)
  $ detox -n -v --special "link to hello.txt"
  Scanning: link to hello.txt
  link to hello.txt -> link_to_hello.txt
  ```
  Note the "Scanning:" line itself is printed regardless (that's from the file-list loop, before the type check), but the actual clean/rename attempt only happens with `--special`.
- **Symlinks during recursion**: same `--special` gate applies inside `parse_dir` (`S_ISREG(stat_info.st_mode) || options->special`). Directories are always recursed into structurally, but per the man page and the `lstat`-based `S_ISDIR` check, **a symlink pointing at a directory is never treated as a directory to recurse into** — `lstat` (not `stat`) is used throughout, so a symlink's own mode (`S_ISLNK`) is what's tested, never the mode of its target; a symlink-to-directory is therefore neither `S_ISDIR` nor `S_ISREG` and is only touched (renamed as a special file, not traversed) when `--special` is given.
- **`--special` does not imply `-r` into symlinked directories** — this is explicit in the man page ("detox will not recurse into symlinks that point at directories") and consistent with the `lstat`-only design above.

---

## 7. Encoding "detection" behavior — there isn't any

detox performs **no charset auto-detection whatsoever**. The user statically picks a sequence (e.g. `default`, `iso8859_1`, `iso8859_1-legacy`, `utf_8`) via `-s`/`DETOX_SEQUENCE`/config, and each filter blindly assumes its target encoding:

- The `safe` filter is a byte-oriented filter with no table entries above 0x7F (in the current, non-legacy tables) and no `default`, so it is encoding-agnostic by omission — it happens to be safe to run on UTF-8 names only because it doesn't touch anything ≥0x80 at all.
- The `iso8859_1` filter treats **every** byte ≥0x80 as a standalone Latin-1/CP-1252 code unit and transcodes it byte-by-byte to its UTF-8 encoding — [src/clean_string.c:29-88](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L29-L88). If the input is _already_ UTF-8, this is catastrophically wrong: each byte of a multi-byte UTF-8 sequence gets independently reinterpreted as a Latin-1 character and re-encoded, producing classic "mojibake". **Verified**:
  ```
  $ touch café.txt              # é stored as UTF-8 bytes 0xC3 0xA9
  $ detox -n -v -s iso8859_1 café.txt
  café.txt -> cafÃ©.txt
  ```
  (0xC3 got transcoded to U+00C3 `Ã`, and 0xA9 to U+00A9 `©`, i.e. exactly the well-known "double-encoding" mojibake pattern.)
- The `utf_8` filter correctly decodes multi-byte UTF-8 and is safe to run on genuine UTF-8 filenames (verified: `café.txt` and an emoji filename pass through the `utf_8` sequence completely unchanged, since neither character is a control character in `unicode.tbl`).
- **There is no mechanism in the source that inspects the byte stream and picks `iso8859_1` vs `utf_8` automatically.** Choosing wrong is entirely on the operator/config-author. The two filters are explicitly documented as "mutually exclusive" (detoxrc man page) precisely because running both, or running the wrong one, corrupts data — but nothing in the program prevents a user from configuring both or from choosing the ISO-8859-1 sequence against UTF-8-encoded filenames.
- Locale-sensitivity exists only for the **`start lang` block** inside a `.tbl` file (per §3) — that reads the process's language locale to select an override sub-block within an otherwise-static table the user chose. This is not encoding detection of the _input filename_; it only affects which _replacement string_ is used for a given already-known input character.

macOS-specific note observed during testing: attempting to create a filename containing a raw non-UTF-8 byte sequence (e.g. a literal Latin-1 `é` = `0xE9`, not valid UTF-8 on its own) fails at the OS level (`OSError: [Errno 92] Illegal byte sequence`) — APFS/HFS+ requires valid UTF-8 filenames, so genuinely mis-encoded Latin-1/CP-1252 filenames (the classic case this filter targets) could not be materialized in this sandbox to test directly; the iso8859_1 mojibake reproduction above instead demonstrates the _reverse_ failure mode (correct UTF-8 misinterpreted as Latin-1), which is the same underlying bug class and is fully reproducible.

---

## 8. Observed bugs / surprises, with reproducible commands

All below run from `/private/tmp/.../scratchpad/detox-probe`.

1. **Wrong-filter mojibake** (see §7) — `detox -s iso8859_1` corrupts genuine UTF-8 names. Reproducible:
   `touch café.txt && detox -n -v -s iso8859_1 café.txt` → `cafÃ©.txt`.

2. **`-n`/dry-run still performs collision detection and can print an error**, even though nothing changes on disk:
   `detox -n -v "COLLIDE FILE.txt" "collide_file.txt"` (on a case-insensitive filesystem) prints `Cannot rename COLLIDE FILE.txt to COLLIDE_FILE.txt: file already exists` — a dry run's stderr output is not purely informational preview text; it includes live filesystem-dependent collision checks (via a real `lstat()` on the would-be destination) that can differ across filesystems (case-sensitive ext4 vs. case-insensitive default-APFS/NTFS).

3. **`--special` is required even for symlinks explicitly named on the command line**, not just during `-r` recursion. **Observed:** `detox -n -v mysymlink` prints only `Scanning: mysymlink` — no rename line, no error, exit 0. Confirmed by reading `src/detox.c`'s top-level dispatch and reproduced above in §6. _Opinion, not observation:_ this is an easy trap, because the output is indistinguishable from "the name was already clean."

4. **`safe` filter does not touch `%`, `#`-mid-string, `^`, `~`, `=`, `+`, `,`** or any byte ≥0x80 (current v3.0.1 table) — filenames like `100% done.txt` only get the space fixed (`100%_done.txt`); the `%` itself survives. This is a real behavior change from detox v1/v2 (which had a much larger transliteration table) and can surprise anyone expecting "detox" to normalize percent signs or symbols; it's disclosed in the man page HISTORY section but easy to miss.

5. **The default sequence has no `max_length` filter at all** — a 254-byte filename (`'b'*250 + '.txt'`) passed through unchanged under the default sequence; length limiting is opt-in per-sequence/per-config, not automatic. (macOS/APFS itself refused to create a 300+ byte name with `OSError: [Errno 63] File name too long` — the OS enforces ~255 bytes; detox does not protect against this unless a `max_length` filter is explicitly configured.)

6. **Leading `#`/`-`/`_` stripping happens only in `wipeup`, not `safe`** — a filename starting with `#` (`#hashfile.txt`) is unaffected by `safe` (which has no entry for `0x23`) and is fixed only because `wipeup` unconditionally strips leading `# - _` from the front of the string, independent of the run-collapsing logic. If a config's sequence omits `wipeup`, a leading `#` is never removed.

7. **`wipeup`'s collapse precedence is positional, not "most special wins."** The winner
   within a mixed run is whichever search-set character has the lowest index in the
   hardcoded search string — `".-_"` with `remove_trailing`, `"-_"` without — because the
   loop keeps the lowest `strchr(search, *input_walk)` pointer it has seen
   ([src/clean_string.c:210-222](https://github.com/dharple/detox/blob/0a8e212/src/clean_string.c#L210-L222)).
   The shipped `default` sequence _does_ set `remove_trailing`, so its order is period >
   dash > underscore; a sequence that omits `remove_trailing` gets dash > underscore and
   leaves periods alone. Verified at `0a8e212`: `this__--.txt` → `this.txt` (period wins,
   run includes the adjacent `.`), and `a__-b.txt` → `a-b.txt` (no period in the run, so
   dash wins).

   _Opinion, not observation:_ the man page describes the resulting order correctly, so
   this is not a documentation bug — but the order is an artifact of string layout rather
   than a stated rule, which makes it fragile to reproduce from the prose alone.

8. **`max_length`'s "look back up to 5 characters for an earlier dot" heuristic for double extensions is a fixed, non-configurable magic number.** `.tar.gz` (4 chars between dots) is preserved as a unit; an extension scheme with a slightly longer first suffix (e.g. `.tar.bz2` has `.tar` = 4 chars, still under 5, so `name.tar.bz2` → verified conceptually would also keep `.tar.bz2` as a unit) works, but anything with a "sub-extension" name longer than 4 characters (5th char triggers the `break`) would **not** be recognized as a compound extension and would instead only preserve the last `.ext`. Not independently re-verified beyond the `.tar.gz` case in §2, but the 5-character window is explicit and hardcoded in `clean_max_length()`.

9. **Config file `ignore{}` and dotfile-skipping only apply during `-r` recursion**, never to files passed explicitly on the command line, and never to the initial (non-recursive) invocation — `detox somefile` on a file matching an `ignore` entry or starting with `.` is processed normally. Verified in §6.

---

## Confidence & Sources

**Verified by running the installed binary** (highest confidence — all commands reproducible verbatim in `/private/tmp/.../scratchpad/detox-probe`):

- All CLI flag behaviors demonstrated with actual invocations (§1: `-h`, `-L -v`, `-n`, `-s`, `-f`, `--special`, `DETOX_SEQUENCE`, `-V`).
- Default `safe`+`wipeup` sequence effects on: spaces, brackets, ampersand, leading `#`/`_`/`-` runs, control byte `0x01`, `%`-encoding survival, case preservation, UTF-8 bytes (café, emoji) passing through unchanged, directory renaming during recursion.
- `iso8859_1`, `utf_8`, `lower`, `uncgi` sequences via `-s`.
- `max_length` via a hand-written custom `detoxrc` (`-f custom.detoxrc`), including the `.tar.gz` double-extension case.
- Collision/clobber refusal on a case-insensitive filesystem, including under `-n`.
- Dotfile skipping under `-r` vs. explicit naming.
- Symlink skip-without-`--special`, and rename-with-`--special`.
- `inline-detox` basic text-rewrite behavior and one `uncgi-only` sequence run.
- `detox -L -v` full sequence/filter dump of the shipped `/opt/homebrew/etc/detoxrc`.

**Read directly, full text, high confidence (primary documents)**:

- `man detox` (detox.1), `man detoxrc` (detoxrc.5), `man detox.tbl` (detox.tbl.5) — all read in full via `col -b`.
- Shipped `/opt/homebrew/etc/detoxrc` (full file, 134 lines).
- Shipped tables, full text: `safe.tbl` (97 lines), `iso8859_1.tbl` (131 lines), `cp1252.tbl` (65 lines), `unicode.tbl` (222 lines).
- Legacy `unicode.tbl` at `.../share/detox/legacy/unicode.tbl` (partial — read first ~300 lines of a longer transliteration table; full extent not read but the header, `default _` line, and representative Latin-1/Latin-Extended-A entries were captured verbatim).
- C source, read in full: `src/detox.c`, `src/file.c`, `src/filelist.c`, `src/filter.c`, `src/clean_string.c`, `src/clean_utf_8.c`, `src/parse_options.c`, `src/table.c`, `src/sequence.c`; `src/config_file.c` read partially (the config-file search-path block specifically, via targeted `grep -A`).
- `CHANGELOG.md` header entries (version numbers/dates only, not full changelog bodies).

**Inferred / not independently re-verified**:

- The exact `max_length` 5-character-lookback boundary behavior for extension names _longer_ than 4 characters (item 8 in §8) is derived from reading `clean_max_length()`'s loop condition (`extension - input_walk > 5` breaks the backward scan) rather than from an additional test run with such a filename.
- That the on-disk shipped tables are bit-identical to the compiled-in builtins is inferred from the existence of `bin/generate-builtin.sh` in the source tree (which generates builtin C tables from the `.tbl` files) rather than from a byte-level diff of binary vs. table file.
- Homebrew's `DATADIR`/`SYSCONFDIR` build-time values were inferred from where `detox -L -v` reported its source file and from successfully loading `/opt/homebrew/share/detox/*.tbl` without an explicit `-f`/`filename`, not from inspecting the Homebrew build recipe's configure flags directly (though `.brew/detox.rb` exists in the Cellar and was not opened).
- Behavior of a genuinely mis-encoded (non-UTF-8) Latin-1/CP-1252 filename under the `iso8859_1` filter was **not** directly reproduced, because APFS refused to create such a filename in this sandbox (`OSError: Illegal byte sequence`); the reverse corruption (valid UTF-8 fed to `iso8859_1`) was reproduced instead and is presented in §7/§8 as the verified evidence for this bug class.
- Hard-link collision-refusal (`st_nlink > 1` branch in `parse_file`) was read in source but **not** successfully reproduced experimentally — the constructed test did not exercise the code path (the destination name didn't already exist at the time of the attempted rename), so that path is source-verified only, not behaviorally verified.

---

## Review record (stage 3)

Three independent reviews (L1 source fidelity, L2 completeness/cross-document
consistency, L3 clarity/reliability) were adjudicated on 2026-07-31 against the pinned
clone at `0a8e212` and a binary built from it. "Verified" below means the adjudicator
re-checked the claim personally, not that a reviewer asserted it.

| Finding (reviewer)                                                                                                | Verdict      | Action or reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------------------------------------------------------------------------------- | ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `parse_dir` cited as living in `src/filelist.c` (L1)                                                              | ACCEPTED     | **Verified source-read:** `grep -rn parse_dir src/*.c src/*.h` → definition at `src/file.c:166`, call sites `src/detox.c:103` and `src/file.c:221`, zero hits in `filelist.c` (which holds only `filelist_*` argv bookkeeping). Both citations rewritten to `src/file.c` with pinned links.                                                                                                                                                                                                                                                                                                                                                                  |
| `-r` described as gating all subdirectory descent (L1, L2, L3 — all three)                                        | ACCEPTED     | The document's most consequential error. **Verified empirically and by source-read:** on `dir one/{file one.txt, sub one/nested one.txt}`, `detox -n -v "dir one"` renamed the directory, `file one.txt` _and_ `sub one`, leaving `sub one/nested one.txt` alone; adding `-r` also renamed the nested file. `main()` calls `parse_dir()` unconditionally for a directory argument ([src/detox.c:101-104](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L104)); `options->recurse` gates only the self-call inside the walk ([src/file.c:218-227](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L218-L227)). §1 and §6 rewritten. |
| A legacy `safe.tbl` exists with aggressive transliteration (L2)                                                   | ACCEPTED     | **Verified source-read:** `table/legacy/` at `0a8e212` contains exactly `cp1252.tbl`, `iso8859_1.tbl`, `unicode.tbl`, `unidecode.tbl`; `git log --all -- 'table/legacy/safe.tbl'` returns nothing, so no such file has ever existed. §4 corrected in place, with the reachability point (code-point-keyed, so `utf_8`-only) stated explicitly.                                                                                                                                                                                                                                                                                                               |
| Plain `-L` claimed to print a `source file:` line (L1, L3)                                                        | ACCEPTED     | **Verified empirically and by source-read:** plain `-L` printed `available sequences:` + names + `files to ignore:`, no source file; `source file:` is inside the `main_options->verbose` branch at [src/config_file_dump.c:40-41](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L40-L41). Split across the `-L` / `-L -v` clauses.                                                                                                                                                                                                                                                                                                   |
| `-n` "implies verbose" stated flatly (L1 soft, L3 MAJOR)                                                          | ACCEPTED     | **Verified empirically:** `detox -n "needsclean file.txt"` (no `-v`) printed the `old -> new` line but no `Scanning:` banner. `-n` sets only `dry_run`; the preview is gated on `verbose \|\| dry_run` ([src/file.c:142](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L142)), the banner on `verbose` alone ([src/detox.c:93](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L93)). Asymmetry now stated in the `-n` row.                                                                                                                                                                                                             |
| `wipeup` paragraph contained an unedited "wait, more precisely" self-correction and mis-described the run (L3)    | ACCEPTED     | **Verified empirically:** `this__--.txt` → `this.txt` (run is `__--.`, period wins at index 0) and `a__-b.txt` → `a-b.txt` (no period in the run, dash wins). Fragment deleted; §2.3 and §8 item 7 rewritten with both reproductions.                                                                                                                                                                                                                                                                                                                                                                                                                        |
| §8 item 7's parenthetical contradicts itself about `remove_trailing` (L2)                                         | ACCEPTED     | Rewritten as separate sentences: the shipped `default` sequence does set `remove_trailing`; a sequence omitting it gets dash > underscore.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| No pinned commit / no date in front matter (L1 MINOR, L2 MINOR ×2, L3 CRITICAL)                                   | ACCEPTED     | Front-matter block added naming `0a8e2127e3c59cb419912d77c50f592b6460480a` and 2026-07-31, matching docs 10-13's convention, plus a note that the original capture used an unpinned tip-of-`main` tree.                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Load-bearing claims carry no `[path:line](github…)` links (L3)                                                    | ACCEPTED     | Links added for the collision algorithm, the `-r` gating, the `--special` gates, the `-n` gating asymmetry, config search order, `-s` precedence, all six filters, the table search path, and the `-L` dump split. Deliberately **not** applied sentence-by-sentence.                                                                                                                                                                                                                                                                                                                                                                                        |
| L3's suggested line ranges: `file.c:121-153`, `config_file.c:56-93`, `clean_string.c:29-80`, search string at 209 | MODIFIED     | **Verified source-read**, all four were slightly off. Used `file.c:124-140` (the `lstat`-old/`lstat`-new existence and inode check; `parse_file` itself is 69-160), `config_file.c:38-93` (`config_file_load` starts at 38), `clean_string.c:29-88` (`clean_iso8859_1`'s actual extent), and search string at **line 210**, not 209.                                                                                                                                                                                                                                                                                                                         |
| Exit-code behavior missing entirely (L2)                                                                          | MODIFIED     | Added as a short paragraph with a pointer to `10-detox-cli-surface.md` for the full table, rather than duplicating that table. **Verified empirically:** `detox -n "nonexistent one.txt" "needsclean file.txt"` printed the error to stderr, renamed the other file, and exited **0**.                                                                                                                                                                                                                                                                                                                                                                       |
| inline-detox "supports … but not `-r`/`--special`/`-n`" reads as rejection (L2)                                   | ACCEPTED     | **Verified source-read:** both binaries share one option string `"hrvV?Ls:f:n"` and one `longopts[]` at [src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140). Reworded to "accepted and silently ignored."                                                                                                                                                                                                                                                                                                                                                                                                      |
| `DATADIR` search step's `#ifdef` nuance omitted (L2 MINOR)                                                        | ACCEPTED     | **Verified source-read:** the guard is at [src/filter.c:42-43](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L42-L43). Added, with a pointer to doc 13 §1.2.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Observed behavior and author opinion are interleaved (L3 MINOR)                                                   | MODIFIED     | Explicit `_Interpretation_` / `_Opinion, not observation:_` markers added at the three places where the opinion was load-bearing (§5's same-inode escape-hatch reading, §8 items 3 and 7). A doc-wide prefix convention was **not** adopted — it would add noise to descriptive prose that carries no opinion.                                                                                                                                                                                                                                                                                                                                               |
| Downgrade the `-v` vs `-vv` claim to doc 10's `[UNVERIFIED]` (L2 MAJOR)                                           | **REJECTED** | This document was right and doc 10 was under-verified — deferring to the less-verified document would lose information. **Verified independently:** `grep -rn verbose src/*.c` at `0a8e212` finds every read of the field (`config_file_dump.c:24,31,40`, `detox.c:93`, `file.c:142`), all pure truthiness tests; the only other `verbose` is a local in `check_table.c` passed to `table_dump()`, unreachable from either shipped binary. Claim kept and its evidence strengthened in §1 instead.                                                                                                                                                           |
| Hedging is inconsistent across sections; §8 item 7 is fine as-is (L3 MINOR)                                       | **REJECTED** | No action requested by the reviewer and none taken as a standalone item — the specific under-hedged claims it pointed at (`-n`, `-L`) were not hedged but _corrected_, which is the better outcome. Noted as addressed rather than as a defect.                                                                                                                                                                                                                                                                                                                                                                                                              |
| Doc 01 lacks a peer stage-2 validation log (L2 MAJOR)                                                             | MODIFIED     | This section is the answer, but it is an adjudication of three reviews, not a per-claim re-derivation of the whole document. The scope note at the top now says so, and cedes authority to docs 10-13. A 30-row claim-by-claim log was deliberately not manufactured, because docs 10-13 already hold those enumerations.                                                                                                                                                                                                                                                                                                                                    |
| Narrow the document's role rather than retiring it (L2 recommendation)                                            | ACCEPTED     | Judged correct on the merits: docs 10-13 supersede this document's enumerations, but nothing there carries §7-§8's runnable reproductions or the single coherent bug narrative that doc 00 cites. Retitled to "Behavior Narrative and Empirical Bug Overview", added the "Scope of this document" block with the tie-break rule (10-13 win) and a topic-to-document routing table. No content was deleted.                                                                                                                                                                                                                                                   |

Out of scope, noted for whoever edits doc 10: L2's evidence table repeats the
`src/filelist.c: parse_dir` filename error while corroborating a doc 10 claim. The
function is in `src/file.c`; doc 10 was not edited as part of this pass.
