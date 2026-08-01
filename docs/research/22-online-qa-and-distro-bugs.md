---
date: 2026-07-31
venue_scope: >
  Stack Overflow, Unix & Linux SE, Ask Ubuntu, ServerFault, Reddit, plus distro
  bug trackers (Debian BTS, Ubuntu/Launchpad) and the upstream GitHub issue
  tracker (dharple/detox), for the `detox` filename-cleaning utility. Excludes
  the Wix `Detox` mobile E2E test framework and the diet/health sense.
queries_run: 20 (15 general web searches + 5 site-targeted Q&A searches)
source_count: 14 distinct cited sources (8 GitHub issues, 2 Debian bug records, 1 Launchpad bug record, 3 blog/forum pages) — of which 3 were not directly fetchable (narkive 503, forum.linuxconfig.org 403, and the linuxconfig forum thread is title/summary only)
notes: Limited Q&A site content; detox is niche. Real issues found on GitHub and distro trackers.
---

# Online Q&A and Distro Bug Reports: detox (filename-cleaning utility)

> **Upstream status (added stage 3):** `dharple/detox` was **archived on 2026-07-12**
> (verified via the GitHub API; see doc 21). The issue tracker is read-only, so no
> "Unresolved" issue below can now be fixed upstream, and the "Current"/"Maintained"
> labels in the distro table further down describe **distro packaging only**, not an
> actively developed upstream.

## Search Coverage

**Q&A Sites Queried** (site-specific searches):

- `site:unix.stackexchange.com` — No results
- `site:stackoverflow.com` — No results (results were Wix Detox testing framework, not filename tool)
- `site:askubuntu.com` — No results
- `site:serverfault.com` — No results
- `site:reddit.com` — No results

**Distro & GitHub Queries** (general + site-targeted):

- Debian/Ubuntu package trackers
- Launchpad bug reports
- GitHub dharple/detox issues
- General web searches for detox problems

**Conclusion:** The filename-cleaning detox utility has virtually no presence on mainstream Q&A sites. Real user problems surface on GitHub issues, Debian/Ubuntu bug trackers, and technical blogs. The tool is niche enough that Stack Exchange/Reddit discussions do not reflect it meaningfully.

---

## GitHub Issues (dharple/detox)

| Issue | Link                                                                                                           | Date Reported | Problem                                                                                                                                                                                                                     | Status                                                                                                                      |
| ----- | -------------------------------------------------------------------------------------------------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| #11   | [crash on directory with carriage returns and spaces](https://github.com/dharple/detox/issues/11)              | ~2017         | User copied text from webpage into mkdir, creating filename with newlines and spaces; reported detox crashed, but later testing did not reproduce crash. May be library error, not detox itself.                            | Unresolved; archived repo (read-only)                                                                                       |
| #14   | [Malformed UTF-8 characters when no default character is set](https://github.com/dharple/detox/issues/14)      | ~2017         | UTF-8 translation creates malformed (incomplete) output characters; two off-by-one errors in UTF-8 translation code. Referenced Debian bug #861537.                                                                         | Fixed in v1.3.2-1 (Debian); upstream issue closed 2021-01-31 (verified via GitHub API). **Historical — absent from v2/v3.** |
| #19   | [Empty default "eats up" valid characters](https://github.com/dharple/detox/issues/19)                         | ~2017         | With empty default character, "every second" character (both safe and unsafe) was stripped. File `01 5G Core Networks.pdf` became `0 GCr ewrspf1`. Character table included `+`, `-`, `.`, `_`, `~` but these were removed. | Unresolved                                                                                                                  |
| #24   | [Remove --remove-trailing command line option](https://github.com/dharple/detox/issues/24)                     | ~2017         | --remove-trailing is deprecated; wipeup filter handles this functionality.                                                                                                                                                  | Closed (feature request)                                                                                                    |
| #30   | [man: detox -c](https://github.com/dharple/detox/issues/30)                                                    | ~2017         | **Documentation bug:** Example in manpage shows `detox -c my_detoxrc -L -v` but `-c` option does not exist and is not documented. Correct flag is `-f`.                                                                     | Unresolved                                                                                                                  |
| #79   | [simple example for converting an offending char to custom string](https://github.com/dharple/detox/issues/79) | ~2018         | User asked for example config to convert specific character to custom string (not just underscore). Feature request, not a bug.                                                                                             | Closed unimplemented in the 2026-07-12 wind-down — the tracker now has **0** open items (doc 02, verified via API)          |
| #128  | [make distcheck failed](https://github.com/dharple/detox/issues/128)                                           | ~2024         | Distribution tarball build failure.                                                                                                                                                                                         | Fixed in v2.0.2                                                                                                             |
| #129  | [Timeout on unit tests](https://github.com/dharple/detox/issues/129)                                           | ~2024         | Unit tests timing out.                                                                                                                                                                                                      | Fixed in v2.0.3 and v3.0.1                                                                                                  |

