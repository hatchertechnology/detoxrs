---
date: 2026-07-31
venue_scope: >
  Community discussion threads about the `detox` filename-cleaning CLI tool
  (github.com/dharple/detox). Targeted: Reddit (r/linux, r/commandline,
  r/linuxquestions, r/DataHoarder, r/sysadmin, r/archlinux, r/debian),
  Hacker News, Lobste.rs, Linux distro forums (Arch BBS, Ubuntu Forums,
  LinuxQuestions.org, Debian/openSUSE lists and forums, Mabox, Linux.org,
  FedoraForum), Usenet/mailing-list archives, and blog-comment sections.
  Excludes: the diet/health "detox" sense, and the unrelated npm/PyPI
  packages named "detox" (Wix's mobile E2E test framework, `detoxpy`, etc.).
queries_run:
  - "detox filename utility reddit"
  - "site:reddit.com detox filenames linux"
  - "site:reddit.com/r/commandline detox"
  - "site:reddit.com/r/linuxquestions detox filenames"
  - "detox filenames site:reddit.com"
  - "\"detox\" \"use\" reddit rename files linux praise recommend"
  - "detox linux tool review \"I use\" workflow reddit sysadmin"
  - "\"detox -r\" filenames spaces underscores"
  - "detox rename files linux forum"
  - "site:news.ycombinator.com detox rename files"
  - "Hacker News Algolia detox filename tool" + hn.algolia.com API queries (query=detox, tags=comment; query=detox filename)
  - "detox utility DataHoarder rename filenames"
  - "site:ubuntuforums.org detox filenames"
  - "detox filename cron script rename photos music collection"
  - "linuxquestions.org detox sanitize filenames"
  - "\"detox\" filenames site:linuxquestions.org"
  - "detox 2 I'm back linuxquestions detox code review"
  - "bash detox html references linuxquestions"
  - "detox filenames arch linux bbs forum"
  - "site:lobste.rs detox"
  - "fedora forum OR opensuse forum detox filenames"
  - "opensuse forum detox \"transfers from Windows\" filenames proper"
  - "\"detox\" filename tool bug encoding mangled unicode issue"
  - "\"detox\" \"find\" \"-print0\" \"xargs\" filenames"
  - "detox vs rename vs pyrenamer vs bulk rename utility linux comparison"
  - "detox filenames photoprism OR nextcloud OR jellyfin forum discourse"
  - "detox filenames \"scanned documents\" OR \"scanner\" cron"
  - "\"detox\" filename tool \"renamed my\" OR \"deleted\" OR \"overwrote\" complaint"
  - "detox filenames alias .bashrc script wrapper \"detox -r\""
  - "debian-user mailing list detox filenames"
  - "detox package Debian bug report rename filenames"
source_count: >
  11 pages fetched and read directly, itemized: 6 that yielded cited evidence
  (Arch BBS 2009, Arch BBS 2005, apt-upgrade.me, Lobste.rs, Linux.org, Mabox
  forum) and 5 fetched specifically to confirm a negative result (an openSUSE
  Tumbleweed thread and a dev.to listicle, both search false positives with no
  detox mention; putorius.net and delightlylinux.wordpress.com, checked for
  reader comments and found to have none; and the HN threads Algolia pointed at,
  where the attributed comments were not present). Plus blocked/offline venues
  documented below (linuxquestions.org 403, ubuntuforums.org DNS failure,
  narkive.com 503), from which nothing is quoted.

---

# `detox` — online community discussion research

Scope note up front: `detox` is a ~25-year-old, extremely narrow utility (rename files, strip bad
characters). It generates almost no dedicated discussion on major venues — most "hits" are
tutorial/blog articles restating the man page, not discussion. This document reports what was
actually found, including the negative results, rather than padding with tutorial content dressed
up as community sentiment.

## Confirmed real workflows and use cases

### Migrating/importing files from Windows and macOS onto Linux

- **Arch Linux Forums, "[solved] repairing filename encoding"** (2009), user `manouchk`, resolved
  2009-05-10: after asking how to fix filename charset problems, the OP reports success combining
  `convmv` (encoding conversion) with `detox` (character stripping):

  > "I'm solving now using convmv and detox to get rid of the non-ascii caracters!"

  Posted commands:

  ```
  convmv -f iso-8859-1 -t utf8 --replace --notest -r ~/Documents/*
  convmv -f utf8 -t iso-8859-1 --fixdouble --replace --notest -r ~/Documents/*
  detox -s utf_8 -r ~/Documents
  ```

  This is a real, dated, verbatim workflow: two-step encoding repair (convmv) followed by detox as
  the cleanup pass, scoped with `-s utf_8` and run recursively.
  [https://bbs.archlinux.org/viewtopic.php?id=63393](https://bbs.archlinux.org/viewtopic.php?id=63393)

- **Arch Linux Forums, "[REQUEST] detox, txt2regex, sprog"** (2005-06-29), AUR package request
  thread. OP `zsoltika` (Budapest, Hungary) states the actual job:

  > "I have to work a lot with 1000s of textfiles with different tools, so I used to use detox and
  > txt2regex back my good — now more better — old Gentoo days."

  and later clarifies the concrete pain point:

  > "sometimes working on 1000's of files with accents in their names and spaces, and so on..."

  A skeptical reply from `Gullible Jones` initially argued detox was unnecessary ("Just use
  quotation marks for files with nonstandard names when you're working from the command line"),
  then reversed after the accented-filename point landed:

  > "Ah... Accents... I forgot about those completely. Yep, I can see why it's useful now..."

  This is the clearest "comparison where someone considered NOT using detox, then changed their
  mind" found in this research: the counter-argument was "just quote your filenames," defeated by
  the batch/scale case (thousands of files) and non-ASCII characters, where manual quoting doesn't
  scale.
  [https://bbs.archlinux.org/viewtopic.php?id=13387](https://bbs.archlinux.org/viewtopic.php?id=13387)

### Backup pipeline breaking on a single non-compliant filename (sharp edge, then fix)

- **apt-upgrade.me blog, "Cleaning Up Filenames on Linux with detox — The Tiny Tool That Saved My
  Backups"** (dated 2025-07-24, per URL/publish date; personal blog post, not a forum thread, but
  a first-person account with a specific, dated incident). The author's `rdiff-backup` job
  crashed mid-run because of one file with a non-compliant name synced in from macOS ("Unicode
  quirks and special symbols that Linux and the backup tool couldn't process"). Fix: insert detox
  as a preprocessing step before the backup runs:

  ```
  detox -r -s utf_8 -s iso8859_1 -v /data
  rdiff-backup /data /mnt/backup/data
  ```

  Author's own words on the outcome:

  > "my rdiff-backup runs have been smooth as butter — even when syncing messy files from macOS,
  > USB sticks, or synced cloud folders."

  This is a concrete sharp-edge-then-mitigation story: an untamed filename took down an entire
  backup run, and detox-as-preprocessing-step is the pattern adopted afterward. No comments were
  present on the post at fetch time.
  [https://www.apt-upgrade.me/2025/07/24/detox-the-tiny-tool-that-saved-my-backups/](https://www.apt-upgrade.me/2025/07/24/%F0%9F%A7%BC-cleaning-up-filenames-on-linux-with-detox-the-tiny-tool-that-saved-my-backups/)

### Bulk media/document cleanup (from search snippets only — could not fetch primary source)

Ubuntu Forums search results (site itself is now offline, see below) surfaced titles/snippets
indicating real workflows, but the pages could not be independently fetched to verify verbatim
text, so these are reported as snippet-only, lower-confidence:

- A thread "Vanishing mp3s" apparently involving detox and a music library where files with slash
  characters in names caused problems.
- A thread "Clean names of all pdf files" apparently using a `find`+`xargs`+`detox` pipeline of
  the form `find . -iname '*pdf' -print0 | xargs -0 detox -v`.
- A thread "Automatic duplication of image library with resize" apparently using
  `detox -r /music/transferred_from_elsewhere/` style invocations, floated as a cron candidate.

These three are flagged explicitly as **unverified** — the search engine's own summarization of
snippets, not text read directly off the page (the venue is dead; see below).

### Terminal-tools listicles endorsing detox (light "why I use it")

- **Hacker News comment thread on "New(ish) command line tools" (jvns.ca blog post)**, HN item
  [31009313](https://news.ycombinator.com/item?id=31009313), dated around 2022-04. A search-engine
  summary (via Algolia) attributed a comment to user `ckunte` recommending detox for "sanitising
  filenames." **Could not independently verify this quote on the live HN page** — a direct fetch
  of the thread did not surface any comment mentioning detox. Reporting as unconfirmed; the
  Algolia-summarized version should not be treated as a verified quote.
- Similarly, Algolia search surfaced apparent HN mentions in threads about "Managing my personal
  knowledge base" (`fit2rule`, periodic detox runs "for consistent-naming for regexes") and "Notes
  for new Make users" (a Makefile target suggestion). **Neither was verifiable on direct fetch of
  the linked HN pages** — the comments were not found in the fetched thread content. These are
  reported as unconfirmed leads, not evidence.

### Lobste.rs — feature request framed as a critique

- **Lobste.rs, "What are your favourite pieces of software?"**
  [https://lobste.rs/s/d0ptcu/what_are_your_favourite_pieces_software](https://lobste.rs/s/d0ptcu/what_are_your_favourite_pieces_software),
  comment dated 2023-03-13, user listed as `inactive-user` (account since deleted/renamed) in the
  fetched content:

  > "https://github.com/dharple/detox — would be good if it was a library, so that one could have
  > one config, and have detoxing on the command line and in fileselector dialogs."

  This is a genuine, verifiable piece of DISLIKE-adjacent feedback: not a bug, but a structural
  gap — detox ships only as a standalone CLI, so there's no shared config between it and a
  GUI file-manager's rename dialog. The commenter still likes the tool enough to list it as a
  favorite; the complaint is about integration surface, not correctness.

### Forum posts that are single, unanswered informational posts (no real discussion)

- **Linux.org, "I just learned a new command to remove the spaces in all my filenames-detox"**,
  posted 2021-11-19 by user `LiquidSky`. Confirmed via direct fetch: this is a single post
  introducing the tool (`pamac install detox`, `-r` behavior, example `shopping list.txt` →
  `shopping_list.txt`) with **no replies**. Not a discussion — a solo tip post.
  [https://www.linux.org/threads/i-just-learned-a-new-command-to-remove-the-spaces-in-all-my-filenames-detox.47876/](https://www.linux.org/threads/i-just-learned-a-new-command-to-remove-the-spaces-in-all-my-filenames-detox.47876/)
- **Mabox Linux Forum, "CLI: detox - Clean up filenames"**, posted 2021-11-19 by user `LiquidSky`
  (same author, cross-posted content). Confirmed via direct fetch: single post, no replies found.
  [https://forum.maboxlinux.org/t/cli-detox-clean-up-filenames/721](https://forum.maboxlinux.org/t/cli-detox-clean-up-filenames/721)

## Version history note surfaced in search (unverified detail, flagged)

Search-engine summarization (not a page I could fetch and quote directly, since LinuxQuestions.org
blocks fetching — see below) repeatedly surfaced the claim that detox v2/v3 **deliberately walked
back aggressive transliteration** of non-ASCII into ASCII, in favor of only touching "truly
problematic" characters, partly because modern Unix systems assume UTF-8 now. This lines up with
the GitHub project's own documented history but I could not independently confirm this was
discussed by a real user (as opposed to being restated project changelog text) in any of the three
LinuxQuestions.org threads found (`detox - sanitize filenames`, `detox 2 - I'm back`, `bash detox
html references`) because LinuxQuestions.org's anti-bot protection blocked every fetch attempt
(see below). Treat this as a documented project fact, not a verified community discussion point.

## Sharp edges search — explicit dry hole

Multiple targeted searches for data loss, unexpected renames, or bug complaints (`"detox" filename
tool "renamed my" OR "deleted" OR "overwrote" complaint`, `"detox" filename tool bug encoding
mangled unicode issue`) turned up **no user-reported incident of detox destroying data or silently
overwriting a file**. The tool's own documented behavior (declining to rename if the target name
already exists, and Python reimplementations appending a `-1` suffix instead of overwriting) came
up in search-engine summaries as the reason such complaints don't seem to exist — but this is a
documentation claim, not a confirmed community sentiment, since it wasn't traced to a specific
verified user quote.

## Searches that found nothing

- **Reddit** (r/linux, r/commandline, r/linuxquestions, r/DataHoarder, r/sysadmin, r/archlinux,
  r/debian): zero substantive threads found across seven distinct `site:reddit.com` /
  `site:reddit.com/r/...` queries plus generic "reddit detox filenames" phrasing. Every result
  either returned Wikipedia's drug-detoxification disambiguation page, PyPI packages unrelated to
  this tool, or no Reddit URLs at all. **This is a real, notable finding**: despite being exactly
  the kind of "small useful CLI tool" that r/commandline and r/linux typically discuss, `detox`
  does not appear to have generated a single retrievable Reddit thread.
- **Hacker News** (direct `site:news.ycombinator.com` search and the Algolia API): no HN story ever
  submitted about detox itself. The only appearances are as a passing mention inside comments on
  unrelated "list of command-line tools" posts, and even those could not be verified by fetching
  the actual thread content (see above) — the tool text search matched but the live page content
  did not corroborate the match.
- **Ubuntu Forums**: the domain (`ubuntuforums.org`) is entirely offline — `WebFetch` and a
  sandboxed `curl` both returned DNS resolution failures (`ENOTFOUND`), consistent with the site
  having shut down. Several promising thread titles surfaced via search snippets (Windows filename
  compatibility, vanishing MP3s, PDF cleanup, image library dedup) but none could be read directly,
  so none of their content is reported as verified above.
- **LinuxQuestions.org**: three relevant threads were identified by title/URL (`[SOLVED] detox -
sanitize filenames`, 2016-11-11; `detox 2 - I'm back`; `bash detox html references`), but the
  site returned HTTP 403 to every fetch attempt (both `WebFetch` and a sandboxed `curl` with a
  browser user-agent), indicating active bot-blocking (Cloudflare or similar). Content could not be
  verified; nothing from these threads is quoted above.
- **Debian mailing lists / bug tracker**: no specific `debian-user` thread or bug report about
  detox surfaced in search; results returned only the Debian package listing pages (packages.debian.org),
  which are not discussion.
- **openSUSE forums / Usenet archive**: one plausible thread ("openSUSE 12.3 - detox is absent," an
  archived Usenet/mailing-list mirror on narkive.com) was found by search but returned HTTP 503 on
  fetch and could not be read. A separate, unrelated openSUSE Tumbleweed file-transfer forum thread
  was fetched directly and confirmed to contain **no mention of detox at all** — it was a false
  positive from search snippet matching, not a real detox discussion.
- **Fedora Forum**: no thread specifically about detox found; only generic forum activity-stream
  pages surfaced.
- **PhotoPrism / Nextcloud / Jellyfin self-hosting community discourse**: searched explicitly for
  detox being used as a pre-import filename sanitizer for media-server libraries; no hits — the
  self-hosted-media community does not appear to reference detox in its public forums/GitHub
  Discussions.
- **Scanned-document / OCR workflow discussion**: no thread pairing detox with scanner output or
  OCR pipelines was found.
- **DEV Community ("dev.to")**: one plausible "poor man's CLI tools" listicle with comments was
  fetched directly; confirmed it does not mention detox anywhere in the article or comments.
- **Comment sections on tutorial blogs**: fetched two of the most common detox how-to articles
  directly (putorius.net, delightlylinux.wordpress.com) specifically to check for reader comments
  beyond the article text. Neither had any comments at fetch time.

## Method notes / caveats

- `WebFetch` was blocked (HTTP 403/503, or DNS failure) on: linuxquestions.org (403, twice, plus a
  sandboxed curl with a spoofed user-agent also got 403 — this looks like active Cloudflare bot
  protection, not a fluke), ubuntuforums.org (DNS failure — site appears fully offline), and
  narkive.com (503). `web.archive.org` fetches were categorically refused by the tool itself
  ("Claude Code is unable to fetch from web.archive.org"), so no Wayback Machine fallback was
  available for the blocked pages.
- Several Algolia/HN-search-summarized "hits" (the jvns.ca tools-list comment, the personal-
  knowledge-base comment, the Makefile-target comment) could not be reproduced when the actual HN
  thread page was fetched and read directly. Per the honesty rules for this task, those are
  reported as unconfirmed leads rather than quoted claims.
- No fabricated URLs, quotes, or usernames are included above; every quote is attributed to a
  source that was either fetched directly (Arch BBS ×2, apt-upgrade.me, Lobste.rs, Linux.org,
  Mabox, opensuse forum false-positive check, dev.to false-positive check) and reproduced verbatim
  as returned by that fetch, or explicitly flagged as unverified/snippet-only.

---

## Review record (stage 3)

Adjudication of the three stage-3 reviews (L1 link verification, L2 evidential honesty, L3
structure). Rejections are included with reasons.

| Finding (reviewer)                                                                                                                                                                                             | Verdict               | Action or reason                                                                                                                                                                                                                                                                                                       |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| All 6 cited links in this document resolve 200; 3 of the load-bearing claims here (Arch BBS 2009 commands, Arch BBS 2005 reversal quote, Lobste.rs library comment) were re-fetched and verified verbatim (L1) | **Accept, no change** | Confirmed. Nothing to fix.                                                                                                                                                                                                                                                                                             |
| No overstatement found in this document; the data-loss dry hole (§"Sharp edges search") is correctly left as a null result rather than converted into "detox never destroys data" (L2)                         | **Accept, no change** | This is the behavior the corpus wants. Explicitly preserved — see the next row, where doc 22 had violated it.                                                                                                                                                                                                          |
| Doc 22 asserted users "run detox without preview, and lose data", contradicting this document's explicit dry hole (L2, CRITICAL)                                                                               | **Accept**            | Fixed in doc 22, not here. This document was correct and is now cited by doc 22's retraction as the authority for the null result.                                                                                                                                                                                     |
| Front matter should use the same field names and a body-verifiable source count as docs 21/23 (L3)                                                                                                             | **Accept**            | Field names already matched (`venue_scope`, `queries_run`, `source_count`). The count "11" was unverifiable as written, so it is now itemized: 6 sources yielding cited evidence + 5 fetched to confirm a negative, with the blocked/offline venues listed separately.                                                 |
| Internal inconsistency found during adjudication, not by any reviewer                                                                                                                                          | **Accept**            | §"Searches that found nothing" said "fetched three of the most common detox how-to articles" but named only two (putorius.net, delightlylinux). Corrected to two.                                                                                                                                                      |
| Final sentence ("No fabricated URLs, quotes, or usernames are included above...") reads as defensive; move to an appendix or delete (L3, MINOR)                                                                | **Reject**            | This is a provenance attestation, not clutter. In a corpus where the reviewers' single most valuable finding was that no quotes were fabricated, the sentence that itemizes which sources were fetched directly and which are flagged unverified is load-bearing. Deleting it removes accountability to save one line. |
| This document is not redundant with the synthesis and should be kept as a primary source (L3)                                                                                                                  | **Accept, no change** | Agreed on the reasoning given: the verbatim exchanges (the Arch BBS skeptic reversing on accented filenames) and the itemized negative results are what let a reader judge signal against noise. The synthesis abstracts them away by design and cannot substitute.                                                    |
