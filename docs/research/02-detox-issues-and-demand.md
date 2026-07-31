# detox (dharple/detox) -- Issue Tracker & PR Mining: User Demand Signal

Source: `https://github.com/dharple/detox`, archived (paused by maintainer). Pulled via
unauthenticated GitHub REST API (`/repos/dharple/detox/issues`, `/pulls`, `/issues/{n}/comments`)
plus the repo tarball (`README.md`, `CHANGELOG.md`) on 2026-07-31.

**Repo stats at time of pull:** archived = true, open issues = 0, 140 issues+PRs total (all closed -- the tracker has zero open items), 446 stars, 24 forks, last push 2026-07-12.

This is not a partial sample: the `/issues` endpoint with `state=all&per_page=100` across 3 pages
returned all 140 items, and `open_issues_count` from the repo metadata confirms 0 are open. Every
issue/PR below is drawn from that full set.

## Issue/PR Table

Only items carrying a demand signal are listed (bug reports with reproducible complaints, feature
requests, and PRs). Pure build/CI/packaging chores with no user-facing ask are omitted from the
table but folded into the packaging theme below.

| # | Type | State | Title | Ask | Maintainer response | Resolution |
|---|------|-------|-------|-----|---------------------|------------|
| [#140](https://github.com/dharple/detox/issues/140) | Issue | closed | replace full-width characters | Handle Unicode full-width/halfwidth forms (used by SingleFile browser addon for Windows compat) | "I don't plan on working on this... putting detox on hold" -- suggested `rename` as workaround | Not implemented, closed on pause |
| [#136](https://github.com/dharple/detox/pull/136) | PR | closed (unmerged) | new sequence: fat . for exFAT/DOS SD card limits | Add a built-in sequence for exFAT/DOS filename limits (aria2c has no Windows-safe flag) | "At this time, I don't plan on merging this... putting detox on hold" | Rejected, not merged |
| [#130](https://github.com/dharple/detox/pull/130) | PR | closed (unmerged) | Add option to overwrite/replace existing files | `-F` flag to let detox overwrite collisions instead of refusing | Detailed technical rejection: risk of collapsing N files to 1 file if sequences/tables map multiple names to the same output, `readdir()` ordering hazards, needs `S_ISREG`/`S_ISDIR` checks, BSD/macOS syscall differences | Not merged -- "I don't want to be responsible for destroying other people's data" |
| [#124](https://github.com/dharple/detox/issues/124) | Issue | closed | option to ignore spaces and fix only wrong chars | Let users keep spaces, replace only truly unsafe chars, without hand-editing `.tbl`/`.rc` | Gave manual `safe.tbl`/`detoxrc` edit workaround; later: "I don't have enough time... thinking about a wrapper layer or total rewrite... I'd like to move it toward 'it just works'" | Not implemented; explicit roadmap statement given here |
| [#122](https://github.com/dharple/detox/issues/122) | Issue | closed | Optional new flag needed (= `-o` overwrite) | Same overwrite ask as #130, different reporter | No maintainer reply beyond community | Not implemented |
| [#121](https://github.com/dharple/detox/issues/121) | Issue | closed | Why detox removes underscores surrounding hyphens? | Preserve `_-_` separator convention in music filenames; disagreement with default hyphen/underscore collapsing | Pointed to custom-sequence workaround | Not implemented natively |
| [#119](https://github.com/dharple/detox/issues/119) | Issue | closed | Is there any way to undo changes? | Undo/rollback support | "No, there's no undo." | Confirmed absent, closed |
| [#118](https://github.com/dharple/detox/issues/118) | Issue | closed | Add ability to set a max filename length | Truncate to a max length | Already exists (`max_length` filter) -- doc/discoverability gap | Resolved via existing feature |
| [#117](https://github.com/dharple/detox/issues/117) | Issue | closed | Don't append underscore after deaccented characters | Diacritic stripping without trailing `_` (e.g. "łódź"→"lodz" not "l_o_dz_") | Community workaround only | Not fixed |
| [#116](https://github.com/dharple/detox/issues/116) | Issue | closed | 0x202F (narrow no-break space) not replaced on macOS | Cross-platform Unicode handling inconsistency (works on Linux, fails on macOS) | Added a unit test, admitted "I'm not currently able to run unit tests on macOS," bug reproduced by user on 3.0.1; closed on pause | Unresolved -- "I don't plan on working any further... putting detox on hold" |
| [#113](https://github.com/dharple/detox/pull/113) | PR | **merged** | Remove transliteration | Strip transliteration from main tables, filter only unsafe chars | Merged as part of v3 rewrite | Merged -- became v3.0.0 |
| [#112](https://github.com/dharple/detox/issues/112) | Issue | closed | Shift detox from transliteration to handling problematic characters | Tracking ticket for the v3 philosophy shift | "Ticket for tracking creation of v3 and shift in focus" | Implemented (v3) |
| [#111](https://github.com/dharple/detox/issues/111) / [#124 dup] | Issue | closed | option to ignore spaces and fix only wrong chars (duplicate ask) | Same as #124 | No substantive fix | Not implemented |
| [#110](https://github.com/dharple/detox/issues/110) | Issue | closed | add `--git` option to use `git mv` instead of `mv` | Git-aware rename so history/blame survive | Declined: "I won't be doing this... uses `rename()`", suggested `git add -A` workaround | Not implemented |
| [#109](https://github.com/dharple/detox/issues/109) | Issue | closed | Equal sign not tamed | `=` in filenames breaks some shell contexts | -- | Not resolved in table shown |
| [#108](https://github.com/dharple/detox/issues/108) | Issue | closed | unsupported unicode length | Files silently skipped on long/invalid Unicode sequences, no way to force/bypass | -- | Unresolved |
| [#106](https://github.com/dharple/detox/issues/106) | Issue | closed | Handle 2044 (Fraction Slash) | Re-add a specific Unicode translation | Investigated, asked for repro details | Re-added in 3.0.0-beta2 per CHANGELOG |
| [#105](https://github.com/dharple/detox/issues/105) / [#89](https://github.com/dharple/detox/issues/89) | Issue | closed | Space handling confusing / editing space out breaks parsing | Users repeatedly can't figure out how to keep spaces; editing `safe.tbl` to remove the space rule causes adjacent-char corruption (`abc def.xyz`→`abc ef.xyz`) | Doc pointer / no fix for the corruption | Config-file complexity theme; corruption bug in #89 not resolved in visible thread |
| [#102](https://github.com/dharple/detox/issues/102) | Issue | closed | add a flag to lowercase filenames | Uppercase→lowercase conversion flag | -- | Not implemented as native flag (community `find`+`-s lower` workaround exists per #95) |
| [#101](https://github.com/dharple/detox/pull/101) | PR | **merged** | look for detoxrc in $XDG_CONFIG_HOME | XDG base-dir compliance | Merged | Merged into v2.0.0 |
| [#99](https://github.com/dharple/detox/issues/99) | Issue | closed | Please provide a way to retain German Umlaute ÄÜÖäüö | Transliteration was too aggressive, destroyed intentional non-ASCII | "Version 3 of detox removes all of the transliteration, so this should no longer be a problem" | Resolved by v3 default-behavior change |
| [#98](https://github.com/dharple/detox/issues/98) | Issue | closed | Replace + remove punctuation in one command | More flexible one-shot character mapping | -- | Not directly resolved |
| [#97](https://github.com/dharple/detox/pull/97) | PR | **merged** | Fixed umlaut conversion (Ü→Ue not UE) | Casing bug in umlaut expansion | Merged | Merged |
| [#96](https://github.com/dharple/detox/issues/96) | Issue | closed | double free/corruption with custom sequence on 2TB recursive run | Memory-safety crash on a large directory tree with a custom sequence | -- | Unresolved in visible thread; only large-scale stability bug found |
| [#95](https://github.com/dharple/detox/issues/95) | Issue | closed | Ignore folders? | Target only files, skip directories | "no way to specifically target directories or files with detox itself" -- `find`+`-exec` workaround | Confirmed limitation, not fixed |
| [#94](https://github.com/dharple/detox/issues/94) | Issue | closed | How to ignore macOS `Icon␍` files? | Exclude specific macOS metadata files | Community-only | Not resolved via native config |
| [#93](https://github.com/dharple/detox/pull/93) | PR | **merged** | CircleCI project setup | CI migration off Travis | Merged | Merged |
| [#92](https://github.com/dharple/detox/pull/92) | PR | **merged** | Add `-Werror` to `AC_CHECK_CFLAG` | Build hardening | Merged | Merged |
| [#91](https://github.com/dharple/detox/issues/91) | Issue | closed | Compiling under CircleCI is broken | CI breakage | Fixed via #92 | Resolved |
| [#90](https://github.com/dharple/detox/issues/90) | Issue | closed | Operate on stdin list, output on stdout | Pipe-friendly batch mode | "We already have some support for that... `detox --inline` or `inline-detox`" | Resolved -- feature already existed |
| [#89](https://github.com/dharple/detox/issues/89) | Issue | closed | Not replacing space eats up the next character | Editing out the space rule corrupts adjacent chars | -- | Bug, unresolved in thread |
| [#88](https://github.com/dharple/detox/pull/88) | PR | **merged** | Simple maintenance improvements | Misc cleanup | Merged | Merged |
| [#87](https://github.com/dharple/detox/issues/87) | Issue | closed | How can I delete certain chars instead of replacing them? | Delete vs. replace semantics unclear | Maintainer confirmed empty-string replacement works, found + acknowledged 2 related bugs (`remove_trailing` doesn't strip trailing `_` without extension; `defaultdefault` string-duplication bug) | Partially resolved (documented), 2 sub-bugs opened |
| [#86](https://github.com/dharple/detox/issues/86) | Issue | closed | `utf_8-only` doing more than transliteration (touching brackets) | Filter scope-creep: UTF-8 filter also doing "safe"-filter-like substitutions | -- | Confirmed as design confusion, addressed conceptually by v3's filter-scope cleanup (#40/#112) |
| [#84](https://github.com/dharple/detox/issues/84) | Issue | closed | Is there a way to pass a custom `.tbl` file to a filter? | Custom translation table support, unclear syntax | Community answered (`filename "path/to.tbl"` in sequence block) | Resolved via docs/community, config syntax confirmed painful |
| [#83](https://github.com/dharple/detox/issues/83) | Issue | closed | Released tarballs missing CHANGELOG/LICENSE/THANKS | Packaging completeness | -- | Packaging/distro theme |
| [#81](https://github.com/dharple/detox/issues/81) | Issue | closed | "Value too large for defined data type" on Raspbian armv7l | Crash on ARM32/Raspbian building large files | Reproduced, root-caused to autoconf large-file-support flag, fixed | Fixed in v1.4.4 |
| [#80](https://github.com/dharple/detox/issues/80) | Issue | closed | Compilation under Windows with MSYS2 fails | Windows build breakage | -- | Windows portability theme |
| [#77](https://github.com/dharple/detox/issues/77) | Issue | closed | Compiling error on windows 10 (msys2/mingw64) | `lstat()` missing on MSYS2 | Provided patch, diagnosed msys2 lacks `lstat()` permanently, needed autoconf check + fallback to `stat()` | Unresolved -- "At this time, I don't plan on working any further on this... putting detox on hold" |
| [#75](https://github.com/dharple/detox/issues/75) | Issue | closed | unconvertible files? | Odd Unicode (0xF022 PUA) causing unclear errors | Investigated, asked for repro info, no closure recorded | Unresolved/stalled |
| [#74](https://github.com/dharple/detox/issues/74) | Issue | closed | inline-detox fails if last char of stdin isn't a newline | Streaming/pipe edge case | Fixed | Fixed in 2.0.0-beta2 |
| [#73](https://github.com/dharple/detox/issues/73) | Issue | closed | NetBSD's `cp` doesn't support `-n` | BSD portability in test suite | -- | BSD portability theme |
| [#69](https://github.com/dharple/detox/issues/69) | Issue | closed | Fix unit tests under macOS | macOS test infra gap | -- | macOS portability theme |
| [#59](https://github.com/dharple/detox/issues/59) | Issue | closed | Find a replacement for Travis | CI migration | Led to CircleCI (#93) | Resolved |
| [#55](https://github.com/dharple/detox/issues/55) | Issue | closed | `max_length` filter chops UTF-8 chars | Multi-byte-unsafe truncation | -- | UTF-8/i18n theme |
| [#53](https://github.com/dharple/detox/issues/53) | Issue | closed | Update transliterations through Latin Extended-B using Unicode docs | Broaden transliteration tables | Superseded by v3 removing transliteration | Superseded |
| [#47](https://github.com/dharple/detox/issues/47) | Issue | closed | Look into using Text::Unidecode's tables | Better transliteration source data | Added `unidecode.tbl` in 2.0.0-beta1 | Implemented, later mooted by v3 |
| [#42](https://github.com/dharple/detox/issues/42) | Issue | closed | Refactor config_file_spoof, add other sequences | Config/sequence architecture cleanup | -- | Config complexity theme |
| [#40](https://github.com/dharple/detox/issues/40) | Issue | closed | UTF-8 filter behaves like the safe filter | Filter responsibilities conflated (0x20–0x3F get "safe"-ified inside the UTF-8 filter) | -- | Design confusion theme, fed into v3 rewrite |
| [#33](https://github.com/dharple/detox/issues/33) | Issue | closed | Add support for 4-byte UTF-8 | Missing coverage for 4-byte sequences (emoji, some CJK/supplementary planes) | -- | i18n/CJK-adjacent theme |
| [#29](https://github.com/dharple/detox/issues/29) | Issue | closed | Safe filter behaves differently when table is missing | Inconsistent defaulting: missing table strips UTF-8, table-based safe leaves it alone | -- | Config/encoding-consistency theme |
| [#21](https://github.com/dharple/detox/issues/21) | Issue | closed | Update the default runtime behavior | Track: make default = safe + wipeup only, move iso8859_1/utf_8 to opt-in transliteration | Implemented | Implemented -- became v2 default |
| [#19](https://github.com/dharple/detox/issues/19) | Issue | closed | Empty default "eats up" valid characters | Custom safe-table + empty default strips wanted chars unpredictably | -- | Config complexity / safe-charset theme |
| [#17](https://github.com/dharple/detox/issues/17) | Issue | closed | Detox doesn't handle filenames with newlines | Newlines in filenames not neutralized by default | Fixed by adding `0x0A`/`0x0D` to `safe.tbl`; user later found it hadn't reached their distro's shipped table | Fixed in v1.4.0, packaging-lag friction visible |
| [#14](https://github.com/dharple/detox/issues/14) | Issue | closed | Malformed UTF-8 when no default char set -- fails to "fall through" | UTF-8 off-by-one translation bugs corrupting filenames (produced literal `<C2>`/`<C3>` artifacts) | Root-caused two off-by-one errors, fixed, verified with Debian maintainer | Fixed in v1.3.1 |
| [#9](https://github.com/dharple/detox/issues/9) | Issue | closed | safe filter mishandles TAB in filenames | Tabs not neutralized | -- | Unresolved in visible thread |
| [#7](https://github.com/dharple/detox/issues/7) | Issue | closed | Specify character set from the command line | CLI flags (`-c`, `-d`) to add/delete chars without editing tables -- filed by the Debian package maintainer | "This fits nicely in with my vision of v2, pushing all of the actual sequencing to the command line and away from config files" | Explicit roadmap statement; not fully implemented as literal `-c`/`-d` flags |

PRs merged (8 of 13 PRs found): #18, #88, #92, #93, #97, #101, #107, #113.
PRs closed unmerged (5): #15, #58, #130, #133, #136 -- all of #130/#133/#136 rejected explicitly by
the maintainer (two on safety grounds, all three post-2025 ones closed with the "putting detox on
hold" message).

## Theme Synthesis (ranked by evidence weight)

Evidence weight = number of distinct issues/PRs/reporters touching the theme, from the full
140-item set, not just the table above.

### 1. Config-file / sequence-syntax complexity -- **highest weight** (~15+ issues: #7, #19, #29, #42, #50, #52, #84, #89, #94, #95, #102, #105, #111, #118, #121, #122, #124)
Users repeatedly cannot get simple outcomes (keep spaces, ignore one char class, lowercase names,
skip folders, use a custom `.tbl`) without hand-editing translation tables and `detoxrc` sequence
blocks whose syntax the man pages don't make intuitive (#78 "Im not sure why the man pages are
confusing me so much" reinforces this). Several users hit the same "keep spaces" question years
apart (#89, #105, #111, #124) -- the maintainer's own final comment on #124 confirms this is the
single most-requested change ("I have had many requests of this nature"). **Hard requirement** per
the maintainer's own README framing ("the days of weighty configuration files are behind us").

### 2. Safe-charset disagreements -- high weight (~8 issues: #19, #29, #40, #86, #89, #100, #109, #117, #121)
No consensus on what the default "unsafe" set should be: users want to keep hyphens vs.
underscores in specific positions (#121), keep diacritics without trailing underscore (#117), stop
`utf_8` filter from doing safe-filter-style substitution on brackets (#86, #100), or handle `=`
(#109). This is a nice-to-have/UX-quality theme rather than a hard functional gap, but recurring
across many independent users.

### 3. Unicode/UTF-8/encoding correctness bugs -- high weight (~10 issues: #9, #14, #17, #29, #33, #40, #41, #55, #75, #108, #116, #120, #140)
Includes genuine correctness bugs (off-by-one UTF-8 translation producing `<C2>`/`<C3>` garbage,
#14), missing coverage (4-byte UTF-8 #33, unsupported-length errors #108, fraction slash #106,
Unicode Tags/hidden chars #120, full-width forms #140), and a cross-platform inconsistency (0x202F
narrow no-break space works on Linux but not macOS, #116, still open when the repo paused). Directly
maps to the maintainer's stated principle that "users looking for help with their files shouldn't
need to be well-versed in character encoding" -- this theme is the empirical evidence behind that
statement. **Hard requirement.**

### 4. Cross-platform portability (Windows/macOS/BSD) -- high weight (~9 issues: #35, #38, #58, #60, #69, #73, #77, #80, #91, #116)
Windows/MSYS2 builds break repeatedly and for a structural reason: MSYS2 has no `lstat()` and,
per the maintainer's research, likely never will (#77) -- a real architectural constraint, not just
a build-script gap. macOS unit tests can't even be run by the maintainer himself (#69, #116),
leaving macOS-specific bugs permanently unverified. BSD variants have their own `cp`/toolchain quirks
(#73). This is a durable structural weakness, not a one-off -- **effectively a hard requirement for
any successor tool** if cross-platform support is a goal.

### 5. Transliteration policy reversal -- high weight, but **resolved** (~9 issues: #21, #47, #48, #49, #52, #53, #99, #112, #113)
A full arc: early demand for richer transliteration tables (#47 Unidecode, #53 Latin Extended-B),
followed by user backlash that transliteration was too aggressive and destructive (#99, German
umlauts), leading to the v2 change making transliteration opt-in (#21) and the v3 change removing
it from the default tables entirely (#112/#113, merged). This theme is resolved by the project's
own history and directly explains the "no more weighty config files... it needs to just work"
philosophy -- the maintainer converged on doing less by default, not more.

### 6. Collision/overwrite handling -- medium weight, **explicitly rejected on safety grounds** (#122, #130 [PR])
Two independent asks for an overwrite flag. The maintainer's rejection of PR #130 is the single
most technically substantive comment in the whole tracker: he flags that overwrite could
collapse N files down to 1 if the translation table maps multiple names to the same output, that
`readdir()` ordering interacts badly with in-place renames, that `S_ISREG`/`S_ISDIR` checks are
required, and that BSD/macOS syscalls behave differently. This reads as a considered design
constraint, not neglect -- **a hard requirement for any rewrite to solve this correctly**, since it's
the main blocker cited for adding the single most-requested unsafe feature.

### 7. Undo / dry-run trust -- medium weight (#119 undo denied; README's own emphasis on `-n`/`--dry-run` as "the most important option to learn")
Only one direct "is there undo" ask (#119, answered "no"), but the README's framing of `--dry-run`
as the first thing to learn signals the maintainer already treats irreversible renaming as the
project's central risk -- consistent with the overwrite rejection above. Low issue count but high
maintainer-attention weight; treat as a hard requirement (preview before mutate) for a rewrite.

### 8. Packaging / build-system churn -- medium weight (~12 issues, mostly maintenance: #1, #2, #59, #65, #66, #68, #76, #82, #83, #91, #92, #93, #126, #127, #128)
Ongoing autoconf/automake/CMake friction, missing files in release tarballs (#83, #126), CI
platform churn (Travis→CircleCI, #58/#59/#91/#93), checksum/signature requests (#127). This is
maintainer/packager pain more than end-user pain -- a "keep it simple to build and ship" requirement
for any successor, but not a user-facing feature gap.

### 9. Wide/4-byte Unicode & CJK-adjacent coverage -- medium-low weight (#33, #108, #120, #123, #140)
4-byte UTF-8 (emoji, supplementary planes) support gaps (#33, #108, #123 "how to remove 4-byte
emoji"), full-width/halfwidth CJK-style forms (#140), and hidden Unicode Tags characters (#120).
No issue in the full 140-item set explicitly asks about CJK (Chinese/Japanese/Korean) glyph
handling by name -- this is inferred adjacency (wide-character/4-byte support), not a directly
verified CJK-specific demand. Flagging this gap explicitly per the confidence note below.

### 10. Sequence/CLI ergonomics -- medium-low weight (#7, #42, #62, #90, #102, #110)
Requests for CLI-level control instead of table editing (#7, filed by the Debian maintainer, which
the project's own maintainer called well-aligned with his v2 vision), a `--git`-mv option
(rejected -- detox uses `rename()`, not shell `mv`), a lowercase flag (#102, not implemented
natively), and confirmation that stdin/stdout pipe mode already exists (#90, `inline-detox`).

### Themes checked but with weak or no evidence
- **Symlink handling**: only tangential -- msys2's lack of `lstat()` (#77) implies symlink-awareness
  exists in the codebase (detox is described as reviewing "the link's name, not the linked file's
  name") but no issue directly reports a symlink bug or requests symlink-following behavior.
- **Inode/hardlink issues**: no issue found addressing inodes or hardlinks at all.
- **Performance on large trees**: only one adjacent data point -- #96, a memory-corruption crash
  ("double free or corruption") on a 2TB recursive run with a custom sequence. This is a
  correctness/stability bug triggered at scale, not a reported slowness/throughput complaint. No
  issue asks for faster large-tree processing.
- **Sequence selection ergonomics** as a distinct complaint (beyond general config complexity) -- folded into theme 1 and 10 above; no issue isolates "choosing which sequence to run" as its own
  pain point separate from "how do I customize what a sequence does."

## Maintainer's Stated Future Direction

Verbatim from `README.md` (top-of-file "NOTE" section, current as of the 2026-07-31 pull):

> From the feedback I've received over the years, it's clear what direction `detox` needs to go
> in. The days of weighty configuration files are behind us, and users looking for help with their
> files shouldn't need to be well-versed in character encoding. `detox` needs to be easier to work
> with, using command-line options and a config file that lets you pre-select those options.
>
> It needs to *just work*. Period.
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
- **#7** (undated, pre-v2, filed by the Debian package maintainer Eriberto): the maintainer says
  the CLI-flags-for-charset request "fits nicely in with my vision of v2, pushing all of the
  actual sequencing to the command line and away from config files and custom conversion tables" -- an earlier, narrower version of the same direction later restated in the README.
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

**Medium confidence / inferred, not directly quoted:**
- Theme rankings ("high/medium/low weight") are my classification of issue counts per theme, done
  by keyword search + manual reading -- a different tagger could bucket a few borderline issues
  (e.g. #86, #100) differently between "safe-charset disagreement" and "encoding correctness."
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
  issues (#33, #108, #123, #140), not by any issue naming CJK scripts explicitly.
- No inode/hardlink issues were found at all; reported above as a confirmed absence, not an
  oversight.
- No mailing list, Discord, forum, or non-GitHub discussion was checked -- this research is scoped
  to the GitHub tracker plus in-repo docs only, per the task instructions.
- Rate limit (60 req/hr unauthenticated) was respected; ~46 of 60 requests remained unused at the
  end of this session, so the limit was not the constraint on coverage -- the constraint was a
  deliberate choice to prioritize high-comment-count issues over exhaustive comment-thread reading.