---

## Debian Bug Tracker

| Bug #    | Title                                                                                               | Link                                                                                                                                                                     | Date       | Issue                                                                                                                                                                           | Resolution                                                                                                                                     |
| -------- | --------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| #861537  | detox: causes malformed UTF-8 characters when no default character is set - fails to "fall through" | [Narkive Archive](https://linux.debian.bugs.dist.narkive.com/MSgroioz/bug-861537-detox-causes-malformed-utf-8-characters-when-no-default-character-is-set-fails-to-fall) | ~2017      | UTF-8 character translation produces malformed (incomplete) output. With certain UTF-8 characters and no default in config, detox mangles the filename.                         | Fixed in detox v1.3.2-1. Patch by Vasily Kolobkov, Zenaan Harkness, Quentin Guittard, Joao Eriberto Mota Filho.                                |
| #1080967 | detox: let distro's _FORTIFY_SOURCE take precedence                                                 | [Debian Mail Archive](https://www.mail-archive.com/debian-bugs-dist@lists.debian.org/msg1989864.html)                                                                    | 2024-09-05 | Upstream Makefiles hardcode `_FORTIFY_SOURCE=2`, conflicting with Debian/Ubuntu build flags and preventing distro security settings from being applied. Fails to build (FTBFS). | Fixed in detox v2.0.0-4 by removing hardcoded `_FORTIFY_SOURCE=2` from `src/Makefile.am` and `tests/unit/Makefile.am`. Patch by Nick Rosbrook. |

---

## Ubuntu Bug Tracker (Launchpad)

