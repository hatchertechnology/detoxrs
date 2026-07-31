# Prior Art and Constraints for a Rust `detox` Successor

Scope: competitive landscape for filename-sanitizing / bulk-renaming CLIs, reusable Rust crates,
and the hard filesystem/Unicode/OS constraints a successor must respect. Detox's own behavior is
covered elsewhere; this doc treats it only as a positioning anchor.

## detox — positioning anchor

- Language: C. License: BSD-3-Clause. Installed locally via Homebrew at `detox 3.0.1`.
- `brew info detox` on this machine reports: **"Deprecated because it is not maintained upstream!
  It will be disabled on 2027-07-28."** Upstream (dharple/detox) has had no meaningful release
  cadence in years — this is the market gap a Rust successor is aimed at.
- CLI model: `detox [-hLnrvV] [-f configfile] [-s sequence] [--dry-run] [--recursive] [--special] file [file...]`
  — sequence-of-translation-tables config model (per-locale `.conf` files chained into a
  "sequence"), no regex engine, no undo, no conflict-safe atomic rename, no Unicode-security
  awareness (no bidi/zero-width/confusable handling — predates that being a mainstream concern).

## Comparative tools

| Tool                                                          | Lang                 | License                         | Maturity / last release                                                                                           | Install                                               | Strengths                                                                                                                                                                                                                                                                                                                                                                                   | Weaknesses                                                                                                                                                                                                                                                                                                                     | CLI/UX model                                                                                                                                                                    |
| ------------------------------------------------------------- | -------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **detox**                                                     | C                    | BSD-3                           | 3.0.1, unmaintained upstream, Homebrew flags disable-by-2027-04-28→2027-07-28                                     | Homebrew, apt, source                                 | Simple, fast, config-driven translation tables, recursive mode                                                                                                                                                                                                                                                                                                                              | No regex, no dry-run diff view, no undo, no atomicity, no Unicode-security                                                                                                                                                                                                                                                     | Positional args + config sequence                                                                                                                                               |
| **Perl `rename`/`prename`**                                   | Perl                 | Perl/Artistic                   | Ships with Perl since ~1990s, actively bundled (util-linux and Debian ship variants)                              | Bundled with Perl/most distros                        | Arbitrary Perl expression per file (`rename 's/foo/bar/' *`), extremely flexible                                                                                                                                                                                                                                                                                                            | Two same-named binaries on different distros conflicting in behavior (util-linux vs Perl) causes real user confusion; no dry-run by default (needs `-n`); no built-in undo                                                                                                                                                     | `rename PERLEXPR files...`                                                                                                                                                      |
| **util-linux `rename`**                                       | C                    | GPL-2.0                         | Ships with util-linux, continuously maintained as part of that suite                                              | Default on Fedora/CentOS/Arch                         | Trivial literal substring syntax (`rename from to files`), fast, no deps                                                                                                                                                                                                                                                                                                                    | No regex at all, so far weaker than Perl version; naming collision with Perl rename is a long-standing distro-compat headache                                                                                                                                                                                                  | `rename FROM TO files...`                                                                                                                                                       |
| **`mmv`**                                                     | C                    | BSD-ish (varies by fork)        | Original ~1990, largely unmaintained; forks exist (`itchyny/mmv` rewrites concept in Go)                          | apt/pld-linux packages                                | Wildcard pattern-with-backreference syntax (`mmv 'file*.c' '#1.bak'`) is intuitive for simple batch patterns; pre-flight collision detection before any move happens                                                                                                                                                                                                                        | No regex; wildcard-only patterns limit expressiveness; abandonware upstream                                                                                                                                                                                                                                                    | `mmv fromPattern toPattern`                                                                                                                                                     |
| **`qmv`/`imv` (renameutils)**                                 | C                    | GPL                             | Package maintained in Debian/Ubuntu archives, upstream (Ville Mattila) inactive for years                         | apt (`renameutils`)                                   | `qmv` opens an editable file-list in `$EDITOR`, diffs your edits against the original list, and applies them — arguably the best "renaming as text editing" UX yet designed; `imv` gives Readline-editable one-at-a-time rename                                                                                                                                                             | Requires a text editor round-trip; conflict/cycle resolution is opaque; no regex/pattern engine                                                                                                                                                                                                                                | Editor-buffer-diff model (qmv) / interactive single-file Readline edit (imv)                                                                                                    |
| **`f2`**                                                      | Go                   | MIT                             | Actively maintained, ~2.4k GitHub stars, current major version v2, regular releases                               | `go install`, npm, Homebrew, scoop, prebuilt binaries | **Strongest modern competitor.** Dry-run by default (must pass `-x/--exec` to actually apply); built-in undo via a rename-history log; automatic conflict detection/resolution (auto-appends counters, warns on case-only collisions and overwrite-of-existing); rich variable interpolation (EXIF, ID3, hashes, dates, ordinal counters); CSV-driven batch mode; JSON output for scripting | Go binary size; regex flavor limited to Go's RE2 (no backreferences/lookaround); Unicode-security posture (bidi/homoglyph/zero-width stripping) is not a first-class feature — it treats these as "just characters"                                                                                                            | Verb-first flag CLI: `f2 -f 'find' -r 'replace' [-R] [-x]`; separate "recipes" wiki teaches composition; safety-first (dry-run default) is the single biggest UX lesson to copy |
| **`rnr`**                                                     | Rust                 | MIT                             | `ismaelgv/rnr`, cargo/crates.io + Homebrew + AUR, moderate activity (~278 commits), not abandoned but slow-moving | `cargo install rnr`, Homebrew, AUR, prebuilt binaries | Regex with capture groups, recursive with depth limit, backup + dump-file undo, dry-run default (needs `-f` to force), transformation helpers (`upper`/`lower`/etc.)                                                                                                                                                                                                                        | **Self-documented limitation: "only UTF-8 valid input arguments and filenames"** — meaning it cannot even represent a non-UTF-8 (raw-bytes) Unix filename, a real-world case a Rust successor must not repeat; only one capture-group replacement per file by default; ignores directories unless `-D` passed, an easy footgun | Positional `rnr [FLAGS] [OPTIONS] EXPRESSION REPLACEMENT FILES...`                                                                                                              |
| **`repren`**                                                  | Python (stdlib only) | MIT/Apache-class (jlevy/repren) | ~373 stars, 167 commits, actively touched (recently added Claude Code skill support)                              | `uv tool install repren`, `uvx repren@latest`, pip    | Multi-pattern _simultaneous_ substitution (can swap `foo`↔`bar` in one pass without a temp rename), case-preserving variants for `camelCase`/`snake_case`/`kebab-case` refactors, renames content **and** paths together, dry-run/backup/undo                                                                                                                                               | No `.gitignore` awareness (must pass explicit `--exclude`), regex-only (no semantic/AST awareness), aimed at source-tree refactors more than general file hygiene                                                                                                                                                              | `repren --from X --to Y [--full] [DIR...]` or `--patterns=file` for bulk multi-pattern jobs                                                                                     |
| **`zmv` (zsh)**                                               | zsh function         | MIT-ish (zsh license)           | Ships with zsh, stable/unchanging for decades                                                                     | `autoload -Uz zmv` then use                           | Full glob + shell-variable-substitution power (`zmv '(*).jpg' '$1.jpeg'`), zero install, `-n` dry-run, case transforms via `${(L)1}`/`${(U)1}`                                                                                                                                                                                                                                              | Shell-only, not portable to bash/POSIX sh, no undo, easy to shoot yourself with substitution errors, no conflict pre-check                                                                                                                                                                                                     | Function call: `zmv 'srcpattern' 'dstpattern'`                                                                                                                                  |
| **`convmv`**                                                  | Perl                 | GPL                             | Long-stable, still packaged by all major distros                                                                  | apt/dnf/pacman                                        | Purpose-built for filename _byte-encoding_ conversion (e.g. CP1252→UTF-8), test-mode default (needs `--notest` to commit), rewrites symlink targets too                                                                                                                                                                                                                                     | Single-purpose (encoding only, not general sanitizing/renaming); Perl-encoding-detection heuristics are imperfect on ambiguous byte strings                                                                                                                                                                                    | `convmv -f FROM_ENC -t TO_ENC [--notest] files...`                                                                                                                              |
| **`slug`/`slugify` CLIs** (Node `slugify`, `pyslugify`, etc.) | JS/Python, varies    | MIT (typical)                   | Fragmented ecosystem, many tiny competing packages                                                                | npm/pip                                               | Simple, well-understood “make this a URL-safe slug” transform; good deunicode-style transliteration tables                                                                                                                                                                                                                                                                                  | Slugs are lossier than filename sanitizing needs (drops case, collapses to `-`), not designed to preserve human-readable file names, not filesystem-aware (ignores platform reserved names, byte-length limits)                                                                                                                | Library call or thin CLI wrapper; no batch/dry-run/undo concepts                                                                                                                |
| **Python `pathvalidate`**                                     | Python               | MIT                             | Actively maintained (thombashi), v3.3.x, no external deps, ships a `pathvalidate-cli`                             | pip                                                   | Explicit per-platform target validation (`platform="windows"/"universal"/"POSIX"`), separate `validate_*` (raise) vs `sanitize_*` (fix) APIs — a good API split to imitate                                                                                                                                                                                                                  | Library-first, the CLI wrapper is thin/secondary; Python packaging story is heavier than a single static binary                                                                                                                                                                                                                | `pathvalidate.sanitize_filename(name, platform=...)`                                                                                                                            |
| **Python `python-slugify` / `sanitize-filename`**             | Python               | MIT                             | Stable, widely used as dependencies rather than standalone CLIs                                                   | pip                                                   | Simple, focused, good defaults                                                                                                                                                                                                                                                                                                                                                              | Not CLI tools in practice — used as libraries; sanitize-filename package is minimal (single function)                                                                                                                                                                                                                          | N/A — library import                                                                                                                                                            |

