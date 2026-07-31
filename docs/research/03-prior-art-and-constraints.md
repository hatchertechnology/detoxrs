# Prior Art and Constraints for a Rust `detox` Successor

Scope: competitive landscape for filename-sanitizing / bulk-renaming CLIs, reusable Rust crates,
and the hard filesystem/Unicode/OS constraints a successor must respect. Detox's own behavior is
covered elsewhere; this doc treats it only as a positioning anchor.

> **Evidence precedence (read before citing anything here).** This document was written first and
> later audited. Where it conflicts with a later document, the later document wins:
>
> - **Doc 06 (`06-validation-constraints.md`) wins on the constraints below.** It re-tested them
>   empirically; constraints 2 and 3 in this document were **wrong as originally written** and have
>   been corrected in place (marked **[CORRECTED]**). Constraint numbers are stable — doc 00 cites
>   them by number — so nothing here is renumbered.
> - **Docs 10–13 win on any fact about the upstream C `detox`** (CLI surface, config file, filter
>   tables, build/runtime inputs). This document's detox notes are positioning colour, not ground
>   truth.
> - **Doc 23 (`23-online-alternatives-and-ecosystem.md`) is newer and broader on the competitive
>   landscape** and is authoritative on detox's maintenance status.
> - One correction runs the other way: doc 06's claim that no Rust crate wraps macOS
>   `renamex_np` is itself **refuted** — see constraint 10.
>
> Do not treat a constraint here as authoritative without checking whether a later doc revisited it.

## detox — positioning anchor

- Language: C. License: BSD-3-Clause. Installed locally via Homebrew at `detox 3.0.1`.
- **Upstream is archived, not merely quiet.** `GET https://api.github.com/repos/dharple/detox`
  (re-verified 2026-07-31) returns `"archived": true`, `pushed_at` `2026-07-12T02:21:55Z`, 446
  stars, 0 open issues. Doc 23 dates the archival to 2026-07-12 and records the maintainer placing
  the project on indefinite hold for time reasons. This — not the downstream Homebrew flag — is the
  primary fact about detox's lifecycle.
- `brew info detox` on this machine reports: **"Deprecated because it is not maintained upstream!
  It will be disabled on 2027-07-28."** Upstream (dharple/detox) has had no meaningful release
  cadence in years — this is the market gap a Rust successor is aimed at.
- CLI model: `detox [-hLnrvV] [-f configfile] [-s sequence] [--dry-run] [--recursive] [--special] file [file...]`
  — sequence-of-translation-tables config model (per-locale `.conf` files chained into a
  "sequence"), no regex engine, no undo, no conflict-safe atomic rename, no Unicode-security
  awareness (no bidi/zero-width/confusable handling — predates that being a mainstream concern).

## Comparative tools

