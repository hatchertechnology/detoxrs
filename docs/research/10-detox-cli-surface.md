# detox CLI surface — verified enumeration

**Front matter**
- Source read: full clone of `github.com/dharple/detox`, pinned at commit `0a8e2127e3c59cb419912d77c50f592b6460480a` (short `0a8e212`, tag `v3.0.1` + 4 commits).
- Files read in full: `src/parse_options.c`, `src/detox.c`, `src/file.c`, `src/filelist.c`, `src/detox_struct.h`, `src/config_file.c`, `src/sequence.c`, `src/config_file_dump.c`, `man/detox.1`, `man/inline-detox.1`, `README.md`, `CHANGELOG.md`.
- Not read line-by-line: `src/config_file_yacc.c`/`config_file_lex.c` (detoxrc grammar internals), `src/filter.c`, `src/clean_*.c` (per-character filter semantics) — out of scope, this doc covers the CLI surface only.
- Date of this research: 2026-07-31.
- Link format used below: `[src/file:LINE](https://github.com/dharple/detox/blob/0a8e212/src/file#L LINE)`.

`detox` and `inline-detox` are **the same binary**; behavior branches on `basename(argv[0])` — [src/parse_options.c:130-137](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L130-L137). Both binaries share one `getopt`/`getopt_long` call with the identical option string and `longopts[]` table — [src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140). This means options that are meaningless in inline mode (`-r`, `--recursive`, `--special`) are **silently accepted and silently ignored** by `inline-detox` rather than rejected — see "Cross-cutting findings" below.

## Complete option table

| Short | Long | Arg | Binary | Set on `options_t` | Default | Source |
|---|---|---|---|---|---|---|
| `-f configfile` | *(none)* | required | both | `check_config_file` | unset → falls back to search path | [src/parse_options.c:153-158](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L153-L158) |
| `-h` | `--help` | none | both | — (prints + `exit(EXIT_SUCCESS)`) | — | [src/parse_options.c:145-151](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L145-L151) |
| `-L` | *(none)* | none | both | `list_sequences=1` | 0 | [src/parse_options.c:160-162](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L160-L162) |
| `-n` | `--dry-run` | none | both (accepted, no-op for `inline-detox`) | `dry_run=1` | 0 | [src/parse_options.c:164-166](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L164-L166) |
| `-r` | *(none, but `--recursive` also sets it)* | none | both (accepted, no-op for `inline-detox`) | `recurse=1` | 0 | [src/parse_options.c:168-170](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L168-L170) |
| `-s sequence` | *(none)* | required | both | `sequence_name` | `getenv("DETOX_SEQUENCE")`, else NULL → chooser picks `"default"`/first | [src/parse_options.c:172-177](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L172-L177), [src/parse_options.c:133](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L133) |
| `-v` | `--verbose` | none, **repeatable** | both | `verbose++` (counter, not bool) | 0 | [src/parse_options.c:179-181](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L179-L181) |
| `-V` | *(none)* | none | both | — (prints `PACKAGE_STRING` + `exit(EXIT_SUCCESS)`) | — | [src/parse_options.c:183-185](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L183-L185) |
| `-?` | *(none, undocumented)* | none | both | — (prints usage + `exit(EXIT_SUCCESS)`) | — | [src/parse_options.c:187-190](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L187-L190) |
| *(none)* | `--inline` | none | both, only meaningful for `detox` binary | `is_inline_mode=1` | `is_inline_bin` (from argv[0]) | [src/parse_options.c:47](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L47), [src/parse_options.c:194-196](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L194-L196) |
| *(none)* | `--recursive` | none | both (accepted, no-op for `inline-detox`) | `recurse=1` | 0 | [src/parse_options.c:48](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L48), [src/parse_options.c:198-200](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L198-L200) |
| *(none)* | `--special` | none | both (accepted, no-op for `inline-detox`) | `special=1` | 0 | [src/parse_options.c:49](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L49), [src/parse_options.c:202-204](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L202-L204) |
| positional | `file [file ...]`/`--` | 0+ paths | both | `main_options->files` (filelist) | required for `detox`; optional for `inline-detox` (reads stdin) | [src/parse_options.c:230-241](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L230-L241) |

`getopt`/`getopt_long` option string is `"hrvV?Ls:f:n"` for both binaries — [src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140) (fallback `getopt` without long-opt support: [src/parse_options.c:142](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L142), when `HAVE_GETOPT_LONG` is undefined). `--` (end-of-options marker) is standard `getopt` behavior — not special-cased in this source, so it works exactly as `getopt` provides it; not independently exercised by detox code.

Only `-n`/`--dry-run`, `-h`/`--help`, `-v`/`--verbose` have a genuine long-form equivalent wired via `getopt_long`'s 4th field pointing at the flag var directly ([src/parse_options.c:42-44](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L42-L44)); `--inline`, `--recursive`, `--special` are long-only options that funnel through the shared `long_option` static and the `case 0:` branch ([src/parse_options.c:47-49](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L47-L49), [src/parse_options.c:192-215](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L192-L215)). There is **no `--version` long option** despite `-V` existing — confirmed absent from `longopts[]` — [src/parse_options.c:39-53](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L39-L53).