Sources for this table are in **Confidence & Sources** below; GitHub star/commit counts are point-in-time
snapshots (fetched 2026-07-31) and will drift.

### Rust crates worth reusing (not competitors, building blocks)

| Crate                                                   | Purpose                                                                                          | Note for this project                                                                                                                                                                  |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `sanitize-filename`                                     | Node-`sanitize-filename`-alike: strip reserved chars/Windows device names, truncate to 255 bytes | Good starting point, but check its truncation logic against grapheme-safety (see constraints below) before trusting it blindly                                                         |
| `sanitise-file-name`                                    | Alt spelling, claims fewer allocations (1 vs 3-4)                                                | Worth benchmarking against `sanitize-filename`                                                                                                                                         |
| `unicode-normalization`                                 | NFC/NFD/NFKC/NFKD per UAX #15                                                                    | Needed for the macOS NFD-on-write / NFC-elsewhere gap (see constraints)                                                                                                                |
| `deunicode`                                             | ASCII transliteration ("Beyoncé" → "Beyonce")                                                    | For an opt-in "ASCII-only" mode; lossy by design                                                                                                                                       |
| `slug`                                                  | URL-slug generation                                                                              | Only relevant if the tool offers a slugify mode distinct from filename-safe mode                                                                                                       |
| `unicode-security` / `unicode_skeleton` / `confusables` | UTS #39 confusable/skeleton detection, mixed-script detection                                    | Core to detecting homoglyph and Trojan-Source-style attacks in filenames                                                                                                               |
| `unicode-segmentation`                                  | Grapheme-cluster iteration                                                                       | Mandatory for any byte-length truncation to avoid splitting a cluster (see constraints)                                                                                                |
| `unicode-width`                                         | Display-width calculation                                                                        | For any TUI/table rendering of proposed renames (CJK/emoji width)                                                                                                                      |
| `encoding_rs` / `chardetng`                             | Encoding detection/conversion                                                                    | Needed to replicate `convmv`'s use case (non-UTF-8 legacy filenames on Unix)                                                                                                           |
| `clap`                                                  | CLI parsing                                                                                      | De facto standard; derive macros keep boilerplate low                                                                                                                                  |
| `figment` / `config` + `serde`                          | Layered config (files, env, CLI overrides)                                                       | To replicate detox's "sequence" config idea without its rigidity                                                                                                                       |
| `ignore` / `walkdir` / `jwalk`                          | Directory traversal, `.gitignore`-aware                                                          | `ignore` crate directly solves repren's "no .gitignore" weakness; `jwalk` for parallel walks                                                                                           |
| `globset`                                               | Glob matching                                                                                    | For `mmv`/`zmv`-style pattern UX                                                                                                                                                       |
| `regex`                                                 | Regex engine                                                                                     | Rust's `regex` crate is RE2-derived (no backreferences/lookaround) — same ceiling `f2` hits; note this as a known trade-off, not a bug to "fix" with `fancy-regex` unless truly needed |
| `indicatif`                                             | Progress bars                                                                                    | For large recursive batch jobs                                                                                                                                                         |
| `trash`                                                 | Send-to-trash instead of hard delete                                                             | Relevant for "undo a batch rename" safety net, complementary to a rename-log-based undo (à la `f2`/`rnr`)                                                                              |
| `rayon`                                                 | Data parallelism                                                                                 | For scanning/hashing large trees before a batch rename                                                                                                                                 |

