# detox (dharple/detox) -- Issue Tracker & PR Mining: User Demand Signal

Source: `https://github.com/dharple/detox`, archived (paused by maintainer). Pulled via
unauthenticated GitHub REST API (`/repos/dharple/detox/issues`, `/pulls`, `/issues/{n}/comments`)
plus the repo tarball (`README.md`, `CHANGELOG.md`) on 2026-07-31.

**Repo stats at time of pull:** archived = true, open issues = 0, 140 issues+PRs total (all closed -- the tracker has zero open items), 446 stars, 24 forks, last push 2026-07-12.

This is not a partial sample: the `/issues` endpoint with `state=all&per_page=100` across 3 pages
returned all 140 items, and `open_issues_count` from the repo metadata confirms 0 are open. Every
issue/PR below is drawn from that full set.

### The 2026-07-12 wind-down sweep -- read this before reading any "closed" below

34 issues/PRs were closed inside a single ~50-minute window on 2026-07-12 (00:25--01:15 UTC), each
carrying an identical templated comment ("At this time, I don't plan on working ... putting `detox`
on hold") and the label `closed-with-detox`. Eight more were closed the same day without that label
(42 same-day closures in total). Verified independently at stage 3 by pulling labels and `closed_at`
for all 140 items (`GET /search/issues?q=repo:dharple/detox`, 2 pages, 2026-07-31).

"Closed" on a swept item is an **administrative sweep, not triage**. It supports neither reading:
not "the maintainer considered this and rejected it", and not "this is live open demand". Several
swept items had sat untouched for years first (#55, #69, #77 all filed 2021). Only items closed
_before_ 2026-07-12 carry an individual disposition.

- Labeled sweep (34): #7, #45, #49, #51, #54, #55, #61, #69, #70, #71, #76, #77, #85, #96, #104,
  #109, #114, #115, #116, #117, #120, #122, #123, #124, #125, #127, #131, #132, #133, #134, #136,
  #137, #139, #140.
- Closed the same day, unlabeled (8): #75, #79, #98, #103, #106, #108, #135, #138.

Swept items are marked **sweep** in the table's State column and `swept` in the theme lists.

### What this evidence base can and cannot support

This section was added at stage-3 review because the sibling document `user_feedback_online.md`
concluded that this tracker carries severe motivated-filer bias, and that several themes below have
no independent corroboration anywhere online. The tracker is still the best available record of what
`detox` users asked for. The point is calibration, not dismissal.

Three structural limits, all verified at stage 3 against all 140 items:

1. **Half the tracker is the maintainer talking to himself.** 73 of 140 items (52%) were filed by
   `dharple` (`author_association: OWNER`). 48 of those were created in February 2021 alone (#20--#70,
   spread across 2021-02-02 to 2021-02-28) during the v2/v3 planning burst. A maintainer's own
   planning ticket is evidence of **maintainer intent**, not of user demand. Every theme list below
   splits the two, and every table row now records its filer.
2. **Absolute volume is tiny.** 140 items across ~9 years of GitHub history for a niche CLI with an
   unknown install base. Theme rankings are relative-within-tracker orderings, not demand estimates.
   "17 issues" is not a market size and must not be read downstream as one.
3. **Only the annoyed filed.** Satisfied users and users who silently gave up leave no trace here, so
   the tracker over-represents friction and under-represents whatever already worked.

What this evidence base **can** support: that specific defects existed and were reproducible; what
the maintainer himself concluded and intended; and the relative ordering of _externally reported_
friction within this one venue.

What it **cannot** support: absolute demand size, "most users want X", or any claim that a theme with
zero external filers reflects a user need at all.

Each theme below is tagged **corroborated** / **partly corroborated** / **tracker-only**, following
`user_feedback_online.md`'s divergence analysis. `tracker-only` does not mean false -- it means
single-venue, so a motivated-filer artifact cannot be ruled out.

## Issue/PR Table

Only items carrying a demand signal are listed (bug reports with reproducible complaints, feature
requests, and PRs). Pure build/CI/packaging chores with no user-facing ask are omitted from the
table but folded into the packaging theme below.

"The maintainer" throughout this document means `dharple`, the repo owner. **State column:** every
item is closed (the tracker has zero open items), so the useful facts are _when_ and _by whom filed_.
`filer: owner` = filed by `dharple` (maintainer intent); `filer: third party` = external report
(demand signal); `filer: contributor` = external, with prior merged commits. **sweep** = closed in
the 2026-07-12 wind-down, i.e. never individually adjudicated. The Resolution column's verdict is
only trustworthy for items closed before 2026-07-12.

| #                                                                                                       | Type  | State                                                                        | Title                                                               | Ask                                                                                                                                                            | Maintainer response                                                                                                                                                                                                                                                                                   | Resolution                                                                                         |
| ------------------------------------------------------------------------------------------------------- | ----- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| [#140](https://github.com/dharple/detox/issues/140)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | replace full-width characters                                       | Handle Unicode full-width/halfwidth forms (used by SingleFile browser addon for Windows compat)                                                                | "I don't plan on working on this... putting detox on hold" -- suggested `rename` as workaround                                                                                                                                                                                                        | Not implemented, closed on pause                                                                   |
| [#136](https://github.com/dharple/detox/pull/136)                                                       | PR    | closed unmerged 2026-07-12 **sweep**<br>filer: third party                   | new sequence: fat . for exFAT/DOS SD card limits                    | Add a built-in sequence for exFAT/DOS filename limits (aria2c has no Windows-safe flag)                                                                        | "At this time, I don't plan on merging this... putting detox on hold"                                                                                                                                                                                                                                 | Rejected, not merged                                                                               |
| [#137](https://github.com/dharple/detox/issues/137)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | `malloc(): invalid size (unsorted)` when deleting characters        | Memory-safety crash on certain input with a delete-characters config                                                                                           | None -- only the templated wind-down comment                                                                                                                                                                                                                                                          | Unresolved; added at stage-3 review (was missing entirely)                                         |
| [#130](https://github.com/dharple/detox/pull/130)                                                       | PR    | closed unmerged 2025-10-11<br>filer: third party                             | Add option to overwrite/replace existing files                      | `-F` flag to let detox overwrite collisions instead of refusing                                                                                                | Detailed technical rejection: risk of collapsing N files to 1 file if sequences/tables map multiple names to the same output, `readdir()` ordering hazards, needs `S_ISREG`/`S_ISDIR` checks, BSD/macOS syscall differences                                                                           | Not merged -- "I don't want to be responsible for destroying other people's data"                  |
| [#124](https://github.com/dharple/detox/issues/124)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | option to ignore spaces and fix only wrong chars                    | Let users keep spaces, replace only truly unsafe chars, without hand-editing `.tbl`/`.rc`                                                                      | Gave manual `safe.tbl`/`detoxrc` edit workaround; later (2025-08-11, verbatim): "I can't commit to any enhancements. I don't have the time... I'm thinking about how to make detox easier to use, either with a wrapper layer... or with a total rewrite. I'd like to move it toward 'it just works'" | Not implemented; explicit roadmap statement given here                                             |
| [#122](https://github.com/dharple/detox/issues/122)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | Optional new flag needed (= `-o` overwrite)                         | Same overwrite ask as #130, different reporter                                                                                                                 | No maintainer reply beyond community                                                                                                                                                                                                                                                                  | Not implemented                                                                                    |
| [#121](https://github.com/dharple/detox/issues/121)                                                     | Issue | closed 2024-11-07<br>filer: third party                                      | Why detox removes underscores surrounding hyphens?                  | Preserve `_-_` separator convention in music filenames; disagreement with default hyphen/underscore collapsing                                                 | Pointed to custom-sequence workaround                                                                                                                                                                                                                                                                 | Not implemented natively                                                                           |
| [#119](https://github.com/dharple/detox/issues/119)                                                     | Issue | closed 2024-11-07<br>filer: third party                                      | Is there any way to undo changes?                                   | Undo/rollback support                                                                                                                                          | "No, there's no undo."                                                                                                                                                                                                                                                                                | Confirmed absent, closed                                                                           |
| [#118](https://github.com/dharple/detox/issues/118)                                                     | Issue | closed 2024-11-07<br>filer: third party                                      | Add ability to set a max filename length                            | Truncate to a max length                                                                                                                                       | Already exists (`max_length` filter) -- doc/discoverability gap                                                                                                                                                                                                                                       | Resolved via existing feature                                                                      |
| [#117](https://github.com/dharple/detox/issues/117)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | Don't append underscore after deaccented characters                 | Diacritic stripping without trailing `_` (e.g. "łódź"→"lodz" not "l_o_dz_")                                                                                    | Community workaround only                                                                                                                                                                                                                                                                             | Not fixed                                                                                          |
| [#116](https://github.com/dharple/detox/issues/116)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | 0x202F (narrow no-break space) not replaced on macOS                | Cross-platform Unicode handling inconsistency (works on Linux, fails on macOS)                                                                                 | Added a unit test, admitted "I'm not currently able to run unit tests on macOS," bug reproduced by user on 3.0.1; closed on pause                                                                                                                                                                     | Unresolved -- "I don't plan on working any further... putting detox on hold"                       |
| [#113](https://github.com/dharple/detox/pull/113)                                                       | PR    | **merged** 2024-03-31<br>filer: owner                                        | Remove transliteration                                              | Strip transliteration from main tables, filter only unsafe chars                                                                                               | Merged as part of v3 rewrite                                                                                                                                                                                                                                                                          | Merged -- became v3.0.0                                                                            |
| [#112](https://github.com/dharple/detox/issues/112)                                                     | Issue | closed 2024-03-31<br>filer: owner                                            | Shift detox from transliteration to handling problematic characters | Tracking ticket for the v3 philosophy shift                                                                                                                    | "Ticket for tracking creation of v3 and shift in focus"                                                                                                                                                                                                                                               | Implemented (v3)                                                                                   |
| [#111](https://github.com/dharple/detox/issues/111) (duplicate of #124)                                 | Issue | closed 2024-02-12<br>filer: third party                                      | option to ignore spaces and fix only wrong chars (duplicate ask)    | Same as #124                                                                                                                                                   | No substantive fix                                                                                                                                                                                                                                                                                    | Not implemented                                                                                    |
| [#110](https://github.com/dharple/detox/issues/110)                                                     | Issue | closed 2024-03-31<br>filer: third party                                      | add `--git` option to use `git mv` instead of `mv`                  | Git-aware rename so history/blame survive                                                                                                                      | Declined: "I won't be doing this... uses `rename()`", suggested `git add -A` workaround                                                                                                                                                                                                               | Not implemented                                                                                    |
| [#109](https://github.com/dharple/detox/issues/109)                                                     | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | Equal sign not tamed                                                | `=` in filenames breaks some shell contexts                                                                                                                    | --                                                                                                                                                                                                                                                                                                    | Not resolved in table shown                                                                        |
| [#108](https://github.com/dharple/detox/issues/108)                                                     | Issue | closed 2026-07-12 **sweep (unlabeled)**<br>filer: third party                | unsupported unicode length                                          | Files silently skipped on long/invalid Unicode sequences, no way to force/bypass                                                                               | --                                                                                                                                                                                                                                                                                                    | Unresolved                                                                                         |
| [#106](https://github.com/dharple/detox/issues/106)                                                     | Issue | closed 2026-07-12 **sweep (unlabeled)**<br>filer: third party                | Handle 2044 (Fraction Slash)                                        | Re-add a specific Unicode translation                                                                                                                          | Investigated, asked for repro details                                                                                                                                                                                                                                                                 | Re-added in 3.0.0-beta2 per CHANGELOG                                                              |
| [#105](https://github.com/dharple/detox/issues/105) / [#89](https://github.com/dharple/detox/issues/89) | Issue | closed 2024-03-31<br>filer: third party                                      | Space handling confusing / editing space out breaks parsing         | Users repeatedly can't figure out how to keep spaces; editing `safe.tbl` to remove the space rule causes adjacent-char corruption (`abc def.xyz`→`abc ef.xyz`) | Doc pointer / no fix for the corruption                                                                                                                                                                                                                                                               | Config-file complexity theme; corruption bug in #89 not resolved in visible thread                 |
| [#102](https://github.com/dharple/detox/issues/102)                                                     | Issue | closed 2023-07-24<br>filer: third party                                      | add a flag to lowercase filenames                                   | Uppercase→lowercase conversion flag                                                                                                                            | --                                                                                                                                                                                                                                                                                                    | Not implemented as native flag (community `find`+`-s lower` workaround exists per #95)             |
| [#101](https://github.com/dharple/detox/pull/101)                                                       | PR    | **merged** 2024-03-30<br>filer: contributor                                  | look for detoxrc in $XDG_CONFIG_HOME                                | XDG base-dir compliance                                                                                                                                        | Merged                                                                                                                                                                                                                                                                                                | Merged into v2.0.0                                                                                 |
| [#99](https://github.com/dharple/detox/issues/99)                                                       | Issue | closed 2024-03-31<br>filer: third party                                      | Please provide a way to retain German Umlaute ÄÜÖäüö                | Transliteration was too aggressive, destroyed intentional non-ASCII                                                                                            | "Version 3 of detox removes all of the transliteration, so this should no longer be a problem"                                                                                                                                                                                                        | Resolved by v3 default-behavior change                                                             |
| [#98](https://github.com/dharple/detox/issues/98)                                                       | Issue | closed 2026-07-12 **sweep (unlabeled)**<br>filer: third party                | Replace + remove punctuation in one command                         | More flexible one-shot character mapping                                                                                                                       | --                                                                                                                                                                                                                                                                                                    | Not directly resolved                                                                              |
| [#97](https://github.com/dharple/detox/pull/97)                                                         | PR    | **merged** 2023-12-01<br>filer: contributor                                  | Fixed umlaut conversion (Ü→Ue not UE)                               | Casing bug in umlaut expansion                                                                                                                                 | Merged                                                                                                                                                                                                                                                                                                | Merged                                                                                             |
| [#96](https://github.com/dharple/detox/issues/96)                                                       | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | double free/corruption with custom sequence on 2TB recursive run    | Memory-safety crash on a large directory tree with a custom sequence                                                                                           | --                                                                                                                                                                                                                                                                                                    | Unresolved in visible thread; only large-scale stability bug found                                 |
| [#95](https://github.com/dharple/detox/issues/95)                                                       | Issue | closed 2022-10-10<br>filer: third party                                      | Ignore folders?                                                     | Target only files, skip directories                                                                                                                            | "no way to specifically target directories or files with detox itself" -- `find`+`-exec` workaround                                                                                                                                                                                                   | Confirmed limitation, not fixed                                                                    |
| [#94](https://github.com/dharple/detox/issues/94)                                                       | Issue | closed 2022-08-05<br>filer: third party                                      | How to ignore macOS `Icon␍` files?                                  | Exclude specific macOS metadata files                                                                                                                          | Community-only                                                                                                                                                                                                                                                                                        | Not resolved via native config                                                                     |
| [#93](https://github.com/dharple/detox/pull/93)                                                         | PR    | **merged** 2022-06-17<br>filer: owner                                        | CircleCI project setup                                              | CI migration off Travis                                                                                                                                        | Merged                                                                                                                                                                                                                                                                                                | Merged                                                                                             |
| [#92](https://github.com/dharple/detox/pull/92)                                                         | PR    | **merged** 2022-06-17<br>filer: owner                                        | Add `-Werror` to `AC_CHECK_CFLAG`                                   | Build hardening                                                                                                                                                | Merged                                                                                                                                                                                                                                                                                                | Merged                                                                                             |
| [#91](https://github.com/dharple/detox/issues/91)                                                       | Issue | closed 2022-06-17<br>filer: owner                                            | Compiling under CircleCI is broken                                  | CI breakage                                                                                                                                                    | Fixed via #92                                                                                                                                                                                                                                                                                         | Resolved                                                                                           |
| [#90](https://github.com/dharple/detox/issues/90)                                                       | Issue | closed 2022-05-05<br>filer: third party                                      | Operate on stdin list, output on stdout                             | Pipe-friendly batch mode                                                                                                                                       | "We already have some support for that... `detox --inline` or `inline-detox`"                                                                                                                                                                                                                         | Resolved -- feature already existed                                                                |
| [#89](https://github.com/dharple/detox/issues/89)                                                       | Issue | closed 2022-01-21<br>filer: third party                                      | Not replacing space eats up the next character                      | Editing out the space rule corrupts adjacent chars                                                                                                             | --                                                                                                                                                                                                                                                                                                    | Bug, unresolved in thread                                                                          |
| [#88](https://github.com/dharple/detox/pull/88)                                                         | PR    | **merged** 2021-11-06<br>filer: contributor                                  | Simple maintenance improvements                                     | Misc cleanup                                                                                                                                                   | Merged                                                                                                                                                                                                                                                                                                | Merged                                                                                             |
| [#87](https://github.com/dharple/detox/issues/87)                                                       | Issue | closed 2021-11-06<br>filer: third party                                      | How can I delete certain chars instead of replacing them?           | Delete vs. replace semantics unclear                                                                                                                           | Maintainer confirmed empty-string replacement works, found + acknowledged 2 related bugs (`remove_trailing` doesn't strip trailing `_` without extension; `defaultdefault` string-duplication bug)                                                                                                    | Partially resolved (documented), 2 sub-bugs opened                                                 |
| [#86](https://github.com/dharple/detox/issues/86)                                                       | Issue | closed 2021-11-06<br>filer: third party                                      | `utf_8-only` doing more than transliteration (touching brackets)    | Filter scope-creep: UTF-8 filter also doing "safe"-filter-like substitutions                                                                                   | --                                                                                                                                                                                                                                                                                                    | Confirmed as design confusion, addressed conceptually by v3's filter-scope cleanup (#40/#112)      |
| [#84](https://github.com/dharple/detox/issues/84)                                                       | Issue | closed 2021-11-06<br>filer: third party                                      | Is there a way to pass a custom `.tbl` file to a filter?            | Custom translation table support, unclear syntax                                                                                                               | Community answered (`filename "path/to.tbl"` in sequence block)                                                                                                                                                                                                                                       | Resolved via docs/community, config syntax confirmed painful                                       |
| [#83](https://github.com/dharple/detox/issues/83)                                                       | Issue | closed 2021-11-06<br>filer: third party                                      | Released tarballs missing CHANGELOG/LICENSE/THANKS                  | Packaging completeness                                                                                                                                         | --                                                                                                                                                                                                                                                                                                    | Packaging/distro theme                                                                             |
| [#81](https://github.com/dharple/detox/issues/81)                                                       | Issue | closed 2021-08-14<br>filer: third party                                      | "Value too large for defined data type" on Raspbian armv7l          | Crash on ARM32/Raspbian building large files                                                                                                                   | Reproduced, root-caused to autoconf large-file-support flag, fixed                                                                                                                                                                                                                                    | Fixed in v1.4.4                                                                                    |
| [#80](https://github.com/dharple/detox/issues/80)                                                       | Issue | closed 2021-07-28<br>filer: owner                                            | Compilation under Windows with MSYS2 fails                          | Windows build breakage                                                                                                                                         | --                                                                                                                                                                                                                                                                                                    | Windows portability theme                                                                          |
| [#77](https://github.com/dharple/detox/issues/77)                                                       | Issue | closed 2026-07-12 **sweep**<br>filer: third party                            | Compiling error on windows 10 (msys2/mingw64)                       | `lstat()` missing on MSYS2                                                                                                                                     | Provided patch, diagnosed msys2 lacks `lstat()` permanently, needed autoconf check + fallback to `stat()`                                                                                                                                                                                             | Unresolved -- "At this time, I don't plan on working any further on this... putting detox on hold" |
| [#75](https://github.com/dharple/detox/issues/75)                                                       | Issue | closed 2026-07-12 **sweep (unlabeled)**<br>filer: third party                | unconvertible files?                                                | Odd Unicode (0xF022 PUA) causing unclear errors                                                                                                                | Investigated, asked for repro info, no closure recorded                                                                                                                                                                                                                                               | Unresolved/stalled                                                                                 |
| [#74](https://github.com/dharple/detox/issues/74)                                                       | Issue | closed 2021-03-13<br>filer: owner                                            | inline-detox fails if last char of stdin isn't a newline            | Streaming/pipe edge case                                                                                                                                       | Fixed                                                                                                                                                                                                                                                                                                 | Fixed in 2.0.0-beta2                                                                               |
| [#73](https://github.com/dharple/detox/issues/73)                                                       | Issue | closed 2021-03-06<br>filer: third party                                      | NetBSD's `cp` doesn't support `-n`                                  | BSD portability in test suite                                                                                                                                  | --                                                                                                                                                                                                                                                                                                    | BSD portability theme                                                                              |
| [#69](https://github.com/dharple/detox/issues/69)                                                       | Issue | closed 2026-07-12 **sweep**<br>filer: owner                                  | Fix unit tests under macOS                                          | macOS test infra gap                                                                                                                                           | --                                                                                                                                                                                                                                                                                                    | macOS portability theme                                                                            |
| [#59](https://github.com/dharple/detox/issues/59)                                                       | Issue | closed 2021-02-22<br>filer: owner                                            | Find a replacement for Travis                                       | CI migration                                                                                                                                                   | Led to CircleCI (#93)                                                                                                                                                                                                                                                                                 | Resolved                                                                                           |
| [#55](https://github.com/dharple/detox/issues/55)                                                       | Issue | closed 2026-07-12 **sweep**<br>filer: owner                                  | `max_length` filter chops UTF-8 chars                               | Multi-byte-unsafe truncation                                                                                                                                   | --                                                                                                                                                                                                                                                                                                    | UTF-8/i18n theme                                                                                   |
| [#53](https://github.com/dharple/detox/issues/53)                                                       | Issue | closed 2021-02-17<br>filer: owner                                            | Update transliterations through Latin Extended-B using Unicode docs | Broaden transliteration tables                                                                                                                                 | Superseded by v3 removing transliteration                                                                                                                                                                                                                                                             | Superseded                                                                                         |
| [#47](https://github.com/dharple/detox/issues/47)                                                       | Issue | closed 2021-02-17<br>filer: owner                                            | Look into using Text::Unidecode's tables                            | Better transliteration source data                                                                                                                             | Added `unidecode.tbl` in 2.0.0-beta1                                                                                                                                                                                                                                                                  | Implemented, later mooted by v3                                                                    |
| [#42](https://github.com/dharple/detox/issues/42)                                                       | Issue | closed 2021-03-01<br>filer: owner                                            | Refactor config_file_spoof, add other sequences                     | Config/sequence architecture cleanup                                                                                                                           | --                                                                                                                                                                                                                                                                                                    | Config complexity theme                                                                            |
| [#40](https://github.com/dharple/detox/issues/40)                                                       | Issue | closed 2021-02-14<br>filer: owner                                            | UTF-8 filter behaves like the safe filter                           | Filter responsibilities conflated (0x20–0x3F get "safe"-ified inside the UTF-8 filter)                                                                         | --                                                                                                                                                                                                                                                                                                    | Design confusion theme, fed into v3 rewrite                                                        |
| [#33](https://github.com/dharple/detox/issues/33)                                                       | Issue | closed 2021-02-15<br>filer: owner                                            | Add support for 4-byte UTF-8                                        | Missing coverage for 4-byte sequences (emoji, some CJK/supplementary planes)                                                                                   | --                                                                                                                                                                                                                                                                                                    | i18n/CJK-adjacent theme                                                                            |
| [#29](https://github.com/dharple/detox/issues/29)                                                       | Issue | closed 2021-02-14<br>filer: owner                                            | Safe filter behaves differently when table is missing               | Inconsistent defaulting: missing table strips UTF-8, table-based safe leaves it alone                                                                          | --                                                                                                                                                                                                                                                                                                    | Config/encoding-consistency theme                                                                  |
| [#21](https://github.com/dharple/detox/issues/21)                                                       | Issue | closed 2021-02-14<br>filer: owner                                            | Update the default runtime behavior                                 | Track: make default = safe + wipeup only, move iso8859_1/utf_8 to opt-in transliteration                                                                       | Implemented                                                                                                                                                                                                                                                                                           | Implemented -- became v2 default                                                                   |
| [#19](https://github.com/dharple/detox/issues/19)                                                       | Issue | closed 2021-02-01<br>filer: third party                                      | Empty default "eats up" valid characters                            | Custom safe-table + empty default strips wanted chars unpredictably                                                                                            | --                                                                                                                                                                                                                                                                                                    | Config complexity / safe-charset theme                                                             |
| [#17](https://github.com/dharple/detox/issues/17)                                                       | Issue | closed 2021-02-01<br>filer: third party                                      | Detox doesn't handle filenames with newlines                        | Newlines in filenames not neutralized by default                                                                                                               | Fixed by adding `0x0A`/`0x0D` to `safe.tbl`; user later found it hadn't reached their distro's shipped table                                                                                                                                                                                          | Fixed in v1.4.0, packaging-lag friction visible                                                    |
| [#14](https://github.com/dharple/detox/issues/14)                                                       | Issue | closed 2021-01-31<br>filer: third party                                      | Malformed UTF-8 when no default char set -- fails to "fall through" | UTF-8 off-by-one translation bugs corrupting filenames (produced literal `<C2>`/`<C3>` artifacts)                                                              | Root-caused two off-by-one errors, fixed, verified with Debian maintainer                                                                                                                                                                                                                             | Fixed in v1.3.1                                                                                    |
| [#11](https://github.com/dharple/detox/issues/11)                                                       | Issue | closed 2021-02-02<br>filer: third party                                      | crash on directory with carriage returns and spaces                 | Crash (memory-safety) on filenames containing CR + spaces                                                                                                      | --                                                                                                                                                                                                                                                                                                    | Closed 2021 without a recorded fix note; added at stage-3 review (was missing entirely)            |
| [#9](https://github.com/dharple/detox/issues/9)                                                         | Issue | closed 2021-02-02<br>filer: owner                                            | safe filter mishandles TAB in filenames                             | Tabs not neutralized                                                                                                                                           | --                                                                                                                                                                                                                                                                                                    | Unresolved in visible thread                                                                       |
| [#7](https://github.com/dharple/detox/issues/7)                                                         | Issue | closed 2026-07-12 **sweep**<br>filer: owner (relaying Debian pkg maintainer) | Specify character set from the command line                         | CLI flags (`-c`, `-d`) to add/delete chars without editing tables -- filed by the Debian package maintainer                                                    | "This fits nicely in with my vision of v2, pushing all of the actual sequencing to the command line and away from config files"                                                                                                                                                                       | Explicit roadmap statement; not fully implemented as literal `-c`/`-d` flags                       |

**PR tally.** 13 PRs exist in the 140-item set (re-verified at stage 3); 8 merged, 5 unmerged.
Merged/unmerged flags per PR rest on stage-2 reviewer L1's per-PR `merged` field check, not on a
stage-3 re-pull.

- Merged (8): [#18](https://github.com/dharple/detox/pull/18) (ppc64le Travis build, contributor),
  [#88](https://github.com/dharple/detox/pull/88) (maintenance, contributor),
  [#92](https://github.com/dharple/detox/pull/92) (`-Werror`, owner),
  [#93](https://github.com/dharple/detox/pull/93) (CircleCI setup, owner),
  [#97](https://github.com/dharple/detox/pull/97) (umlaut casing, contributor),
  [#101](https://github.com/dharple/detox/pull/101) (`$XDG_CONFIG_HOME`, contributor),
  [#107](https://github.com/dharple/detox/pull/107) (macOS install docs, contributor),
  [#113](https://github.com/dharple/detox/pull/113) (remove transliteration, owner).
- Unmerged (5): [#15](https://github.com/dharple/detox/pull/15) ("1.3.0.mikros", owner's own,
  self-closed 2020-01-13), [#58](https://github.com/dharple/detox/pull/58) ("Travis macos", owner's
  own, self-closed 2021-02-20), [#130](https://github.com/dharple/detox/pull/130) (overwrite flag,
  third party), [#133](https://github.com/dharple/detox/pull/133) (man-page update, third party),
  [#136](https://github.com/dharple/detox/pull/136) (exFAT sequence, third party).
- **Correction (stage 3):** an earlier draft said "#130/#133/#136 rejected explicitly ... two on
  safety grounds". Only **#130** carries a substantive safety rejection. #136 was closed with the
  generic wind-down template. #133 is a _documentation_ PR ("Mention #101 PR changes in man pages")
  swept on 2026-07-12 with no rejection rationale of any kind. Two of the five unmerged PRs (#15,
  #58) are the owner's own throwaway branches and are not rejections of anyone's contribution.

## Theme Synthesis (ordering held stable for downstream citations)

### How "evidence weight" is computed -- read before using any number below

Theme numbers 1--10 are **stable identifiers**, not a live ranking: doc 00 cites themes by number,
so nothing below is renumbered even where a recount changes the ordering. Where a recount changes a
theme's standing, that is stated in place.

Stage-3 review found the previous version's weights unusable in three ways, all now fixed:

1. **The header counts disagreed with the theme's own bracketed list**, by 1--3 items, in themes 2,
   3, 4 and 8 (L3, confirmed by L1 and by my own recount). Every count below is now the exact
   length of the list printed beside it.
2. **"Evidence weight" was presented as a plain issue count but was not one.** A second, unstated
   adjustment was applied for user-facing versus maintainer-only pain: theme 8 has more raw items
   (15) than theme 3 (14) or theme 4 (10), yet was ranked below both, on the unstated ground that
   packaging pain is the maintainer's, not a user's. That adjustment is legitimate but must be
   visible, so counts and adjustment are now reported separately.
3. **Raw counts silently mixed the maintainer's own tickets with independent user reports.** Every
   theme below now splits **external filers** (`author_association` != `OWNER`; the demand signal)
   from **owner-filed** (`dharple`; maintainer intent, not demand). Verified for all 140 items via
   `GET /search/issues?q=repo:dharple/detox` (2 pages, 2026-07-31): 73 of 140 items are OWNER-filed,
   62 `NONE`, 5 `CONTRIBUTOR`.

**Themes overlap; counts are not a partition.** An issue may appear under several themes, so theme
counts do not sum to 140 and cannot be added together. Known shared items: #29 (themes 1, 2, 3),
#19/#89/#121 (1, 2), #7/#42/#102 (1, 10), #122 (1, 6), #52 (1, 5), #40 (2, 3), #116 (3, 4),
#33/#108/#120/#140 (3, 9), #91 (4, 8). Theme 1 minus everything it shares with theme 10 is still 14
items (12 external), so the overlap does not change its standing.

Bucketing is **the author's classification** from keyword search plus manual reading, not a
deterministic metric; a different tagger could move borderline items (#86, #100) between themes 2
and 3. This caveat is repeated in Confidence & Sources; it is stated here because this is where it
bites.

**Corroboration tags** follow `user_feedback_online.md`'s divergence analysis (its "top problems"
table and "where online sentiment diverges" section). `tracker-only` does **not** mean false -- it
means single-venue, so a motivated-filer artifact cannot be ruled out. Every state marker is
`swept` = closed in the 2026-07-12 wind-down (never individually adjudicated), `swept-unlabeled` =
closed that same day without the label, or an individual close date.

| Theme                            | Items | External | Owner | Swept | Corroboration       | Standing after recount                             |
| -------------------------------- | ----- | -------- | ----- | ----- | ------------------- | -------------------------------------------------- |
| 1. Config/sequence complexity    | 17    | **12**   | 5     | 3     | tracker-only        | Highest on both raw and external count. Unchanged. |
| 2. Safe-charset disagreements    | 9     | 7        | 2     | 2     | tracker-only        | Highest external _ratio_ (7/9). Unchanged.         |
| 3. Unicode/UTF-8 correctness     | 14    | 7        | 7     | 7     | **corroborated**    | Ties theme 2 on external count; see note.          |
| 4. Cross-platform portability    | 10    | **3**    | 7     | 3     | partly corroborated | Materially weaker as _demand_; see theme text.     |
| 5. Transliteration reversal      | 9     | **1**    | 8     | 1     | tracker-only        | Almost entirely maintainer intent; see note.       |
| 6. Collision/overwrite           | 2     | 2        | 0     | 1     | tracker-only        | Low count, both external. Unchanged.               |
| 7. Undo / dry-run trust          | 1     | 1        | 0     | 0     | split (see text)    | Low count; dry-run value corroborated, undo not.   |
| 8. Packaging/build churn         | 15    | 3        | 12    | 2     | partly corroborated | Confirms it is maintainer pain: 12/15 owner-filed. |
| 9. Wide/4-byte Unicode, CJK-adj. | 5     | 3        | 2     | 4     | tracker-only        | Weak, as already disclaimed. Unchanged.            |
| 10. Sequence/CLI ergonomics      | 6     | 3        | 3     | 1     | tracker-only        | Unchanged.                                         |
| (Crash/stability, see below)     | 5     | **5**    | 0     | 3     | tracker-only        | **New at stage 3**; not numbered, see note.        |

**Reconciling L1 and L3 on theme 1.** L3 was right that the header numbers were wrong; L1 was right
that theme 1 still comes out on top. Both hold: theme 1 is 17 items (not "~15+"), and it leads on
raw count (17 vs. 15 for theme 8, 14 for theme 3) _and_ on external count (12 vs. 7 for themes 2 and
3), and still leads after removing every item it shares with another theme. Doc 00's use of theme 1
as its highest-weight evidence survives; the _number_ it should cite is **17 items, 12 of them from
external filers**, not "~15".

**Two orderings that do not survive the recount**, noted rather than renumbered: theme 2 sits above
theme 3 despite having fewer items (9 vs. 14), and theme 8 sits below themes 3 and 4 despite having
more (15). Both are consequences of the maintainer-versus-user adjustment in point 2 above, and both
are defensible once that adjustment is explicit -- but a reader sorting by raw count will get a
different order, and should.

Each theme below is split into **Evidence** (facts derived from the issues) and **Design implication
(author's read)** (the load-bearing verdicts doc 00 consumes). The verdicts are opinion, not tracker
content.

### 1. Config-file / sequence-syntax complexity -- 17 items, 12 external (**tracker-only**)

- External filers (12): [#19](https://github.com/dharple/detox/issues/19) (closed 2021-02),
  [#84](https://github.com/dharple/detox/issues/84) (closed 2021-11),
  [#89](https://github.com/dharple/detox/issues/89) (closed 2022-01),
  [#94](https://github.com/dharple/detox/issues/94) (closed 2022-08),
  [#95](https://github.com/dharple/detox/issues/95) (closed 2022-10),
  [#102](https://github.com/dharple/detox/issues/102) (closed 2023-07),
  [#105](https://github.com/dharple/detox/issues/105) (closed 2024-03),
  [#111](https://github.com/dharple/detox/issues/111) (closed 2024-02),
  [#118](https://github.com/dharple/detox/issues/118) (closed 2024-11),
  [#121](https://github.com/dharple/detox/issues/121) (closed 2024-11),
  [#122](https://github.com/dharple/detox/issues/122) (swept),
  [#124](https://github.com/dharple/detox/issues/124) (swept).
- Owner-filed (5): [#7](https://github.com/dharple/detox/issues/7) (swept, but relaying the Debian
  package maintainer -- see the table row),
  [#29](https://github.com/dharple/detox/issues/29) (closed 2021-02),
  [#42](https://github.com/dharple/detox/issues/42) (closed 2021-03),
  [#50](https://github.com/dharple/detox/issues/50) (closed 2021-02),
  [#52](https://github.com/dharple/detox/issues/52) (closed 2024-04).

**Evidence.** Users repeatedly could not get simple outcomes (keep spaces, ignore one char class,
lowercase names, skip folders, use a custom `.tbl`) without hand-editing translation tables and
`detoxrc` sequence blocks. `ylwhatt` in
[#78](https://github.com/dharple/detox/issues/78) (external, `question` label, closed 2021-07): "Im
not sure why the man pages are confusing me so much." Four separate external filers hit the same
"keep spaces" question across three years (#89 in 2022-01, #105 in 2023-06, #111 in 2024-02, #124 in
2025-02) -- this is the one theme whose temporal spread is genuinely multi-year and genuinely
external, not an artifact of the 2021-02 planning burst. The maintainer's own final comment on #124
says "I have had many requests of this nature", which is first-hand maintainer testimony to volume
beyond what the tracker itself shows. The three owner-filed architecture tickets (#42, #50, #52) are
the maintainer's plan to fix it, not further reports of it.

**Design implication (author's read).** Hard requirement, and the strongest-supported one in this
document. It is also consistent with the maintainer's own README framing ("the days of weighty
configuration files are behind us"). Caveat: `user_feedback_online.md` found **zero** independent
echo of this complaint outside the tracker at this depth, so the strength here is depth and
repetition within one venue, not breadth across venues.

### 2. Safe-charset disagreements -- 9 items, 7 external (**tracker-only**)

- External filers (7): [#19](https://github.com/dharple/detox/issues/19) (closed 2021-02),
  [#86](https://github.com/dharple/detox/issues/86) (closed 2021-11),
  [#89](https://github.com/dharple/detox/issues/89) (closed 2022-01),
  [#100](https://github.com/dharple/detox/issues/100) (closed 2022-12),
  [#109](https://github.com/dharple/detox/issues/109) (swept),
  [#117](https://github.com/dharple/detox/issues/117) (swept),
  [#121](https://github.com/dharple/detox/issues/121) (closed 2024-11).
- Owner-filed (2): [#29](https://github.com/dharple/detox/issues/29) (closed 2021-02),
  [#40](https://github.com/dharple/detox/issues/40) (closed 2021-02).

**Evidence.** There was no consensus on what the default "unsafe" set should be: filers wanted
hyphens preserved over underscores in specific positions (#121), diacritics kept without a trailing
underscore (#117), the `utf_8` filter to stop doing safe-filter-style substitution on brackets (#86,
#100), or `=` handled (#109). Highest external ratio of any theme (7 of 9), and the filers are
distinct people spread over 2020--2024. **Cross-theme note:** #86 (here) and
[#40](https://github.com/dharple/detox/issues/40) (theme 3) describe the _same_ underlying
scope-creep defect -- the UTF-8 filter doing safe-filter work -- from the external and maintainer
sides respectively. They are deliberately filed under different themes and cross-referenced rather
than merged, because merging would change what theme 2 and theme 3 mean and doc 00 cites both by
number. Doc 00 currently cites "#40, #86, doc 02 theme 2"; #40 is a theme 3 item here, and that
citation should read "themes 2 and 3".

**Design implication (author's read).** Nice-to-have / UX-quality, not a hard functional gap -- but
recurring across many independent filers, which is exactly the population a successor's defaults
would have to satisfy.

### 3. Unicode/UTF-8/encoding correctness bugs -- 14 items, 7 external (**corroborated**)

- External filers (7): [#14](https://github.com/dharple/detox/issues/14) (closed 2021-01),
  [#17](https://github.com/dharple/detox/issues/17) (closed 2021-02),
  [#75](https://github.com/dharple/detox/issues/75) (swept-unlabeled),
  [#106](https://github.com/dharple/detox/issues/106) (swept-unlabeled),
  [#108](https://github.com/dharple/detox/issues/108) (swept-unlabeled),
  [#116](https://github.com/dharple/detox/issues/116) (swept),
  [#140](https://github.com/dharple/detox/issues/140) (swept).
- Owner-filed (7): [#9](https://github.com/dharple/detox/issues/9) (closed 2021-02),
  [#29](https://github.com/dharple/detox/issues/29) (closed 2021-02),
  [#33](https://github.com/dharple/detox/issues/33) (closed 2021-02),
  [#40](https://github.com/dharple/detox/issues/40) (closed 2021-02),
  [#41](https://github.com/dharple/detox/issues/41) (closed 2021-02),
  [#55](https://github.com/dharple/detox/issues/55) (swept),
  [#120](https://github.com/dharple/detox/issues/120) (swept).

(#106 is added to this list at stage 3: the previous version discussed it in prose but omitted it
from the bracketed list, which is why the old header said 13.)

**Evidence.** Genuine correctness bugs (two off-by-one UTF-8 translation errors producing literal
`<C2>`/`<C3>` garbage, #14, fixed in v1.3.1), missing coverage (4-byte UTF-8 #33,
unsupported-length errors #108, fraction slash #106 re-added in 3.0.0-beta2, hidden Unicode Tags
#120, full-width forms #140), and one cross-platform inconsistency (0x202F narrow no-break space
replaced on Linux but not macOS, #116, **unresolved -- closed without a fix** in the wind-down
sweep; the earlier phrasing "still open when the repo paused" contradicted this document's own
headline stat of zero open issues and is corrected here: "open" is reserved for the GitHub state
field, which is `closed` for all 140 items).

This is the **only theme with independent external corroboration**: the #14 off-by-one bug was
caught separately in the Debian BTS (#861537), a genuinely different reporting channel converging on
the same root cause. It is also, per L2's audit, the theme not inflated by the 2021-02 planning
burst on the external side -- though note the split is an even 7/7, so half of this theme is still
maintainer intent.

**Design implication (author's read).** Hard requirement. Maps directly onto the maintainer's stated
principle that "users looking for help with their files shouldn't need to be well-versed in
character encoding"; this theme is the empirical record behind that sentence.

### 4. Cross-platform portability (Windows/macOS/BSD) -- 10 items, only 3 external (**partly corroborated**)

- External filers (3): [#73](https://github.com/dharple/detox/issues/73) (closed 2021-03),
  [#77](https://github.com/dharple/detox/issues/77) (swept),
  [#116](https://github.com/dharple/detox/issues/116) (swept).
- Owner-filed (7): [#35](https://github.com/dharple/detox/issues/35) (closed 2021-02),
  [#38](https://github.com/dharple/detox/issues/38) (closed 2021-02),
  [#58](https://github.com/dharple/detox/pull/58) (closed 2021-02),
  [#60](https://github.com/dharple/detox/issues/60) (closed 2021-02),
  [#69](https://github.com/dharple/detox/issues/69) (swept),
  [#80](https://github.com/dharple/detox/issues/80) (closed 2021-07),
  [#91](https://github.com/dharple/detox/issues/91) (closed 2022-06).

**Evidence, restated after recount.** This theme's raw count was the most misleading in the previous
version. **7 of its 10 items are the maintainer's own**, six of them ("Confirm macOS support" #35,
"Confirm FreeBSD support" #38, "Travis macos" #58, "Fix tests for macOS environment" #60, "Fix unit
tests under macOS" #69) filed inside the single 2021-02 planning burst, and #80 ("Compilation under
Windows with MSYS2 fails") self-filed in 2021-07. Only **three** items are external reports: NetBSD
`cp -n` in the test suite (#73), the MSYS2 `lstat()` build failure (#77), and the macOS 0x202F bug
(#116).

L2 flagged this and put the external count at 4, naming #73, #77, #80 and #116. That is off by one:
**#80 is `dharple`/OWNER**, so the external count is 3, not 4 -- the theme is slightly _weaker_ as
demand evidence than L2 said. What remains true and important is the _substance_: MSYS2 has no
`lstat()` and per the maintainer's research likely never will (#77), which is an architectural
constraint rather than a build-script gap; and the maintainer could not run macOS unit tests himself
(#69, #116), leaving macOS-specific bugs permanently unverifiable. Distro build friction is
independently corroborated (Debian #1080967, Launchpad LP:2079767), but the MSYS2/Windows structural
claim has **zero** independent echo.

**Design implication (author's read).** Still a hard requirement for a successor _if_ cross-platform
support is a goal -- but the argument now has to be made on the _consequence_ (an unbuildable and
untestable platform is a durable structural weakness) and not on demand volume. Doc 00 currently
cites this as "~9 portability issues"; that figure should be corrected to "10 items, 3 of them
external reports, 7 the maintainer's own test-infrastructure tickets."

### 5. Transliteration policy reversal -- 9 items, only 1 external, and **resolved** (**tracker-only**)

- External filers (1): [#99](https://github.com/dharple/detox/issues/99) (closed 2024-03).
- Owner-filed (8): [#21](https://github.com/dharple/detox/issues/21) (closed 2021-02),
  [#47](https://github.com/dharple/detox/issues/47) (closed 2021-02),
  [#48](https://github.com/dharple/detox/issues/48) (closed 2021-02),
  [#49](https://github.com/dharple/detox/issues/49) (swept),
  [#52](https://github.com/dharple/detox/issues/52) (closed 2024-04),
  [#53](https://github.com/dharple/detox/issues/53) (closed 2021-02),
  [#112](https://github.com/dharple/detox/issues/112) (closed 2024-03),
  [#113](https://github.com/dharple/detox/pull/113) (**merged** 2024-03).

**Evidence.** The arc is real and internally consistent: early demand for richer transliteration
tables (#47 Unidecode, #53 Latin Extended-B), then backlash that transliteration was too aggressive
and destroyed intentional non-ASCII (#99, German umlauts), then the v2 change making transliteration
opt-in (#21), then the v3 change removing it from the default tables entirely (#112 tracking, #113
merged). It is the one part of the roadmap that actually shipped.

**But 8 of the 9 items are the maintainer's own**, and the sole external item is the single backlash
report (#99). Neither reviewer stated this ratio; it is the sharpest instance of the
maintainer-intent-versus-user-demand conflation in the document. Combined with
`user_feedback_online.md` finding **zero** online reaction to the change in either direction --
despite it being the largest behavioral shift in the tool's history -- the honest reading is that
this theme documents **one maintainer's converged philosophy, corroborated by one user**, not a
demonstrated user consensus.

**Design implication (author's read).** The direction ("do less by default") is well-evidenced as
_the maintainer's_ conclusion and is stated in his own voice in the README. It is **not** evidenced
as broad user demand, and doc 00's P4 should not be read as resting on user demand. Per L2, P4 is
the design principle most exposed to the motivated-filer critique; this recount confirms that and
sharpens it from "tracker-only" to "tracker-only and almost entirely owner-filed."

### 6. Collision/overwrite handling -- 2 items, both external, **rejected with a substantive rationale** (**tracker-only**)

- External filers (2): [#122](https://github.com/dharple/detox/issues/122) (swept),
  [#130](https://github.com/dharple/detox/pull/130) (closed 2025-10, individually adjudicated).

**Evidence, separated into three distinct claims** (previously run together):

1. _Demand exists, and it is small:_ two independent external asks for an overwrite flag. Two. This
   is not a volume argument and must not be cited as one.
2. _The maintainer's technical objection is credible:_ his rejection of #130 is the single most
   technically substantive comment in the tracker -- overwrite could collapse N files to 1 where the
   translation table maps multiple names to the same output; `readdir()` ordering interacts badly
   with in-place renames; `S_ISREG`/`S_ISDIR` checks are required; BSD/macOS syscalls differ.
   Verified verbatim by L1, which rates this the best-sourced entry in the document. #130 is also
   one of the few high-signal items closed **before** the sweep, so its "rejected" state is genuine
   individual triage, not administrative closure. #122 by contrast was swept and never adjudicated.
3. _That a safe design exists is not established here._ The tracker shows the maintainer considered
   overwrite unsafe. It does not show anyone building a safe overwrite mechanism, and it does not
   distinguish "users want this solved safely" from "users want overwrite and would accept the
   risk."

**Design implication (author's read).** A rewrite should treat collisions as a first-class outcome.
That is a design inference drawn from claim 2, not something the tracker settled -- the previous
version's phrasing ("a hard requirement for any rewrite to solve this correctly") asserted it as
though it followed from the evidence. It belongs in doc 00 as a design choice, and doc 00 does in
fact use it that way (leaning on rejection-reasoning quality rather than demand volume), which is
the most defensible use of a low-count theme in the chain.

### 7. Undo / dry-run trust -- 1 item (**split corroboration**)

- External filers (1): [#119](https://github.com/dharple/detox/issues/119) (closed 2024-11,
  individually answered "No, there's no undo").

**Evidence.** Exactly one direct "is there undo" ask, answered no. The weight here is not issue
count: the README makes `-n`/`--dry-run` "the most important option to learn", so the maintainer
already treated irreversible renaming as the project's central risk, consistent with the #130
rejection. Corroboration is **split**, and the split matters: dry-run-as-safety-net is the single
most corroborated positive across five independent tutorial authors (putorius.net, Delightly Linux,
Gentoo Wiki, apt-upgrade.me, Mabox) per `user_feedback_online.md` -- but the specific _undo_ ask has
no online echo at all. The corroboration is for the value of preview, not for demand for undo.

**Design implication (author's read).** Preview-before-mutate is a hard requirement. Note that this
document under-cites its own best evidence: the five-tutorial corroboration lives in
`user_feedback_online.md`, not here, and doc 00 should cite that rather than #119 alone.

### 8. Packaging / build-system churn -- 15 items, only 3 external (**partly corroborated**)

- External filers (3): [#82](https://github.com/dharple/detox/issues/82) (closed 2021-08),
  [#83](https://github.com/dharple/detox/issues/83) (closed 2021-11),
  [#126](https://github.com/dharple/detox/issues/126) (closed 2025-08).
- Owner-filed (12): [#1](https://github.com/dharple/detox/issues/1) (closed 2017-03),
  [#2](https://github.com/dharple/detox/issues/2) (closed 2017-03),
  [#59](https://github.com/dharple/detox/issues/59) (closed 2021-02),
  [#65](https://github.com/dharple/detox/issues/65) (closed 2024-04),
  [#66](https://github.com/dharple/detox/issues/66) (closed 2021-02),
  [#68](https://github.com/dharple/detox/issues/68) (closed 2021-03),
  [#76](https://github.com/dharple/detox/issues/76) (swept),
  [#91](https://github.com/dharple/detox/issues/91) (closed 2022-06),
  [#92](https://github.com/dharple/detox/pull/92) (closed 2022-06),
  [#93](https://github.com/dharple/detox/pull/93) (closed 2022-06),
  [#127](https://github.com/dharple/detox/issues/127) (swept),
  [#128](https://github.com/dharple/detox/issues/128) (closed 2025-08).

**Evidence.** Ongoing autoconf/automake/CMake friction, missing files in release tarballs (#83,
#126), CI platform churn (Travis to CircleCI: #59, #91, #92, #93), checksum/signature requests
(#127). The 12-of-15 owner-filed split **confirms** the prose claim that this is maintainer/packager
pain rather than end-user pain -- this is the one theme where the recount supports the original
characterization instead of undercutting it. The three external items are all from packagers
(`eribertomota` the Debian maintainer, `slankes`, `nieder`), a distinct and more technically engaged
channel than either GitHub end users or the forums. Distro build friction is independently
corroborated (Debian #1080967, Launchpad LP:2079767), though for a different specific bug than any
listed here.

**Design implication (author's read).** "Keep it simple to build and ship" is a requirement for a
successor, but it is not a user-facing feature gap. This is why theme 8 is ranked below themes 3 and
4 despite having more raw items -- the adjustment that was previously unstated.

### 9. Wide/4-byte Unicode & CJK-adjacent coverage -- 5 items, 3 external (**tracker-only**)

- External filers (3): [#108](https://github.com/dharple/detox/issues/108) (swept-unlabeled),
  [#123](https://github.com/dharple/detox/issues/123) (swept),
  [#140](https://github.com/dharple/detox/issues/140) (swept).
- Owner-filed (2): [#33](https://github.com/dharple/detox/issues/33) (closed 2021-02),
  [#120](https://github.com/dharple/detox/issues/120) (swept).

**Evidence.** 4-byte UTF-8 gaps (#33, #108, #123 "How to remove 4-byte emoji?"), full-width/halfwidth
CJK-style forms (#140), and hidden Unicode Tags characters (#120). Two of the five are owner-filed:
#33 is a planning-burst ticket and #120 is the maintainer's own idea prompted by an Ars Technica
article on hidden-character attacks, not a user ask. L2 flagged #120 and put the external count at
4; the correct figure is **3**, because #33 is also OWNER. So this theme is "2 maintainer ideas + 3
external asks", not "5 coverage gaps".

**No issue in the full 140-item set names CJK (Chinese/Japanese/Korean) glyph handling.** The CJK
label is inferred adjacency (wide-character/4-byte support), and 4 of the 5 items were closed in the
wind-down sweep, so nothing here was individually adjudicated either.

**Design implication (author's read).** Weakest of the numbered themes. Treat as "4-byte UTF-8 must
not be silently mishandled", which is really theme 3, and drop any CJK-specific demand claim.

### 10. Sequence/CLI ergonomics -- 6 items, 3 external (**tracker-only**)

- External filers (3): [#90](https://github.com/dharple/detox/issues/90) (closed 2022-05),
  [#102](https://github.com/dharple/detox/issues/102) (closed 2023-07),
  [#110](https://github.com/dharple/detox/issues/110) (closed 2024-03).
- Owner-filed (3): [#7](https://github.com/dharple/detox/issues/7) (swept, relaying the Debian
  package maintainer), [#42](https://github.com/dharple/detox/issues/42) (closed 2021-03),
  [#62](https://github.com/dharple/detox/issues/62) (closed 2021-02).

**Evidence.** Requests for CLI-level control instead of table editing (#7 -- filed by `dharple` but
explicitly relaying Eriberto, the Debian package maintainer, so it is third-party demand arriving
through the owner's account; the maintainer called it well-aligned with his v2 vision), a `--git mv`
option (#110, declined on the substantive ground that detox uses `rename()` rather than shelling out
to `mv`), a lowercase flag (#102, not implemented natively), and confirmation that stdin/stdout pipe
mode already existed (#90, `inline-detox`). #110 and #90 were both individually adjudicated before
the sweep, so their dispositions are real.

**Design implication (author's read).** Overlaps theme 1 heavily (#7, #42, #102 are shared); a
successor that fixes theme 1 by moving control to flags largely absorbs this theme.

### Crash / memory-safety stability -- 5 items, **all 5 external** (new at stage 3, deliberately not numbered)

Theme numbering is held stable for doc 00's citations, so this cluster gets no number. It is
recorded here because it is the largest all-external cluster in the tracker and the previous version
scattered it across a single dismissive bullet.

- All external filers (5): [#11](https://github.com/dharple/detox/issues/11) (closed 2021-02, no
  recorded fix), [#56](https://github.com/dharple/detox/issues/56) ("Segfault when parsing more than
  10 files at once", `bug`, closed 2021-02),
  [#85](https://github.com/dharple/detox/issues/85) ("inline-detox -- segmentation fault", `bug`,
  swept), [#96](https://github.com/dharple/detox/issues/96) (double free on a 2TB recursive run,
  `bug`, swept), [#137](https://github.com/dharple/detox/issues/137) (`malloc(): invalid size`,
  `bug`, swept).

L2 flagged #11 and #137 as an undercount and put the total at "at least three". My own sweep of all
140 items found **five**, adding #56 and #85, and all five are `author_association: NONE` -- not one
is maintainer-filed. Three of the five (#85, #96, #137) were closed in the wind-down sweep with no
fix and no triage. This is the only cluster in the tracker with a perfect external ratio, which
makes it, by the provenance standard applied everywhere else in this document, the **cleanest demand
signal in the whole evidence base** -- and the previous version cited only one of the five, under a
heading dismissing the topic.

**Design implication (author's read).** Memory-safety defects in a C filename walker, reported
independently five times and never collectively resolved, are a hard requirement for a successor to
eliminate by construction. This is the tracker's strongest argument for a memory-safe rewrite, and
it is an argument about _defect class_, not about feature demand.

### Themes checked but with weak or no evidence

- **Symlink handling** -- **corrected at stage 3; the previous version's dismissal was wrong.** The
  previous text said "no issue directly reports a symlink bug." There is one:
  [#23](https://github.com/dharple/detox/issues/23) "Fix relative link recursion or remove support
  for it" (`bug`, closed 2021-02-22), in which the maintainer ran detox with `-r --special` over a
  `/tmp` symlink pointing at `../..` and hit unbounded recursion across his entire projects
  directory. It is a real, reproducible, high-consequence bug. It is also **filed by `dharple`
  (OWNER) about his own testing, with zero external corroboration** -- so it is evidence of a defect
  and of its blast radius, not of user demand. [#20](https://github.com/dharple/detox/issues/20)
  ("Add tests for --special", OWNER) is likewise a maintainer test-gap note. Anything downstream
  using #23 should say "maintainer-discovered, no external report, included on consequence grounds."
  The codebase does appear symlink-aware (MSYS2's missing `lstat()`, #77, implies it; detox is
  documented as reviewing "the link's name, not the linked file's name"), and no external filer ever
  requested symlink-following behavior.
- **Inode/hardlink issues**: no issue in the 140-item set addresses inodes or hardlinks. Confirmed
  absence, re-checked at stage 3 against the full item list.
- **Performance on large trees** -- **narrowed at stage 3.** Still true that **no issue asks for
  faster large-tree processing**; there is no throughput or slowness complaint anywhere in the
  tracker. What the previous version got wrong was folding #96 in here as "only one adjacent data
  point": #96 is a _crash_, not a slowness report, and it is one of five independent external crash
  reports now recorded in the crash/memory-safety cluster above. Performance-as-speed remains
  unevidenced. Stability under load is not.
- **Sequence selection ergonomics** as a complaint distinct from general config complexity: folded
  into themes 1 and 10. No issue isolates "choosing which sequence to run" as its own pain point
  separate from "how do I customize what a sequence does."

## Maintainer's Stated Future Direction

Verbatim from `README.md` (top-of-file "NOTE" section, current as of the 2026-07-31 pull):

> From the feedback I've received over the years, it's clear what direction `detox` needs to go
> in. The days of weighty configuration files are behind us, and users looking for help with their
> files shouldn't need to be well-versed in character encoding. `detox` needs to be easier to work
> with, using command-line options and a config file that lets you pre-select those options.
>
> It needs to _just work_. Period.
>
> What's equally clear is that I don't want to write another massive app in C.
>
> ... So, `detox` is paused. I hope to pick it up again at some point and rebuild it from scratch,
> in a different language, with a friendlier UI.

Additional roadmap statements found in issues (not in the README):

- **#124** (2025-08-11): "I'm thinking about how to make detox easier to use, either with a
  wrapper layer on top of the base functionality, or with a total rewrite. I'd like to move it
  toward 'it just works,' but getting there without completely destroying someone's files is a
  hard problem to solve." -- This is the clearest evidence tying the "just works" goal directly to
  the collision/overwrite-safety concern (theme 6).
- **[#7](https://github.com/dharple/detox/issues/7)** (2017-03, pre-v2): the maintainer says the
  CLI-flags-for-charset request "fits nicely in with my vision of v2, pushing all of the actual
  sequencing to the command line and away from config files and custom conversion tables" -- an
  earlier, narrower version of the same direction later restated in the README. **Filer correction
  (stage 3):** reviewer L1 reported #7 as filed by `eribertomota`. The API's `user.login` for #7 is
  `dharple` (`OWNER`); the _body_ opens "From Eriberto (the Debian package maintainer)", i.e. the
  maintainer filed it on Eriberto's behalf. The demand is genuinely third-party, but the account is
  the owner's, which is why every filer marker for #7 in this document reads "owner (relaying Debian
  pkg maintainer)" rather than plain "third party".
- **#112 / #113**: tracking issue + merged PR explicitly documenting the v3 philosophy shift from
  "transliterate everything" to "only handle truly problematic characters" -- the one part of the
  roadmap that was actually shipped, not just stated.
- No commit messages, discussion posts, or wiki pages beyond the above were found stating further
  roadmap detail; CHANGELOG.md only documents shipped changes, not intentions.

## Confidence & Sources

**High confidence (directly verified via GitHub API/tarball, quoted or closely paraphrased above):**

- Full issue/PR inventory: 140 items, all closed, 0 open (`/repos/dharple/detox` metadata +
  `/issues?state=all` across 3 pages, cross-checked).
- README.md roadmap text (exact quote, current file as of pull date).
- CHANGELOG.md version history back through 2.0.0-beta1.
- Comment threads fetched in full for ~24 issues/PRs (bodies for all 140 were fetched; comments
  fetched for the highest-signal ~24 by comment count plus the theme-relevant PRs #130/#136/#15/#58).
- Merged vs. unmerged status for all 13 PRs found (`/pulls/{n}.merged` field).
- **Added at stage 3:** `user.login`, `author_association`, `created_at`, `closed_at`, labels and
  title for **all 140 items**, pulled complete via `GET /search/issues?q=repo:dharple/detox`
  (2 pages, 2026-07-31). Every filer marker, every theme count, the 73/140 OWNER figure, the 48
  February-2021 owner tickets, the 34-item labeled sweep and the 8 unlabeled same-day closures are
  computed directly from that pull, not from any reviewer's sample.

**Medium confidence / inferred, not directly quoted:**

- Theme _bucketing_ remains my classification, done by keyword search + manual reading -- a
  different tagger could move borderline issues (e.g. #86, #100) between "safe-charset disagreement"
  and "encoding correctness." What is no longer a judgement call, as of stage 3, is the **counts**:
  each theme's item count is the exact length of its own printed list, and the external/owner split
  is a mechanical `author_association` lookup.
- Whether issues #9 (TAB in filenames) and #75 (unconvertible PUA character) and #108 (unicode
  length) were ever actually fixed -- their comment threads show investigation but no explicit
  "fixed in vX.Y.Z" confirmation was found; CHANGELOG.md does not mention them by number.

**Explicitly NOT verified / could not confirm:**

- Comments were **not** fetched for roughly 116 of the 140 issues (only bodies were pulled for
  those). It's possible additional maintainer roadmap statements or user pushback exist in comment
  threads not reviewed here, particularly on the lower-comment-count build/packaging issues, which
  were deliberately deprioritized as lower-signal for the "what did users want" question.
- No direct evidence of CJK (Chinese/Japanese/Korean)-specific user requests was found -- the
  "i18n/CJK" theme in the prompt is only weakly supported by adjacent 4-byte-UTF-8/full-width-form
  issues (#33, #108, #120, #123, #140), not by any issue naming CJK scripts explicitly -- and of
  those five, #33 and #120 are the maintainer's own, leaving three external asks.
- No inode/hardlink issues were found at all; reported above as a confirmed absence, not an
  oversight.
- No mailing list, Discord, forum, or non-GitHub discussion was checked here -- this document is
  scoped to the GitHub tracker plus in-repo docs. Cross-venue evidence lives in
  `user_feedback_online.md` and docs 20--23, and the corroboration tags above are sourced from it.
- **Stage-3 rate limit.** The unauthenticated core REST limit (60/hr) was already exhausted by the
  three stage-2 reviewers before adjudication began (`/rate_limit` reported `core: 0/60`), so no
  per-issue comment thread could be re-pulled at stage 3. The 10 remaining `search` calls were spent
  on the two full-inventory pulls described above, which is why stage-3 verification is complete for
  **metadata** (filer, dates, labels, state, titles for all 140) and **zero** for comment-body
  re-verification. Quote fidelity therefore still rests on L1's stage-2 verbatim checks, which
  covered 18 comment threads including every high-rhetorical-weight quote (#130, #140, #116, #110,
  #99, #90, #136, #7).

## Review record (stage 3)

Three independent stage-2 reviewers examined this document under different lenses: **L1** citation
audit (verified ~36 issues individually against the API), **L2** analytical validity / bias /
temporal validity, **L3** clarity, structure and missing links. Every finding is adjudicated below,
including the ones rejected.

### Verification coverage at stage 3

- **Complete (all 140 items):** `user.login`, `author_association`, `created_at`, `closed_at`,
  labels, title -- two `GET /search/issues?q=repo:dharple/detox` pages, 2026-07-31. Every count,
  filer marker, state marker and sweep classification in this document is recomputed from that pull.
- **Not re-verified at stage 3:** any comment body. The unauthenticated core REST limit was already
  at `0/60` when adjudication began (exhausted by the stage-2 reviewers), so per-issue and per-PR
  comment fetches were impossible. Quote fidelity rests on L1's 18 verbatim comment-thread checks;
  PR `merged` flags rest on L1's per-PR check of all 13 PRs. Neither was re-pulled.
- **Where the limit stopped me:** I could not independently re-read the #124 thread to confirm the
  splice fix a previous pass applied, nor re-read #130's rejection comment, nor check whether the
  three swept crash bugs (#85, #96, #137) carry any reply beyond the wind-down template. All three
  are marked as resting on L1.

### Findings

| Finding                                                                                                                                                  | Reviewer  | Verdict                                       | Action or reason                                                                                                                                                                                                                                                                                                                                                                           |
| -------------------------------------------------------------------------------------------------------------------------------------------------------- | --------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 34 issues bulk-closed in one ~50-min window 2026-07-12 with a templated comment and `closed-with-detox`; "closed" is an administrative sweep, not triage | L1        | **ACCEPTED**                                  | Confirmed exactly (34 labeled + 8 unlabeled same-day = 42). Dedicated section added before the table; `swept` markers on every table row and theme citation; explicitly stated that the sweep supports neither "considered and rejected" nor "live open demand".                                                                                                                           |
| Theme header counts disagree with their own bracketed lists by 1-3 items                                                                                 | L3, L1    | **ACCEPTED**                                  | Recounted every theme myself from its own list. Real counts: T1=17, T2=9, T3=14, T4=10, T5=9, T6=2, T7=1, T8=15, T9=5, T10=6. All headers now carry the exact list length. (T3 is 14 not L1/L3's 13 because #106 was discussed in prose but missing from the list; added.)                                                                                                                 |
| "Evidence weight" is presented as a raw count but silently applies an unstated user-vs-maintainer-pain adjustment                                        | L3        | **ACCEPTED**                                  | The adjustment is real (T8 has 15 items yet ranks below T3's 14 and T4's 10). Weighting method now stated up front as three separate published figures -- raw count, external count, adjustment -- with the T8 case named explicitly as the worked example.                                                                                                                                |
| Theme 1 nonetheless holds up as the highest-weight theme on recount                                                                                      | L1        | **ACCEPTED**                                  | Independently confirmed and reconciled with L3's arithmetic complaint: both are right. T1 leads on raw count (17), on external count (12 vs. 7 next), and still leads after subtracting every item it shares with T10 (14/12). Stated in a dedicated reconciliation paragraph.                                                                                                             |
| Themes double-count issues (#7, #102 in both T1 and T10) without disclosing overlap                                                                      | L1        | **ACCEPTED**                                  | Full overlap inventory published; counts declared explicitly non-additive and not a partition; T1's standing shown to survive removal of all shared items.                                                                                                                                                                                                                                 |
| Theme counts conflate the maintainer's own OWNER-filed tickets with independent user reports                                                             | L2        | **ACCEPTED**                                  | The single most important finding in the three reviews. Every theme now splits external filers from owner-filed, with counts. Verified for all 140 items, not a sample: 73 OWNER, 62 NONE, 5 CONTRIBUTOR.                                                                                                                                                                                  |
| Theme 4 external count: L2 says 6 of 10 owner-filed, 4 external (#73, #77, #80, #116)                                                                    | L2        | **MODIFIED**                                  | Direction right, arithmetic wrong, and wrong in the _unfavourable_ direction. **#80 is `dharple`/OWNER**, so theme 4 is 7 owner / **3 external**, not 6/4. Corrected in place, with L2's figure named so the discrepancy is auditable.                                                                                                                                                     |
| Theme 1 drops from "~15+" to ~14 external once owner tickets are removed (L2 names #42, #50, #52)                                                        | L2        | **MODIFIED**                                  | L2 missed two: #7 and #29 are also OWNER. Theme 1 is 17 items, 5 owner-filed, **12 external** -- not 14. Still comfortably the highest-weight theme, so the conclusion stands with a corrected number.                                                                                                                                                                                     |
| Theme 9: #120 is owner-filed, so 4 of 5 are external                                                                                                     | L2        | **MODIFIED**                                  | #33 is _also_ OWNER. Theme 9 is 2 owner / **3 external**. Relabelled "2 maintainer ideas + 3 external asks".                                                                                                                                                                                                                                                                               |
| Theme 5 (transliteration) is the design principle most exposed to motivated-filer bias                                                                   | L2        | **ACCEPTED and strengthened**                 | Neither reviewer published the ratio. It is **8 of 9 owner-filed, 1 external** (#99) -- the sharpest instance of the conflation anywhere in the document. Theme now states it documents one maintainer's converged philosophy corroborated by one user, not user consensus.                                                                                                                |
| Issue state is not surfaced next to citations                                                                                                            | L3        | **ACCEPTED**                                  | Every theme citation now carries state inline: `swept`, `swept-unlabeled`, or an individual close date. Sweep-versus-triage distinction stated in the table preamble and per theme; #130 and #110 explicitly flagged as genuinely adjudicated pre-sweep.                                                                                                                                   |
| ~28 load-bearing prose citations are bare issue numbers with no link                                                                                     | L3        | **ACCEPTED**                                  | Every issue number in every theme list is now a link. Spot-checked existing table links: all sampled `issues/` vs `pull/` paths match the Type column (L3's own sample of ~15 found no mismatch; #58, #92, #93, #113, #130 confirmed as `/pull/`).                                                                                                                                         |
| Observation and interpretation are typographically indistinguishable in theme prose                                                                      | L3        | **ACCEPTED**                                  | Every theme split into **Evidence** and **Design implication (author's read)**, with the latter labelled as opinion.                                                                                                                                                                                                                                                                       |
| "Theme rankings are my classification" caveat is buried 150 lines below the rankings                                                                     | L3        | **ACCEPTED**                                  | Moved inline to the top of the theme section (kept in Confidence & Sources too, narrowed there to _bucketing_ since the counts are now mechanical).                                                                                                                                                                                                                                        |
| Theme 6 slides from "the maintainer said no" to "therefore a rewrite must solve it"                                                                      | L2        | **ACCEPTED**                                  | Theme 6 split into three numbered claims: demand exists and is tiny (2 items); the technical objection is credible; a safe design is _not_ established here. The design inference is labelled as belonging to doc 00.                                                                                                                                                                      |
| #116 described as "still open when the repo paused" contradicts the doc's own 0-open-issues stat                                                         | L2        | **ACCEPTED**                                  | Reworded to "unresolved -- closed without a fix"; "open" now reserved for the GitHub state field throughout.                                                                                                                                                                                                                                                                               |
| Symlink dismissal is wrong: #23 exists, and is OWNER-filed with zero external corroboration                                                              | L2        | **ACCEPTED**                                  | The old text ("no issue directly reports a symlink bug") was factually false. #23 now described in full, with its OWNER provenance, and the prescription that downstream use say "maintainer-discovered, no external report, included on consequence grounds". #20 likewise flagged as a maintainer test-gap note.                                                                         |
| Crash-bug undercount: #11 and #137 are external crash reports the doc omits ("at least three")                                                           | L2, L1    | **MODIFIED**                                  | Right, but an undercount of its own. My sweep of all 140 items found **five** external crash reports -- #11, #56, #85, #96, #137 -- adding #56 ("Segfault when parsing more than 10 files at once") and #85 ("inline-detox -- segmentation fault"). All five `NONE`, none owner-filed. Recorded as a new unnumbered cluster; it is the only perfect-external-ratio cluster in the tracker. |
| "Performance on large trees" dismissal is wrong                                                                                                          | L2        | **MODIFIED**                                  | Half right. The dismissal of _speed_ demand is correct and retained -- no issue anywhere asks for faster processing. What was wrong was filing #96 here as "one adjacent data point" when it is a crash. Narrowed to distinguish performance-as-speed (unevidenced) from stability-under-load (five reports).                                                                              |
| #7 filed by `eribertomota`                                                                                                                               | L1        | **REJECTED**                                  | Directly contradicted by the API: `user.login` for #7 is `dharple`, `author_association: OWNER`. L1 appears to have read the body ("From Eriberto (the Debian package maintainer)") as the filer. The demand is third-party but the account is the owner's; recorded as "owner (relaying Debian pkg maintainer)" and the discrepancy documented in place.                                  |
| Merge #40 and #86 into one theme, since they describe the same scope-creep defect                                                                        | L3        | **MODIFIED**                                  | Substance accepted, remedy rejected. Merging would change what themes 2 and 3 mean, and doc 00 cites both by number -- the brief requires stable numbering. Cross-referenced in both directions instead, with a note that doc 00's "#40, #86, doc 02 theme 2" citation should read "themes 2 and 3".                                                                                       |
| Absolute tracker volume is small; theme rankings are not demand estimates                                                                                | L2        | **ACCEPTED**                                  | Added to the "what this evidence base can and cannot support" section as one of three structural limits, with the explicit warning that "17 issues" is not a market size.                                                                                                                                                                                                                  |
| Add per-theme corroborated / tracker-only marking; state the motivated-filer limit near the top                                                          | brief, L2 | **ACCEPTED**                                  | Dedicated section added near the top; per-theme tags in the summary table and in each heading. Tags sourced from `user_feedback_online.md`, whose divergence section and top-problems table I read directly to confirm L2's mapping. Only theme 3 is independently corroborated; theme 7 is split; 4 and 8 partial; the remaining six are tracker-only.                                    |
| Present-tense theme prose reads as live demand on an archived tracker                                                                                    | L3        | **ACCEPTED**                                  | Theme prose shifted to past tense ("Users repeatedly could not get...", "There was no consensus...").                                                                                                                                                                                                                                                                                      |
| #78's quote is unattributed and unlinked; attribution convention is inconsistent                                                                         | L3        | **ACCEPTED**                                  | Now `ylwhatt` in linked #78 with label and close date. Owner quotes are anchored to `dharple` via the stated convention in the table preamble.                                                                                                                                                                                                                                             |
| #124's table quote splices two comments; "I don't have enough time" is not verbatim                                                                      | L1        | **ACCEPTED** (already applied)                | Fixed by an earlier pass to the verbatim "I can't commit to any enhancements. I don't have the time..." Not re-verified at stage 3 (core rate limit exhausted); rests on L1.                                                                                                                                                                                                               |
| "#130/#133/#136 rejected explicitly, two on safety grounds" -- only #130 has a safety rationale                                                          | L1, L3    | **ACCEPTED** (already applied)                | Fixed by an earlier pass; the PR-tally correction paragraph names #133 as a documentation PR swept with no rationale, and #15/#58 as the owner's own throwaway branches rather than rejected contributions.                                                                                                                                                                                |
| #137 and #11 missing from the document entirely                                                                                                          | L1        | **ACCEPTED** (already applied, then extended) | Table rows added by an earlier pass; extended here into the five-item crash cluster.                                                                                                                                                                                                                                                                                                       |
| Dangling `[#124 dup]` markdown link renders as literal text                                                                                              | L3        | **ACCEPTED** (already applied)                | Replaced with plain text "(duplicate of #124)".                                                                                                                                                                                                                                                                                                                                            |
| PR summary cites #18, #107, #15, #58, #133 with no description or link                                                                                   | L3        | **ACCEPTED** (already applied)                | One-line description + link now given for all 13 PRs in the tally.                                                                                                                                                                                                                                                                                                                         |
| A third of citations appear only in theme prose, never in the table, so they get less scrutiny                                                           | L1        | **MODIFIED**                                  | Adding ~30 table rows would double the table for little gain. Instead every theme-only citation now carries the same three facts the table's State column gives -- link, filer, state -- computed from the full-inventory pull, so they are no longer less-verified than table rows; they are less _narrated_. Disclosed rather than eliminated.                                           |
| PR merge/reject tally not independently re-verified                                                                                                      | L2        | **ACCEPTED as a limitation**                  | Cannot be fixed at stage 3: core rate limit was exhausted. Already flagged in the tally ("rest on stage-2 reviewer L1's per-PR `merged` field check, not on a stage-3 re-pull") and repeated in the coverage statement above.                                                                                                                                                              |
| Doc 00 should cite corrected counts (theme 1 "~15", theme 4 "~9 portability issues")                                                                     | L2, L3    | **ACCEPTED, out of scope to fix**             | The corrected figures a downstream doc should cite are stated in place in themes 1 and 4 ("17 items, 12 external"; "10 items, 3 external"). Doc 00 is not edited here -- the brief restricts edits to this file -- so this is a flagged, not a discharged, action.                                                                                                                         |

**Net effect on this evidence base's credibility.** It shifts substantially, in both directions.
Downward on _provenance_: the tracker is 52% the maintainer talking to himself, one theme is 8/9
owner-filed, another is 7/10, and only one of ten themes has any corroboration outside this single
venue. Downward on _finality_: 42 of 140 items were closed administratively on the archive date, so
a third of the tracker carries no disposition at all. Upward on _citation accuracy_: L1 found no
fabricated issue numbers, no fabricated usernames and no invented quotes across ~36 individually
checked issues, and the two quotes carrying the most downstream weight are verbatim. Upward on
_precision_: every count in this document is now mechanically derived from a complete pull of all 140
items rather than estimated. And the document's central downstream claim -- theme 1 as the
highest-weight theme, backing doc 00's P1 -- survives every recount performed here. What changed is
that the reader can now see which parts are user demand and which are one maintainer's plan.
