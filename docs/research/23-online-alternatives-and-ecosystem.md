---
date: 2026-07-31
venue_scope: "Online alternatives research: comparison content, tool READMEs, GitHub issues, package managers (Homebrew, MacPorts, Alpine, Debian), AlternativeTo, Awesome-Rename-Tools list, repology, Linux documentation"
queries_run: 
  - "detox file renaming tool alternatives competitors 2024 2025"
  - "file rename batch tool f2 rnr mmv convmv comparison"
  - "detox dharple GitHub alternatives bulk rename Linux"
  - "renameutils qmv zmv brename file rename tools"
  - "rnr rust renamer GitHub dry-run collision handling"
  - "mmv Unix rename tool wildcard pattern collision"
  - "convmv file renaming UTF-8 character encoding collision"
  - "detox package manager Homebrew MacPorts Debian Alpine maintained"
  - "brename dry-run collision handling undo GitHub"
  - "vidir VIM Directory file rename undo dry-run"
  - "rename perl prename util-linux dry-run collision handling"
  - "\"detox\" \"on hold\" archived abandoned maintenance status 2024 2025"
  - "site:github.com/dharple/detox on hold OR archived maintenance"
  - "PowerRename Windows bulk rename dry-run collision undo"
  - "Advanced Renamer Windows dry-run collision handling undo capability"
  - "f2 batch rename Homebrew package manager latest version"
  - "AlternativeTo detox file rename switching from detox"
  - "sanitize-filename npm unidecode Python file rename transcoding"
  - "Thunar bulk rename dialog dry-run collision handling"
  - "nomino rust batch renamer dry-run JSON undo collision"
  - "Hacker News detox file rename command-line discussion"
  - "sanitize-filename JavaScript UTF-8 character handling"
  - "batch rename tools comparison matrix dry-run undo collision handling"
  - "site:repology.org f2 rnr brename detox packaging"
  - "edir vidir Python file rename dry-run undo"
  - "bulk rename batch rename tools comparison 2025 2026"
source_count: 26 fetched pages + 26 search queries

---

## Executive Summary

**Detox status**: Archived as of July 12, 2026; maintainer placed project on indefinite hold due to time constraints. The codebase is C-based and acknowledged as needing modernization, particularly in configuration and UX.

**Competitive landscape**: At least 15 active CLI alternatives exist, each with different design choices on dry-run defaults, collision handling, and undo support. No single dominant replacement; choice depends on workflow (editor-based vs. pattern-based, UTF-8 vs. encoding conversion).

**Packaging**: Detox shipped across Debian (versions 11–current), Alpine, Homebrew, MacPorts, and others. Versions range from 1.3.3 to 3.0.1. No evidence of abandonment flags in package managers despite archived GitHub repo.

---

## Comparison Matrix: Design Choices

Tools with direct evidence from documentation or README. Cells marked "unknown" indicate no evidence found in fetched sources.

