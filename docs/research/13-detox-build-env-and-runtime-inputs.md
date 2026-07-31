---
title: detox — build-time, environment, and non-flag runtime inputs
pinned_commit: 0a8e2127e3c59cb419912d77c50f592b6460480a (tag v3.0.1 + 4 commits)
date: 2026-07-31
scope: |
  Every detox input that is NOT a CLI flag, config-file directive, or table-file
  entry: configure.ac options/AC_DEFINEs, compile-time path substitution,
  environment variables, locale/encoding behavior, filesystem inputs (stat,
  symlinks, recursion, unreadable dirs), packaging inputs (snap), bin/ scripts,
  and a documentation inventory.
read: |
  configure.ac, Makefile.am, src/Makefile.am, src/parse_options.c,
  src/config_file.c, src/parse_table.c, src/filter.c, src/file.c,
  src/filelist.c, src/detox.c, src/detox_struct.h, snap/snapcraft.yaml,
  bin/*, BUILD.md, README.md, CHANGELOG.md, HACKING-v1.md, THANKS.md, man/*
repo: https://github.com/dharple/detox
---

All line-numbered links below point at commit `0a8e212` on GitHub
(`https://github.com/dharple/detox/blob/0a8e212/<path>#L<n>`).

## 1. Build-time inputs (`configure.ac` / `Makefile.am`)

### 1.1 `configure.ac` options

| Option | Type | Default | Effect | Source |
|---|---|---|---|---|
| `--with-check` | `AC_ARG_WITH` | `no` | Requires `check >= 0.10.0` via pkg-config; on success `AC_DEFINE([HAVE_LIBCHECK],[1])` and sets `AM_CONDITIONAL([WITH_CHECK])`, which gates the unit-test build (`tests/unit/Makefile`). Errors out (`AC_MSG_FAILURE`) if requested but `check` isn't found. No effect on installed-binary runtime behavior — test-only. | [configure.ac:61-82](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L61-L82) |
| `--with-coverage` | `AC_ARG_WITH` | `no` | Adds `-fprofile-arcs` (if supported) and `AC_DEFINE([SUPPORT_COVERAGE],[1])`; sets `AM_CONDITIONAL([WITH_COVERAGE])`, which in `src/Makefile.am` adds `-ftest-coverage` to `DEFS` and enables the `coverage`/`coverage-text` make targets. Build/test-only. | [configure.ac:88-103](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L88-L103); [src/Makefile.am:89-106](https://github.com/dharple/detox/blob/0a8e212/src/Makefile.am#L89-L106) |
| `--enable-debug` | `AC_ARG_ENABLE` | `false` | `AC_DEFINE([DEBUG],[1], [Enables verbose debugging in key points])`. Gates `#ifdef DEBUG` blocks (e.g. UTF-8 boundary tracing in `parse_inline`, [src/file.c:285-374](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L285-L374)) that print step-by-step diagnostics to stderr during inline mode. Runtime-visible if compiled in. | [configure.ac:109-119](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L109-L119) |

Non-option build machinery in `configure.ac` that still shapes the binary:

- `AC_CHECK_FUNCS([getopt_long])` → `HAVE_GETOPT_LONG`; when absent, `--help`/long options degrade to short-only getopt and the usage message text itself changes (`-h` vs `-h --help`). [configure.ac:9](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L9); consumed at [src/parse_options.c:23-25](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L23-L25) and [src/parse_options.c:101-105](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L101-L105).
- `AC_STRUCT_ST_BLOCKS` → `HAVE_STRUCT_STAT_ST_BLOCKS`; when defined, `parse_table()` sizes its hash table from `st_blocks*512` instead of `st_size` for sparse/zero-size-reporting filesystems. [configure.ac:10](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L10); [src/parse_table.c:59-63](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L59-L63).
- `AC_SYS_LARGEFILE` — enables large-file `stat`/off_t support on 32-bit systems; affects ability to stat very large files but not observed logic branches. [configure.ac:12](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L12).
- `AC_CHECK_PROGS([MANDOC])` → `AM_CONDITIONAL([MANDOC_INSTALLED])` — build/doc-lint tooling only, no runtime effect. [configure.ac:14-15](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L14-L15).
- Compiler-flag autodetection (`-flto=auto`, `-fstack-clash-protection`, `-fstack-protector-strong`) — hardening flags, no behavioral effect. [configure.ac:49-51](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L49-L51).

### 1.2 Compile-time path substitution (`AM_CFLAGS`, `src/Makefile.am`)

| Macro | Value | Consumer | Effect |
|---|---|---|---|
| `DATADIR` | `$(datadir)` (autoconf, typically `<prefix>/share`) | [src/filter.c:42-49](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L42-L49), [src/config_file.c:58-63](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L58-L63)* | First search location for translation tables (`$DATADIR/detox/<name>`); guarded by `#ifdef DATADIR` since automake always defines it, this is effectively unconditional. |
| `SYSCONFDIR` | `$(sysconfdir)` (typically `<prefix>/etc`) | [src/config_file.c:58-63](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L58-L63) | First candidate for the system config file: `$SYSCONFDIR/detoxrc`. |
| — | `-DYY_NO_INPUT -DYY_NO_UNPUT` | lexer (`config_file_lex.l`) | Flex-generated-code hygiene, no runtime behavior. |
| — | `-D_FORTIFY_SOURCE=2` | libc | Hardening only. |

\* Correction: `config_file.c`'s use of `SYSCONFDIR` is separate from `filter.c`'s use of `DATADIR` — both are compiled in via `AM_CFLAGS` in [src/Makefile.am:9-16](https://github.com/dharple/detox/blob/0a8e212/src/Makefile.am#L9-L16).

Fallback search order once `DATADIR`/`SYSCONFDIR` candidates fail (all hardcoded, not overridable at runtime except via env vars in §2):

- Config file: `$SYSCONFDIR/detoxrc` → `/etc/detoxrc` → `/usr/local/etc/detoxrc` → (env-var candidates, §2) → built-in spoofed default. [src/config_file.c:56-91](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L56-L91)
- Translation table: `$DATADIR/detox/<name>` → `/usr/share/detox/<name>` → `/usr/local/share/detox/<name>`. [src/filter.c:42-65](https://github.com/dharple/detox/blob/0a8e212/src/filter.c#L42-L65)

### 1.3 Version string

`AC_INIT([detox], [3.0.1], ...)` sets `PACKAGE_VERSION`/`VERSION` in the generated `src/config.h`, which is what `-V` prints at runtime. This is the only place the runtime version string originates; it is a build input, not a flag. [configure.ac:2](https://github.com/dharple/detox/blob/0a8e212/configure.ac#L2)

### 1.4 Installed data files (from `Makefile.am`, not table content itself but *which tables exist*)

`dist_pkgdata_DATA` installs `cp1252.tbl`, `iso8859_1.tbl`, `safe.tbl`, `unicode.tbl` directly under `$(pkgdatadir)` (i.e. `$DATADIR/detox/`); `dist_legacy_DATA` installs the four legacy equivalents one level down at `$(pkgdatadir)/legacy/`. This governs which table *names* are resolvable at runtime by `filter_find_table()`, out of scope for table *content* but in scope as a build-time path layout input. [Makefile.am:14-26](https://github.com/dharple/detox/blob/0a8e212/Makefile.am#L14-L26)

## 2. Environment variables

Exhaustive — `grep -rn getenv src/` returns exactly three call sites, all shown below. No other `getenv` calls exist anywhere in `src/`.

| Variable | Read in | Effect |
|---|---|---|
| `DETOX_SEQUENCE` | [src/parse_options.c:133](https://github.com/dharple/detox/blob/0a8e212/src/parse_options.c#L133) | Sets the default sequence name (same slot as `-s`); read unconditionally at options-init time, before argv parsing, so `-s` on the command line overrides it later in the same function. |
| `HOME` | [src/config_file.c:73-79](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L73-L79) | Builds `$HOME/.detoxrc` as a config-file candidate, merged (not replacing) after the sysconfdir candidates via `parse_config_file(..., config_file, ...)` — i.e. it layers on top of system config rather than overriding it. |
| `XDG_CONFIG_HOME` | [src/config_file.c:81-87](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L81-L87) | Builds `$XDG_CONFIG_HOME/detox/detoxrc` as a further config-file candidate, merged after `$HOME/.detoxrc`. **Not XDG-spec-compliant fallback**: if `XDG_CONFIG_HOME` is unset, detox does *not* fall back to `$HOME/.config/detox/detoxrc` — it simply skips this candidate (`getenv` returns `NULL`, the `if` at line 82 is false). [UNVERIFIED-by-me-confirmed]: this is a straight read of the code, not a behavior guess. |

Config-file candidates accumulate (via `parse_config_file(file, existing_config_file, ...)` merge semantics) in this fixed order: `$SYSCONFDIR/detoxrc` → `/etc/detoxrc` → `/usr/local/etc/detoxrc` → `$HOME/.detoxrc` → `$XDG_CONFIG_HOME/detox/detoxrc` → built-in spoofed default if all fail. [src/config_file.c:38-97](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L38-L97)

No `TMPDIR`, `LANG`, `LC_ALL`, `PATH`, `XDG_DATA_HOME`, or `NO_COLOR`-style variables are read anywhere in `src/`. Absence confirmed by `grep -rn getenv src/*.c` (3 hits, all above) and `grep -rln "getenv" src/` (2 files: `parse_options.c`, `config_file.c`).

## 3. Locale / encoding environment

detox **does** call `setlocale(LC_CTYPE, "")` — exactly once, in `parse_table()`, immediately after `stat()`-ing the table file and before parsing it. [src/parse_table.c:49-52](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L49-L52)

The returned locale name (`system_ctype`, falling back to `""` if `setlocale` fails) is then used for exactly one purpose: matching table-file `start "<lang>"` / `end` blocks. A table can bracket a set of translation rules with `start "en_US.UTF-8"` ... `end`; detox does `strncasecmp(parsed, system_ctype, strlen(parsed))` to decide whether the current process locale matches the block's declared language tag, and only activates rules inside a matching block. [src/parse_table.c:97-127](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L97-L127)

This means: **whatever `LC_CTYPE` (or `LANG`/`LC_ALL` as read by libc's `setlocale("")`) is set to in the process environment directly gates which conditional translation-table rules apply.** This is a genuine, if narrow, environment-driven behavior: it's the C library's own `LC_CTYPE`/`LANG`/`LC_ALL` resolution (not a `getenv` call in detox's own code) that ultimately decides the match — detox itself never reads `LANG` or `LC_ALL` directly; it defers entirely to libc's `setlocale(LC_CTYPE, "")`, which on Linux/glibc reads `LC_ALL`, then `LC_CTYPE`, then `LANG`, in that order.

No other locale category (`LC_COLLATE`, `LC_MESSAGES`, etc.) is set or queried anywhere in `src/` — grepped `setlocale|LC_` across `src/*.c` and `src/*.h`; the only hits are the three lines above plus the `#include <locale.h>`. Absence of `LC_MESSAGES`/i18n message catalogs (no `gettext`, `catgets`, `.po` files in the repo) is also confirmed: `grep -rn "gettext\|catgets\|dgettext"` over the repo returns nothing. All user-facing strings (`usage_message`, `help_message` in `parse_options.c`) are static English literals with no localization layer. [UNVERIFIED nothing further to check — this is an absence finding backed by the grep above.]

## 4. Filesystem inputs

### 4.1 What is stat'd, and how

- **Command-line arguments** (non-inline mode): `lstat()` on each path from `main_options->files`, dispatching on `S_ISDIR` → recurse-eligible directory handling, `S_ISREG` → direct rename, else → only handled if `-S`/`options->special` is set. [src/detox.c:97-109](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L97-L109)
- **Inline-mode file arguments**: `lstat()`; directories are rejected outright with `"%s: is a directory\n"` (inline mode never recurses). [src/detox.c:115-124](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L115-L124)
- **Table files**: `stat()` (not `lstat`) purely to size the initial hash table from `st_size` (or `st_blocks` if `HAVE_STRUCT_STAT_ST_BLOCKS`) — a sizing hint, not a gate. [src/parse_table.c:44-63](https://github.com/dharple/detox/blob/0a8e212/src/parse_table.c#L44-L63)
- **Directory walk** (`parse_dir`, only reached when `-r`/recurse and the top-level path is a dir): `lstat()` the directory itself first (bails if it fails or isn't a dir), then `opendir()`/`readdir()`, then `lstat()` each entry to classify it. [src/file.c:176-232](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L176-L232)
- **Rename safety check** (`parse_file`): `lstat()` both old and prospective new path; if the new path exists, compares `st_dev`/`st_ino`/`st_nlink` — refuses to rename (prints "file already exists") if they differ or if the old file has more than one hard link (to avoid clobbering or silently merging hardlinked files). [src/file.c:124-140](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L124-L140)

### 4.2 Symlink handling

detox uses `lstat`, not `stat`, everywhere in the file-walk and rename-safety paths (§4.1) — **symlinks themselves are the objects acted on**; detox never dereferences a symlink to inspect what it points to. A symlink is neither `S_ISDIR` nor `S_ISREG`, so in non-recursive/non-special mode a bare symlink argument on the command line is silently skipped (falls through all three `if`/`else if` branches at [src/detox.c:101-109](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L101-L109)) unless `-S`/`special` is passed, in which case it's treated like a regular file and renamed. The lone exception is `stat()` in `parse_table.c` on table files, which is not user-path-facing.

### 4.3 Recursion behavior

Recursion is entirely gated by `options->recurse` (the `-r` flag, out of scope as a flag but the mechanism is in scope): in `main()`, a directory argument is *always* passed to `parse_dir()`, but inside `parse_dir()` itself, subdirectories found during the walk are only recursed into `if (options->recurse)` — i.e. `parse_dir` runs exactly one level unless recursion is explicitly requested, and recursion re-enters `parse_dir` unconditionally otherwise (no depth limit, no cycle detection for symlink loops — since directories are only entered via `S_ISDIR` on `lstat`, a symlink-to-directory is never itself entered as a directory, so classic symlink recursion loops are structurally avoided). [src/file.c:207-228](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L207-L228)

Ignored entries: dotfiles (`filename[0] == '.'`, i.e. all hidden files/dirs including `.` and `..`) and any name explicitly present in `options->files_to_ignore` (a config-file list, out of scope). [src/file.c:33-48](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L33-L48)

### 4.4 Unreadable / unopenable directories

If `opendir()` fails (permission denied, etc.), detox prints `"unable to parse: %s\n"` with `strerror(errno)` to stderr and returns — **except** if `errno == EMFILE` (too many open file descriptors), in which case it calls `exit(EXIT_FAILURE)` immediately rather than continuing the walk. All other opendir failures are non-fatal and the walk continues with siblings/other args. [src/file.c:192-203](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L192-L203)

### 4.5 Special files

`options->special` (the `-S`/`--special` long option, out of scope as a flag) is the runtime gate that makes detox operate on non-regular, non-directory filesystem objects (device nodes, FIFOs, sockets, dangling symlinks, etc.) both at the top level ([src/detox.c:107](https://github.com/dharple/detox/blob/0a8e212/src/detox.c#L107)) and during directory walks ([src/file.c:224](https://github.com/dharple/detox/blob/0a8e212/src/file.c#L224)) — the mechanism itself (what "special" means, structurally: "not S_ISDIR and not S_ISREG") is the in-scope finding.

## 5. Packaging inputs

### 5.1 `snap/snapcraft.yaml`

- `base: core22`, `confinement: devmode`. **`devmode` confinement means no sandboxing is actually enforced** — the snap runs with full access to the host filesystem exactly as an unconfined binary would; the `plugs`/`slots` needed for `strict` confinement were never defined (comment at line 12 says "use 'strict' once you have the right plugs and slots"). So the only packaging-relevant finding is: **as shipped, the snap manifest implies detox needs unrestricted filesystem access under confined models — a successor should plan for `home`/`removable-media` plugs at minimum if strict confinement is ever pursued.** [snap/snapcraft.yaml:11-12](https://github.com/dharple/detox/blob/0a8e212/snap/snapcraft.yaml#L11-L12)
- The snap builds from a **pinned upstream release tarball** (`v3.0.0-beta2`, stale relative to the repo's current `3.0.1`), not from the in-tree source — i.e. the snap packaging in this repo is not kept in lockstep with releases. [snap/snapcraft.yaml:19](https://github.com/dharple/detox/blob/0a8e212/snap/snapcraft.yaml#L19)
- Only one app is exposed, `detox` (not `inline-detox`), running `usr/local/bin/detox`. No `check-table`, `escape-utf-8`, or `inline-detox` app entries exist. [snap/snapcraft.yaml:25-27](https://github.com/dharple/detox/blob/0a8e212/snap/snapcraft.yaml#L25-L27)
- No other distro packaging hints (no `debian/`, `rpm/`, `.spec`, Homebrew formula, etc.) exist in the repo. Confirmed by `ls` at repo root — only `snap/` is present as a packaging directory.

## 6. `bin/` directory

All nine scripts are **build/release/test tooling, none are runtime-loaded or shipped in the installed binary path** (none referenced by `Makefile.am`'s `bin_PROGRAMS`/install targets, only by the `internals`/`valgrind`/dev-doc targets):

| Script | Purpose |
|---|---|
| `generate-builtin.sh` | Regenerates `src/builtin_table.c` from `table/*.tbl` via the `generate-builtin-table` helper binary — this is how the four primary tables get compiled *into* the `detox` binary as a fallback/default, rather than only living on disk. Invoked by `make internals`. [bin/generate-builtin.sh](https://github.com/dharple/detox/blob/0a8e212/bin/generate-builtin.sh) |
| `generate-embedded-detoxrc.php` | Reads `etc/detoxrc`, strips comments/whitespace, and emits a C string literal (`static char *detoxrc = ...`) — this is the literal source of the built-in "spoofed" config used when no config file is found on disk (`spoof_config_file()`, referenced at [src/config_file.c:90](https://github.com/dharple/detox/blob/0a8e212/src/config_file.c#L90)). [bin/generate-embedded-detoxrc.php](https://github.com/dharple/detox/blob/0a8e212/bin/generate-embedded-detoxrc.php) |
| `generate-legacy-tests.sh` | Dev/test scaffolding for `tests/legacy/`. |
| `generate-pdf.sh` | Regenerates the `man/*.pdf` files from the `man/*.1`/`.5` roff sources — documentation build only. |
| `generate-unit-tests.sh` | Dev/test scaffolding for `tests/unit/` (the `check`-based suite gated by `--with-check`). |
| `make-cp1252.sh`, `make-iso8859-1.sh` | One-off generators used historically to produce `table/cp1252.tbl` / `table/iso8859_1.tbl` — table *content* generation, out of scope, but confirms those two tables are algorithmically derived, not hand-authored. |
| `mallocfail-wrapper.sh` | Test harness for injecting malloc failures (fault-injection testing). |
| `simple-test.sh` | Ad hoc smoke-test script. |

None of these affect a normal end-user's runtime invocation of `detox`/`inline-detox`; they matter only to a successor's *build/release* pipeline design (in particular: embedding a default config and embedding default tables are both real, deliberate detox behaviors worth replicating, sourced from `etc/detoxrc` and `table/*.tbl` respectively at build time).

## 7. Documentation inventory

| File | Contents relevant to a successor's design |
|---|---|
| `README.md` | Project overview; explicitly states v3 dropped "transliterate all of Unicode" in favor of targeting "truly problematic characters" — a scope-narrowing decision worth inheriting or deliberately revisiting. Also flags packaging-maintainer concerns: `pkg-config`/`pkgconf` and possibly `libtool` build deps, and that default config/table files are no longer `.sample`-suffixed (historical footgun for packagers). [README.md](https://github.com/dharple/detox/blob/0a8e212/README.md) |
| `BUILD.md` | Full dev toolchain list (autoconf, automake, bison/yacc, flex, gcc, make, **php** — required only for `generate-embedded-detoxrc.php` — pkg-config); lint tools (astyle, mandoc); static analysis (cppcheck, sparse); test tools (check, lcov, valgrind, strace); the `make internals`/`make coverage`/`make distcheck` release checklist, including the `-ftest-coverage` linking hack noted in §1.1. Useful as a checklist of "what a from-scratch Rust build must NOT need" (no flex/bison/php equivalent required if table/config formats are redesigned). [BUILD.md](https://github.com/dharple/detox/blob/0a8e212/BUILD.md) |
| `CHANGELOG.md` | Keep-a-Changelog-format history back through v1.3.0; v3.0.1's only change is a unit-test timeout increase (#129) — confirms 3.0.1 is not a behavior-changing release relative to 3.0.0, useful for pinning "current behavior" baselines. [CHANGELOG.md](https://github.com/dharple/detox/blob/0a8e212/CHANGELOG.md) |
| `HACKING-v1.md` | v1-specific end-user troubleshooting guide (how to find which config file is active via `-L -v`); largely superseded by v2/v3 but documents the historical `-L -v` diagnostic convention a successor may want to keep. [HACKING-v1.md](https://github.com/dharple/detox/blob/0a8e212/HACKING-v1.md) |
| `THANKS.md` | Attribution list; notably credits `ninedotnine` for adding `$XDG_CONFIG_HOME` support and `a1346054` for maintenance work — corroborates §2's finding that XDG support was a community-driven addition, not core design, which may explain its non-spec-compliant fallback behavior (no `$HOME/.config` fallback). Also credits Sean M. Burke's `Text::Unidecode` (via a Behat PHP port) as the source of `unidecode.tbl`'s transliteration data — a licensing/provenance fact for anyone porting that table's *content* (out of this doc's scope but worth flagging forward). [THANKS.md](https://github.com/dharple/detox/blob/0a8e212/THANKS.md) |
| `man/detox.1` (+ `.pdf`) | Primary CLI man page — flag reference, out of scope here except as corroboration for `-S`/special and `-r`/recurse semantics used in §4. |
| `man/inline-detox.1` (+ `.pdf`) | inline-detox man page — corroborates the stdin/stdout streaming mode described in §4.1's inline-mode branch. |
| `man/detoxrc.5` (+ `.pdf`) | Config file format — out of scope (config file), but is the authoritative doc for the `start "<lang>"`/`end` locale-gated block syntax exercised in §3. |
| `man/detox.tbl.5` (+ `.pdf`) | Translation table format — out of scope (table file), but documents the same `start`/`end` block syntax at the table level referenced in §3. |
| `LICENSE` | Licensing terms — not behavior-relevant, noted for completeness. |

No other doc files exist in the repo (`find . -iname '*.md' -o -iname '*.txt'` at root scope returns only the five `.md` files above plus nested `LICENSE`).

## 8. Summary of absence findings (things detox does NOT do)

- Never calls `gettext`/`catgets`/any i18n message layer — all UI strings are static English (§3).
- Never reads `LANG`, `LC_ALL`, `LC_MESSAGES`, `LC_COLLATE`, `TMPDIR`, `PATH`, `XDG_DATA_HOME`, `NO_COLOR`, or any variable beyond the three in §2's table — confirmed by exhaustive `getenv` grep.
- `$XDG_CONFIG_HOME` unset does **not** trigger the XDG-spec default of `$HOME/.config` — it's simply skipped (§2).
- No depth limit or symlink-loop guard in recursion — structurally unnecessary because `lstat`+`S_ISDIR` never follows a symlink-to-directory as a directory (§4.3).
- No distro packaging manifests (deb/rpm/Homebrew) in-repo — only `snap/` (§5).
- Snap confinement is `devmode` (unsandboxed) — never actually confined despite being packaged as a snap (§5.1).