## Cross-platform naming lessons from adjacent tools

- **git `core.precomposeUnicode`**: Git on macOS stores what HFS+/APFS hands it (NFD-decomposed),
  but Git's index/protocol assumes NFC. Setting `core.precomposeUnicode=true` makes Git recompose
  NFD→NFC before comparing/storing, avoiding spurious "untracked file" noise when a repo is shared
  between macOS and Linux/Windows. **Lesson: a renaming tool must pick one normalization form for
  its own bookkeeping (undo logs, dedupe checks) independent of what the OS handed it, or it will
  see the "same" file as two different names.**
- **rsync/rclone**: rclone ships `--local-unicode-normalization` (NFC) specifically because syncing
  from macOS (NFD-ish) to other backends produces spurious "different file" diffs; the maintainers'
  own issue tracker (#1472, #4228) documents the pain both of normalizing and of _not_ normalizing
  (normalizing can itself lose distinctness between two legitimately different byte sequences that
  render identically). **Lesson: normalization is a policy choice with real trade-offs, not a
  free correctness fix — make it an explicit, opt-in flag, not silent default behavior.**
- **Samba mangled names / 8.3**: Samba's name-mangling algorithm (5 chars + `~` + 2-char hash + 3
  chars of extension) is the classic cautionary tale of algorithmic short-name generation:
  collisions are avoided via hashing, not truncation, specifically so unrelated files don't merge
  into the same short name. **Lesson: if the tool ever needs to shorten overlong names (255-byte
  limit) for a batch of similarly-prefixed files, truncation alone will produce collisions —
  disambiguate with a hash or counter suffix, not silent truncation.**