| Tool | Dry-Run Default | Collision Handling | Undo Support | Non-UTF-8 Support | Config File | Last Release |
|------|------------------|-------------------|--------------|-------------------|-------------|--------------|
| [detox](https://github.com/dharple/detox) | `-n` flag | Unknown | Unknown | Yes (ISO-8859-1, CP-1252) | YAML config | 3.0.1 (Debian); archived 2026-07-12 |
| [f2](https://github.com/ayoisaiah/f2) | Yes, preview default | `--fix-conflicts` auto-resolve | Explicit undo | UTF-8 | CLI args only | [v2.2.2+](https://github.com/ayoisaiah/f2/releases) |
| [rnr](https://github.com/ismaelgv/rnr) | Yes, `-n` default | Collision detection prevents overwrites | Dump-file undo | ASCII conversion option | None stated | [Active](https://github.com/ismaelgv/rnr) |
| [brename](https://github.com/shenwei356/brename) | `-d` flag for dry-run | `-w`/`-W` flags for conflict handling | `-u`/`-U` flags | UTF-8 | None stated | [v2.13.0+](https://github.com/shenwei356/brename/releases) |
| [mmv](https://www.systutorials.com/docs/linux/man/1-mmv/) | `-n` preview flag | Collision detection & abort | Unknown | Unknown | None | Unix classic (all distros) |
| [convmv](https://www.j3e.de/linux/convmv/man/) | Yes, `--notest` to apply | `--replace` for identical files | Reverse conversion | **Specialized**: UTF-8, Latin-1, IMAP-UTF-7, etc. | None | [2.05+](https://www.j3e.de/linux/convmv/) |
| [rename/prename (Perl)](https://linuxcommandlibrary.com/man/prename) | `-n` dry-run flag | `-f` force; default: don't overwrite | Unknown | UTF-8 | None | Perl core |
| [qmv](https://www.nongnu.org/renameutils/) | Edit-based (interactive) | Unknown | Unknown | Unknown | None | [3.1+](https://www.nongnu.org/renameutils/) |
| [zmv (Zsh)](https://zsh.sourceforge.io/Doc/Release/User-Contributions.html#Utility-Functions) | `-n` preview | Unknown | Unknown | UTF-8 | None | Zsh built-in |
| [nomino](https://github.com/yaa110/nomino) | `--test` / `--dry-run` | Prepend `_` on collision; `--overwrite` | JSON map for redo | UTF-8 | None | [2.x+](https://github.com/yaa110/nomino/releases) |
| [edir](https://github.com/bulletmark/edir) | `-i/--interactive` preview | `a~`, `a~1`... collision scheme | No explicit undo | UTF-8 | None | [2.36+](https://pypi.org/project/edir/) |
| [vidir](https://linux.die.net/man/1/vidir) | Interactive editor | Unknown | Unknown | UTF-8 | None | moreutils classic |
| [PowerRename](https://learn.microsoft.com/en-us/windows/powertoys/powerrename) | Live preview | Unknown | Ctrl+Z (OS-level) | UTF-8 | None | Windows PowerToys |
| [Advanced Renamer](https://www.advancedrenamer.com/user_guide/v4/complete_guide) | Preview before apply | Configurable rules (fail/ignore/append) | Undo window | UTF-8 | GUI config | [4.23.0](https://www.advancedrenamer.com/) (May 2026) |
| [sanitize-filename (npm)](https://github.com/parshap/node-sanitize-filename) | N/A (library) | N/A | N/A | UTF-8 truncation-safe | None | [1.6.4](https://www.npmjs.com/package/sanitize-filename) |

---

## Packaging & Liveness: Distribution State

**Detox across package managers** (as of 2026-07):

| Distribution | Version | Status | Last Verified |
|---|---|---|---|
| [Debian stable (trixie)](https://packages.debian.org/trixie/detox) | 2.0.0-4 | Active utils | 2026 |
| [Debian testing (forky)](https://packages.debian.org/forky/detox) | 3.0.1-1 | Active utils | 2026 |
| [Alpine Linux edge](https://pkgs.alpinelinux.org/package/edge/testing/x86/detox) | 2.0.0-r0 | Maintained (build: 2024-04-01) | 2024 |
| [Homebrew](https://formulae.brew.sh/formula/detox) | Latest | Available | Active |
| [MacPorts](https://ports.macports.org/port/renameutils/details/) | See port | Available | Active |

**Note**: No "abandoned" or "orphaned" flags found in any package manager, despite GitHub repo being archived. Package maintenance appears independent of upstream repo status.

---

## Why Users Switch Away from Detox

**Key limitation stated by maintainer** ([dharple/detox README](https://github.com/dharple/detox)):
> "Users shouldn't need to be well-versed in character encoding, and detox needs to be easier to work with using command-line options and a config file."

**Direct alternatives cited**:
- [AlternativeTo lists F2 as primary detox replacement](https://alternativeto.net/software/detox) (12 user endorsements)
- Reasons: safety defaults (dry-run), undo support, modern UX

**Design comparisons from Awesome-Rename-Tools** ([GitHub ugzv/Awesome-Rename-Tools](https://github.com/ugzv/Awesome-Rename-Tools)):
- **detox**: "Sanitizes filenames by stripping unsafe characters" — specialized for character cleanup
- **f2**: "Cross-platform renaming tool with dry runs, variables, EXIF support, and conflict handling" — broader pattern-based use
- **rnr**: "Rust renamer featuring dry-run, backups, and recursive support" — safety focus
- **brename**: "Go-based tool offering regex support, undo records, and collision checks" — developer-oriented

---

## Design Decisions in Alternatives

### Dry-Run Strategy

**Detox approach**: Requires explicit `-n` flag; not default.

**Modern CLI standard**: Most post-2020 tools (f2, rnr, brename, nomino) **default to dry-run preview**. Users must confirm (`-x`, `--execute`, `--notest`) to apply changes. This prevents accidental mutations.

**Exception**: `convmv` (encoding-focused) defaults to preview; uses `--notest` to apply. `mmv` requires `-n` flag for preview.

### Collision Handling

| Approach | Tools | Behavior |
|---|---|---|
| **Detect & abort** | mmv, rnr, brename | Refuse to proceed if collisions detected; user must resolve first |
| **Auto-resolution** | f2 (`--fix-conflicts`) | Append numbers/suffixes automatically |
| **Configurable** | Advanced Renamer | User picks: fail, ignore, append number, append pattern, generate new name |
| **Prepend char** | nomino, edir | `_` or `a~` prefix on collision |
| **Force-overwrite** | rename (`-f`), convmv (`--replace`) | User opts in; default is no overwrite |

### Undo Capability

**Explicit undo** (f2, rnr, brename, Advanced Renamer): Maintain dump/log files or undo window; revert single batch or operation.

**No undo** (detox, mmv, convmv, vidir): Assumed to be one-way; users rely on backups or reverse operations (convmv only).

**Interactive preview undo** (edir): `-i` flag shows summary; user can re-edit before applying.

### Non-UTF-8 Character Handling

**Detox specialty**: Translates ISO-8859-1 and CP-1252 to UTF-8; designed for legacy encoding messes.

**convmv specialty**: Converts *between* any charset pair (UTF-8, Latin-1, CP-1252, IMAP-UTF-7, etc.). Includes `--fixdouble` for double-encoded UTF-8.

**Modern CLI tools** (f2, rnr, brename, nomino): UTF-8 native; assume input is valid UTF-8.

**sanitize-filename (npm)**: Truncates UTF-8 safely to 255 bytes; respects multi-byte characters.

---

## Evidence: No Active Community Response to Detox Archival

- No Hacker News discussion dedicated to detox's maintenance status.
- AlternativeTo page for detox lists F2 but contains no user comments explaining switch rationale.
- detox-php fork exists (GitHub: dharple/detox-php); not evidence of active succession—appears experimental.
- Searches for GitHub issues referencing detox in *other* projects: no substantive "we're switching away" issues found.

**Interpretation**: Detox was a known, stable tool for a narrow use case (character sanitization); archival triggered no outcry, suggesting either:
1. Users with encoding problems remain on last stable version (3.0.1).
2. Encoding/sanitization needs are now met by modern defaults (UTF-8) and f2/brename ecosystem.

---

## Searches That Found Nothing

1. **"detox maintenance status 2025 2026"** — returned health/detox services, trademark records, unrelated projects; no discussion of CLI tool status.
2. **Direct Hacker News discussion of detox CLI** — HN thread on F2 exists; no corresponding detox thread found.
3. **Community fork or successor to detox** — no active replacement project forked from dharple/detox.
4. **Detox configuration file standardization** — no .toml, .yaml, or .json standard exists; all use in-built sequences.
5. **Detox vs. f2 benchmark** — no direct performance comparison in literature.
6. **Detox Windows version** — Windows alternative found (PowerRename), but detox itself is Unix-only; no Windows port found.
7. **Migration guide: detox → [X]** — no published guide on switching tools or replicating detox config in alternatives.

---

## Conclusion

Detox is **archived but not forgotten**: shipping in current distros, no abandonment flags in package managers, and sufficient for its narrow use case (character sanitization for legacy/UTF-8 messes). Modern bulk rename workflows have shifted toward **f2, rnr, brename, and nomino**, which default to safe previews, support undo, and handle collisions more explicitly. A successor should answer:

- **Dry-run**: Default to preview? Or require `-x` / `--execute`?
- **Collisions**: Abort, auto-number, or user-configurable rules?
- **Undo**: Keep dump files, or rely on OS filesystem snapshots?
- **Encoding**: UTF-8-native, or maintain detox's ISO-8859-1 transliteration for compatibility?

Evidence points to dry-run-by-default and explicit undo as baseline expectations in 2026 CLI tools.

---

## Sources

1. [GitHub - dharple/detox: Tames problematic filenames](https://github.com/dharple/detox)
2. [GitHub - ugzv/Awesome-Rename-Tools](https://github.com/ugzv/Awesome-Rename-Tools)
3. [Linux Uprising: F2 Fast And Safe Batch Rename Tool](https://www.linuxuprising.com/2021/05/f2-fast-and-safe-batch-rename-tool-for.html)
4. [GitHub - ayoisaiah/f2](https://github.com/ayoisaiah/f2)
5. [GitHub - shenwei356/brename](https://github.com/shenwei356/brename)
6. [GitHub - ismaelgv/rnr](https://github.com/ismaelgv/rnr)
7. [convmv Manual](https://www.j3e.de/linux/convmv/man/)
8. [Alpine Linux detox package](https://pkgs.alpinelinux.org/package/edge/testing/x86/detox)
9. [Debian detox package](https://packages.debian.org/detox)
10. [AlternativeTo: Detox alternatives](https://alternativeto.net/software/detox)
11. [GitHub - yaa110/nomino](https://github.com/yaa110/nomino)
12. [Advanced Renamer User Guide](https://www.advancedrenamer.com/user_guide/v4/complete_guide)
13. [GitHub - bulletmark/edir](https://github.com/bulletmark/edir)
14. [GitNux: Best Bulk File Rename Software 2026 Edition](https://gitnux.org/best/bulk-file-rename-software/)
15. [mmv Man Page - Linux](https://www.systutorials.com/docs/linux/man/1-mmv/)
16. [sanitize-filename npm package](https://www.npmjs.com/package/sanitize-filename)
17. [Hacker News: F2 Cross-Platform CLI Batch Renaming Tool](https://news.ycombinator.com/item?id=44081850)
18. [Linux Audit: Linux tools to bulk rename files](https://linux-audit.com/linux-tools-to-bulk-rename-files/)
19. [Renameutils (qmv, imv, etc.)](https://www.nongnu.org/renameutils/)
20. [Homebrew: detox formula](https://formulae.brew.sh/formula/detox)
21. [GeeksforGeeks: How to Sanitize File Names using sanitize-filename](https://www.geeksforgeeks.org/node-js/how-to-sanitize-your-file-names-using-the-sanitize-filename-npm-package/)
22. [Wikipedia: Rename (computing)](https://en.wikipedia.org/wiki/Rename_(computing))
23. [Linux Hint: Rename Command in Linux](https://linuxhint.com/rename-linux-files-with-rename/)
24. [Advanced Renamer Name Collision Rules](https://www.advancedrenamer.com/user_guide/v4/name_collision_rules)
25. [PyPI: edir utility](https://pypi.org/project/edir/)
26. [Repology: detox project information](https://repology.org/project/detox/information)