| Bug #      | Title                                   | Link                                                                                        | Date       | Issue                                                                                                                              | Resolution                                                                                                         |
| ---------- | --------------------------------------- | ------------------------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| LP:2079767 | _FORTIFY_SOURCE build failure on Ubuntu | [Launchpad Mail](https://www.mail-archive.com/ubuntu-bugs@lists.ubuntu.com/msg6208621.html) | 2024-09-05 | Same as Debian #1080967: upstream hardcoded `_FORTIFY_SOURCE=2` conflicts with Ubuntu build flags. Package fails to build (FTBFS). | Fixed in detox v2.0.0-3ubuntu1 with patch `d/p/use-distro-fortify-source.patch`. Status changed to "Fix Released". |

---

## Real-World Problems Reported (non-Q&A sources)

### 1. One backup run aborted by one macOS-originated filename (single dated incident)

**Source:** [apt-upgrade.me blog post (2025-07-24)](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/)

**The Problem:**
User's automated `rdiff-backup` backup job crashed mid-run due to "non-compliant filename" that the backup tool couldn't process. Root cause: files from macOS containing "Unicode quirks, invisible characters, and creative use of special symbols." The backup tool "`choked on that malformed filename and bailed out,` resulting in complete failure of the versioning system."

**The Solution as this author adopted it:**
Running `detox` before `rdiff-backup`. The author reports no further failures afterwards.

**Quote from the author:**

> "one of my automated backups on Linux unexpectedly crashed mid-run"

**Evidence strength:** **one source, one first-person incident, dated 2025-07-24.** It is
detailed and specific, but it is a single blog post — not a corroborated pattern. No severity
rating is assigned here: the reported outcome was an aborted backup run, and the post does not
report any file being lost, corrupted, or overwritten. Whether pre-backup sanitization is
generally necessary is not established by this one account.

---

### 2. Configuration and Feature Limitations

**Source:** [Delightly Linux blog (2023-12-07)](https://delightlylinux.wordpress.com/2023/12/07/clean-up-filenames-with-detox/)

**Limitation noted:**
Author observed that detox only cleans problematic characters—it does not correct other issues. Specifically: "the misspelled 'filname' is still 'filname'. detox does not correct spelling or rename words. It only removes problematic characters."

**Use case:** Best suited for environments where non-technical users create poorly-named files (e.g., `"Untitled '(Future Update\[s\])' Document.txt"`, `"**IMPORTANT**.txt"`, `"(Secret/Hide) My Notes!.odt"`).

---

### 3. Linux Forum Confusion (one poster, page not fully fetchable)

**Source:** [Linux Config Forum (topic title/summary only; page returns HTTP 403, re-checked 2026-07-31)](https://forum.linuxconfig.org/t/how-to-use-detox-version-1-45/7950)

**Confusion evident:**
**One** forum poster expressed uncertainty about how to configure and use detox v1.45,
specifically confused about "2 additional files" in that version. This is **one reported instance
of configuration confusion**, on a page that could not be fetched in full — not a demonstrated
pattern across new users.

---

## Version History: Major Changes

### Version 3.0.0 (2025-08-03)

- Moved legacy translation tables; removed Unicode transliteration attempts
- Changed CP-1252 and ISO-8859-1 handling to transcode to UTF-8
- Now looks for detoxrc in `$XDG_CONFIG_HOME`
- **Breaking change:** Reduced aggressive transliteration

### Version 2.0.0 (2024-03-30)

- **Breaking:** Transliteration no longer automatic; users must specify `detox -s utf_8` to replicate v1 behavior
- Config files no longer end with `.sample`
- **Recursion change:** Files/directories starting with `.` now ignored during recursion (affects `.git/`, `.cache/`, etc.)

### Version 1.4.5+ (2021+)

- Regression testing added
- Safe filter updated to convert newlines, carriage returns, tabs to underscores

---

## User Confusion Patterns

### Known Misunderstandings

**Framing note (added stage 3):** items 1, 4 and 5 below are **inferred from the changelog and
man page**, not from any user report found in this research. Only items 2 and 3 trace to an
actual reported issue (#30, #19). Read the labels on each.

1. **Configuration file location** (inferred from changelog, no user report found)**:** Users uncertain where detoxrc goes; v3.0.0 added `$XDG_CONFIG_HOME` support but this is not widely known.

2. **Flag confusion:** Manpage documents `-c` option but it doesn't exist; correct flag is `-f` (Issue #30). Users copying examples from man page will fail.

3. **Default character behavior:** Empty default character causes character loss (Issue #19); users need to understand the difference between empty vs. non-empty defaults.

4. **Recursion gotcha** (changelog fact; no user report found)**:** v2.0.0+ ignores
   dot-files/dot-directories by default during recursion. This is a documented behavior change,
   **not** a reported complaint — no user anywhere in this research said they were caught by it.

5. **Transliteration expectations** (changelog fact; no user report found)**:** v2.0.0 broke
   backward compatibility—users upgrading from v1 expecting automatic transliteration of
   accents/special chars need to explicitly add `-s utf_8`. No user report of being caught by
   this was found in any venue (doc 20 and doc 21 independently reach the same null result).

6. **Dry-run awareness — no evidence either way.** Every tutorial fetched leads with `-n`
   (doc 21), so the flag is well publicized. **No report was found of any user running detox
   without a preview and losing data**, and doc 20's targeted search for exactly that
   ("renamed my" / "deleted" / "overwrote" complaints) was an explicit dry hole. An earlier
   revision of this document asserted that users "run detox without preview, and lose data";
   that claim had no source and is retracted.

---

## Searches That Found Nothing

| Query                                                   | Site           | Result                                          |
| ------------------------------------------------------- | -------------- | ----------------------------------------------- |
| `site:unix.stackexchange.com detox rename filename`     | Unix SE        | No results                                      |
| `site:stackoverflow.com detox command filenames spaces` | Stack Overflow | No results (hits were Wix Detox test framework) |
| `site:askubuntu.com detox`                              | Ask Ubuntu     | No results (hits were health/diet detox)        |
| `site:serverfault.com detox rename`                     | ServerFault    | No results                                      |
| `site:reddit.com detox filename rename`                 | Reddit         | No results                                      |

**Interpretation:** The filename-cleaning detox tool is too niche for mainstream Q&A communities. Users who encounter issues either:

- File distro bugs (Debian/Ubuntu)
- Report on GitHub (dharple/detox)
- Document on personal blogs
- Ask on technical forums (linuxconfig.org)

---

## Distro Package Status

| Distribution       | Version                                                                            | Availability    | Status     |
| ------------------ | ---------------------------------------------------------------------------------- | --------------- | ---------- |
| Ubuntu             | 3.0.1-1 (Stonking dev), 2.0.0-3ubuntu1 (Questing 25.10), 1.4.5-5 (Noble 24.04 LTS) | In main repos   | Current    |
| Debian             | 3.0.1-1, 2.0.0-4                                                                   | In main repos   | Current    |
| Arch Linux         | 3.0.1-1                                                                            | AUR/extra       | Current    |
| Kali Linux         | 3.0.1-1                                                                            | In repos        | Current    |
| Fedora/RHEL/CentOS | Varies (python3-detox for Python pkg)                                              | EPEL, rpmfind   | Limited    |
| Gentoo             | Available                                                                          | Wiki documented | Maintained |

Distro package maintenance appears stable; recent versions ship v3.0.1 or v2.0.0+ across major
Linux distros. **"Current"/"Maintained" above describes packaging only.** Upstream is archived
(2026-07-12), so v3.0.1 is the terminal upstream release and these packages can only track
distro-local patches from here.

---

## Key Takeaways for UX/Docs

1. **Documentation accuracy:** Manpage example (Issue #30) uses non-existent flag; needs audit of all examples.

2. **Configuration discovery:** Users unaware of detoxrc location changes in v3 and `$XDG_CONFIG_HOME` support; should be prominent in docs.

3. **Safe by default:** Dry-run (`-n`) is a critical feature. Note that no evidence was found
   that users miss it — every tutorial in doc 21 leads with it — so this is a design preference,
   not an evidenced user complaint.

4. **Breaking changes:** v2.0.0 and v3.0.0 introduced breaking changes; upgrade guides needed to reduce user friction.

5. **Recursion behavior:** Dot-file ignoring (v2.0.0+) should be documented explicitly with
   examples. (Researcher recommendation from the changelog — no user reported being caught by it.)

6. **Character set edge cases:** With an empty default character, detox strips characters the
   table declares safe, producing a badly mangled name (Issue #19 — `01 5G Core Networks.pdf`
   became `0 GCr ewrspf1`). This is a **wrong-output** footgun, not data loss: the file is
   renamed, not lost. Worth a warning either way.

7. **Cross-platform issues:** macOS-created files cause real backup failures; use-case documentation around pre-backup sanitization would help.

---

## Metadata

- **Research date:** 2026-07-31
- **Queries executed:** 20 (15 web searches + 5 targeted site searches)
- **Distinct cited sources:** 14 — 8 GitHub issues, 2 Debian bug records, 1 Launchpad bug record,
  3 blog/forum pages. Of these, 3 could not be fetched directly (narkive 503,
  forum.linuxconfig.org 403, re-verified 2026-07-31).
- **Q&A coverage:** Negative (no results on major sites; tool is niche)
- **Distro coverage:** Positive (available on Debian, Ubuntu, Arch, Fedora, Gentoo; regularly updated)

---

## Review record (stage 3)

This document took the most correction of the four. All three reviewers converged on it, and the
findings held up on independent check.

| Finding (reviewer)                                                                                                                                                                                                      | Verdict                                                               | Action or reason                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| "Dry-run misuse: Users don't know about `-n`... run detox without preview, and **lose data**" has zero citation and contradicts doc 20's explicit finding that no data-loss complaint was ever found (L2, **CRITICAL**) | **Accept**                                                            | Confirmed: no source anywhere in this document, and doc 20 ran targeted searches for exactly this ("renamed my" / "deleted" / "overwrote") and found nothing. An invented harm claim is the worst possible defect here. The item is now "Dry-run awareness — no evidence either way", states the null result, notes that every tutorial in doc 21 leads with `-n`, and explicitly retracts the earlier assertion. The parallel line in §"Key Takeaways" was corrected too. |
| Issue #19 relabeled "causes data loss" — it is character stripping producing a wrong name, not lost data (L2, MAJOR)                                                                                                    | **Accept**                                                            | Severity inflation. Reworded to name the actual failure (the `01 5G Core Networks.pdf` → `0 GCr ewrspf1` example already in this document) and to state plainly that the file is renamed, not lost. Noted that the synthesis had already avoided this error independently.                                                                                                                                                                                                 |
| Single blog incident generalized into "Impact: High—data loss risk" and "a critical integration point for backup pipelines" (L2, MAJOR)                                                                                 | **Accept**                                                            | Both the invented severity rating and the architectural generalization are gone. Section retitled to say "single dated incident", evidence strength stated as one source, and it now records that the post reports an aborted run and no file lost, corrupted, or overwritten.                                                                                                                                                                                             |
| This document never mentions that upstream is archived (2026-07-12), unlike docs 20/21/23, while its distro table calls detox "Current"/"Maintained" (L2, MAJOR)                                                        | **Accept**                                                            | This was live-project framing on a frozen upstream. Archival banner added at the top, noting the tracker is read-only so no "Unresolved" issue can now be fixed. The distro table now states that "Current"/"Maintained" describes packaging only and that v3.0.1 is the terminal upstream release.                                                                                                                                                                        |
| One confused forum poster generalized to "a barrier for **new users**" (plural) (L2, MINOR)                                                                                                                             | **Accept**                                                            | Reworded to "one reported instance of configuration confusion", with the 403 status and the fact that only the title/summary was ever readable stated in the source line itself.                                                                                                                                                                                                                                                                                           |
| Front matter uses `venue`/`sources_fetched` where docs 20/21/23 use `venue_scope`/`source_count`; also `queries_run: 15` conflicts with the body's "15 web searches + 5 targeted site searches" (L3, MAJOR)             | **Accept**                                                            | Normalized to `venue_scope` / `source_count`, scope expanded to name the venues and the exclusions the way the sibling docs do, and `queries_run` corrected to 20 to match the body. The `notes` field is kept (docs 20/21/23 have none) because it carries a real finding rather than boilerplate; that is a deliberate divergence, not an oversight.                                                                                                                     |
| Source count should match the body (L3, MINOR)                                                                                                                                                                          | **Accept**                                                            | Recounted against the body: 8 GitHub issues + 2 Debian bug records + 1 Launchpad record + 3 blog/forum pages = 14 distinct cited sources, which matches the stated figure. The front matter and the metadata block now both add that 3 of the 14 were never fetchable (narkive 503, forum.linuxconfig.org 403 — both re-verified 2026-07-31).                                                                                                                              |
| Issue #14 / Debian #861537 should be marked historical, not read as live (L3)                                                                                                                                           | **Accept**                                                            | Row now reads "Fixed in v1.3.2-1 (Debian); upstream issue closed 2021-01-31 (verified via GitHub API). **Historical — absent from v2/v3.**"                                                                                                                                                                                                                                                                                                                                |
| Narkive source for Debian #861537 returns 503, undermining verifiability (L1, MAJOR)                                                                                                                                    | **Modify**                                                            | Re-verified: still 503. But the link text already said "Narkive Archive", the same bug is independently corroborated by GitHub issue #14 (which does resolve 200), and the fix version is confirmable from the Debian changelog. Relabeled in the front matter as one of the 3 unfetchable sources rather than removed — the claim does not rest on the dead page alone.                                                                                                   |
| The v2.0.0+ dot-file recursion gotcha should be promoted to a top problem in the synthesis (L3, MEDIUM)                                                                                                                 | **Reject** — see the synthesis's review record for the full reasoning | It is a changelog fact with **no user report behind it**. This document's own wording ("users expecting `.git` to be processed will be **surprised**" / "**shocked**") was researcher prediction presented as user experience. Rather than promote it, the §"User Confusion Patterns" entries are now individually labeled: items 1, 4 and 5 are marked as inferred from the changelog with no user report found; only items 2 and 3 trace to real issues (#30, #19).      |
| This document is not redundant with the synthesis; keep as primary source (L3)                                                                                                                                          | **Accept, no change**                                                 | Agreed. The per-issue resolution history (which bugs are fixed, in which version, and which remain open on a now-read-only tracker) is the material the synthesis compresses into single table cells.                                                                                                                                                                                                                                                                      |