| Tool                                                          | Lang                 | License                         | Maturity / last release                                                                                                                               | Install                                               | Strengths                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Weaknesses                                                                                                                                                                                                                                                                                                                     | CLI/UX model                                                                                                                                                                    |
| ------------------------------------------------------------- | -------------------- | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **detox**                                                     | C                    | BSD-3                           | 3.0.1; upstream GitHub repo archived 2026-07-12; Homebrew deprecated, scheduled for disable 2027-07-28                                                | Homebrew, apt, source                                 | Simple, fast, config-driven translation tables, recursive mode                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | No regex, no dry-run diff view, no undo, no atomicity, no Unicode-security                                                                                                                                                                                                                                                     | Positional args + config sequence                                                                                                                                               |
| **Perl `rename`/`prename`**                                   | Perl                 | Perl/Artistic                   | Ships with Perl since ~1990s, actively bundled (util-linux and Debian ship variants)                                                                  | Bundled with Perl/most distros                        | Arbitrary Perl expression per file (`rename 's/foo/bar/' *`), extremely flexible                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Two same-named binaries on different distros conflicting in behavior (util-linux vs Perl) causes real user confusion; no dry-run by default (needs `-n`); no built-in undo                                                                                                                                                     | `rename PERLEXPR files...`                                                                                                                                                      |
| **util-linux `rename`**                                       | C                    | GPL-2.0                         | Ships with util-linux, continuously maintained as part of that suite                                                                                  | Default on Fedora/CentOS/Arch                         | Trivial literal substring syntax (`rename from to files`), fast, no deps                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | No regex at all, so far weaker than Perl version; naming collision with Perl rename is a long-standing distro-compat headache                                                                                                                                                                                                  | `rename FROM TO files...`                                                                                                                                                       |
| **`mmv`**                                                     | C                    | BSD-ish (varies by fork)        | Original ~1990, largely unmaintained; forks exist (`itchyny/mmv` rewrites concept in Go)                                                              | apt/pld-linux packages                                | Wildcard pattern-with-backreference syntax (`mmv 'file*.c' '#1.bak'`) is intuitive for simple batch patterns; pre-flight collision detection before any move happens                                                                                                                                                                                                                                                                                                                                                                                                                                                               | No regex; wildcard-only patterns limit expressiveness; abandonware upstream                                                                                                                                                                                                                                                    | `mmv fromPattern toPattern`                                                                                                                                                     |
| **`qmv`/`imv` (renameutils)**                                 | C                    | GPL                             | Package maintained in Debian/Ubuntu archives, upstream (Ville Mattila) inactive for years                                                             | apt (`renameutils`)                                   | `qmv` opens an editable file-list in `$EDITOR`, validates your edited list against the original (sanity/conflict checks, not a visual diff — the manpage does not describe a diff view) and applies it. _Assessment (author's opinion, not a sourced fact): the best "renaming as text editing" UX yet designed._ `imv` gives Readline-editable one-at-a-time rename                                                                                                                                                                                                                                                               | Requires a text editor round-trip; conflict/cycle resolution is opaque; no regex/pattern engine                                                                                                                                                                                                                                | Editor-buffer-diff model (qmv) / interactive single-file Readline edit (imv)                                                                                                    |
| **`f2`**                                                      | Go                   | MIT                             | Actively maintained; latest release v2.2.2 (2025-11-10), 2,427 stars (GitHub API, 2026-07-31)                                                         | `go install`, npm, Homebrew, scoop, prebuilt binaries | _Assessment (author's opinion, not a sourced fact): the strongest modern competitor._ Dry-run by default (must pass `-x/--exec` to actually apply); built-in undo via a per-directory JSON backup file (named from an MD5 hash of the directory path, **overwritten on each run** — not an append-only history log, so only the most recent batch per directory is undoable); automatic conflict detection/resolution (auto-appends counters, warns on case-only collisions and overwrite-of-existing); rich variable interpolation (EXIF, ID3, hashes, dates, ordinal counters); CSV-driven batch mode; JSON output for scripting | Go binary size; regex flavor limited to Go's RE2 (no backreferences/lookaround); Unicode-security posture (bidi/homoglyph/zero-width stripping) is not a first-class feature — it treats these as "just characters"                                                                                                            | Verb-first flag CLI: `f2 -f 'find' -r 'replace' [-R] [-x]`; separate "recipes" wiki teaches composition; safety-first (dry-run default) is the single biggest UX lesson to copy |
| **`rnr`**                                                     | Rust                 | MIT                             | `ismaelgv/rnr`, cargo/crates.io + Homebrew + AUR, moderate activity (~279 commits), latest release v0.5.1 (2025-12-13), not abandoned but slow-moving | `cargo install rnr`, Homebrew, AUR, prebuilt binaries | Regex with capture groups, recursive with depth limit, backup + dump-file undo, dry-run default (needs `-f` to force), transformation helpers (`upper`/`lower`/etc.)                                                                                                                                                                                                                                                                                                                                                                                                                                                               | **Self-documented limitation: "only UTF-8 valid input arguments and filenames"** — meaning it cannot even represent a non-UTF-8 (raw-bytes) Unix filename, a real-world case a Rust successor must not repeat; only one capture-group replacement per file by default; ignores directories unless `-D` passed, an easy footgun | Positional `rnr [FLAGS] [OPTIONS] EXPRESSION REPLACEMENT FILES...`                                                                                                              |
| **`repren`**                                                  | Python (stdlib only) | MIT/Apache-class (jlevy/repren) | ~373 stars, 167 commits, actively touched (recently added Claude Code skill support)                                                                  | `uv tool install repren`, `uvx repren@latest`, pip    | Multi-pattern _simultaneous_ substitution (can swap `foo`↔`bar` in one pass without a temp rename), case-preserving variants for `camelCase`/`snake_case`/`kebab-case` refactors, renames content **and** paths together, dry-run/backup/undo                                                                                                                                                                                                                                                                                                                                                                                      | No `.gitignore` awareness (must pass explicit `--exclude`), regex-only (no semantic/AST awareness), aimed at source-tree refactors more than general file hygiene                                                                                                                                                              | `repren --from X --to Y [--full] [DIR...]` or `--patterns=file` for bulk multi-pattern jobs                                                                                     |
| **`zmv` (zsh)**                                               | zsh function         | MIT-ish (zsh license)           | Ships with zsh, stable/unchanging for decades                                                                                                         | `autoload -Uz zmv` then use                           | Full glob + shell-variable-substitution power (`zmv '(*).jpg' '$1.jpeg'`), zero install, `-n` dry-run, case transforms via `${(L)1}`/`${(U)1}`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Shell-only, not portable to bash/POSIX sh, no undo, easy to shoot yourself with substitution errors, no conflict pre-check                                                                                                                                                                                                     | Function call: `zmv 'srcpattern' 'dstpattern'`                                                                                                                                  |
| **`convmv`**                                                  | Perl                 | GPL                             | Long-stable, still packaged by all major distros                                                                                                      | apt/dnf/pacman                                        | Purpose-built for filename _byte-encoding_ conversion (e.g. CP1252→UTF-8), test-mode default (needs `--notest` to commit), rewrites symlink targets too                                                                                                                                                                                                                                                                                                                                                                                                                                                                            | Single-purpose (encoding only, not general sanitizing/renaming); Perl-encoding-detection heuristics are imperfect on ambiguous byte strings                                                                                                                                                                                    | `convmv -f FROM_ENC -t TO_ENC [--notest] files...`                                                                                                                              |
| **`slug`/`slugify` CLIs** (Node `slugify`, `pyslugify`, etc.) | JS/Python, varies    | MIT (typical)                   | Fragmented ecosystem, many tiny competing packages                                                                                                    | npm/pip                                               | Simple, well-understood “make this a URL-safe slug” transform; good deunicode-style transliteration tables                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Slugs are lossier than filename sanitizing needs (drops case, collapses to `-`), not designed to preserve human-readable file names, not filesystem-aware (ignores platform reserved names, byte-length limits)                                                                                                                | Library call or thin CLI wrapper; no batch/dry-run/undo concepts                                                                                                                |
| **Python `pathvalidate`**                                     | Python               | MIT                             | Actively maintained (thombashi), v3.3.x, no external deps, ships a `pathvalidate-cli`                                                                 | pip                                                   | Explicit per-platform target validation (`platform="windows"/"universal"/"POSIX"`), separate `validate_*` (raise) vs `sanitize_*` (fix) APIs — a good API split to imitate                                                                                                                                                                                                                                                                                                                                                                                                                                                         | Library-first, the CLI wrapper is thin/secondary; Python packaging story is heavier than a single static binary                                                                                                                                                                                                                | `pathvalidate.sanitize_filename(name, platform=...)`                                                                                                                            |
| **Python `python-slugify` / `sanitize-filename`**             | Python               | MIT                             | Stable, widely used as dependencies rather than standalone CLIs                                                                                       | pip                                                   | Simple, focused, good defaults                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Not CLI tools in practice — used as libraries; sanitize-filename package is minimal (single function)                                                                                                                                                                                                                          | N/A — library import                                                                                                                                                            |

Sources for this table are in **Confidence & Sources** below; GitHub star/commit counts are point-in-time
snapshots (fetched 2026-07-31) and will drift.

**Read `23-online-alternatives-and-ecosystem.md` alongside this table.** Doc 23 is newer, covers six
tools absent here (`brename`, `nomino`, `edir`, `vidir`, PowerRename, Advanced Renamer), and compares
them on the axes that matter most for design (dry-run default, collision handling, undo, non-UTF-8
support, config file, last release). This table is retained for what doc 23 does not carry —
language, license, CLI/UX model, and the per-tool strength/weakness reasoning the design borrows
from — but doc 23 wins on any overlapping fact, including detox's maintenance status. Cells in the
Strengths/Weaknesses columns mix observed features with author assessment; assessments are labelled.

### Rust crates worth reusing (not competitors, building blocks)

| Crate                                                                                                                                                                                     | Purpose                                                                                                | Note for this project                                                                                                                                                                                                                                                                                                                                                                                  |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`sanitize-filename`](https://crates.io/crates/sanitize-filename)                                                                                                                         | Node-`sanitize-filename`-alike: strip reserved chars/Windows device names, truncate to 255 bytes       | **Truncates to a UTF-8 codepoint boundary, not a grapheme-cluster boundary** — confirmed by reading its `src/lib.rs` (the loop walks back from byte 255 until `is_char_boundary`). It will split a base+combining-mark pair or a ZWJ emoji sequence. Reference implementation only, not the truncation logic to adopt (see constraint 7)                                                               |
| [`sanitise-file-name`](https://crates.io/crates/sanitise-file-name)                                                                                                                       | Alt spelling, claims fewer allocations (1 vs 3-4)                                                      | Allocation claim is the crate's own and unbenchmarked here `[UNVERIFIED]`; worth benchmarking against `sanitize-filename`                                                                                                                                                                                                                                                                              |
| [`unicode-normalization`](https://crates.io/crates/unicode-normalization)                                                                                                                 | NFC/NFD/NFKC/NFKD per [UAX #15](https://www.unicode.org/reports/tr15/)                                 | Needed for the macOS NFD-on-write / NFC-elsewhere gap (see constraints)                                                                                                                                                                                                                                                                                                                                |
| [`deunicode`](https://crates.io/crates/deunicode)                                                                                                                                         | ASCII transliteration ("Beyoncé" → "Beyonce")                                                          | For an opt-in "ASCII-only" mode; lossy by design                                                                                                                                                                                                                                                                                                                                                       |
| [`slug`](https://crates.io/crates/slug)                                                                                                                                                   | URL-slug generation                                                                                    | Only relevant if the tool offers a slugify mode distinct from filename-safe mode                                                                                                                                                                                                                                                                                                                       |
| [`unicode-security`](https://crates.io/crates/unicode-security) / [`unicode_skeleton`](https://crates.io/crates/unicode_skeleton) / [`confusables`](https://crates.io/crates/confusables) | [UTS #39](https://www.unicode.org/reports/tr39/) confusable/skeleton detection, mixed-script detection | Core to detecting homoglyph and Trojan-Source-style attacks in filenames. **Staleness flags (snapshot 2026-07-31): `unicode_skeleton` last released 0.1.1 on 2017-10-08 — de facto unmaintained; `confusables` last released 0.1.0 on 2023-08-23, its only release.** Apply the same "verify not abandoned before depending on it" caution this doc applies to detox itself; prefer `unicode-security` |
| [`unicode-segmentation`](https://crates.io/crates/unicode-segmentation)                                                                                                                   | Grapheme-cluster iteration per [UAX #29](https://www.unicode.org/reports/tr29/)                        | Mandatory for any byte-length truncation to avoid splitting a cluster (see constraints)                                                                                                                                                                                                                                                                                                                |
| [`unicode-width`](https://crates.io/crates/unicode-width)                                                                                                                                 | Display-width calculation                                                                              | For any TUI/table rendering of proposed renames (CJK/emoji width)                                                                                                                                                                                                                                                                                                                                      |
| [`encoding_rs`](https://crates.io/crates/encoding_rs) / [`chardetng`](https://crates.io/crates/chardetng)                                                                                 | Encoding detection/conversion                                                                          | Needed to replicate `convmv`'s use case (non-UTF-8 legacy filenames on Unix)                                                                                                                                                                                                                                                                                                                           |
| [`clap`](https://crates.io/crates/clap)                                                                                                                                                   | CLI parsing                                                                                            | De facto standard; derive macros keep boilerplate low                                                                                                                                                                                                                                                                                                                                                  |
| [`figment`](https://crates.io/crates/figment) / [`config`](https://crates.io/crates/config) + [`serde`](https://crates.io/crates/serde)                                                   | Layered config (files, env, CLI overrides)                                                             | To replicate detox's "sequence" config idea without its rigidity. `figment` last released 0.10.19 on 2024-05-17 (snapshot 2026-07-31) — maintained but slow-moving                                                                                                                                                                                                                                     |
| [`ignore`](https://crates.io/crates/ignore) / [`walkdir`](https://crates.io/crates/walkdir) / [`jwalk`](https://crates.io/crates/jwalk)                                                   | Directory traversal, `.gitignore`-aware                                                                | `ignore` directly solves repren's "no .gitignore" weakness. **Staleness flag: `jwalk` last released 0.8.1 on 2022-12-15 (snapshot 2026-07-31) — usable but not actively maintained.** `ignore`'s own `WalkParallel` likely covers the parallel-walk need without a second, staler dependency                                                                                                           |
| [`globset`](https://crates.io/crates/globset)                                                                                                                                             | Glob matching                                                                                          | For `mmv`/`zmv`-style pattern UX                                                                                                                                                                                                                                                                                                                                                                       |
| [`regex`](https://crates.io/crates/regex)                                                                                                                                                 | Regex engine                                                                                           | Rust's `regex` crate is RE2-derived (no backreferences/lookaround) — same ceiling `f2` hits; note this as a known trade-off, not a bug to "fix" with `fancy-regex` unless truly needed                                                                                                                                                                                                                 |
| [`indicatif`](https://crates.io/crates/indicatif)                                                                                                                                         | Progress bars                                                                                          | For large recursive batch jobs                                                                                                                                                                                                                                                                                                                                                                         |
| [`trash`](https://crates.io/crates/trash)                                                                                                                                                 | Send-to-trash instead of hard delete                                                                   | Relevant for "undo a batch rename" safety net, complementary to a rename-log-based undo (à la `f2`/`rnr`)                                                                                                                                                                                                                                                                                              |
| [`rayon`](https://crates.io/crates/rayon)                                                                                                                                                 | Data parallelism                                                                                       | For scanning/hashing large trees before a batch rename                                                                                                                                                                                                                                                                                                                                                 |
| [`rustix`](https://crates.io/crates/rustix)                                                                                                                                               | Safe wrappers for `renameat2` (Linux) **and** `renameatx_np` (macOS)                                   | Added on this pass: `rustix::fs::renameat_with` with `RenameFlags::NOREPLACE`/`EXCHANGE` covers **both** platforms, refuting doc 06's "no crate wraps `renamex_np`" finding — see constraint 10                                                                                                                                                                                                        |

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
2. **[CORRECTED] Case-insensitive filesystems (APFS default, exFAT, NTFS default) — case-only
   renames need no temp-name dance.** Verified locally: creating `CaseTest.txt` and testing
   `[ -f casetest.txt ]` succeeds — same inode, so the filesystem is case-insensitive but
   case-preserving. **The original version of this constraint claimed a case-only rename therefore
   collides with itself and requires a two-step rename through an intermediate temp name. That is
   false and has been removed.** Doc 06 Test 3 refuted it using both `os.rename` and a compiled raw
   `rename(2)` call, on the boot volume and on fresh case-sensitive and case-insensitive `hdiutil`
   APFS images; this pass re-ran it live on this machine (Darwin 25.5, APFS): `os.rename('A.txt',
'a.txt')` succeeds in one call, no `EEXIST`, and the inode is unchanged (`151670402` before and
   after) — the directory entry is simply re-cased. On case-sensitive filesystems (ext4, APFS
   case-sensitive) a case-only rename is an ordinary rename between two distinct names and likewise
   needs nothing special.
   → **Implication: do NOT special-case "differs only by case" as needing a temp intermediate — that
   is a code path with no problem behind it. A case-only rename is a single `rename(2)`/
   `renameat`. Two things do still matter: (a) case-only renames must be recognized when detecting
   collisions among _proposed_ names, because on a case-insensitive filesystem `A.txt` and `a.txt`
   are the same destination; and (b) if a case-only rename ever does fail with `EEXIST` on some
   filesystem or network mount, that is a per-filesystem quirk needing its own citation and its own
   narrow fallback — not a general rule. No such filesystem has been identified here
   `[UNVERIFIED — no counter-example found]`.**
3. **[CORRECTED — CONTESTED] Windows reserved device names** (`CON`, `PRN`, `AUX`, `NUL`,
   `COM1`-`COM9`, `LPT1`-`LPT9`, plus superscript-digit variants `COM¹`/`LPT²` etc. per Microsoft
   Learn). The **bare** name as the leaf component is reserved; that part is not in dispute. What
   happens when an extension is appended (`NUL.txt`) is **genuinely disputed**, and the original
   version of this constraint asserted one side ("reserved _regardless_ of extension … Windows 11
   relaxed this for some contexts") as settled fact. State of the evidence:
   - **Classic Win32 namespace rules** (Microsoft Learn, "Naming Files, Paths, and Namespaces")
     describe reserved names applying with an extension too, in any directory. This is the
     historically documented behavior and what pre-Windows-11 systems, SMB peers, and many
     third-party tools implement.
   - **On Windows 11, path normalization reportedly no longer special-cases a DOS device name that
     carries an extension** — i.e. `con.txt`/`nul.txt` are said to be creatable, while the bare
     `CON`/`NUL` leaf remains reserved (`NUL` most strongly). Source: the `python/cpython#95486`
     discussion, where two CPython core developers who maintain Windows path handling (eryksun,
     ChrisDenton) describe the change. Note this is the **opposite direction** from what the
     original text implied, which described Windows 11 as relaxing something while `NUL.txt`
     stayed invalid.
   - Two secondary sources (Meziantou's blog, a Microsoft Q&A thread) restate the old universal rule
     with no Windows 11 carve-out, contradicting the CPython thread.
   - Not settled here: no Windows 11 machine was available in this environment, so **neither side
     was tested empirically** `[UNVERIFIED — needs a live Windows 11 test]`. Microsoft Learn's
     file-creation guidance itself has not been updated to describe a carve-out.
     → **Implication: check the stem, not the whole filename, and keep the conservative
     (pre-Windows-11, extension-insensitive) reserved-name check as the default — files travel to
     older systems, SMB shares, and other toolchains regardless of what the local Windows 11 build
     permits. But record this as a deliberately conservative _assumption_, not a verified fact, and
     do not hard-code a Windows-11-specific relaxation until it is tested on real hardware.**
4. **Trailing dots and spaces are stripped by Windows path normalization** (per Microsoft
   Learn "Naming Files, Paths, and Namespaces"). Wording refined per doc 06 row 7c: Microsoft's own
   text is softer than "silently stripped" — "Although the underlying file system may support such
   names, the Windows shell and user interface does not" — so this is a shell/UI-layer
   inconsistency, not guaranteed to be a hard filesystem-level strip in every case. The practical
   consequence is unchanged, which means a file this tool creates as `"foo. "`
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
   and not 255 UTF-8 characters. Doc 06 re-confirmed this with a stronger four-way test (ASCII,
   2-byte, 3-byte BMP, and 4-byte astral characters) and reached the same number. It remains
   empirically strong but **not documented by Apple** `[UNVERIFIED against a first-party spec]`.
   → **Implication: truncation logic must be filesystem-aware (255
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

    **[CORRECTED] A Rust crate already wraps both platforms — no hand-written FFI shim is needed.**
    Doc 06 (row 4e / Corrections item 5) concluded that neither `rustix` nor `nix` exposes the macOS
    flags and that a raw `libc` shim was therefore required. **That is wrong for `rustix`**, verified
    this pass by reading vendored [`rustix`](https://crates.io/crates/rustix) 1.1.4 source in
    `~/.cargo/registry`:
    - `src/backend/libc/fs/types.rs:527` — `#[cfg(apple)] bitflags! { pub struct RenameFlags { const
EXCHANGE = RENAME_SWAP; const NOREPLACE = RENAME_EXCL; } }`
    - `src/backend/libc/fs/syscalls.rs:584` — `#[cfg(apple)] fn renameat2(...)` weak-links
      `renameatx_np` and calls it, falling back to plain `rename` (or `ENOSYS` if flags are set) on
      macOS < 10.12.
    - `src/fs/at.rs:292` — the public `renameat_with(old_dirfd, old, new_dirfd, new, flags)` is gated
      `#[cfg(any(apple, linux_kernel, target_os = "redox"))]` and carries
      `#[doc(alias = "renameatx_np")]`.

    The same wiring is present in `rustix` 1.0.1 and 1.0.7, so this is not new in 1.1.4. The likely
    reason doc 06 (and a reviewer of this document) missed it: docs.rs renders `rustix` for a Linux
    target by default, which hides every `#[cfg(apple)]` item — `RenameFlags::NOREPLACE`/`EXCHANGE`
    do not appear in the default docs.rs view. **Verify against source, not docs.rs, for
    platform-gated APIs.** Two caveats remain: `rustix` does not probe the
    `VOL_CAP_INT_RENAME_EXCL` volume capability for you (that still needs `getattrlist`, i.e. `libc`
    or a helper), and the `nix` side of doc 06's claim was not re-checked here
    `[UNVERIFIED for nix]`.

    → **Implication: use `rustix::fs::renameat_with` with `RenameFlags::NOREPLACE` on both Linux
    (`renameat2`) and macOS (`renameatx_np`) — one safe API, no `unsafe` FFI shim to write or audit.
    Keep `libc` only for the `getattrlist` capability probe and `statfs`. Retain the graceful
    fallback (check-then-rename with a documented, narrow TOCTOU window, or a `link()`+`unlink()`
    trick) for filesystems/OSes that reject the flag — including macOS volumes lacking
    `VOL_CAP_INT_RENAME_EXCL` and pre-3.15/unsupported-filesystem Linux. Never call
    plain `rename(2)`/`std::fs::rename` for a batch operation where a name collision is possible.**

    (Superseded: the original implication named `renamex_np` directly and implied a
    platform-conditional wrapper had to be written by hand. Collision-possible batches remain the
    normal case whenever sanitizing produces duplicate output names.)

11. **Cross-device rename, hardlinks, symlinks, rename-during-walk.** Lettered sub-points below
    match how doc 00 cites this constraint (11a/11b/11c).
    - **11a — `EXDEV`:** `rename(2)` fails with `EXDEV` across filesystem/mountpoint boundaries
      (must fall back to copy+unlink, losing atomicity and hardlink identity).
    - **11b — hardlinks and symlinks:** renaming a file with existing hardlinks only affects that
      one directory entry (other links keep the old name — may be desired or surprising depending
      on intent); renaming a symlink vs. its target requires care about which one the user meant
      (`O_NOFOLLOW` semantics).
    - **11c — rename-during-walk:** renaming files while a recursive directory walk (`walkdir`/
      `ignore`) is still iterating that same tree is a classic TOCTOU/"renamed the thing I was
      about to descend into" hazard.
      → **Implication: (a) detect `EXDEV` and either refuse cross-device
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

### Deliberately not constrained here (and where the design sourced them instead)

The twelve numbered constraints are frozen — doc 00 cites them by number, so nothing is renumbered
and nothing is appended. Four OS-constraint categories doc 00 needed are **not** covered above; they
were sourced elsewhere and should be looked up there rather than assumed absent:

- **Signal handling** (SIGINT/SIGTERM mid-batch, SIGKILL leaving a partially applied batch) — doc 00
  §5.8.
- **Errno taxonomy for journal and rename I/O** (`EROFS`, `ENOSPC`, `EACCES`, `EMFILE`, `ENFILE`) —
  doc 00's error-handling sections.
- **Metadata survival across rename** (mode, ownership, xattrs, ACLs, resource forks) — doc 00; note
  a same-filesystem `rename(2)` preserves the inode and therefore all of it, while the `EXDEV`
  copy+unlink fallback in 11a does not, which is the reason that fallback is opt-in.
- **Locale/i18n effects** (`LC_ALL`/`LANG` on case mapping and collation) — docs 11/13 for the C
  tool's behavior, doc 00 for the successor's.

## Confidence & Sources

**High confidence — verified locally on this machine (Darwin 25.5, APFS, `python3`/shell):**

- APFS normalization-preserving-not-sensitive behavior (NFC write, NFD lookup succeeds, directory
  listing preserves the exact bytes given).
- APFS case-insensitive-but-case-preserving behavior (`CaseTest.txt` / `casetest.txt` alias to the
  same inode).
- **A case-only rename succeeds in a single `rename(2)` call on case-insensitive APFS** — re-run on
  this machine 2026-07-31: `touch A.txt` (inode 151670402) → `os.rename('A.txt', 'a.txt')` → inode
  151670402, no error. This refutes constraint 2 as originally written; see doc 06 Test 3 for the
  wider matrix (raw C `rename(2)`, plus case-sensitive and case-insensitive `hdiutil` APFS images).
- **`rustix` 1.1.4 wraps macOS `renameatx_np`** — read from vendored source in
  `~/.cargo/registry/.../rustix-1.1.4/src/{backend/libc/fs/types.rs,backend/libc/fs/syscalls.rs,fs/at.rs}`
  (also present in 1.0.1 and 1.0.7). `#[cfg(apple)] RenameFlags::{NOREPLACE = RENAME_EXCL, EXCHANGE
= RENAME_SWAP}` and a weak-linked `renameatx_np` behind the public `renameat_with`. Refutes doc
  06 row 4e for `rustix`; the equivalent claim about `nix` was not re-checked.
- `detox`'s upstream GitHub repo is archived: `"archived": true`, `pushed_at`
  `2026-07-12T02:21:55Z`, 446 stars, 0 open issues (GitHub REST API, 2026-07-31).
- `f2` latest release **v2.2.2, published 2025-11-10**; `rnr` latest release **v0.5.1, published
  2025-12-13**; `f2` stars 2,427 (GitHub REST API `releases/latest`, 2026-07-31). This closes the
  open question the earlier version of this document flagged about release dates.
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

- [f2 GitHub](https://github.com/ayoisaiah/f2/) / [f2 Wiki](https://github.com/ayoisaiah/f2/wiki/) — feature list, dry-run-by-default, MIT license, Go install path. Version/stars now pinned via the GitHub API (see High confidence above); the undo mechanism is a per-directory JSON backup file overwritten each run, per [f2's own undo docs](https://f2.freshman.tech/guide/undoing-mistakes), not an append-only history log.
- [rnr GitHub](https://github.com/ismaelgv/rnr) — install methods, CLI flags, UTF-8-only limitation. Latest release now pinned via the GitHub API (see above).
- [repren GitHub](https://github.com/jlevy/repren) — Python/stdlib-only claim, star/fork/commit counts, install via `uv tool`.
- [renameutils qmv/imv man pages](https://manpages.debian.org/unstable/renameutils/qmv.1.en.html) — editor-buffer model description. The manpage describes validation/sanity-checking of the edited list, not a diff view; "diffs your edits" was this document's gloss and has been softened.
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

- **Windows 11 reserved-name-with-extension behavior (constraint 3)** — contested between primary-ish
  sources; no Windows 11 machine available here. `[UNVERIFIED]`, and the design's conservative
  default is an assumption, not a finding.
- Whether `mmv`'s exact wildcard/backreference syntax matches the example given — the original
  project's own man page could not be fetched from a primary host; the syntax shown is the widely
  reproduced form. `[UNVERIFIED]`
- Whether `nix` exposes the macOS rename flags — doc 06 says no; only the `rustix` half of that claim
  was re-checked here (and refuted). `[UNVERIFIED for nix]`
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
  search, `rust-lang/libs-team` issue #131 ("Add `std::fs::rename_noreplace`") was open, so the
  standard library does **not** provide this. **Correction to the original wording:** the successor
  does not need to hand-roll the syscall bindings. `rustix::fs::renameat_with` +
  `RenameFlags::NOREPLACE` already covers Linux `renameat2` **and** macOS `renameatx_np` (verified
  from source — see constraint 10); `libc` is needed only for the `getattrlist`
  `VOL_CAP_INT_RENAME_EXCL` probe. The earlier phrasing ("a small platform-conditional
  `libc`/`rustix` wrapper") was vague, and doc 06's stronger version of it — that no crate wraps the
  macOS call at all — is wrong.

## Review record (stage 3)

Three reviewers examined this document (L1 external source fidelity, L2 completeness and
cross-document consistency, L3 clarity/structure/link hygiene). Verdicts below are the adjudicator's,
not the reviewers'. **No constraint was renumbered**; corrected constraints keep their number and are
marked **[CORRECTED]** in place.

| Finding (reviewer)                                                                                                                           | Verdict      | Action or reason                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------------------------------------------- | ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Constraint 2's temp-name two-step for case-only renames is false (L1, L2, L3)                                                                | **ACCEPTED** | Re-tested live here: single `os.rename('A.txt','a.txt')` on APFS succeeded, inode 151670402 unchanged. Constraint 2 rewritten to state direct rename works; the temp-name mandate is deleted; collision-detection nuance kept.                                                                                                                                                                                                                                                                                                                     |
| L3's proposed replacement wording ("a single `rename(2)` call that changes only case is a no-op") (L3)                                       | **REJECTED** | It preserves the false premise. The call is not a no-op — it re-cases the entry and returns success. L3 correctly spotted the ambiguity but its fix would have hardened the error.                                                                                                                                                                                                                                                                                                                                                                 |
| Constraint 3's Windows-11 direction of travel is contested (L2, and L1's "under-specified" flag)                                             | **ACCEPTED** | Rewritten as an evidence summary: bare leaf name reserved (undisputed); extension case disputed, with `python/cpython#95486` on one side and MS Learn plus two secondary sources on the other; marked `[UNVERIFIED]` pending a real Win11 test.                                                                                                                                                                                                                                                                                                    |
| Neither `rustix` nor `nix` exposes macOS `renamex_np`; doc 03 must say a hand-written FFI shim is needed (L1 MAJOR, L2 MAJOR, doc 06 row 4e) | **REJECTED** | **Wrong for `rustix`.** Verified against vendored `rustix` 1.1.4 source: `#[cfg(apple)] RenameFlags::{NOREPLACE=RENAME_EXCL, EXCHANGE=RENAME_SWAP}` (types.rs:527) and `#[cfg(apple)] renameat2` weak-linking `renameatx_np` (syscalls.rs:584), public via `renameat_with` (at.rs:292, `#[cfg(any(apple, linux_kernel, …))]`). Same code in 1.0.1/1.0.7. Both reviewers relied on docs.rs, which hides Apple-gated items on its default Linux target. Constraint 10 and the closing open-question now state the opposite: no `unsafe` shim needed. |
| Crate table: `unicode_skeleton` (2017) and `jwalk` (2022) recommended with no staleness flag (L2 MAJOR)                                      | **ACCEPTED** | Both rows now carry dated staleness flags and a "verify not abandoned" caution; `ignore`'s `WalkParallel` named as the likely replacement for `jwalk`.                                                                                                                                                                                                                                                                                                                                                                                             |
| `sanitize-filename` hedge should become a settled fact (L2 MINOR, L1 confirmed from source)                                                  | **ACCEPTED** | Row now states flatly that it truncates on a codepoint boundary, not a grapheme boundary, and will split combining/ZWJ sequences.                                                                                                                                                                                                                                                                                                                                                                                                                  |
| Doc 03 missed detox's GitHub archival, the primary lifecycle fact (L2 MAJOR)                                                                 | **ACCEPTED** | Verified independently (`api.github.com/repos/dharple/detox`: `archived: true`, `pushed_at 2026-07-12`, 446 stars). Added to the positioning-anchor section and the tool table; Homebrew flag demoted to a downstream consequence.                                                                                                                                                                                                                                                                                                                 |
| Missing constraints: signals, errno taxonomy, xattr/ACL/ownership survival, locale (L2 MAJOR)                                                | **MODIFIED** | Adding constraints 13–16 would put load-bearing material in the one document that must stay citation-stable, duplicating doc 00. Added a "Deliberately not constrained here" subsection pointing at doc 00 §5.8 and docs 11/13, plus the one filesystem fact that belongs here (same-FS rename preserves the inode and hence all metadata; the `EXDEV` fallback does not).                                                                                                                                                                         |
| Doc 00 cites "constraint 11a" but doc 03 has no lettered split (L2 MINOR)                                                                    | **ACCEPTED** | Constraint 11 is now split into labelled 11a (`EXDEV`), 11b (hardlinks/symlinks), 11c (rename-during-walk). Content unchanged, number unchanged, doc 00's existing citation now resolves.                                                                                                                                                                                                                                                                                                                                                          |
| Constraint 9 is never cited by number in doc 00 (L2 MINOR)                                                                                   | **REJECTED** | Nothing to fix in this document — constraint 9 is correct and correctly numbered. It is a doc 00 traceability suggestion and belongs to the doc 00 pass.                                                                                                                                                                                                                                                                                                                                                                                           |
| detox row's unexplained "2027-04-28→2027-07-28" date range (L2 MINOR)                                                                        | **ACCEPTED** | `brew info detox` on this machine states one date: disable on 2027-07-28. Row corrected; the phantom April date is gone.                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Constraint 4 should carry doc 06 row 7c's mechanism nuance (L2 MINOR)                                                                        | **ACCEPTED** | Now notes MS Learn's softer wording (shell/UI layer, "may support"), practical implication unchanged.                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| f2's undo is a per-directory JSON file overwritten each run, not a history log (L1 MINOR)                                                    | **ACCEPTED** | Table cell and source note corrected; the "only the most recent batch per directory is undoable" consequence is spelled out, since it is a real design lesson.                                                                                                                                                                                                                                                                                                                                                                                     |
| qmv "diffs your edits against the original list" overstates the manpage (L1 MINOR)                                                           | **ACCEPTED** | Softened to "validates … (not a visual diff — the manpage does not describe one)".                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Subjective judgments ("arguably best", "strongest modern competitor") formatted as facts (L3)                                                | **ACCEPTED** | Both relabelled as explicit author assessments; a note under the table says the Strengths/Weaknesses columns mix observation and assessment.                                                                                                                                                                                                                                                                                                                                                                                                       |
| 19 missing inline crates.io links + missing Unicode standard links (L3)                                                                      | **ACCEPTED** | Every crate in the table is now linked to crates.io; UAX #15, UAX #29 and UTS #39 are linked to unicode.org.                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| L3's suggested UAX #15 URL (`tr44/#Canonical_Decomposition`) (L3)                                                                            | **MODIFIED** | Wrong report — UAX #15 is `unicode.org/reports/tr15/`. Linked that instead; `tr29` added for the grapheme-segmentation claim.                                                                                                                                                                                                                                                                                                                                                                                                                      |
| Overlap with doc 23; collapse the tool table to a paragraph (L2 recommendation)                                                              | **MODIFIED** | Kept the table. L1 re-verified its contents this cycle, and it carries language, license, CLI/UX model, and per-tool design lessons that doc 23's matrix does not. Added a precedence pointer: doc 23 is newer, broader (six extra tools), and wins on overlapping facts including detox's status.                                                                                                                                                                                                                                                 |
| Stale version/activity claims; unverifiable claims should be marked (L1, task brief)                                                         | **ACCEPTED** | Pinned from the GitHub API: `f2` v2.2.2 (2025-11-10), 2,427 stars; `rnr` v0.5.1 (2025-12-13). The "release dates unverified" open question is now closed. `[UNVERIFIED]` added to the `sanitise-file-name` allocation claim, the `mmv` syntax example, APFS's limit as an Apple-documented contract, Windows 11 behavior, and the `nix` half of the rename-flag claim.                                                                                                                                                                             |
| All 22 links resolve; constraint numbering structurally clean (L3)                                                                           | **ACCEPTED** | No action needed; no link was removed. New links added this pass were not individually HTTP-checked (crates.io and unicode.org canonical paths).                                                                                                                                                                                                                                                                                                                                                                                                   |

**Verified independently for this adjudication (not taken on a reviewer's word):**

1. Case-only rename on APFS — `touch A.txt`, `ls -i`, `os.rename` to `a.txt`, `ls -i`: same inode
   (151670402), no error. Confirms L1's live test and doc 06 Test 3; refutes constraint 2 as written.
2. `rustix` Apple rename support — read the vendored 1.1.4 source in `~/.cargo/registry` (types.rs,
   syscalls.rs, at.rs, cited by line above) and confirmed the same wiring in 1.0.1 and 1.0.7. This
   reverses both L1's and L2's finding and doc 06 row 4e.
3. detox upstream status — `curl https://api.github.com/repos/dharple/detox`: archived, `pushed_at`
   2026-07-12, 446 stars, 0 open issues.
4. Homebrew deprecation date — `brew info detox`: one disable date, 2027-07-28.
5. `f2` and `rnr` latest releases and `f2` stars — GitHub REST `releases/latest`.
6. Doc 06's own "Corrections Required" list and doc 23's coverage and detox-archival date, read
   directly, to adjudicate precedence and the overlap question.