## Hard constraints (each with a one-line design implication)

1. **Unicode normalization, NFC vs NFD — verified locally.** APFS is _normalization-preserving_
   (it stores exactly the byte sequence you hand it) but _not normalization-sensitive_ (NFC and NFD
   spellings of the same string resolve to the same directory entry for lookup purposes). Verified
   on this machine (Darwin 25.5, APFS): creating a file named NFC `'é'` (`\xc3\xa9`) and then
   calling `os.path.exists()` on the NFD spelling (`'e\xcc\x81'`) returns `True`, and `os.listdir()`
   shows the directory entry stored exactly as given (NFC bytes), not silently converted to NFD.
   → **Implication: the tool must normalize to one canonical form (NFC recommended, matching Linux/
   Windows/web conventions) before comparing/deduplicating names in its own logic, but must not
   assume the OS will do this for it, and must not corrupt a caller-supplied byte string it doesn't
   understand.**
2. **Case-insensitive filesystems (APFS default, exFAT, NTFS default).** Verified locally: creating
   `CaseTest.txt` and testing `[ -f casetest.txt ]` succeeds — same inode. Renaming `A.txt` → `a.txt`
   in one `rename(2)` call on a case-insensitive-but-case-preserving filesystem is a no-op collision
   with itself unless done via a two-step rename through an intermediate temp name.
   → **Implication: every case-only rename must go through a temp-name intermediate (`A.txt` →
   `.tmp-<rand>` → `a.txt`), and the tool must detect "differs only by case" as a distinct rename
   class before executing it.**
