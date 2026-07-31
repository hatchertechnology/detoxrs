---
date: 2026-07-31
venue: Stack Overflow, Unix & Linux SE, Ask Ubuntu, ServerFault, Distro Bug Trackers (Debian/Ubuntu/Launchpad)
queries_run: 15
sources_fetched: 14
notes: Limited Q&A site content; detox is niche. Real issues found on GitHub and distro trackers.
---

# Online Q&A and Distro Bug Reports: detox (filename-cleaning utility)

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

| Issue | Link                                                                                                           | Date Reported | Problem                                                                                                                                                                                                                     | Status                                |
| ----- | -------------------------------------------------------------------------------------------------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| #11   | [crash on directory with carriage returns and spaces](https://github.com/dharple/detox/issues/11)              | ~2017         | User copied text from webpage into mkdir, creating filename with newlines and spaces; reported detox crashed, but later testing did not reproduce crash. May be library error, not detox itself.                            | Unresolved; archived repo (read-only) |
| #14   | [Malformed UTF-8 characters when no default character is set](https://github.com/dharple/detox/issues/14)      | ~2017         | UTF-8 translation creates malformed (incomplete) output characters; two off-by-one errors in UTF-8 translation code. Referenced Debian bug #861537.                                                                         | Fixed in v1.3.2-1                     |
| #19   | [Empty default "eats up" valid characters](https://github.com/dharple/detox/issues/19)                         | ~2017         | With empty default character, "every second" character (both safe and unsafe) was stripped. File `01 5G Core Networks.pdf` became `0 GCr ewrspf1`. Character table included `+`, `-`, `.`, `_`, `~` but these were removed. | Unresolved                            |
| #24   | [Remove --remove-trailing command line option](https://github.com/dharple/detox/issues/24)                     | ~2017         | --remove-trailing is deprecated; wipeup filter handles this functionality.                                                                                                                                                  | Closed (feature request)              |
| #30   | [man: detox -c](https://github.com/dharple/detox/issues/30)                                                    | ~2017         | **Documentation bug:** Example in manpage shows `detox -c my_detoxrc -L -v` but `-c` option does not exist and is not documented. Correct flag is `-f`.                                                                     | Unresolved                            |
| #79   | [simple example for converting an offending char to custom string](https://github.com/dharple/detox/issues/79) | ~2018         | User asked for example config to convert specific character to custom string (not just underscore). Feature request, not a bug.                                                                                             | Open                                  |
| #128  | [make distcheck failed](https://github.com/dharple/detox/issues/128)                                           | ~2024         | Distribution tarball build failure.                                                                                                                                                                                         | Fixed in v2.0.2                       |
| #129  | [Timeout on unit tests](https://github.com/dharple/detox/issues/129)                                           | ~2024         | Unit tests timing out.                                                                                                                                                                                                      | Fixed in v2.0.3 and v3.0.1            |

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

### 1. Cross-Platform File Corruption Risk

**Source:** [apt-upgrade.me blog post (2025-07-24)](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/)

**The Problem:**
User's automated `rdiff-backup` backup job crashed mid-run due to "non-compliant filename" that the backup tool couldn't process. Root cause: files from macOS containing "Unicode quirks, invisible characters, and creative use of special symbols." The backup tool "`choked on that malformed filename and bailed out,` resulting in complete failure of the versioning system."

**The Solution:**
Running `detox` before `rdiff-backup` prevents filename-related failures. This is a critical integration point for backup pipelines.

**Quote from user:**

> "one of my automated backups on Linux unexpectedly crashed mid-run"

**Impact:** High—data loss risk if backup pipeline is interrupted by filename issues.

---

### 2. Configuration and Feature Limitations

**Source:** [Delightly Linux blog (2023-12-07)](https://delightlylinux.wordpress.com/2023/12/07/clean-up-filenames-with-detox/)

**Limitation noted:**
Author observed that detox only cleans problematic characters—it does not correct other issues. Specifically: "the misspelled 'filname' is still 'filname'. detox does not correct spelling or rename words. It only removes problematic characters."

**Use case:** Best suited for environments where non-technical users create poorly-named files (e.g., `"Untitled '(Future Update\[s\])' Document.txt"`, `"**IMPORTANT**.txt"`, `"(Secret/Hide) My Notes!.odt"`).

---

### 3. Linux Forum Confusion

**Source:** [Linux Config Forum (topic title only accessible)](https://forum.linuxconfig.org/t/how-to-use-detox-version-1-45/7950)

**Confusion evident:**
User expressed uncertainty about how to configure and use detox v1.45, specifically confused about "2 additional files" in that version. Suggests documentation or setup complexity is a barrier for new users.

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

1. **Configuration file location:** Users uncertain where detoxrc goes; v3.0.0 added `$XDG_CONFIG_HOME` support but this is not widely known.

2. **Flag confusion:** Manpage documents `-c` option but it doesn't exist; correct flag is `-f` (Issue #30). Users copying examples from man page will fail.

3. **Default character behavior:** Empty default character causes character loss (Issue #19); users need to understand the difference between empty vs. non-empty defaults.

4. **Recursion gotcha:** v2.0.0+ ignores dot-files/dot-directories by default during recursion; users expecting `.git` to be processed will be surprised.

5. **Transliteration expectations:** v2.0.0 broke backward compatibility—users upgrading from v1 expecting automatic transliteration of accents/special chars need to explicitly add `-s utf_8`.

6. **Dry-run misuse:** Users don't know about `-n` (dry-run) flag, run detox without preview, and lose data.

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

Distro package maintenance appears stable; recent versions ship v3.0.1 or v2.0.0+ across major Linux distros.

---

## Key Takeaways for UX/Docs

1. **Documentation accuracy:** Manpage example (Issue #30) uses non-existent flag; needs audit of all examples.

2. **Configuration discovery:** Users unaware of detoxrc location changes in v3 and `$XDG_CONFIG_HOME` support; should be prominent in docs.

3. **Safe by default:** Dry-run (`-n`) is critical feature but not obvious; should be front-and-center in quick-start docs.

4. **Breaking changes:** v2.0.0 and v3.0.0 introduced breaking changes; upgrade guides needed to reduce user friction.

5. **Recursion behavior:** Dot-file ignoring (v2.0.0+) should be documented explicitly with examples; users expecting `.git` to be processed will be shocked.

6. **Character set edge cases:** Empty default character causes data loss (Issue #19); this is a footgun that should be called out in warnings.

7. **Cross-platform issues:** macOS-created files cause real backup failures; use-case documentation around pre-backup sanitization would help.

---

## Metadata

- **Research date:** 2026-07-31
- **Queries executed:** 15 web searches + 5 targeted site searches
- **Pages fetched:** 14
- **Findings:** 8 GitHub issues, 2 Debian bugs, 1 Ubuntu bug, 3+ blog/article sources
- **Q&A coverage:** Negative (no results on major sites; tool is niche)
- **Distro coverage:** Positive (available on Debian, Ubuntu, Arch, Fedora, Gentoo; regularly updated)
