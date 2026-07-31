---
title: detox configuration file (detoxrc) — exhaustive source-verified enumeration
pinned_commit: 0a8e2127e3c59cb419912d77c50f592b6460480a (tag v3.0.1 + 4 commits)
repo: https://github.com/dharple/detox
date: 2026-07-31
files_read:
  - src/config_file.c
  - src/config_file.h
  - src/config_file_lex.l
  - src/config_file_yacc.y
  - src/config_file_spoof.c
  - src/config_file_dump.c
  - src/detox_struct.h
  - src/sequence.c
  - src/filter.c
  - src/parse_options.c
  - src/Makefile.am
  - etc/detoxrc
  - tests/unit/test_spoof_config_file.template
scope: configuration file (detoxrc / .detoxrc) grammar, search path, sequence/merge semantics, filter syntax, built-in defaults, shipped example, error handling
---

Base link form: `https://github.com/dharple/detox/blob/0a8e212/<path>#L<n>`.

## 1. File search order (proven from source)

Function `config_file_load()` in
[src/config_file.c:38-97](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L38-L97).

| Order | Source | Path tried | Condition | On success |
|---|---|---|---|---|
| 0 | [src/config_file.c:45-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L45-L54) | value of `-f <configfile>` (`main_options->check_config_file`) | only if `-f` given | **exclusive** — parsed with `previous_list = NULL`; if `fopen` fails, detox prints `detox: unable to open: %s` and `exit(EXIT_FAILURE)`. None of the other paths below are tried. See [src/config_file.c:49-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L49-L54). |
| 1 | [src/config_file.c:58-63](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L58-L63) | `${SYSCONFDIR}/detoxrc` | only if compiled with `-DSYSCONFDIR` (autoconf `sysconfdir`, set in [src/Makefile.am:11](https://github.com/dharple/detox/blob/0a8e212/src/Makefile.am#L11)) | becomes `config_file` (previous_list=NULL) |
| 2 | [src/config_file.c:65-67](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L65-L67) | `/etc/detoxrc` | only if step 1 produced `config_file == NULL` | previous_list=NULL — **replaces**, does not merge with a hypothetical step-1 result (moot since step 1 already succeeded means this is skipped) |
| 3 | [src/config_file.c:69-71](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L69-L71) | `/usr/local/etc/detoxrc` | only if `config_file == NULL` after step 2 | previous_list=NULL — same replace-only behavior |
| 4 | [src/config_file.c:73-79](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L73-L79) | `$HOME/.detoxrc` | only if `HOME` env var set | passes the **existing** `config_file` as `previous_list` — this call **merges** (see §3) rather than replacing |
| 5 | [src/config_file.c:81-87](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L81-L87) | `$XDG_CONFIG_HOME/detox/detoxrc` | only if `XDG_CONFIG_HOME` env var set | also passes existing `config_file` as `previous_list` — **merges** |
| 6 | [src/config_file.c:89-91](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L89-L91) | none (compiled-in defaults) | only if `config_file` is still `NULL` after all the above | `spoof_config_file()`, see §5 |

Key facts proven directly from the code:

- **`-f` is exclusive.** If given, it is the *only* file considered; the system/home/XDG paths and compiled-in fallback are never consulted — confirmed by the early `if (check_config_file != NULL) { ...; } else { <steps 1-6> }` branch at [src/config_file.c:49-94](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L49-L94).
- **Steps 1-3 (SYSCONFDIR, `/etc`, `/usr/local/etc`) are mutually exclusive "first found wins", not merged.** Each call passes `NULL` as `previous_list` ([src/config_file.c:61](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L61), [:66](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L66), [:70](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L70)), and each is gated on `config_file == NULL` from the prior step, i.e. skip-if-already-found. Since `parse_config_file(..., NULL, ...)` starts a brand-new `config_file_t` (see [src/config_file_yacc.y:200-205](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L200-L205)), whichever of these three is found first is used verbatim and the later ones in this trio are never even attempted.
- **`$HOME/.detoxrc` and `$XDG_CONFIG_HOME/detox/detoxrc` are always attempted (independent of whether steps 1-3 found anything) and MERGE onto whatever was already loaded**, because they pass the running `config_file` pointer as `previous_list` ([src/config_file.c:77](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L77), [:85](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L85)). If `fopen` fails for either (`parse_config_file` returns `previous_list` unchanged at [src/config_file_yacc.y:190-193](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L190-L193)), the running result is simply carried forward — a missing file is a silent no-op, not an error.
- **Compiled-in default (`spoof_config_file()`) only fires if literally nothing was found anywhere** ([src/config_file.c:89-91](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L89-L91)) — it never merges with a partially-loaded config; if even one file loaded (even an empty one that parsed to a non-NULL `config_file_t`), the spoofed defaults are skipped entirely.
- There is **no XDG_CONFIG_DIRS handling**, no glob/directory-of-conf.d handling, and no `~/.config/detox/detoxrc` fallback if `XDG_CONFIG_HOME` is unset (no default substitution of `$HOME/.config` is coded) — confirmed by absence of any such logic in [src/config_file.c](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c). **[UNVERIFIED]** whether packagers commonly set `SYSCONFDIR=/etc` making step 1 and step 2 redundant in most builds — settled by reading the actual `./configure` output/`Makefile` of a built package, not in scope here.

## 2. Merge/replace semantics — proof

`parse_config_file()`, [src/config_file_yacc.y:180-255](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L180-L255):

- If `filename` can't be `fopen`'d, returns `previous_results` unchanged ([:190-193](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L190-L193)) — **missing/unreadable file is silently skipped**, no error, no message.
- `ret` is `previous_results` if non-NULL, else a fresh `config_file_init()` ([:200-205](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L200-L205)).
- The **sequence list is seeded from `previous_results->sequences`** and walked to its tail ([:211-220](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L211-L220)) — new sequences from this file are appended after existing ones.
- The **ignore list is seeded from `previous_results->files_to_ignore` if non-empty** ([:226-230](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L226-L230)) — ignore entries **accumulate/merge** across every successfully-parsed file, never replaced.
- `yyparse()` is called in a loop until `feof(yyin)` ([:239-242](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L239-L242)).

## 3. Duplicate sequence names across (or within) files

`cf_append_sequence_list()`, [src/config_file_yacc.y:273-321](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L273-L321):

1. Walks the existing sequence chain (`cf_sequence_ret`) looking for a `name` match via `strcmp` ([:282-292](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L282-L292)).
2. **If found**: the code takes the found node (`work`) and does **not** allocate a new one. There's an explicit `/* XXX - Free Old Tree */` comment noting the old filter list is leaked, not freed ([:295-299](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L295-L299)).
3. **If not found**: `sequence_init(current_name)` allocates a new node, appended to the tail of the chain ([:300-314](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L300-L314)).
4. Either way, `work->filters = cf_filter_ret` and `work->source_filename = current_filename` are (re)assigned ([:317-318](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L317-L318)).

**Conclusion, proven from source: a sequence with a duplicate name — whether the duplicate appears later in the same file or in a later-merged file — REPLACES the filter list and source-file annotation of the first occurrence in place (keeping its position in the linked list); it does not raise an error and does not append/merge the two filter lists.** This is how a user's `~/.detoxrc` sequence named `default` overrides the built-in/`/etc` `default` sequence's filters while other sequences from the earlier file survive untouched.

## 4. Grammar (EBNF, derived from the Bison grammar)

Tokens from [src/config_file_yacc.y:49-67](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L49-L67), lexer rules from [src/config_file_lex.l:20-67](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L20-L67), grammar productions from [src/config_file_yacc.y:73-173](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L73-L173).

```ebnf
configfile   ::= rule*                                       (* config_file_yacc.y:73-76 *)
rule         ::= sequence | ignore                            (* :78-81 *)

sequence     ::= "sequence" string "{" method+ "}" ";"        (* :83-90 *)
                 (* sequence_close requires CLOSE EOL, i.e. "};" — trailing ';' mandatory *)

method       ::= "uncgi" ";"
               | "lower" ";"
               | wipeup ";"
               | iso8859_1 ";"
               | utf_8 ";"
               | safe ";"
               | max_length ";"                                (* :92-109 *)

iso8859_1    ::= "iso8859_1"
               | "iso8859_1" "{" "}"
               | "iso8859_1" "{" "filename" string ";" "}"
               | "iso8859_1" "{" "builtin"  string ";" "}"     (* :111-118 *)

utf_8        ::= "utf_8"      | same 3 block forms as iso8859_1, token UTF_8   (* :120-127 *)
safe         ::= "safe"       | same 3 block forms as iso8859_1, token SAFE    (* :129-136 *)

wipeup       ::= "wipeup"
               | "wipeup" "{" "}"
               | "wipeup" "{" "remove_trailing" ";" "}"        (* :138-143 *)

max_length   ::= "max_length"
               | "max_length" "{" "}"
               | "max_length" "{" "length" NVALUE ";" "}"      (* :145-150 *)

ignore       ::= "ignore" "{" ignore_filename+ "}" ";"         (* :152-163 *)
ignore_filename ::= "filename" string ";"                      (* :165-168 *)

string       ::= QSTRING | ID                                  (* :170-173 *)
```

Lexer tokens (case-sensitive, exact keyword spellings), [src/config_file_lex.l:28-40](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L28-L40):

| Keyword/symbol | Token | Meaning |
|---|---|---|
| `builtin` | BUILTIN | selects a compiled-in translation table |
| `filename` | FILENAME | selects an on-disk translation table / ignore-entry pattern |
| `ignore` | IGNORE | starts an `ignore { ... }` block |
| `iso8859_1` | ISO8859_1 | ISO-8859-1→UTF-8 filter |
| `length` | LENGTH | `max_length` block's length keyword |
| `lower` | LOWER | lowercase filter |
| `max_length` | MAX_LENGTH | truncate-filename filter |
| `remove_trailing` | REMOVE_TRAILING | wipeup option: also strip trailing dots |
| `safe` | SAFE | safe-character filter |
| `sequence` | SEQUENCE | starts a `sequence NAME { ... }` block |
| `uncgi` | UNCGI | CGI-escape decode filter |
| `utf_8` | UTF_8 | UTF-8 control-code filter |
| `wipeup` | WIPEUP | whitespace/punctuation cleanup filter |
| `{` / `}` | OPEN / CLOSE | block delimiters |
| `;` | EOL | statement/block terminator — note the lexer token is literally named `EOL` but it is the **semicolon**, not the newline; newlines are pure whitespace ([:26](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L26)) |
| `"[^"\n]*["\n]` | QSTRING | double-quoted string; unterminated string (hits `\n` before closing `"`) prints `Unterminated character string` to **stdout** (not stderr) via `printf` and still returns a QSTRING token with the text up to the newline ([src/config_file_lex.l:45-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L45-L54)) — parsing continues, this is not a fatal error at the lexer level |
| `[a-zA-Z][a-zA-Z0-9_]*` | ID | bare identifier, used for unquoted sequence names / filenames |
| `[0-9]+` | NVALUE | integer, `atoi()`'d, used only by `max_length { length N; }` |
| `#.*` | (discarded) | line comment to end of line ([:24](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L24)) — `#` **anywhere** on a line starts a comment, no "must be first column" rule |

Notes on the grammar that are easy to get wrong when reimplementing:
- The `sequence` name is **mandatory** in the grammar (`sequence_open: SEQUENCE string OPEN` — [:86](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L86)); there is no anonymous-sequence-becomes-"default" syntax. The word `default` used in the shipped `etc/detoxrc` ([etc/detoxrc:11](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L11)) is just an ordinary bare `ID` token that happens to spell "default" — it is not a keyword.
- Because of the above, the fallback `if (current_name == NULL) current_name = wrapped_strdup("default");` in `cf_append_sequence_list()` at [src/config_file_yacc.y:276-278](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L276-L278) is **dead code under the current grammar** — `current_name` is unconditionally assigned by the `sequence_open` action ([:86](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L86)) before `cf_append_sequence_list()` can ever run, and it is never reset to NULL between sequences in the same file. **[UNVERIFIED]** whether some parser-error-recovery path can reach `cf_append_sequence_list()` with `current_name == NULL` — would need to trace Bison's generated error-recovery tables in `config_file_yacc.c`, out of scope here.
- "default sequence" (used when `-s` is not given) is a **runtime lookup by the literal string `"default"`**, done in `sequence_choose_default()` at [src/sequence.c:28-54](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L28-L54) — it is not a grammar concept. If no sequence named `default` exists, and no `-s` was given, it falls back to simply the **first sequence in the list** ([src/sequence.c:47-51](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L47-L51)). If `-s NAME` was given and no sequence matches, `which` stays `NULL` (caller must handle this — **[UNVERIFIED]**, not read: what `main()`/caller prints in that case, since `main.c` was outside this task's file list).
- All blocks (`sequence { }`, `ignore { }`, and the optional filter-option blocks `iso8859_1 { }`, `safe { }`, `utf_8 { }`, `wipeup { }`, `max_length { }`) require the `}` to be followed by `;` — enforced by `CLOSE EOL` appearing in every `*_close`/inline block rule ([e.g. :89](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L89), [:113](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L113), [:142](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L142), [:158](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L158)) except the zero-arg block form `iso8859_1 { }` / `safe { }` / etc. which is `TOKEN OPEN CLOSE` with **no trailing semicolon required** ([e.g. :113](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L113) is `ISO8859_1 OPEN CLOSE`, no EOL) — an inconsistency in the grammar between the empty-block form and the populated-block form.

## 5. Filter invocation syntax and per-filter options

Each filter/"method" line inside a `sequence { }` body has one of two shapes: bare keyword+`;`, or keyword+`{ options }`+`;` (except the bare-block empty form noted above). Table derived from [src/config_file_yacc.y:96-150](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L96-L150):

| Filter | Bare form | Options block | Option syntax | Semantics |
|---|---|---|---|---|
| `uncgi` | `uncgi;` | — (no options) | — | [:96](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L96) |
| `lower` | `lower;` | — (no options) | — | [:98](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L98) |
| `wipeup` | `wipeup;` or `wipeup{};` | `wipeup{ remove_trailing; };` | `remove_trailing` bare keyword | [:138-143](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L138-L143) |
| `iso8859_1` | `iso8859_1;` or `iso8859_1{};` | `iso8859_1{ filename "path"; };` or `iso8859_1{ builtin "name"; };` | exactly one of `filename STRING;` / `builtin STRING;` | [:111-118](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L111-L118) |
| `utf_8` | same pattern as `iso8859_1` | same | same | [:120-127](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L120-L127) |
| `safe` | same pattern as `iso8859_1` | same | same | [:129-136](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L129-L136) |
| `max_length` | `max_length;` or `max_length{};` | `max_length{ length N; };` | `length NVALUE;` (integer, `atoi`'d) | [:145-150](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L145-L150) |

- `filename`/`builtin` are **mutually exclusive in the grammar** (only one alternative production is matched at a time), but nothing stops a config from technically appearing twice — the grammar per production only allows one such statement anyway (single `FILENAME string EOL` or `BUILTIN string EOL`, not a list), so a second occurrence would be a **syntax error** (unexpected token before `}`), triggering the fatal path in §7.
- The string argument to `filename`/`builtin` can be a bare `ID` or a `QSTRING` ([:170-173](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L170-L173)); the shipped example always quotes them ([etc/detoxrc:13](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L13) etc.).
- Resolution of a `builtin "name"` table happens elsewhere (`filter.c`), searching `${DATADIR}/detox/<name>`, then `/usr/share/detox/<name>`, then `/usr/local/share/detox/<name>` — [src/filter.c:34-62](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L34-L62). Out of scope for this document (that's filter-table lookup, not config-file grammar) but noted because it's driven directly by the `builtin` directive's string argument.

## 6. `ignore { }` block

```ebnf
ignore ::= "ignore" "{" ( "filename" string ";" )+ "}" ";"
```
[src/config_file_yacc.y:152-168](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L152-L168). Each `filename "pattern";` line is appended via `filelist_put()` into a single global ignore list that is threaded through every config file parsed in the session (see §2) — entries **accumulate**, they are not deduplicated or overridden by name (there's no "name" for an ignore entry to collide on; `filelist_put` at [src/config_file_yacc.y:344-346](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L344-L346) is an unconditional append). Multiple `ignore { }` blocks in one file, or across merged files, all add to the same list.

The shipped default entry is `{arch}` (an Arch/tla version-control directory) — [etc/detoxrc:131-133](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L131-L133) and, identically, hard-coded in the compiled-in defaults at [src/config_file_spoof.c:150-151](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L150-L151).

**[UNVERIFIED]** the "dotfiles auto-ignored except when named on the command line" behavior mentioned in the `etc/detoxrc` comment ([etc/detoxrc:127-129](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L127-L129)) is implemented outside the config-file parser (in the file-walking/recursion code, not read for this task) — the comment is documentation of runtime behavior, not a config-file directive.

## 7. Compiled-in defaults (no config file found)

`spoof_config_file()`, [src/config_file_spoof.c:84-154](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L84-L154). It hand-builds, in C (not by parsing a string), the identical set of 11 sequences and 1 ignore entry that `etc/detoxrc` defines textually — confirmed line-by-line against `etc/detoxrc` and byte-identical to the `detoxrc` string literal embedded in the unit test template ([tests/unit/test_spoof_config_file.template:20-33](https://github.com/dharple/detox/blob/0a8e212/tests/unit/test_spoof_config_file.template#L20-L33)). Each spoofed sequence's `source_filename` is literally the string `"built-in config file"` ([src/config_file_spoof.c:60](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L60)), which is how `-Lv`/dump output ([src/config_file_dump.c:35](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L35)) distinguishes "used the fallback" from "loaded a real file".

Sequences created, in order (all built via helper functions `generate_sequence`/`generate_builtin_filter`/`generate_filter`/`generate_wipeup_filter`, [src/config_file_spoof.c:26-77](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L26-L77)):

| # | Name | Filters (in order) | Line |
|---|---|---|---|
| 1 | `default` | safe(builtin="safe") → wipeup(remove_trailing=1) | [:98-100](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L98-L100) |
| 2 | `iso8859_1` | iso8859_1(builtin="iso8859_1") → [same safe+wipeup as #1, shared pointer] | [:102-105](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L102-L105) |
| 3 | `iso8859_1-legacy` | iso8859_1(builtin="cp1252") → iso8859_1(builtin="iso8859_1") → [shared safe+wipeup] | [:107-111](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L107-L111) |
| 4 | `utf_8` | utf_8(builtin="unicode") → [shared safe+wipeup] | [:113-116](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L113-L116) |
| 5 | `uncgi` | uncgi → [shared safe+wipeup] | [:118-121](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L118-L121) |
| 6 | `lower` | safe(builtin="safe") → lower → wipeup(remove_trailing=1) *(separate, non-shared wipeup instance)* | [:123-127](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L123-L127) |
| 7 | `iso8859_1-only` | iso8859_1(builtin="iso8859_1") | [:129-131](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L129-L131) |
| 8 | `cp1252-only` | iso8859_1(builtin="cp1252") | [:133-135](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L133-L135) |
| 9 | `utf_8-only` | utf_8(builtin="unicode") | [:137-139](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L137-L139) |
| 10 | `uncgi-only` | uncgi | [:141-143](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L141-L143) |
| 11 | `lower-only` | lower | [:145-147](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L145-L147) |

Plus `files_to_ignore = { "{arch}" }` ([:150-151](https://github.com/dharple/detox/blob/0a8e212/src/config_file_spoof.c#L150-L151)). This exactly mirrors `etc/detoxrc`'s content and ordering (compare table above with `etc/detoxrc` §8 below) — the two are kept in sync by hand, there is no code generation linking them; a maintainer editing one without the other would silently desync them. **[UNVERIFIED]**: whether CI/tests enforce that `etc/detoxrc` and `spoof_config_file()` stay identical — `tests/unit/test_spoof_config_file.c`/`.template` do compare parsed-file output against the spoofed structure (test name implies it), but the full test assertions were not read line-by-line here.

## 8. Shipped example config, fully annotated

Full file: [etc/detoxrc](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc). 134 lines, reproduced with per-line annotation:

```
1    #                                    comment (whole-line)
2    # config file for detox(1)
3    #
4    # Remove problematic characters.
5    #
6
7    #
8    # Default sequence.
9    #
10
11   sequence default {                   sequence named "default" (bare ID, not a keyword)  [etc/detoxrc:11]
12       safe {                           filter: safe, options block open                    [:12]
13           builtin "safe";              option: use compiled-in table "safe"                [:13]
14       };                               close safe{} block (CLOSE EOL)                       [:14]
15       wipeup {                        filter: wipeup, options block open                    [:15]
16           remove_trailing;            option: also strip trailing dots/spaces               [:16]
17       };                               close wipeup{} block                                  [:17]
18   };                                   close sequence{} block                                [:18]
```
(remaining sequences in `etc/detoxrc:24-121` follow the identical pattern; see §7 table for the semantic content of each — `iso8859_1`, `iso8859_1-legacy`, `utf_8`, `uncgi`, `lower`, and the `*-only` variants meant for `inline-detox` rather than `detox`, per the comment at [etc/detoxrc:88-90](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L88-L90))

```
131  ignore {                             ignore block open                                    [:131]
132      filename "{arch}";               pattern to always skip during recursive walks         [:132]
133  };                                   close ignore{} block                                  [:133]
```

The comment at [etc/detoxrc:127-129](https://github.com/dharple/detox/blob/0a8e212/etc/detoxrc#L127-L129) ("Any file or directory starting with `.` is automatically ignored except when it is passed on the command-line") documents behavior implemented in the file-walk/recursion code, not the `ignore{}` directive itself — **[UNVERIFIED]** exact mechanism, not in the files read for this task (would require reading the recursion/directory-walk source, e.g. a `detox.c`/`recurse`-related file).

## 9. Error handling

| Condition | Behavior | Source |
|---|---|---|
| `-f <file>` given but unreadable | `fprintf(stderr, "detox: unable to open: %s\n", check_config_file); exit(EXIT_FAILURE);` — fatal, immediate | [src/config_file.c:49-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L49-L54) |
| System/home/XDG path unreadable (any of steps 1-5 in §1) | Silent skip — `parse_config_file` returns `previous_results` unchanged when `fopen()` fails ([src/config_file_yacc.y:190-193](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L190-L193)); no message printed, no exit | [src/config_file.c:58-91](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L58-L91) |
| No file found anywhere | Falls back to compiled-in `spoof_config_file()` — not an error at all | [src/config_file.c:89-91](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L89-L91) |
| Syntax error (Bison parse error, e.g. unknown keyword treated as bare `ID` in wrong position, missing `;`, mismatched `{`/`}`) | Bison's generated parser calls `yyerror(s)`, which does: `fprintf(stderr, "detox: error parsing config file %s: %s\n", current_filename, s); fprintf(stderr, "\tline %d", config_file_lineno); if (yytext) fprintf(stderr, ": %s", yytext); fprintf(stderr, "\n"); exit(EXIT_FAILURE);` — **fatal, process exits**, reports the current filename, line number (1-based, incremented per `\n` seen by the lexer at [src/config_file_lex.l:26](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L26)), and the offending token text | [src/config_file_yacc.y:257-270](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L257-L270) |
| Unrecognized keyword (lexed as `ID` since it doesn't match any of the fixed keyword patterns) | Not a distinct lexer error — it becomes a generic `ID`/`string` token, then almost certainly causes a grammar-level syntax error (caught by the row above) unless it happens to be syntactically valid where any `string` is expected (e.g. as a sequence name or a `builtin`/`filename` argument, where arbitrary identifiers are legal) | Grammar production `string ::= QSTRING \| ID` at [src/config_file_yacc.y:170-173](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L170-L173); no dedicated "unknown directive" diagnostic exists |
| Unterminated quoted string (`"foo` with no closing `"` before EOL) | Lexer prints `Unterminated character string` to **stdout** (via bare `printf`, not stderr) and returns the QSTRING token anyway with content truncated at the newline — **non-fatal at the lexer**; parsing continues and may or may not subsequently hit a grammar error | [src/config_file_lex.l:45-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file_lex.l#L45-L54) |
| Config file exists but is empty / contains only comments | Parses successfully to an (almost) empty `config_file_t` — `sequences == NULL`, `files_to_ignore` empty (unless merged from a previous file) — no error; downstream `sequence_choose_default()` would then return `NULL` if no sequence exists | Inferred from grammar `configfile ::= rule*` (zero repetitions valid) at [src/config_file_yacc.y:73-76](https://github.com/dharple/detox/blob/0a8e212/src/config_file_yacc.y#L73-L76); **[UNVERIFIED]** what the caller does with a NULL chosen-sequence at runtime — not in files read (would be in `main.c`) |

## 10. Fully annotated real example (synthetic, to exercise every grammar rule)

```detoxrc
# per-filter options exercised: filename form, builtin form, max_length, wipeup w/o remove_trailing
sequence my_seq {
    iso8859_1 {
        filename "/etc/detox/my_table.txt";   # iso8859_1 via on-disk table   (config_file_yacc.y:115)
    };
    max_length {
        length 200;                            # truncate to 200 chars       (config_file_yacc.y:149)
    };
    wipeup;                                     # bare form, no options      (config_file_yacc.y:138)
    uncgi;
    lower;
};

sequence "quoted name" {                        # QSTRING as sequence name   (config_file_yacc.y:170)
    safe;
};

ignore {
    filename "*.tmp";
    filename ".git";
};
```

Every directive above traces to a grammar production cited in §4-§6; there is no directive in this synthetic example that isn't covered by a line-numbered citation elsewhere in this document.

## 11. Summary of unresolved items

| Item | Why unresolved | What would settle it |
|---|---|---|
| Exact dotfile-auto-ignore mechanism | Implemented in recursion/file-walk code, not the config parser (out of this task's file scope) | Read `detox.c`/recursion source for `ignore`/dotfile handling |
| Behavior when `-s NAME` names a nonexistent sequence | `sequence_choose_default()` returns NULL in that case; caller behavior not traced | Read `main.c`/caller of `sequence_choose_default()` |
| Whether `test_spoof_config_file.c` asserts full structural equality between parsed `etc/detoxrc` and `spoof_config_file()` | Test body not read line-by-line | Read `tests/unit/test_spoof_config_file.c` assertions |
| Whether typical packaged builds define `SYSCONFDIR` (making steps 1-2 in §1 often redundant) | Depends on the actual `./configure`/build, not source alone | Inspect a built `Makefile`/package spec |
