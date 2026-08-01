---
date: 2026-07-31
venue_scope: >
  Blogs, tutorials, how-to articles, personal wikis, distro documentation
  (Arch Wiki, Gentoo Wiki, Debian/Ubuntu package pages, FreeBSD ports),
  Linux magazine/news sites, package trackers, and video descriptions
  covering the `detox` filename-cleaning utility (github.com/dharple/detox).
  Excludes: diet/health "detox", unrelated npm/PyPI packages named detox
  (Wix's detox mobile E2E framework, detoxpy).
queries_run:
  - detox command line filename cleaning tutorial linux
  - "detox" filenames dharple tutorial howto
  - detox -r -n rename utility examples site:itsfoss.com OR site:tecmint.com OR site:nixcraft.com
  - Arch Wiki detox utility
  - detox vs rename mmv convmv filename cleaner comparison
  - Gentoo wiki detox filenames
  - nixCraft detox command example
  - ostechnix detox rename files
  - FreeBSD ports detox sysutils
  - detox debian package sysutils orphaned
  - github dotfiles "detox -r" script filename cleanup
  - "detox" filenames reddit linux recommend tool
  - "detox" "rnr" OR "f2" OR "qmv" batch rename comparison blog
  - detox filename cleaner "Thunar" OR "PowerRename" comparison
  - detox linux howtoforge OR linuxhandbook OR linux.com filename tutorial
  - asciinema detox filename demo recording
  - site:tecmint.com detox
  - site:howtoforge.com detox filenames
  - site:linuxhandbook.com detox
source_count: 18 pages fetched and read directly (plus GitHub API check); ~15 additional search-result-only mentions not independently fetched and therefore not quoted as read
---

# `detox` in online tutorials and docs

## Headline finding: the upstream repo is archived (July 2026)

Verified directly via the GitHub API (not a tutorial claim):

```
$ curl -s https://api.github.com/repos/dharple/detox | grep -E '"archived"|"pushed_at"|"updated_at"'
"updated_at": "2026-07-26T08:44:53Z",
"pushed_at": "2026-07-12T02:21:55Z",
"archived": true,
```

[github.com/dharple/detox](https://github.com/dharple/detox) is archived as of this research date. The README (fetched) states the maintainer paused active development, citing time constraints and a desire for "a complete rebuild with improved UI and command-line usability." Current stable is v3.0.1. No tutorial site has caught up to this yet — every article below still treats detox as a live, actively-installable tool.

## Best practice consensus across tutorials

Every substantive tutorial fetched leads with the same one best practice: **run `-n`/`--dry-run` before committing.**

- putorius.net: "Always use `-n` first to preview changes before committing" — [Linux Detox: Clean Up Problematic Filenames](https://www.putorius.net/linux-detox-clean-up-filenames-with-space-and-special-characters.html), published Nov 7, 2024, by Steven Vona.
- Delightly Linux: "Always run the `-n` flag first to preview results before executing permanent changes" — [Clean Up Filenames with detox](https://delightlylinux.wordpress.com/2023/12/07/clean-up-filenames-with-detox/), Dec 7, 2023.
- apt-upgrade.me: pairs dry-run with a full backup pipeline: "Run before backups: Clean files first, then backup" and shows `detox -r -s utf_8 -s iso8859_1 -v "$SOURCE"` immediately followed by `rdiff-backup "$SOURCE" "$TARGET"` — [🧼 Cleaning Up Filenames on Linux with detox](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/), Jul 24, 2025.
- Mabox Linux forum: warns that omitting `-r` leaves subdirectory contents untouched, only the top-level name changes — [CLI: detox - Clean up filenames](https://forum.maboxlinux.org/t/cli-detox-clean-up-filenames/721), Nov 19, 2021.
- Gentoo Wiki: "Use the `--dry-run` flag to preview changes before executing actual filename modifications" — [Detox - Gentoo wiki](https://wiki.gentoo.org/wiki/Detox).
- dotlinux.net: additionally recommends backing up critical files before running and documenting `.detoxrc` rules for team environments — [Clean Up Filenames with Detox Command-Line Utility](https://www.dotlinux.net/blog/clean-up-filenames-with-detox-command-line-utility/), last updated Jan 14, 2026.

No tutorial recommends pairing detox with version control or `git mv`; the backup-first pattern (apt-upgrade.me) is the only "sequence with another tool" recipe found.

## Killer feature authors lead with

The tutorials found agree on one framing: **it removes spaces and problematic characters from filenames automatically, in bulk**, presented as the fix for files that came from Windows/macOS or bulk downloads. Note the actual count behind this — **2 directly-fetched sources** (apt-upgrade.me, Delightly Linux) plus one search snippet that could not be verified (linuxconfig.org, below). It is a consistent framing, not a broad measured consensus.

- "This is a great tool to use if you import files from other operating systems or download lots of files online" — search-result synthesis of [linuxconfig.org's article](https://linuxconfig.org/clean-up-filenames-with-detox-command-line-utility) (page itself returned HTTP 403 on direct fetch; this line is from the search snippet only, not independently verified against the live page — flagged per honesty rules).
- apt-upgrade.me leads with a concrete failure story: an automated `rdiff-backup` job silently failed because a macOS-originated file had Unicode quirks Linux backup tooling couldn't handle; detox is positioned as the fix run _before_ backup jobs.
- Delightly Linux frames the use case as hundreds of poorly-named files across many directories, and jokes "this is not a health diet" to disambiguate from the wellness sense.

## Use cases tutorials target

- Cross-OS file imports (Windows CP-1252/macOS-origin files landing on Linux) — apt-upgrade.me, artoflogic.com.
- Bulk media libraries (MP3s, movies) — Debian package description (fetched): "useful to mass rename files automatically... to easily standardize lots of files, such as MP3s or movies."
- Pre-backup filename sanitation so downstream tools (`rdiff-backup`, cloud sync) don't choke — apt-upgrade.me.
- General photo/download folder cleanup as part of a broader "clean up file names" workflow that also uses `chmod`, `convmv`, Perl `rename`, `mogrify`, and `jhead` — [Clean Up File Names - the Art of Logic](https://www.artoflogic.com/2020/03/clean-up-file-names/), Mar 24, 2020.

## Warnings and caveats

- Detox is destructive to the _name_, not content: "detox renames files, so you will lose the original name" — Delightly Linux.
- "It does not correct spelling or rename words. It only removes problematic characters" — Delightly Linux, quoting the tool's own scope limit.
- Duplicate filenames can result after two different original names collapse to the same sanitized name — dotlinux.net.
- Recursive scope is easy to misjudge: without `-r`, only the top-level directory name changes, not its contents — Mabox Linux forum.
- dotlinux.net (updated Jan 14, 2026 — i.e., current) still shows accent-stripping examples like `detox -s iso8859_1 "Café.jpg"` and lists "Accents may not strip without proper `.detoxrc` configuration" as a caveat, and displays version 1.4.5 output. **This is stale/misleading for current v3**: per the Arch man page and the project's own README (both fetched), v3.0.1 deliberately stopped transliterating Unicode to ASCII by default — "detox will no longer try to transliterate all of Unicode into the ASCII character space," moving legacy tables to `table/legacy/`. A reader following dotlinux.net's accent-stripping example on a v3 install may get different results than documented, with no version disclaimer on the page.
- The Gentoo Wiki and Arch man page both document `--special` (needed for symlinks/special files) and `-f configfile`, but no tutorial-level article mentions either — these exist only in reference docs, never in a walkthrough.

## Positioning against alternatives

Direct comparisons are thin. Findings:

- [Art of Logic's "Clean Up File Names"](https://www.artoflogic.com/2020/03/clean-up-file-names/) (Mar 24, 2020) is the only fetched tutorial that pairs detox in a real sequence with other tools: `chmod -Rvc 644 *` → `detox -r -v *` → `convmv -r -f windows-1252 -t UTF-8 .` → Perl `rename` → `mogrify`/`jhead` for images. Detox's role in that pipeline is specifically "remove funny characters," with `convmv` doing encoding conversion and `rename` doing pattern substitution — the three are treated as complementary, not competing.
- [Awesome-Rename-Tools](https://github.com/ugzv/Awesome-Rename-Tools) (curated GitHub list, fetched) lists detox as "Filename sanitizing utility for stripping unsafe characters and transliterating messy names," grouped in "Command Line Tools" alongside `rename` (Perl and util-linux), `mmv`, `F2`, `rnr`, `renameutils` (qmv), `nomino`, `massren`, and PowerShell `Rename-Item`. No prose comparison, just a list entry.
- A search snippet (not independently fetched/verified) attributes to an AlternativeTo user comment that F2 is "a good alternative to detox on windows" and "like a cross-platform detox successor on steroids" — flagged as unverified since the source page wasn't fetched directly.
- [Linux Magazine's "Bulk Renamers"](https://www.linux-magazine.com/Online/Blogs/Off-the-Beat-Bruce-Byfield-s-Blog/Bulk-Renamers) (Mar 18, 2011, fetched) reviews Thunar Bulk Rename, KRename, GPRename, and pyRenamer (GUI tools) — **detox is not mentioned at all**. This is a genuine gap: the most detailed GUI-vs-CLI renamer comparison found doesn't consider detox as a contender.
- No source compared detox to `zmv` or Thunar's bulk renamer head-to-head with actual detox commands running side by side.

## Distro packaging state

| Distro/system            | Version                                              | Notes                                                                                                                                                                              | Source                                                                                                                                 |
| ------------------------ | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| Arch Linux (extra)       | 3.0.1-1                                              | Last updated 2025-08-23                                                                                                                                                            | [archlinux.org](https://archlinux.org/packages/extra/x86_64/detox/)                                                                    |
| Debian (sid/utils)       | 3.0.1-1                                              | Maintainer Joao Eriberto Mota Filho; provides `detox` + `inline-detox`; also in trixie (stable) and forky (testing); **not** on Debian's orphaned-packages list (checked directly) | [packages.debian.org](https://packages.debian.org/sid/utils/detox), [Debian orphaned list](https://www.debian.org/devel/wnpp/orphaned) |
| FreeBSD ports (sysutils) | 3.0.1                                                | Maintainer Kirill Ponomarev; last port update 2026-02-23                                                                                                                           | [FreshPorts](https://www.freshports.org/sysutils/detox/)                                                                               |
| Gentoo (app-misc)        | — (page fetched; no explicit version pinned in text) | Standard `emerge --ask app-misc/detox`                                                                                                                                             | [Gentoo Wiki](https://wiki.gentoo.org/wiki/Detox)                                                                                      |
| Fedora/RHEL              | —                                                    | `sudo dnf install detox -y`                                                                                                                                                        | [putorius.net](https://www.putorius.net/linux-detox-clean-up-filenames-with-space-and-special-characters.html)                         |
| Ubuntu/Debian (apt)      | — (per-article)                                      | Requires `universe` repo per putorius; plain `sudo apt install detox` per Delightly Linux/apt-upgrade.me                                                                           | putorius.net, Delightly Linux                                                                                                          |
| Linux Mint               | —                                                    | Listed in Software Manager; no version/review shown on the software page                                                                                                           | [community.linuxmint.com](https://community.linuxmint.com/software/view/detox)                                                         |

No distro documentation flags detox as orphaned or unmaintained at the packaging level — orphan status is upstream-only (see headline finding above), and it hasn't propagated to any distro page yet.

## Flags seen in the wild

| Flag                           | Frequency across fetched sources                            | Example                                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------ | ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `-n` / `--dry-run`             | Very high — every hands-on tutorial fetched                 | `detox -n *` — [Delightly Linux](https://delightlylinux.wordpress.com/2023/12/07/clean-up-filenames-with-detox/)                                                                                                                                                                                                                                                                                                               |
| `-r` (recursive)               | High — 6 of 9 hands-on sources                              | `detox -r /your/directory` — [apt-upgrade.me](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/)                                                                                                                                                                                                                                               |
| `-v` (verbose)                 | High — 4 of 9 hands-on sources                              | `detox -r -s utf_8 -s iso8859_1 -v /data` — apt-upgrade.me                                                                                                                                                                                                                                                                                                                                                                     |
| `-s sequence`                  | Medium — used when accent/Unicode transliteration is wanted | `detox -s iso8859_1 "Café.jpg"` — [dotlinux.net](https://www.dotlinux.net/blog/clean-up-filenames-with-detox-command-line-utility/). **Version warning: this is a v1.4.5-era example. v3.0.1 deliberately stopped transliterating Unicode to ASCII by default and moved the legacy tables to `table/legacy/`, so this command does not produce the documented v1 result on a current install.** See the caveats section above. |
| `-L` (list sequences)          | Medium — reference-style mentions                           | `detox -L` — Mabox forum, dotlinux.net                                                                                                                                                                                                                                                                                                                                                                                         |
| `-V` / `--version`             | Low                                                         | `detox --version` — dotlinux.net; `detox -V` — Mabox forum                                                                                                                                                                                                                                                                                                                                                                     |
| `-R` (custom replacement char) | Low — one mention only                                      | described, not demonstrated live, per search-snippet summary of a rename-tools roundup                                                                                                                                                                                                                                                                                                                                         |
| `-f configfile`                | Reference-docs only, never in a tutorial walkthrough        | Gentoo Wiki, Arch man page                                                                                                                                                                                                                                                                                                                                                                                                     |
| `--special`                    | Reference-docs only, never in a tutorial walkthrough        | Gentoo Wiki, Arch man page                                                                                                                                                                                                                                                                                                                                                                                                     |
| `--inline` / `inline-detox`    | Low — packaging docs only                                   | Debian package page, Linux Mint software page                                                                                                                                                                                                                                                                                                                                                                                  |

Flags never once seen in a tutorial, blog, or forum post despite being documented: `-f`, `--special`. These are pure reference-manual territory.

## Searches that found nothing

- **howtoforge.com** — no detox article exists (`site:howtoforge.com detox filenames` returned zero on-domain hits).
- **linuxhandbook.com** — no detox article exists (`site:linuxhandbook.com detox` returned zero on-domain hits).
- **tecmint.com** — no dedicated detox article exists; tecmint's own general "rename multiple files" article does not mention detox by name in the search snippet.
- **nixCraft / cyberciti.biz** — no dedicated detox article found under the nixCraft brand; searches surfaced only the unrelated Wix `Detox` mobile-testing framework and generic nixCraft site links.
- **itsfoss.com** — no detox-specific article surfaced in any search variant tried.
- **Reddit** — no specific r/linux (or other subreddit) thread recommending or discussing detox surfaced in search results.
- **asciinema.org** — no detox demo recording found; search returned only asciinema's own generic documentation.
- **YouTube video description** ([Detox - Cleanup Filenames - Linux CLI](https://www.youtube.com/watch?v=JD7jyjV8LRU)) — page fetch returned only YouTube's footer/legal boilerplate, not the actual title, description, or upload date. Could not confirm content; not cited as a source of claims.
- **linux.org forum thread** ("I just learned a new command...") and **linuxconfig.org** — both returned HTTP 403 on direct fetch. Content attributed to linuxconfig.org above is explicitly marked as search-snippet-only and not independently verified.
- **v1 vs v2 tutorial-level comparison** — the only source touching on version differences at all was the project's own `HACKING-v1.md` (fetched), which mentions "instructions are slightly different for version 2" without elaborating. No third-party tutorial anywhere discusses v1/v2/v3 behavioral differences; awareness of the v3 transliteration change is confined to the project's own README and man page, not the tutorial ecosystem.

## Source list (fetched and read directly)

1. [putorius.net — Linux Detox: Clean Up Problematic Filenames](https://www.putorius.net/linux-detox-clean-up-filenames-with-space-and-special-characters.html) (Nov 7, 2024)
2. [Delightly Linux — Clean Up Filenames with detox](https://delightlylinux.wordpress.com/2023/12/07/clean-up-filenames-with-detox/) (Dec 7, 2023)
3. [apt-upgrade.me — Cleaning Up Filenames on Linux with detox](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/) (Jul 24, 2025)
4. [Gentoo Wiki — Detox](https://wiki.gentoo.org/wiki/Detox)
5. [Arch manual pages — detox(1)](https://man.archlinux.org/man/detox.1.en) (v3.0.1-1, man page dated Mar 31, 2024)
6. [Art of Logic — Clean Up File Names](https://www.artoflogic.com/2020/03/clean-up-file-names/) (Mar 24, 2020)
7. [Mabox Linux Forum — CLI: detox - Clean up filenames](https://forum.maboxlinux.org/t/cli-detox-clean-up-filenames/721) (Nov 19, 2021)
8. [Debian — detox in sid](https://packages.debian.org/sid/utils/detox)
9. [GitHub — dharple/detox HACKING-v1.md](https://github.com/dharple/detox/blob/main/HACKING-v1.md)
10. [Linux Magazine — Bulk Renamers](https://www.linux-magazine.com/Online/Blogs/Off-the-Beat-Bruce-Byfield-s-Blog/Bulk-Renamers) (Mar 18, 2011)
11. [GitHub — ugzv/Awesome-Rename-Tools](https://github.com/ugzv/Awesome-Rename-Tools)
12. [Linux Mint Community — detox](https://community.linuxmint.com/software/view/detox)
13. [dotlinux.net — Clean Up Filenames with Detox Command-Line Utility](https://www.dotlinux.net/blog/clean-up-filenames-with-detox-command-line-utility/) (updated Jan 14, 2026)
14. [Debian — Orphaned packages list](https://www.debian.org/devel/wnpp/orphaned) (checked: detox absent)
15. [GitHub — dharple/detox README.md](https://github.com/dharple/detox/blob/main/README.md)
16. [FreshPorts — sysutils/detox](https://www.freshports.org/sysutils/detox/)
17. [Arch Linux — detox package page](https://archlinux.org/packages/extra/x86_64/detox/)
18. [GitHub API — repos/dharple/detox](https://api.github.com/repos/dharple/detox) (direct API call, confirms `archived: true`)

Additional pages surfaced only in search-result snippets (linuxconfig.org, linux.org forum thread, YouTube description, an AlternativeTo comment) are cited above with explicit flags that they were not independently fetched/verified.

---

## Review record (stage 3)

| Finding (reviewer)                                                                                                                                                   | Verdict               | Action or reason                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Tutorial shows v1.4.5-era `detox -s iso8859_1 "Café.jpg"` with no version disclaimer, contradicting v3's default of no transliteration; add a disclaimer (L1, MAJOR) | **Modify**            | Half of this was already done: §"Warnings and caveats" already carried a bold "**This is stale/misleading for current v3**" paragraph with the specific v3 behavior and the `table/legacy/` move. L1 appears to have missed it. But the _same command_ is reproduced in the "Flags seen in the wild" table with no warning at all, and a reader skimming that table would never reach the caveats section. Version warning added to the `-s sequence` row there. |
| "Overwhelmingly: it removes spaces and problematic characters..." rests on 2 verified sources plus 1 explicitly-unverified snippet (L2, MINOR)                       | **Accept**            | Reworded to "The tutorials found agree on one framing", with the actual count (2 fetched + 1 unverified snippet) stated in the section itself rather than only where the snippet is first flagged.                                                                                                                                                                                                                                                               |
| Gentoo Wiki cited for the v3 transliteration change, but the wiki page does not mention version differences (L1, "partially verified")                               | **Reject**            | L1 misread the attribution. This document cites the Gentoo Wiki only for the `--dry-run` quote (which L1 verified). The v3 transliteration change is attributed in the same document to "the Arch man page and the project's own README (both fetched)" — never to the Gentoo Wiki. No claim to fix.                                                                                                                                                             |
| Front matter should use consistent field names and a body-verifiable source count (L3)                                                                               | **Accept, no change** | Verified: fields are already `venue_scope` / `queries_run` / `source_count`, and the stated "18 pages fetched and read directly" matches the numbered source list exactly (18 items).                                                                                                                                                                                                                                                                            |
| "F2" (capital) at one point vs. "f2" (lowercase) elsewhere; standardize to lowercase (L3, MINOR)                                                                     | **Reject**            | The capitalized instances sit inside reported source wording (an AlternativeTo commenter's phrasing and a curated-list entry). Normalizing capitalization inside reported material to satisfy a house style edits the source. The tool is not ambiguous either way.                                                                                                                                                                                              |
| The YouTube-fetch-failure note is "verbose for a brief finding"; condense to "fetch failed; not cited" (L3, MINOR)                                                   | **Reject**            | The detail is the point: it records _what_ came back (footer/legal boilerplate rather than an error), which tells a future researcher the fetch was blocked by rendering, not by a dead URL, and is worth a retry with a different tool. Compressing it to three words destroys the only reusable information in the entry.                                                                                                                                      |
| This document is not redundant with the synthesis; keep as primary source (L3)                                                                                       | **Accept, no change** | Agreed. The flags-in-the-wild table (which documents that `-f` and `--special` appear in _no_ walkthrough anywhere) and the distro packaging table are reference material the synthesis deliberately does not carry, and the tutorial-vs-v3 drift analysis is the origin of synthesis problem #8.                                                                                                                                                                |