3. **Windows reserved device names** (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`,
   plus superscript-digit variants `COM¹`/`LPT²` etc. per Microsoft Learn) are reserved _regardless
   of extension_ (`NUL.txt` is still invalid) in every directory on classic Win32 namespace rules;
   Windows 11 relaxed this for some contexts but the bare name is still reserved.
   → **Implication: sanitize-for-Windows-portability mode must reserved-name-check the stem before
   the extension, not the whole filename, and should stay conservative (assume older Windows
   semantics) since files may later be copied to older systems or shares.**
4. **Trailing dots and spaces are silently stripped by Windows path normalization** (per Microsoft
   Learn "Naming Files, Paths, and Namespaces"), which means a file this tool creates as `"foo. "`
   or `"foo."` on Unix will become `"foo"` — a different, possibly colliding name — the moment it's
   copied to or viewed from Windows/SMB. → **Implication: a "Windows-safe" sanitize mode must strip
   trailing dots/spaces itself, proactively, rather than let the destination filesystem do it
   unpredictably.**
5. **MAX_PATH (260 chars) legacy Win32 limit** still bites unless the caller opts into long-path
   support (`\\?\` prefix or the Windows 10+ long-paths registry/manifest opt-in).
   → **Implication: warn (don't silently truncate) when a proposed rename would push a path over
   260 characters, since the tool can't know the destination's long-path opt-in state.**
6. **POSIX Portable Filename Character Set** is `[A-Za-z0-9._-]` — much narrower than what's
   merely "ugly." Actually shell-unsafe/dangerous characters are a small subset: leading `-`
   (parsed as a flag by any command invoked with a bare filename), embedded `/` (impossible on
   Unix, but relevant when sanitizing for a different target), newline/control characters (break
   line-oriented tools, `ls`, scripts parsing filenames), and glob metacharacters (`*?[]`) when a
   filename is later globbed unquoted. Spaces, most punctuation, and non-ASCII letters are _ugly_,
   not unsafe. → **Implication: separate "shell-dangerous" (leading `-`, control chars, embedded
   newlines) from "merely non-portable" (spaces, unicode, punctuation) as two different severities
   in the sanitizer, so the default mode doesn't over-mangle legitimately fine Unicode filenames.**
7. **255-byte-vs-255-UTF-16-unit component limits differ by filesystem — verified locally, and
   corrects a common online claim.** ext4's `NAME_MAX` is 255 **bytes**. For APFS, this research
   initially found conflicting secondary-source claims ("255 UTF-8 characters" vs "1022 UTF-8
   characters"); direct local testing on Darwin 25.5/APFS shows neither is precisely right: 255
   ASCII characters succeeds, but only 127 four-byte-UTF-8 emoji characters succeed (128 fails with
   `ENAMETOOLONG`) — 127 × 2 UTF-16 code units (emoji outside the BMP are UTF-16 surrogate pairs) =
   254, and 128 × 2 = 256 exceeds the limit. **This is consistent with APFS still enforcing the
   historic HFS+ catalog-record limit of 255 UTF-16 code units per component**, not 255 UTF-8 bytes
   and not 255 UTF-8 characters. → **Implication: truncation logic must be filesystem-aware (255
   bytes on ext4-family; effectively 255 UTF-16 code units, i.e. ≤255 codepoints but fewer for
   astral-plane characters, on APFS) and in all cases must truncate on a grapheme-cluster boundary
   (via `unicode-segmentation`), never mid-codepoint or mid-cluster, to avoid emitting invalid
   UTF-8 or splitting a combining-character sequence.**
8. **Unicode security: bidi controls, zero-width characters, homoglyphs/confusables, RTL override.**
   CVE-2021-42574 ("Trojan Source", Boucher & Anderson, Cambridge 2021) demonstrated that Unicode
   bidirectional control characters (e.g. RLO `U+202E`) can make displayed text order diverge from
   logical/byte order — the same trick works in filenames shown in a file manager or terminal, and
   zero-width characters (`U+200B` etc.) can make two visually-identical filenames byte-distinct
   (or vice versa via confusables per UTS #39). → **Implication: the default sanitize policy should
   strip or escape bidi control characters and zero-width characters outright, and offer a
   `unicode-security`/`unicode_skeleton`-backed warning when a batch rename would create two
   filenames that are confusable/skeleton-identical but byte-different (classic phishing/typosquat
   setup).**
9. **Filenames are bytes on Unix, not guaranteed valid UTF-8.** `rnr`'s own documentation admits
   its scope is "only UTF-8 valid input arguments and filenames" — it cannot even _represent_ a
   Unix filename that happens to be invalid UTF-8 (which POSIX permits; only `NUL` and `/` are
   actually forbidden bytes). → **Implication: in Rust, walk and manipulate filenames as `OsStr`/
   `OsString` (or `Vec<u8>`/`CStr` at the syscall boundary on Unix), and only lossily convert to
   `String`/UTF-8 at the point of _display_ or when a rule genuinely requires text (regex
   substitution). A successor that can't even open a file with a non-UTF-8 name is repeating rnr's
   documented limitation, not surpassing it.**
10. **Atomic/safe rename and clobber prevention.** Plain POSIX `rename(2)` **silently overwrites**
    an existing destination (`man 2 rename` / man7.org: "if newpath already exists it will be
    atomically replaced"). Linux 3.15+ `renameat2(2)` adds `RENAME_NOREPLACE` (returns `EEXIST`
    instead of clobbering; filesystem support varies — ext4 since 3.15, btrfs/tmpfs/cifs since
    3.17, xfs since 4.0, most others by 4.9) and `RENAME_EXCHANGE` for atomic swaps. macOS's
    `renamex_np(2)` offers the equivalent `RENAME_EXCL` (EEXIST on existing destination, requires
    `getattrlist` volume-capability `VOL_CAP_INT_RENAME_EXCL`) and `RENAME_SWAP` for atomic
    file↔file or file↔directory swaps (mutually exclusive with `RENAME_EXCL`).
    → **Implication: use `renameat2`/`RENAME_NOREPLACE` on Linux and `renamex_np`/`RENAME_EXCL` on
    macOS with a graceful fallback (check-then-rename with a documented, narrow TOCTOU window, or
    a `link()`+`unlink()` trick) on filesystems/OSes that don't support the flag — never call
    plain `rename(2)` for a batch operation where a name collision is possible, which is the normal
    case whenever sanitizing produces duplicate output names.**
11. **Cross-device rename, hardlinks, symlinks, rename-during-walk.** `rename(2)` fails with
    `EXDEV` across filesystem/mountpoint boundaries (must fall back to copy+unlink, losing
    atomicity and hardlink identity); renaming a file with existing hardlinks only affects that one
    directory entry (other links keep the old name — may be desired or surprising depending on
    intent); renaming a symlink vs. its target requires care about which one the user meant
    (`O_NOFOLLOW` semantics); and renaming files while a recursive directory walk (`walkdir`/
    `ignore`) is still iterating that same tree is a classic TOCTOU/"renamed the thing I was about
    to descend into" hazard. → **Implication: (a) detect `EXDEV` and either refuse cross-device
    renames by default or explicitly opt into copy+verify+unlink; (b) snapshot the walk's file list
    before applying any renames rather than renaming while iterating live; (c) make symlink
    handling an explicit, documented choice (rename the link vs. follow it) rather than an
    accident of `std::fs::rename`'s default behavior.**
12. **Windows illegal characters.** Beyond reserved names, Windows forbids `< > : " / \ | ? *` and
    ASCII control characters 0–31 in a filename (Microsoft Learn, "Naming Files, Paths, and
    Namespaces"). → **Implication: the Windows-portability sanitize profile needs its own
    character denylist distinct from the "shell-dangerous" Unix list in constraint 6 — the two
    profiles must be selectable independently, not merged into one "safe" set, since e.g. `:` is
    fine on Unix but breaks Windows/exFAT/NTFS.**

## Confidence & Sources

**High confidence — verified locally on this machine (Darwin 25.5, APFS, `python3`/shell):**

- APFS normalization-preserving-not-sensitive behavior (NFC write, NFD lookup succeeds, directory
  listing preserves the exact bytes given).
- APFS case-insensitive-but-case-preserving behavior (`CaseTest.txt` / `casetest.txt` alias to the
  same inode).
- APFS per-component length limit is **255 UTF-16 code units** in practice, not flatly "255 UTF-8
  characters" or "255 bytes" — derived by binary search against 4-byte-UTF-8 emoji (127 succeed,
  128 fail with `ENAMETOOLONG`; 127×2=254 UTF-16 units fits under 255, 128×2=256 doesn't). This
  refines/corrects secondary sources found online that state the limit as flatly "255 characters"
  or "1022 UTF-8 characters" without the UTF-16-code-unit nuance.
- `detox` 3.0.1 installed via Homebrew, and Homebrew's own formula metadata states it is
  deprecated and scheduled for disable on 2027-07-28 due to no upstream maintenance.
- `rnr`'s documented "only UTF-8 valid input arguments and filenames" limitation, read directly
  from its fetched README content.

**Medium confidence — from web search/fetch, not independently re-verified against primary specs:**

- [f2 GitHub](https://github.com/ayoisaiah/f2/) / [f2 Wiki](https://github.com/ayoisaiah/f2/wiki/) — feature list, dry-run-by-default, undo, MIT license, Go install path. Star count (~2.4k) and "current major version v2" are point-in-time and will drift; I did not open the Releases page to confirm the exact latest tag/date.
- [rnr GitHub](https://github.com/ismaelgv/rnr) — install methods, CLI flags, UTF-8-only limitation. Did not confirm exact latest release date/tag.
- [repren GitHub](https://github.com/jlevy/repren) — Python/stdlib-only claim, star/fork/commit counts, install via `uv tool`.
- [renameutils qmv/imv man pages](https://manpages.debian.org/unstable/renameutils/qmv.1.en.html) — editor-buffer-diff model description.
- [util-linux vs Perl rename confusion](https://francopasut.netlify.app/post/linux-rename-confusion/) and [tldr-pages issue #3125](https://github.com/tldr-pages/tldr/issues/3125) — the two-binaries-same-name distro conflict.
- [convmv man page](https://www.mankier.com/1/convmv) — test-mode-default, symlink-target rewriting.
- [mmv man page](https://www.systutorials.com/docs/linux/man/1-mmv/) — wildcard/backreference model, pre-flight collision detection.
- [zmv usage examples](https://blog.smittytone.net/2021/04/03/how-to-use-zmv-z-shell-super-smart-file-renamer/) — autoload requirement, `-n` dry-run, case-transform syntax.
- [git core.precomposeUnicode discussion](https://makandracards.com/makandra/17827-git-mac-working-unicode-filenames) and [git commit 76759c7](https://github.com/git/git/commit/76759c7dff53e8c84e975b88cb8245587c14c7ba) — HFS+ NFD-vs-NFC rationale.
- [rclone unicode normalization issues #1472](https://github.com/rclone/rclone/issues/1472) / [#4228](https://github.com/rclone/rclone/issues/4228) — normalization as a real, debated trade-off, not a free fix.
- [Samba name mangling algorithm, O'Reilly "Using Samba" ch5.4](https://www.oreilly.com/openbook/samba/book/ch05_04.html) — 5-char+hash+3-char mangling scheme.
- [Trojan Source / CVE-2021-42574](https://en.wikipedia.org/wiki/Trojan_Source), [Red Hat RHSB-2021-007](https://access.redhat.com/security/vulnerabilities/RHSB-2021-007) — bidi control character attack mechanics.
- [Microsoft Learn: Naming Files, Paths, and Namespaces](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file) — reserved names, illegal characters, trailing dot/space stripping, MAX_PATH.
- [renameat2(2) man page, man7.org](https://man7.org/linux/man-pages/man2/rename.2.html) and [Debian manpages](https://manpages.debian.org/testing/manpages-dev/renameat2.2.en.html) — `RENAME_NOREPLACE`/`RENAME_EXCHANGE` semantics and filesystem-support version matrix.
- [renamex_np man page, unix.com mirror of Apple docs](https://www.unix.com/man_page/mojave/2/renamex_np/) — `RENAME_EXCL`/`RENAME_SWAP` semantics and mutual exclusivity.
- crates.io pages for `sanitize-filename`, `sanitise-file-name`, `unicode-normalization`, `unicode_skeleton`, `unicode-security`, `confusables` — crate descriptions, not independently benchmarked.
- [pathvalidate GitHub/PyPI](https://github.com/thombashi/pathvalidate) — platform-targeted validate/sanitize split, MIT, active maintenance.

**Not verified / flagged as open questions:**

- Exact current latest-release _dates_ (not just version numbers) for `f2` and `rnr` — I saw
  version/activity signals but did not open each project's Releases page to pin a date; treat
  "actively maintained" as directionally correct but re-check before citing a specific date in the
  design doc.
- Whether APFS's 255-UTF-16-code-unit limit is documented anywhere in Apple's own current
  developer docs (it appears to be an inherited-from-HFS+ implementation detail rather than a
  publicly specified contract) — the local empirical test is solid, but I could not find an
  authoritative Apple statement matching it exactly, only third-party secondary sources with
  conflicting numbers (255 chars vs 1022 chars). Treat the 255-UTF-16-unit figure as
  strongly-evidenced-by-experiment but not Apple-documented.
- exFAT and NTFS component-length behavior (this doc states them by general reputation — 255 UTF-16
  units for NTFS is well-established, exFAT's is less commonly re-verified) — not independently
  tested locally since no exFAT/NTFS volume was available in this environment.
- Whether `std::fs::rename` in current stable Rust exposes any no-clobber option yet — as of the
  search, `rust-lang/libs-team` issue #131 ("Add `std::fs::rename_noreplace`") was open, implying
  the standard library does **not** yet provide this and a successor will need to call the raw
  syscalls (`renameat2`/`renamex_np`) itself, likely via a small platform-conditional `libc`/`rustix`
  wrapper rather than `std::fs`.