## Per-option detail

### `-f configfile`

- Overrides the entire config search path. If parsing that file fails, `config_file_load` prints `detox: unable to open: <path>` to stderr and calls `exit(EXIT_FAILURE)` — no fallback attempted — [src/config_file.c:49-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L49-L54).
- Without `-f`, the search order is: `$SYSCONFDIR/detoxrc` (compile-time, only if `SYSCONFDIR` was defined) → `/etc/detoxrc` (only tried if the previous step didn't yield a config) → `/usr/local/etc/detoxrc` (only tried if still none) → **then, regardless of whether one of the above already succeeded**, `~/.detoxrc` is parsed and merged in via `parse_config_file(path, config_file, ...)` → then, likewise unconditionally, `$XDG_CONFIG_HOME/detox/detoxrc` is parsed and merged in → if nothing was found at all, `spoof_config_file()` (builtin defaults) is used — [src/config_file.c:56-93](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L56-L93). This means the man page's framing ("`~/.detoxrc` normally extends the system-wide file") is accurate, but it's a merge/extend at every step, not a first-match-wins search as the phrase might imply for the three system paths.
- Man page claim "No other config file will be parsed" when `-f` is given: **verified true** — the `-f` branch at [src/config_file.c:49-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L49-L54) never falls through to the search-path branch.

### `-L` (list sequences)

- Sets `list_sequences`; `main()` checks this **after** the config file loads and the sequence is chosen, and short-circuits before the "no sequence to work with" check and before any file processing — [src/detox.c:65-68](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L65-L68). Exit is implicit fallthrough to `exit(EXIT_SUCCESS)`.
- With `-L` alone: prints `available sequences:` then each sequence name indented, marking the active one with `(*)` — [src/config_file_dump.c:22-39](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L22-L39) (corrected from a prior `22-38`, which cut off line 39, the actual `printf` that prints the sequence name and the `(*)` marker).
- With `-L -v`: switches to a different, unindented format — `sequence name: X`, `source file: ...`, then one line per filter/cleaner and its properties (e.g. `remove trailing: yes/no` for wipeup, `length: N` for max_length, `builtin table:`/`translation table:`) — [src/config_file_dump.c:31-71](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L31-L71) (corrected from `24-73`, which included the leading non-verbose branch and the trailing loop-advance statement that aren't part of the verbose dump itself).
- If the config file defines any `files_to_ignore`, both `-L` and `-L -v` append a `files to ignore:` section listing them — [src/config_file_dump.c:76-82](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c#L76-L82).
- `-L` combined with `-r`/`--special`/`-n`: parsed, but has no effect — those flags are never read in the `-L` code path.

### `-n` / `--dry-run` — does **not** actually set `verbose`

- Man page: "This implies the `-v` option" — [man/detox.1:84-88](https://github.com/dharple/detox/blob/0a8e212/man/detox.1#L84-L88).
- Source: `-n` only sets `main_options->dry_run = 1` — [src/parse_options.c:164-166](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L164-L166). It never touches `verbose`.
- **Discrepancy**: the rename-announcement line in `parse_file` is gated on `options->verbose || options->dry_run` — [src/file.c:142-144](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L142-L144) — so for *that one message* `-n` behaves as documented. But the per-file `Scanning: %s` line in `main()` is gated on `options->verbose` **alone**, with no `dry_run` check — [src/detox.c:93-95](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L93-L95). So `-n` without `-v` shows rename previews but not "Scanning:" lines — it does not fully "imply `-v`" as the man page states, only approximates it for one print site.
- Not read/used at all in `parse_inline` ([src/file.c:242-426](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L242-L426) — no reference to `dry_run` in the whole function): passing `-n` to `inline-detox` is silently accepted by the option parser but has **zero effect** on inline processing.

### `-r` / `--recursive` — does not gate the first level of directory content

- Man page frames dotfile-skipping and subdirectory descent entirely under `-r` — [man/detox.1:89-97](https://github.com/dharple/detox/blob/0a8e212/man/detox.1#L89-L97).
- Source: when a positional argument is a directory, `main()` unconditionally calls `parse_dir()` on it once, regardless of `-r` — [src/detox.c:101-104](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L104). So immediate children of a directory named on the command line are always processed (renamed if regular files, or recursed-into-check if subdirectories) even without `-r`.
- Inside `parse_dir`, `options->recurse` gates only the *further* descent into subdirectories found in that first pass: regular files (or any file type when `--special` is set) are renamed unconditionally; subdirectories are themselves renamed unconditionally but only *descended into* (`parse_dir` called again) if `recurse` is set — [src/file.c:218-227](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L218-L227).
- **Net effect**: `detox somedir/` (no `-r`) still renames every file *directly inside* `somedir/`; `-r` is only needed to reach files nested two or more levels deep. This is a materially different mental model from "recurse into subdirectories" read literally, and the man page does not call this out.
- Dotfile/dot-dir skipping (`ignore_file`, filename[0]=='.') applies inside `parse_dir` unconditionally — [src/file.c:33-48](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L33-L48) — so it happens on that same always-runs first pass, not only when `-r` is given. It does **not** apply to a dotfile named directly as a positional argument (only `parse_dir`'s traversal calls `ignore_file`; `parse_file` on an explicit CLI argument does not) — matches the man page's "unless specified on the command line" [man/detox.1:91-95](https://github.com/dharple/detox/blob/0a8e212/man/detox.1#L91-L95), confirmed correct.
- `-r`/`--recursive` accepted by `inline-detox` (shared getopt string) but never read anywhere in `parse_inline`; no-op there.

### `--special`

- Gates whether non-regular, non-directory dirents (symlinks, devices, fifos, sockets) are processed, both in `main()`'s top-level loop — [src/detox.c:107-109](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L107-L109) — and inside `parse_dir` — [src/file.c:224](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L224).
- Symlinks are detected via `lstat` (not `stat`) everywhere paths are classified — [src/detox.c:97](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L97), [src/file.c:176](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L176), [src/file.c:217](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L217) — so a symlink is never classified `S_ISDIR`, and `parse_dir` never descends through a symlink even to a directory target, matching the man page's "will not recurse into symlinks that point at directories" claim [man/detox.1:108-109](https://github.com/dharple/detox/blob/0a8e212/man/detox.1#L108-L109). With `--special`, a symlink-to-directory gets *renamed* (its link name, via `parse_file`) but its target's contents are never walked.
- No short-option equivalent exists; `--special` is long-only.

### `-s sequence`

- Precedence: CLI `-s` (`sequence_name` set via `wrapped_strdup(optarg)`) overwrites whatever `sequence_name` was initialized to from `DETOX_SEQUENCE` — [src/parse_options.c:133](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L133) then [src/parse_options.c:172-177](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L172-L177). So precedence is `-s` > `$DETOX_SEQUENCE` > `"default"`/first-defined.
- `DETOX_SEQUENCE` is documented, but only in `man/detoxrc.5`, not in `man/detox.1`/`man/inline-detox.1`: "There is a special sequence, named `default`, ... This can be overridden through the command line option `-s` or the environmental variable `DETOX_SEQUENCE`." — [man/detoxrc.5:47-54](https://github.com/dharple/detox/blob/0a8e212/man/detoxrc.5#L47-L54). Also introduced per `CHANGELOG.md`'s `## [2.0.0-beta1]` Added section: "Added handling for an environmental variable `DETOX_SEQUENCE`...". (Previously marked `[UNVERIFIED]`; settled by reading `man/detoxrc.5` in full.)
- Sequence resolution (`sequence_choose_default`): looks for a sequence literally named `name` (or `"default"` if `name == NULL`); if not found **and** no name was given on the CLI/env, falls back to the *first* sequence defined in the config — [src/sequence.c:28-54](https://github.com/dharple/detox/blob/0a8e212/src/sequence.c#L28-L54). If a name *was* explicitly given (via `-s` or env) and no sequence with that exact name exists, `sequence_to_use` stays `NULL` and `main()` aborts with `detox: no sequence to work with` on stderr, `exit(EXIT_FAILURE)` — [src/detox.c:73-79](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L73-L79). No "did you mean" or list-of-valid-names is printed.

### `-v` / `--verbose` — integer counter, no observed second-level effect

- `verbose` is incremented (`verbose++`), not set to 1 — [src/parse_options.c:179-181](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L179-L181), and the field is declared `int verbose` — [src/detox_struct.h:97](https://github.com/dharple/detox/blob/0a8e212/src/detox_struct.h#L97). Every call site I found treats it as a boolean (`options->verbose` truthiness in `src/detox.c:93`, `src/file.c:142`, `src/config_file_dump.c:22-33`). **[UNVERIFIED]** whether any deeper "more verbose" behavior exists at `-vv`/`-vvv` — I did not inspect `src/filter.c`/table code for a use of `verbose > 1`; grep across `src/*.c` for `verbose` beyond the files read would settle it.

### `-h`/`--help`, `-V`, `-?`

- All three print immediately and `exit(EXIT_SUCCESS)` (0), even `-?`, which getopt returns both for the truly-unknown-option case and for a recognized-but-erroring case (missing required arg to `-s`/`-f`, since the option string has no leading `:`) — [src/parse_options.c:140](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L140), [src/parse_options.c:187-190](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L187-L190).
- Because `'?'` is itself listed in the option string (`"hrvV?Ls:f:n"`), `-?` is also a **directly typeable, undocumented alias for usage/help** with exit code 0 — not mentioned in either man page or in `help_message`/`usage_message`.
- Help/usage/version all print via `printf` to **stdout**, not stderr, even on the "unknown option" path — [src/parse_options.c:188-190](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L188-L190).
- `-h`/`-?` print differ by binary: `usage_message`/`help_message` vs. `usage_message_inline`/`help_message_inline`, selected via `main_options->is_inline_bin` (derived from `argv[0]`, not from `--inline`) — [src/parse_options.c:146-150](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L146-L150), [src/parse_options.c:188-190](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L188-L190).

### `default:` case — truly-unrecognized short option

- Distinct from `'?'`: if `getopt`/`getopt_long` somehow returns a character not enumerated by the `switch` (author's own comment says this "shouldn't" happen for `getopt_long`'s `case 0` sub-switch, [src/parse_options.c:206-212](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L206-L212)), the outer `switch`'s `default:` prints `unknown option: %c` to **stderr** and calls `exit(EXIT_FAILURE)` — [src/parse_options.c:217-219](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L217-L219). **[UNVERIFIED]** I could not construct a concrete CLI input that reaches this branch rather than `'?'` — the option string already declares `hrvV?Ls:f:n`, and `getopt` returns `'?'` for anything outside that set. Evidence that would settle it: tracing `getopt_long`'s libc behavior for a long option name that partially matches, or building and fuzzing the binary.

## Positional arguments

| Scenario | `detox` behavior | `inline-detox` behavior |
|---|---|---|
| Zero paths | Prints usage to stdout, `exit(EXIT_FAILURE)` — [src/parse_options.c:237-241](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L237-L241) (only when NOT inline mode) | Reads/writes stdin/stdout — `parse_inline(NULL, NULL, ...)` — [src/detox.c:126-128](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L126-L128) |
| One or more paths | Each stored in `main_options->files` in argv order, duplicates preserved (no de-dup) — [filelist_put](https://github.com/dharple/detox/blob/0a8e212/src/filelist.c#L100-L130), consumed in the same order via `filelist_get` — [src/filelist.c:62-74](https://github.com/dharple/detox/blob/0a8e212/src/filelist.c#L62-L74) | Same list mechanics; each path opened via `fopen(path, "r")`, line-by-line, output always to stdout (never renames a file, never writes back to the input file) — [src/file.c:242-273](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L242-L273) |
| Path is a directory | `lstat`'d; `parse_file` renames the dir itself, then `parse_dir` always processes its immediate children (see `-r` section above) — [src/detox.c:101-104](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L104) | `is a directory` printed to stderr, that entry skipped, loop continues to next path — [src/detox.c:119-121](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L119-L121) |
| Path is a regular file | Renamed via `parse_file` (subject to sequence filters) | Read and filtered line-by-line to stdout |
| Path is a special file (symlink, fifo, device, socket) and no `--special` | Silently skipped — no message at all (the `else if` chain in `main()` has no final `else`) — [src/detox.c:105-110](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L105-L110) | N/A — `inline-detox` doesn't have `--special`; classification only splits dir vs. non-dir ([src/detox.c:118-124](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L118-L124)), so a symlink/fifo/etc. path is treated like a regular file and opened with `fopen` |
| Path does not exist | `lstat` fails; `fprintf(stderr, "%s: %s\n", path, strerror(errno))`, loop continues — [src/detox.c:98-99](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L98-L99) and [src/detox.c:115-117](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L115-L117) — **process exit code is unaffected** (see Exit codes) | Same `lstat`, same stderr message, same continue |
| `-` as a path | **Not** treated as stdin/stdout — it's stored and consumed like any other filename string; `detox -` attempts `lstat("-")` on a literal file named `-` in the CWD, `inline-detox -` attempts `fopen("-", "r")` | same |
| `--` end-of-options marker | Not special-cased by detox; behaves per standard `getopt`/`getopt_long` semantics (subsequent args treated as positional even if they start with `-`) — not independently exercised in this source, this is a libc-level guarantee, not a detox-authored one | same |

## Exit codes

| Code | When | Source |
|---|---|---|
| 0 (`EXIT_SUCCESS`) | `-h`/`--help`, `-V`, `-?` (including the "unrecognized option" path!), successful `-L`, and **normal completion of `main()` regardless of any per-file `lstat`/`rename`/`fopen` errors encountered along the way** — `main()`'s final statement is unconditionally `return 0;` — [src/detox.c:131](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L131) | see above |
| 1 (`EXIT_FAILURE`) | `parse_options_getopt` returns `NULL` — **confirmed dead code**: `wrapped_malloc` (used by `options_init`) never returns on OOM, it prints `detox: out of memory: <strerror>` to stderr and calls `exit(EXIT_FAILURE)` itself — [src/wrapped.c:38-42](https://github.com/dharple/detox/blob/0a8e212/src/wrapped.c#L38-L42) — so `main_options == NULL` at [src/detox.c:38-41](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L38-L41) can never be true in a normal (non-`SUPPORT_COVERAGE`-instrumented) build; no config file loadable — [src/detox.c:45-48](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L45-L48); named `-f` config file fails to parse — [src/config_file.c:51-54](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L51-L54); no sequence resolvable — [src/detox.c:77-79](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L77-L79); zero positional args to non-inline `detox` — [src/parse_options.c:238-240](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L238-L240); `default:` branch of the getopt switch (truly-unmatched option char) — [src/parse_options.c:217-219](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L217-L219); `opendir()` fails with `EMFILE` while recursing — [src/file.c:197-200](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L197-L200) (other `opendir` failures just print and return, no exit) |

**Key finding**: `detox` has **no non-zero exit code for "some files could not be processed."** A run over 100 files where 99 don't exist and 1 succeeds exits 0, identical to a fully clean run. Scripts relying on exit code to detect partial failure will not see one.

## stdout vs. stderr

| Message | Stream | Source |
|---|---|---|
| Help / usage / version text | stdout | [src/parse_options.c:146-190](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L146-L190) |
| `Scanning: %s` (verbose) | stdout | [src/detox.c:94](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L94) |
| `old -> new` rename preview/announce | stdout | [src/file.c:143](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L143) |
| `-L` sequence listing | stdout | [src/config_file_dump.c](https://github.com/dharple/detox/blob/0a8e212/src/config_file_dump.c) |
| `path: strerror` (lstat/fopen failure on a positional arg) | stderr | [src/detox.c:99](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L99), [src/detox.c:117](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L117), [src/file.c:259](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L259), [src/file.c:268](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L268) |
| `%s: is a directory` (inline mode) | stderr | [src/detox.c:120](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L120) |
| `Cannot rename X to Y: file already exists` / `: strerror` | stderr | [src/file.c:135](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L135), [src/file.c:153](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L153) |
| `unable to parse: strerror` (opendir failure) | stderr | [src/file.c:194](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L194) |
| `detox: an error occurred while parsing command line arguments` | stderr | [src/detox.c:39](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L39) |
| `detox: no config file to work with` | stderr | [src/detox.c:46](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L46) |
| `detox: no sequence to work with` | stderr | [src/detox.c:77](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L77) |
| `detox: unable to open: X` | stderr | [src/config_file.c:52](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L52) |
| `unknown option: %c` (default case) | stderr | [src/parse_options.c:218](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L218) |

(Corrected: the prior single row lumped these five distinct stderr messages against a source list `detox.c:39,46,77` ordered by line number rather than matching message order — `39` is actually "an error occurred...", not "no config file to work with" as its position implied. Split into one row per message, each pinned to its actual line.)

Note the inconsistency: the `'?'` (unknown-option / missing-arg) path prints to **stdout** and exits 0, while the structurally similar `default:` (unmatched-option-char) path prints to **stderr** and exits 1. These two paths are reachable via different, overlapping inputs and disagree on both stream and exit status for what a user would experience as "I typed a bad flag."

## Mutual exclusion / interaction matrix

| Combination | Behavior |
|---|---|
| `-L` + any of `-n`/`-r`/`--special` | `-L` short-circuits before those flags are ever consulted; they're silently irrelevant, not rejected — [src/detox.c:65-68](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L65-L68) |
| `-L` + `-v` | Changes `-L` output format (see `-L` detail above) — not mutually exclusive, they compose |
| `-h`/`-V`/`-?` + anything else | These call `exit()` from directly inside the `getopt` loop, so options *before* them on the command line still ran their side effects (e.g. `detox -v -h` still increments `verbose` before printing help), but nothing after the loop (config load, file processing) ever executes — [src/parse_options.c:144-220](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L144-L220) |
| `-f file1 -f file2` (repeated) | Last one wins; `wrapped_strdup` overwrites `check_config_file` each time with no free of the prior value — source comment flags this as a known leak: `/* XXX - free multiple check_config_files */` — [src/parse_options.c:154-157](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L154-L157) |
| `-s seq1 -s seq2` (repeated) | Same pattern, last wins, same leak, same author comment — [src/parse_options.c:173-176](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L173-L176) |
| `-v -v -v` (repeated) | `verbose` becomes 3; no call site distinguishes beyond truthiness (see `-v` detail) |
| `--special` + `-r` | Composes: special files are processed at every level `-r` reaches; man page's CHANGELOG entry for 2.0.0-beta1 claims a security fix here ("Symlinks that point at directories are no longer followed when `--special` and `-r` are specified together") confirmed structurally true by the `lstat`-based classification (see `--special` detail) |
| `--inline` on the `detox` binary vs. running `inline-detox` | Documented as identical ("Running `detox --inline` is identical to running `inline-detox`" — [man/inline-detox.1:52-56](https://github.com/dharple/detox/blob/0a8e212/man/inline-detox.1#L52-L56)). **Verified with one caveat**: `is_inline_bin` (used only to select which help/usage text prints) stays 0 when invoked as `detox --inline`, so `-h`/`-?` under `detox --inline -h` print the **`detox` usage/help text, not the inline-detox text** — [src/parse_options.c:136-137](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L136-L137) vs. [src/parse_options.c:146-150](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L146-L150). Everything else (`is_inline_mode`-gated behavior in `main()`) is identical. |
| env `DETOX_SEQUENCE` + `-s` | `-s` wins (set after; see `-s` detail) |

## Version differences (from CHANGELOG.md)

| Version | CLI-surface-relevant change | Source |
|---|---|---|
| 2.0.0-beta1 | Removed deprecated `--remove-trailing` CLI option; use `wipeup { remove_trailing; };` in detoxrc instead | [CHANGELOG.md](https://github.com/dharple/detox/blob/0a8e212/CHANGELOG.md) `## [2.0.0-beta1]`, Removed section |
| 2.0.0-beta1 | BREAKING: transliteration no longer happens by default; equivalent to old default behavior is now `detox -s utf_8` | same, Changed section |
| 2.0.0-beta1 | Files/dirs starting with `.` are ignored during recursion (this is the `ignore_file` behavior verified above) | same |
| 2.0.0 | Verbose mode enabling changed to accept either `-v` or `--verbose` (implies pre-2.0.0 had no `--verbose` long form, or a different flag) — **[UNVERIFIED]** exact pre-2.0.0 CLI surface; this pinned checkout is 3.0.1, no 1.x source present to confirm | [CHANGELOG.md](https://github.com/dharple/detox/blob/0a8e212/CHANGELOG.md) `## [2.0.0]` |
| 2.0.0 | Added `$XDG_CONFIG_HOME` config search path (confirmed present in this checkout's `config_file.c`) | same |
| 3.0.0-beta1 | Removed `utf_8-legacy` sequence name (config-file/table surface, not a CLI flag) | same |

No CHANGELOG entries between 3.0.0-beta1 and 3.0.1 (HEAD) touch the option parser (`src/parse_options.c`) — confirmed by reading the full changelog through 1.4.5 and observing only table/build/test fixes after 3.0.0-beta1.

## Undocumented-in-man vs. present-in-parser (and vice versa)

| Item | Man page | Parser | Verdict |
|---|---|---|---|
| `-?` as help alias | Not mentioned | Present, functional | **Undocumented feature** |
| `-n`/`-r` accepted (but no-op) by `inline-detox` | Not mentioned (inline-detox.1 synopsis omits them) | Accepted by getopt string shared across both binaries; simply never read in the inline code path | **Undocumented no-op**, not a rejection |
| `--recursive`/`--special` accepted (but no-op) by `inline-detox` | Not mentioned | Same mechanism | **Undocumented no-op** |
| `-r` gating only "second level and deeper" of directory descent | man/detox.1 implies `-r` is required for any subdirectory processing | First level always processed regardless of `-r` (see detail above) | **Documented behavior does not match implementation** — most significant finding in this document |
| `-n` "implies `-v`" | man/detox.1:86-88 states this plainly | Only true for the rename-preview print, not the `Scanning:` print, and `verbose` itself is never set | **Partial discrepancy** |
| `DETOX_SEQUENCE` env var | Absent from detox.1/inline-detox.1; documented in `man/detoxrc.5:47-54` | Present, read at options-init time, overridable by `-s` | **Undocumented in the two man pages a CLI user would read first** (`detox.1`, `inline-detox.1`); fully documented in `detoxrc.5` |
| `--version` long option | Neither man page lists one (both only show `-V`) | Confirmed absent from `longopts[]` | **Consistent** — no discrepancy |

## Still [UNVERIFIED]

- Whether `verbose > 1` (i.e. `-vv`) changes anything beyond the boolean checks found in the four files read — settle by grepping `verbose` across all of `src/*.c`, particularly `src/filter.c` and the table-loading code.
- Whether a real terminal input can reach the `default:` branch in `parse_options.c` (as opposed to `'?'`) — settle by testing `getopt_long`'s behavior for ambiguous/ill-formed long-option input on the target libc, or building and fuzzing.
- Pre-2.0.0 verbose-flag CLI surface referenced by the CHANGELOG — settle by checking out the 1.4.5 tag and reading its `parse_options.c` equivalent (not present in this HEAD checkout).

(Two items previously listed here — `DETOX_SEQUENCE` documentation in `man/detoxrc.5`, and whether `parse_options_getopt` can return `NULL` — were settled during stage-2 validation; see the log below.)

## Validation log (stage 2)

Method: re-read `src/parse_options.c`, `src/detox.c`, `src/file.c`, `src/config_file.c`, `src/config_file_dump.c`, `src/sequence.c`, `src/filelist.c`, `src/detox_struct.h`, `man/detox.1`, `man/inline-detox.1`, `man/detoxrc.5`, `src/wrapped.c`, `CHANGELOG.md`, `src/config_file_spoof.c` in full against the pinned commit `0a8e212`, checking every cited line range. Also built the project from source (`brew install automake`, `autoreconf -fi`, `./configure`, `make` — clean build, no patches needed) and empirically exercised the disputed behaviors in a scratch directory under macOS/Darwin (arm64, gcc via Xcode CLT).

| Claim | Verdict | Evidence |
|---|---|---|
| Option table rows for `-f`,`-h`,`-L`,`-n`,`-r`,`-s`,`-v`,`-V`,`-?`,`--inline`,`--recursive`,`--special`, positional args — all line ranges | CONFIRMED | Re-read `src/parse_options.c:39-244` line-by-line; every cited range matches the switch-case boundaries exactly. |
| `getopt`/`getopt_long` option string `"hrvV?Ls:f:n"`, fallback without `HAVE_GETOPT_LONG` | CONFIRMED | Read-only; [src/parse_options.c:140,142]. |
| Only `-n`,`-h`,`-v` have real long-form via `getopt_long`'s 4th field; `--inline`/`--recursive`/`--special` funnel through `case 0` | CONFIRMED | Read-only; [src/parse_options.c:42-53,192-215]. |
| No `--version` long option | CONFIRMED | Read-only; `longopts[]` at [src/parse_options.c:39-53] has no `version` entry. |
| `-f` error message and exit, no fallback | CONFIRMED | Read-only; [src/config_file.c:49-54]. |
| Config search order: SYSCONFDIR → /etc → /usr/local/etc are first-match, but `~/.detoxrc` and `$XDG_CONFIG_HOME/...` are unconditionally merged in afterward | CONFIRMED | Read-only; [src/config_file.c:56-93] re-read, confirms the two later steps run regardless of whether an earlier step already set `config_file`. |
| Man page "no other config file parsed" with `-f` | CONFIRMED | Read-only; `-f` branch never falls through. |
| `-L` short-circuits after config load + sequence choice | CONFIRMED | Read-only + empirical: `detox -L` printed the sequence list without requiring positional files. |
| `-L` output format (plain list with `(*)`) | CORRECTED (line range) + CONFIRMED (content) | Empirically ran `detox -L`; output matched exactly (`available sequences:` then indented names, `default (*)`). Line range fixed from `22-38` to `22-39` — the original range excluded line 39, the actual name+`(*)` `printf`. |
| `-L -v` output format (verbose dump) | CORRECTED (line range) + CONFIRMED (content) | Empirically ran `detox -L -v`; output matched (`sequence name:`, `source file:`, per-filter `cleaner:`/`builtin table:`/`remove trailing:` lines). Line range tightened from `24-73` to `31-71`. |
| `files_to_ignore` section appended to `-L`/`-L -v` | CONFIRMED | Read-only; [src/config_file_dump.c:76-82]. |
| `-n`/`--dry-run` does not set `verbose`; man page's "implies `-v`" is only true for the rename-preview line, not `Scanning:` | CONFIRMED (empirically) | Ran `detox -n` (no `-v`) on a directory with one renameable file: printed `old -> new` preview, did **not** print any `Scanning:` line. Matches doc exactly. |
| `-n` has zero effect in `parse_inline` | CONFIRMED | Read-only; `dry_run` not referenced anywhere in [src/file.c:242-426]. |
| `-r` does not gate the first level of directory content; first-level children are always processed, only deeper descent needs `-r` | CONFIRMED (empirically) | Built a `dir1/{file, sub1/nested, .hidden}` tree. `detox dir1` (no `-r`) renamed `dir1/file one.txt` but left `dir1/sub1/nested one.txt` untouched. `detox -r dir1` then renamed the nested file too. Exactly matches the doc's most significant finding. |
| Dotfile skip in `parse_dir` is unconditional (not gated by `-r`), doesn't apply to an explicit CLI argument | CONFIRMED (empirically + read) | `.hidden one.txt` was never renamed in either run above (confirms `ignore_file`, [src/file.c:33-48]); explicit-argument path (`parse_file`) has no `ignore_file` call, confirmed by re-reading `main()`. |
| `--special` gates non-regular/non-dir dirents; symlinks classified via `lstat`, never descended into even with `--special` | CONFIRMED | Read-only; [src/detox.c:97,107-109], [src/file.c:176,217,224]. |
| `-s` precedence: `-s` > `$DETOX_SEQUENCE` > `"default"`/first-defined | CONFIRMED | Read-only; [src/parse_options.c:133,172-177], [src/sequence.c:28-54]. |
| `DETOX_SEQUENCE` undocumented in man pages | CORRECTED | It **is** documented, but only in `man/detoxrc.5:47-54`, not in `detox.1`/`inline-detox.1`. Original doc left this `[UNVERIFIED]`; now confirmed and cited. |
| No sequence resolvable → `detox: no sequence to work with`, exit 1 | CONFIRMED | Read-only; [src/detox.c:73-79]. |
| `verbose` is an incremented counter, not a bool; no observed use beyond truthiness in the files read | CONFIRMED, deeper check unverifiable at this budget | Read-only; [src/detox_struct.h:97], call sites re-checked. Did not grep `src/filter.c`/table code for `verbose > 1`; left `[UNVERIFIED]`. |
| `-h`/`-V`/`-?` all print + `exit(EXIT_SUCCESS)`, to stdout | CONFIRMED (empirically for `-?`) | `detox -?` (quoted to dodge shell globbing) exited 0 and printed usage to stdout, nothing to stderr. |
| `-?` is an undocumented, directly-typeable help alias | CONFIRMED | Read-only; absent from `help_message`/`usage_message` text ([src/parse_options.c:59-93]); confirmed present and functional via `-?` test above. |
| `default:` branch (truly unmatched option char): stderr, `exit(EXIT_FAILURE)` | CONFIRMED | Read-only; [src/parse_options.c:217-219]. Reachability by a real terminal input left `[UNVERIFIED]` (unchanged). |
| Zero positional args (non-inline): usage to stdout, `exit(EXIT_FAILURE)`; inline mode reads stdin instead | CONFIRMED (empirically for detox) | `detox` with no args printed usage to stdout and exited 1. |
| No de-dup of positional file args; consumed in argv order | CONFIRMED | Read-only; [src/filelist.c:100-130] (`filelist_put`) and [src/filelist.c:62-74] (`filelist_get`). |
| `-` is not special-cased as stdin/stdout | CONFIRMED | Read-only; no `strcmp(..., "-")` anywhere in `parse_options.c`/`detox.c`/`file.c`. |
| `--` end-of-options marker not detox-specific | CONFIRMED | Read-only; no handling in source, standard `getopt` behavior. |
| Exit code table (0 always on normal completion regardless of per-file errors; 1 for the five listed failure modes) | CONFIRMED, one entry improved | Read-only re-check of all five failure-mode line citations. The `parse_options_getopt() == NULL` entry was previously flagged `[UNVERIFIED]` re: reachability — now **CONFIRMED unreachable**: `wrapped_malloc` (`src/wrapped.c:24-45`) calls `exit(EXIT_FAILURE)` itself on OOM in a non-`SUPPORT_COVERAGE` build, so it never returns `NULL` to make `options_init`/`parse_options_getopt` return `NULL`. |
| "No non-zero exit for partial failure" key finding | CONFIRMED | Read-only; `main()`'s final statement is unconditionally `return 0;` ([src/detox.c:131]), independent of any per-file `lstat`/`rename`/`fopen` error printed along the way. |
| stdout/stderr message table | CORRECTED (one row split) | The row bundling `detox: no config file to work with` / `no sequence to work with` / `unable to open: X` / `an error occurred...` / `unknown option: %c` against a same-order-looking source list `detox.c:39,46,77` was wrong: line 39 is actually "an error occurred...", not the first message in that cell. Split into 5 rows, each with the correct single line. |
| `'?'` path (stdout, exit 0) vs `default:` path (stderr, exit 1) inconsistency | CONFIRMED | Read-only; re-verified both branches' streams and exit codes. |
| Mutual-exclusion/interaction matrix (all 8 rows, incl. `detox --inline -h` printing `detox`'s help text, not inline's) | CONFIRMED | Read-only; [src/parse_options.c:136-137,146-150] re-checked — `is_inline_bin` is set only from `argv[0]`, `--inline` never touches it. |
| CHANGELOG version-differences table (5 rows) and "no parser-relevant changes between 3.0.0-beta1 and 3.0.1" claim | CONFIRMED | Re-read `CHANGELOG.md` in full from `3.0.1` down through `1.4.1`; each cited entry's wording matches, and no intervening entry touches `parse_options.c`. |
| Undocumented-vs-parser table (7 rows) | CORRECTED (1 row) | The `DETOX_SEQUENCE` row's verdict changed from `[UNVERIFIED]` to a confirmed, cited "undocumented in `detox.1`/`inline-detox.1`, documented in `detoxrc.5`". Other 6 rows confirmed unchanged. |

**Empirically tested** (built from source, ran in a scratch dir): `-L`, `-L -v`, `-n` without `-v`, `-r` vs. no `-r` on a nested directory tree with a dotfile, `-?`, zero-argument invocation. **Read-only verification**: all option-table line citations, config search-path merge logic, exit-code/stream tables, man-page wording, CHANGELOG entries, `wrapped_malloc` OOM behavior, `detoxrc.5` wording. **Left unresolved** (unchanged from stage 1, genuinely out of budget): `-vv`+ behavior beyond boolean truthiness in `src/filter.c`/table code, and real-terminal reachability of the `default:` getopt branch.
