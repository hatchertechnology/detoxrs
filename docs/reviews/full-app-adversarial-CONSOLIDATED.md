# Full-app adversarial review — CONSOLIDATED

**Subject:** detoxrs @ `a144fe9`
**Consolidator:** adjudicating seven independent reviewers (opus-1/2/3, haiku-1/2/3/4)
**Date:** 2026-08-03
**Method:** every finding ranked HIGH or CRITICAL below was independently re-reproduced by
the consolidator in an isolated `git archive HEAD | tar -x` copy under the session
scratchpad. Nothing under `crates/` in the live repository was modified. Three throwaway
mutation copies (`mut`, `mut2`, `mut3`) were made in the scratchpad only.

Baseline re-established by the consolidator: live tree clean (`git diff HEAD -- crates/`
empty), isolated suite green, **166 tests, 0 failures** across 11 targets. Note that this
contradicts haiku-4's reported "173 tests" — see §6.

**This is a report, not a remediation.** No production code was changed. No commits.

---

## 1. Executive summary

detoxrs is **not safe to use on data you cannot afford to lose the names of**, and it is
**not safe to ship**. The commit is fine to push as a development commit — the defects are
already in `a144fe9`, and pushing changes nothing — but it must not be tagged, released, or
recommended to a user until C-1 through C-7 are fixed.

The tool's engineering quality is genuinely high in the places it was designed hardest: the
no-clobber rename, the dirfd pin, the intent-before-rename protocol, the journal record trust
boundary, and the pure-core transform's safety closure and idempotence all survived serious
adversarial probing by three independent reviewers and hold up. The failures are concentrated
in the seams _between_ those good pieces — where a correct core is fed by an uncorrected
input, or where a correct guarantee is asserted about the wrong unit.

The single worst defect is **C-1**: a filename containing `\` is renamed by `-x` and then
refused by `undo` forever, because `is_plain_basename` rejects a character that `transform`
itself rewrites. The journal record is well-formed, durable, and permanently unusable. The
run reports `2 renamed, 0 failed`, exit 0; the undo reports `1 reverted, 0 refused` while
one name is gone for good. This is the exact failure the entire design exists to prevent,
and it is caused by one clause with no test behind it.

Beyond that, four independent mechanisms each let an ordinary, non-adversarial invocation
either destroy information or half-apply: a spelling-based ordering key renames a parent
before its contents (C-2); a byte budget mis-applied to a UTF-16-limited filesystem truncates
legal CJK filenames and collides them (C-3); the collision numberer emits destinations the
pipeline will rename again (C-4); and an invisible character in front of a dotfile silently
un-hides it (C-5). None of these need an attacker.

Finally: **the preview cannot be trusted as a safety control.** It escapes only Unicode `Cc`.
Bidi overrides, zero-width characters, Tags, `U+2028`/`U+2029` and `Zs` spaces all reach the
terminal raw — in a tool that cites CVE-2021-42574 as the reason its own stage 4 exists
(C-7). Two reviewers reported this area as PASS; both were wrong, and one of them printed
the raw bytes in its own transcript without noticing.

**Two of the seven reviews are substantially void.** haiku-3 and haiku-4 returned near-total
PASS in exactly the areas where the Opus reviewers, working the same code, found reproducible
defects. Their PASS verdicts do not survive audit: several are contradicted by their own
stated evidence, one describes behaviour the code does not have, and all of their
test-run-derived claims were made in a working tree that another reviewer was actively
mutating. See §6.

---

## 2. Findings table

Ordered by severity. Confidence: **CONFIRMED-BY-ME** = re-reproduced by the consolidator in
an isolated copy; **CONFIRMED-BY-REVIEWER** = reproduced by a reviewer using an isolated
copy, accepted without independent repro; **PLAUSIBLE** = reasoned from code, not triggered;
**REFUTED** = determined wrong.

| ID       | Severity | Title                                                                                     | File:line                                           | Reported by          | Confidence              |
| -------- | -------- | ----------------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------- | ----------------------- |
| **C-1**  | CRITICAL | A name containing `\` is renamed by `-x` and can never be undone; reported as success     | `detoxrs/src/journal.rs:433`                        | O3-1                 | CONFIRMED-BY-ME         |
| **C-2**  | HIGH     | Order safety keyed on path _spelling_: a parent is renamed before its contents            | `detoxrs-core/src/plan.rs:414`                      | O2-1 (+O2-6)         | CONFIRMED-BY-ME         |
| **C-3**  | HIGH     | 255-_byte_ budget truncates legal APFS names, collides them, and never says so            | `detoxrs-core/src/policy.rs:101`, `plan.rs:355`     | O2-2, O2-5, O3-8     | CONFIRMED-BY-ME         |
| **C-4**  | HIGH     | `numbered()` emits destinations that are not fixed points: the tool renames its output    | `detoxrs-core/src/plan.rs:593`                      | O1-1                 | CONFIRMED-BY-ME         |
| **C-5**  | HIGH     | An invisible character before a dotfile's dots destroys its dotfile status                | `detoxrs-core/src/pipeline.rs:196`                  | O1-2                 | CONFIRMED-BY-ME         |
| **C-6**  | HIGH     | A same-directory hardlink makes detoxrs rename an entry the user never named              | `detoxrs/src/walk.rs:172`                           | O1-3                 | CONFIRMED-BY-ME         |
| **C-7**  | HIGH     | The preview escapes only `Cc`: bidi, zero-width, `Zl`/`Zp`/`Zs` reach the terminal raw    | `detoxrs/src/report.rs:380`                         | O1-4, O1-8           | CONFIRMED-BY-ME         |
| **C-8**  | MEDIUM   | A run that could not do what it was asked exits 0                                         | `detoxrs/src/apply.rs:76`, `walk.rs:240`            | O2-3                 | CONFIRMED-BY-ME         |
| **C-9**  | MEDIUM   | Cleaning a tree silently breaks relative symlinks inside it, and reports success          | `detoxrs/src/apply.rs:204`                          | O2-4                 | CONFIRMED-BY-ME         |
| **C-10** | MEDIUM   | Journal _filenames_ are wholly unvalidated: overflow panic, and wrong-batch undo          | `detoxrs/src/journal.rs:149`, `:101`, `:537`        | O3-3, O3-4           | CONFIRMED-BY-ME         |
| **C-11** | MEDIUM   | `undo`'s tally hides items dropped before the apply loop: "0 refused" for lost names      | `detoxrs/src/main.rs:270`                           | O3-2                 | CONFIRMED-BY-ME         |
| **C-12** | MEDIUM   | Collision numbering is blind past `-2` for non-recursive args, and blames a false race    | `detoxrs/src/walk.rs:290`                           | O1-5                 | CONFIRMED-BY-ME         |
| **C-13** | MEDIUM   | The stage-13 convergence bound is free: 3 → 1 keeps all 166 tests green                   | `detoxrs-core/src/pipeline.rs:58`                   | O1-7, H2-2           | CONFIRMED-BY-ME         |
| **C-14** | MEDIUM   | `detoxrs *` is quadratic: one full `read_dir` + an `lstat` per entry, per argument        | `detoxrs/src/walk.rs:172`                           | O1-6                 | CONFIRMED-BY-ME         |
| **C-15** | MEDIUM   | `--json`, the declared stable contract, omits the batch id and journal path               | `detoxrs/src/report.rs:297`                         | O3-6                 | CONFIRMED-BY-ME         |
| **C-16** | LOW      | The Order-safety property test cannot fail for C-2; `Entry::depth`'s claim is untested    | `detoxrs-core/tests/prop_plan.rs:288`               | O2-6                 | CONFIRMED-BY-REVIEWER   |
| **C-17** | LOW      | `undo --last` run twice re-applies the renames and calls the redo "reverted"              | `detoxrs/src/main.rs:328`                           | O2-9                 | CONFIRMED-BY-ME         |
| **C-18** | LOW      | The journal is created world-readable and holds every renamed absolute path               | `detoxrs/src/journal.rs:93`, `:103`                 | O3-5                 | CONFIRMED-BY-ME         |
| **C-19** | LOW      | `RenameErr` renders "rename failed" where no rename was attempted; `ENOTDIR` unmapped     | `detoxrs/src/fsops.rs:83`, `:317`                   | O2-8                 | CONFIRMED-BY-REVIEWER   |
| **C-20** | LOW      | Any `ident_at` error is read as "destination vacant" on the atomic and demoted rungs      | `detoxrs/src/apply.rs:187`, `fsops/fallback.rs:243` | O1-10                | PLAUSIBLE               |
| **C-21** | LOW      | Allocator respell `all()` → `any()` survives the suite (coverage gap, not a clobber)      | `detoxrs-core/src/plan.rs:548`                      | H2-3 (downgraded)    | CONFIRMED-BY-REVIEWER   |
| **C-22** | LOW      | `apply` never verifies the opened directory's identity; `open_dir` has no `O_NOFOLLOW`    | `detoxrs/src/apply.rs:169`, `fsops.rs:300`          | O2-7                 | PLAUSIBLE               |
| **C-23** | LOW      | The sibling-chain assertion keys on `dir` spelling; both collision layers use `dir_ident` | `detoxrs-core/src/plan.rs:484`                      | O1-9                 | PLAUSIBLE               |
| **C-24** | LOW      | A batch whose journal could not be closed still exits 0                                   | `detoxrs/src/main.rs:141`                           | O2-10                | PLAUSIBLE               |
| **C-25** | LOW      | A torn final `intent` degrades "name the interrupted item" to a generic warning           | `detoxrs/src/journal.rs:345`                        | O3-7                 | CONFIRMED-BY-REVIEWER   |
| **C-26** | LOW      | `split_extension`'s 4-byte inner-extension boundary is untested (`<= 4` → `< 4` survives) | `detoxrs-core/src/truncate.rs:119`                  | H2-1                 | CONFIRMED-BY-REVIEWER   |
| **C-27** | LOW      | `-999` is never exercised (`..=` → `..` survives)                                         | `detoxrs-core/src/plan.rs:561`                      | H2-4                 | CONFIRMED-BY-REVIEWER   |
| **C-28** | LOW      | `undo --last` with no batches exits 2 while `undo --list` exits 0                         | `detoxrs/src/main.rs:343`                           | H1-2                 | CONFIRMED-BY-ME         |
| **C-29** | LOW      | Help text omits the `$HOME/.local/state` fallback; `-v` is a count documented as binary   | `detoxrs/src/cli.rs:27`, `:65`                      | H1-1, H1-4           | CONFIRMED-BY-REVIEWER   |
| **R-1**  | REFUTED  | `NotConverged` on names that converge in one iteration                                    | —                                                   | opus-2 (observation) | REFUTED (contamination) |
| **R-2**  | REFUTED  | `assertion failed: is_invisible('\u{061c}')`; `no_pre_existing_clobber` proptest failure  | —                                                   | opus-3 (observation) | REFUTED (contamination) |
| **R-3**  | REFUTED  | "Terminal Injection — PASS, all control characters properly escaped"                      | —                                                   | haiku-3              | REFUTED (see C-7)       |
| **R-4**  | REFUTED  | "Directory ordering: correct (children before parents) — PASS"                            | —                                                   | haiku-4              | REFUTED (see C-2)       |
| **R-5**  | REFUTED  | "Error propagation is correct (exit codes match specification) — PASS"                    | —                                                   | haiku-4, haiku-1     | REFUTED (see C-8)       |
| **R-6**  | REFUTED  | "Identity recheck at apply time … numbered the destination to `a_b-2.txt`"                | —                                                   | haiku-4              | REFUTED (misattributed) |
| **R-7**  | REFUTED  | "No security hardening is required"; "173 tests pass"                                     | —                                                   | haiku-3, haiku-4     | REFUTED                 |

---

## 3. Detail per finding

### C-1 — CRITICAL — a name containing `\` is renamed and can never be undone

`crates/detoxrs/src/journal.rs:433` (`is_plain_basename`) against
`crates/detoxrs-core/src/classes.rs:22` (separator class).
Reported as **O3-1**. Reporting half is **C-11** (O3-2).

`\` is in the separator class, so `transform` rewrites it and `-x` renames every name
containing one. `is_plain_basename`, which gates every `intent` record at replay time,
rejects any `from`/`to` whose bytes contain `\`. The tool therefore renames a class of names
that its own journal refuses to restore. The record is well-formed and durable; `undo`
rejects it identically on every future attempt. There is no second path to recovery.

Reproduced by the consolidator, isolated copy, verbatim output:

```
$ printf 'data' > 'm1/back\slash.txt'; printf 'x' > 'm1/a b.txt'
$ detoxrs -x -r m1
m1/a b.txt  ->  a_b.txt
m1/back\slash.txt  ->  back_slash.txt

2 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.
Undo with: detoxrs undo 000001-20260803T213818Z

$ detoxrs undo --last
detoxrs: journal problem: line 4 has a 'from' that is not a plain filename (a path separator,
  `.`, `..`, or empty); refusing rather than risking a rename outside the pinned directory
detoxrs: journal problem: line 5: a 'done' with no intent before it
.../m1/a_b.txt  ->  a b.txt

1 reverted, 0 refused. This undo is itself batch 000002-20260803T213818Z.
$ ls -b m1
a b.txt
back_slash.txt          # back\slash.txt is unrecoverable
```

The forward run reports **exit 0, 0 failed**. The undo summary reports **0 refused** while
one name is permanently lost — the anomaly lines go to stderr and the summary contradicts
them (C-11). On opus-3's 78-entry tree the same defect lost 4 names out of 70 renames.

**Mutation evidence, re-verified by the consolidator.** Deleting the single clause
`&& !bytes.contains(&b'\\')` in an isolated copy leaves the entire workspace suite green —
**166 tests, 0 failures**. The guard has no test asserting its purpose and none asserting its
false-positive cost.

The stated rationale (C1: portability of a journal across platforms) does not survive
inspection: on Windows `\` cannot appear in a filename at all, so a Windows-written record
can never contain one, and a Unix-written record containing one is exactly the record this
check destroys.

**Why CRITICAL.** It is silent (`0 failed`, exit 0 forward; `0 refused` on undo),
deterministic, permanent, and it defeats the one guarantee the whole design is staked on.
Nothing else in the finding set is unrecoverable.

---

### C-2 — HIGH — order safety is decided by path spelling, so a parent can be renamed first

`crates/detoxrs-core/src/plan.rs:414` (`deterministic_order`,
`depth: Reverse(e.dir.components().count())`), fed by `crates/detoxrs/src/walk.rs:118-145`.
Reported as **O2-1**; test-coverage half is **C-16** (O2-6).
Contradicted by haiku-4 "Directory ordering: PASS" — adjudicated in §4.2.

`plan.rs`'s module doc calls order safety a data-loss property, and `Entry::depth` carries
the comment "Ordering does **not** trust this field: it is derived from `dir` instead, so
Order safety cannot be broken by a walker that miscounts." The derivation is
`e.dir.components().count()` — the component count of whichever argument string first reached
the entry. `walk.rs` never canonicalises it. `Components` drops `.` but keeps `..`, so two
arguments naming the same tree can give a parent a _larger_ count than an entry inside it.
The `dir_ident` machinery already knows these are the same directory; the information needed
to order correctly is present and discarded.

Reproduced by the consolidator, verbatim:

```
$ mkdir -p "de ep/d ir" && echo hi > "de ep/d ir/fi le.txt"
$ detoxrs -r "de ep/d ir" "de ep/../de ep"
de ep/..
  de ep/  ->  de_ep/          <-- the parent is listed FIRST
de ep/d ir
  fi le.txt  ->  fi_le.txt
de ep
  d ir/  ->  d_ir/

$ detoxrs -x -r "de ep/d ir" "de ep/../de ep"
de ep/../de ep  ->  de_ep
detoxrs: de ep/d ir/fi le.txt: cannot open the containing directory: no longer there (ENOENT)
detoxrs: de ep/d ir: cannot open the containing directory: no longer there (ENOENT)

1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 2 failed.      exit 1

$ find . | sort
./de_ep
./de_ep/d ir
./de_ep/d ir/fi le.txt          # permanently half-cleaned
```

Which invocations are safe is accidental: it depends on argument order and string length, not
on containment. Mixing an absolute and a relative spelling of one subtree hits the same key
and happens to come out safe only when the child is discovered under the longer spelling.

**Why HIGH not CRITICAL.** The batch fails loudly (exit 1, two named errors) and re-running
finishes the job, so no data is lost today. It is HIGH because the invariant the design is
staked on is simply false, and because the failure mode degenerates: nothing here stops the
ENOENT case becoming a _wrong-directory_ case if anything recreates the vacated name
mid-batch, after which the only remaining guard is the `(dev, ino)` recheck — which any
second hardlink satisfies (see C-22).

---

### C-3 — HIGH — the default 255-byte budget truncates legal APFS names, collides them, and never says so

`crates/detoxrs-core/src/policy.rs:101` (`M1_MAX_LEN = 255` used for _both_ the byte and the
UTF-16 budget), consumed at `pipeline.rs:218`; the discarded truncation flag is
`pipeline.rs:29` dropped at `plan.rs:355`.
Reported as **O2-2** (HIGH) + **O2-5** (MEDIUM), with **O3-8** (LOW) supplying the
reachability analysis. Merged here; adjudicated in §4.3.

APFS/HFS+ limit names to 255 _UTF-16 code units_, not 255 bytes. Applying a byte budget to a
name that is legal on the filesystem in front of you turns "clean this name" into "shorten
this name" — and `desired_for` runs `transform` on every entry, so truncation alone produces
a `Rename` for a name that is otherwise already clean. Over-truncation is not a conservative
direction: when two truncated prefixes are equal, the collision engine numbers them and the
bytes that told them apart are gone.

Reproduced by the consolidator on APFS, verbatim:

```
$ python3 -c "open('漢'*100+'a.txt','w').write('A'); open('漢'*100+'b.txt','w').write('B')"
   305 bytes  105 chars  105 UTF-16 units    # well inside APFS's 255-unit limit
$ detoxrs -x -r .
./漢…漢a.txt  ->  漢…漢.txt
./漢…漢b.txt  ->  漢…漢-2.txt

2 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.       exit 0
$ # after:
255 bytes '…-2.txt'   B
253 bytes '…漢.txt'   A
```

Exit 0, "0 conflicts", no warning. The user cannot tell which file is which, and the `-2` is
the tool inventing a distinction to replace the one it removed. Both files were legal, both
readable, and nothing about them needed changing.

The reporting half (O2-5) compounds it: `Outcome.truncated` is maintained carefully through
stage 12 and the stage-13 loop, has its own property test, and is then dropped —
`Desired::Rename(o.text)` keeps the text and nothing else. `PlanItem` has no truncation
field, `report::line` no note, `report::json` no key. Confirmed by the consolidator: the JSON
for that run carries `"note": null` for both items. The one transformation that _destroys_
information rather than rearranging it is the one the report cannot mention.

**Why HIGH.** Reachable by any user with CJK, Cyrillic, or emoji filenames on the project's
own tier-1 target, with no adversary and no warning. Not CRITICAL only because the journal
makes it recoverable — but only until the journal is pruned, and only for a user who notices,
and nothing in the output gives them a reason to.

---

### C-4 — HIGH — `numbered()` emits destinations that are not fixed points, so the tool renames its own output

`crates/detoxrs-core/src/plan.rs:593-597`. Reported as **O1-1**.

`numbered()` builds `truncate_graphemes(stem, budget) + "-N" + ext` and never checks that the
result is a fixed point of `transform`. When the stem is truncated to make room for the
suffix, the kept prefix can end in `-`, and the appended `-N` produces a `--` run that
stage 9 (`collapse`) squeezes away. §5.3's whole safety argument rests on `transform` being
idempotent; this path manufactures a name that is not a fixed point of it.
`debug_assert!(fits(...))` checks length and nothing checks safety closure.

Reproduced by the consolidator at the **default** 255-byte limit (`a` × 248 abbreviated):

```
$ ls          # both 255 bytes; the first is already clean, the second cleans to the same name
A…A-_b.txt   A…A- b.txt

$ detoxrs -x -r .
./A…A- b.txt  ->  A…A--2.txt
1 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.

$ detoxrs -r .          # second run on the tree detoxrs just produced
  A…A--2.txt  ->  A…A-2.txt
1 to rename, 1 unchanged, 0 skipped, 0 conflicts.
```

`detoxrs -x` is not a fixed point. The consequence the consolidator also reproduced is the
one that matters: a second `-x` renames again, and the **first batch's undo is then dead**:

```
$ detoxrs -x -r .                       # second forward run
./A…A--2.txt  ->  A…A-2.txt             # batch 000002
$ detoxrs undo 000001-20260803T213831Z
detoxrs: .../A…A--2.txt: no longer readable since the preview: no longer there (ENOENT)
0 reverted, 1 refused.
```

So a user who runs `-x` twice — the most natural thing to do after adding files to a
directory — loses the ability to undo the first run. opus-1's 40 000-directory fuzz found the
same class independently at other limits (`"é\"]-" -> "é_--2"`, `transform("é_--2") == "é_-2"`).

**Why HIGH.** Reachable at the shipped default with no adversary; it breaks the stated
idempotence guarantee and it silently invalidates a journal batch, which is the recovery path
every other finding depends on.

---

### C-5 — HIGH — an invisible character before a dotfile's dots destroys its dotfile status

`crates/detoxrs-core/src/pipeline.rs:196` with `trim` at `:173-186`. Reported as **O1-2**.

`leading_dots` is counted from `run_with`'s _original_ input, before stage 4 deletes
invisibles. `trim` then strips every leading dot and restores exactly `leading_dots` of them.
If the input begins with an invisible character followed by dots, the count is 0, stage 4
removes the invisible, and `trim` strips the now-leading dots and restores none. A hidden
file becomes visible. The doc comment claims Dotfile preservation "in both directions".

Reproduced by the consolidator, verbatim:

```
$ ls -b
\342\200\213..hidden        # U+200B ZWSP + "..hidden"
\342\200\213.bashrc         # U+200B ZWSP + ".bashrc"
\357\273\277.gitignore      # U+FEFF BOM  + ".gitignore"
\342\200\213plain.txt

$ detoxrs -r .
  <ZWSP>..hidden    ->  hidden
  <ZWSP>.bashrc     ->  bashrc
  <ZWSP>plain.txt   ->  plain.txt
  <BOM>.gitignore   ->  gitignore
```

`.bashrc` becomes `bashrc` and `.gitignore` becomes `gitignore` — the config stops being read
by whatever expects a dotfile, and the file stops being hidden. BOM-prefixed names are not
exotic: they come out of Windows-authored zips and CSV/text tooling routinely. opus-1's
200 000-input probe established the loss is one-directional: dots are only ever lost, never
manufactured, so no visible file becomes hidden.

**Why HIGH.** Silent semantic destruction on ordinary input, and the tool's own doc claims
the opposite. The name is recoverable via the journal; the _consequences_ of a config file
going unread for a week are not.

---

### C-6 — HIGH — a same-directory hardlink makes detoxrs rename an entry the user never named

`crates/detoxrs/src/walk.rs:172-183` (`real_entry_name`, called from
`corrected_top_level_path`, `:161`). Reported as **O1-3**.

For every top-level argument, `real_entry_name` lists the containing directory and returns
the name of the **first** entry whose `(dev, ino)` matches. Inode identity is not a unique
key for a directory entry: two hardlinks in one directory share it. `readdir` order then
decides which name detoxrs believes it was given.

Reproduced by the consolidator, verbatim:

```
$ ls -bi
154930359 a b.txt
154930359 c d.txt       # hardlink, same inode

$ detoxrs 'a b.txt'
  c d.txt  ->  c_d.txt [hardlink, nlink=2]
1 to rename, 0 unchanged, 0 skipped, 0 conflicts.

$ detoxrs -x 'a b.txt'
c d.txt  ->  c_d.txt
1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.      exit 0
$ ls -b
a b.txt      c_d.txt
```

The user asked for `a b.txt`; detoxrs renamed `c d.txt` and left the named file dirty. No
data is lost (the inode is shared), but the tool acted on a directory entry outside the
argument it was given, **and its own preview named the wrong file** — so the preview cannot
be used to catch it. The `#[cfg(unix)]` fallback path (`:185`) is immune because it does not
do the lookup at all.

**Why HIGH.** "The tool modified something you did not name, and told you it was doing so"
is a trust failure independent of byte loss, and the preview — the only control the user has
before `-x` — is complicit.

---

### C-7 — HIGH — the preview escapes only `Cc`; bidi, zero-width, `Zl`/`Zp`/`Zs` reach the terminal raw

`crates/detoxrs/src/report.rs:380-397` (`escape_text`), via `escape` at `:333`.
Reported as **O1-4** (MEDIUM) + **O1-8** (LOW). Merged and **upgraded** to HIGH.
Contradicted by haiku-3 "PASS" and, on a different question, by opus-3 "PASS" — adjudicated
in §4.1.

`escape_text` escapes `char::is_control()` (Unicode `Cc` only) and `<`. Nothing else. The
consolidator enumerated it exhaustively by running the real binary over one file per
character class and searching the output bytes:

| Character         | Class | raw bytes in output | escape token emitted |
| ----------------- | ----- | ------------------- | -------------------- |
| `U+202E` RLO      | Cf    | **yes**             | no                   |
| `U+200B` ZWSP     | Cf    | **yes**             | no                   |
| `U+200D` ZWJ      | Cf    | **yes**             | no                   |
| `U+061C` ALM      | Cf    | **yes**             | no                   |
| `U+E0041` TAG     | Cf    | **yes**             | no                   |
| `U+2028` LINE SEP | Zl    | **yes**             | no                   |
| `U+2029` PARA SEP | Zp    | **yes**             | no                   |
| `U+00A0` NBSP     | Zs    | **yes**             | no                   |
| `U+0007` BEL      | Cc    | no                  | `<07>`               |
| `U+009F`          | Cc    | no                  | `<u+009f>`           |

Two distinct harms, both reproduced:

**Visual reordering.** A file named `invoice<U+202E>gpj.exe` renders in the preview as
`invoiceexe.jpg`. This is CVE-2021-42574 verbatim, in the output a user reads immediately
before typing `-x`, in a tool that cites that advisory as the reason stage 4 exists.

**Report-row forgery.** `U+2028`/`U+2029` pass through, and every downstream consumer that
splits on Unicode line breaks (Python `splitlines`, Java, JS, .NET, `less`, most editors)
sees one report row as several. The consolidator's raw capture of a directory containing a
`U+2028` name and a `U+2029` name:

```
'.'
'  a'
'b .txt         ->  a'
'b.txt'
'  c'
'd .txt         ->  c'
'd.txt'
```

Two items, six lines. A crafted name can therefore contribute text that reads as an
additional report row or as a forged summary line. The consolidator built one:

```
harmless a.txt<U+2028>0 to rename, 5 unchanged, 0 skipped, 0 conflicts.<U+2028>Nothing was changed.
```

**Why upgraded to HIGH.** opus-1 ranked this MEDIUM. The preview is not a cosmetic surface —
it is the tool's _only_ safety control ("preview by default"), and it is the one thing
standing between a user and the six data-affecting defects above. A safety control whose
rendering can be falsified by the input it is reviewing is not a MEDIUM. The character set
that must be escaped is exactly the set stage 4 already enumerates in
`detoxrs-core/src/invisible.rs`, so the fix has no new classification burden.

O1-8's separate observation stands and is the same root: `is_invisible` stops at bidi,
zero-width and Tags, and `classify` deletes only `Cc`, so `U+2028`, `U+2029`, `U+180E`,
`U+00A0` and the `U+2000`–`U+200A`/`U+202F`/`U+3000` spaces are all `Keep`. That is a
documented M4 deferral, so _keeping_ them is a choice; _printing them raw_ is not.

---

### C-8 — MEDIUM — a run that could not do what it was asked exits 0

`crates/detoxrs/src/apply.rs:76` (`Summary::exit_code`), `main.rs:174`, `walk.rs:240`.
Reported as **O2-3**. Contradicted by haiku-4 and haiku-1 — adjudicated in §4.4.

`--help` documents exit 1 as "one or more items could not be renamed". Two classes of "could
not be renamed" produce exit 0. Both reproduced by the consolidator:

```
$ echo a > "a b.txt"; echo b > "a_b.txt"
$ detoxrs -x --on-collision skip -r .
  a b.txt  !   conflict (that name is already taken)
0 renamed, 1 unchanged, 0 skipped, 1 conflicts, 0 failed.       exit 0
```

```
$ mkdir secret; echo x > "secret/h idden.txt"; echo y > "o k.txt"; chmod 000 secret
$ detoxrs -x -r .
detoxrs: warning: IO error for operation on ./secret: Permission denied (os error 13)
./o k.txt  ->  o_k.txt
1 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.       exit 0
```

`Resolution::Conflict(_)` items are never attempted so never enter `Summary::failed`;
`walk_into` warns to stderr and nothing downstream records that the snapshot is incomplete.
The second is worse: `0 failed` is a positive claim and an entire subtree was never
inspected. `detoxrs -x -r . && echo clean` prints `clean` in both cases. `WalkError` exists
for exactly this class and is deliberately not used here.

For contrast, `--on-collision fail` _does_ exit 2 correctly (consolidator-verified), so the
inconsistency is between arms of the same feature.

---

### C-9 — MEDIUM — cleaning a tree silently breaks relative symlinks inside it, and reports success

`crates/detoxrs/src/apply.rs:204-228`, `walk.rs:436-447`. Reported as **O2-4**.

detoxrs renames a symlink's directory entry and, in the same batch, renames the file that
symlink points at, and never notices the second rename invalidates the first. §5.6's "renamed
as the link itself, never followed" is about not dereferencing, which is right; it is not an
answer to "the link now points at nothing." Reproduced by the consolidator:

```
$ echo TARGET > "t arget.txt"; ln -s "t arget.txt" "l ink"; cat "l ink"   # -> TARGET
$ detoxrs -x -r .
./l ink  ->  l_ink
./t arget.txt  ->  t_arget.txt
2 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.       exit 0
$ readlink l_ink        # t arget.txt
$ cat l_ink             # cat: l_ink: No such file or directory
```

This is the one case in the tool where a normal, non-adversarial run _creates_ breakage
rather than failing to fix it. The information needed to warn is available with no extra
I/O: the link's target is a name, and the batch knows every name it is about to change in
that directory.

---

### C-10 — MEDIUM — journal filenames are wholly unvalidated: an overflow panic and a wrong-batch undo

`crates/detoxrs/src/journal.rs:149` (`next_seq`), `:101` (`{seq:06}`), `:537` (`out.sort()`),
`main.rs:328` (`resolve_last`). Merges **O3-3** and **O3-4** — one root cause, two symptoms.

`journal.rs`'s own docs anticipate hostile journal _contents_ and harden `parse_intent`
accordingly. Journal _filenames_ get no validation at all: `next_seq` parses the first
`-`-delimited token of every filename in the directory as a `u64` and returns `max + 1`.

**Symptom A — overflow panic.** Reproduced:

```
$ touch "$XDG_STATE_HOME/detoxrs/journal/18446744073709551615-20260101T000000Z.jsonl"
$ detoxrs -x -r .
thread 'main' panicked at crates/detoxrs/src/journal.rs:149:12: attempt to add with overflow
```

Exit 101, where `main.rs` documents exit 2 for "failures where nothing was attempted". In a
release build with overflow checks off this wraps to 0 silently instead. This also refutes
haiku-3's "Panics: PASS — all production code uses proper error handling".

**Symptom B — width poisoning picks the wrong batch.** `{seq:06}` is fixed-width only below
1 000 000, and `list()` sorts lexically, so `"1000000-…"` sorts _before_ `"999999-…"` and
`resolve_last`'s `.rev()` returns the older batch. Reproduced:

```
$ touch "$XDG_STATE_HOME/detoxrs/journal/999998-20200101T000000Z.jsonl"
$ detoxrs -x "one 1.txt"    # Undo with: detoxrs undo 999999-…
$ detoxrs -x "two 2.txt"    # Undo with: detoxrs undo 1000000-…
$ detoxrs undo --last
.../one_1.txt  ->  one 1.txt        # <-- the FIRST run, not the last
1 reverted, 0 refused.              exit 0
$ ls
one 1.txt   two_2.txt               # the batch the user meant is untouched
```

Exit 0, no warning. The user believes they undid their most recent run.

Two routes in: a million real batches, or one stray/planted filename whose leading number is
≥ 999999 — which `next_seq` adopts as the high-water mark without question. Combined with
C-18 (world-readable journal directory) and any shared `XDG_STATE_HOME`, the planted-file
route is the practical one.

---

### C-11 — MEDIUM — `undo`'s tally hides items dropped before the apply loop

`crates/detoxrs/src/main.rs:270-276`. Reported as **O3-2**. The reporting half of C-1.

The closing line is built only from `Summary::renamed` / `Summary::failed`, which count items
that reached `apply::attempt`. An `intent` rejected by `parse_intent` never becomes an
`UndoItem`, so it is in **neither** bucket. The batch's forward count is never compared
against the number of `done` records replayed, so "this batch had 70 renames and I could only
address 66" is information the program has and does not say. Reproduced in the C-1 transcript:
forward `2 renamed`, undo `1 reverted, 0 refused`. Correct output would be
`1 reverted, 0 refused, 1 could not be undone at all`.

The anomaly lines do reach stderr and the exit code is 1, but the _summary_ — the line the
user actually reads — actively contradicts them, and on a large batch the anomalies scroll
away. This is what converts C-1 from "loud failure" into "silent loss".

---

### C-12 — MEDIUM — collision numbering is blind past `-2` for non-recursive args, and blames a false race

`crates/detoxrs/src/walk.rs:290-320` (`seed_pre_existing_destination`). Reported as **O1-5**.

The function seeds exactly one pre-existing entry: the _unnumbered_ destination
`dir.join(&wanted.text)`. `plan()` is I/O-free, so if that name is taken it renumbers to `-2`
— a name nothing ever checked against the filesystem. `apply`'s step-2 recheck then finds the
occupant and refuses with "appeared since the preview", which is a false statement: the file
was there before the walk started. Reproduced:

```
$ ls -b
a b.txt   a_b-2.txt   a_b.txt
$ detoxrs -x 'a b.txt'
detoxrs: a b.txt: a_b-2.txt appeared since the preview; not renamed
0 renamed, 1 unchanged, 0 skipped, 0 conflicts, 1 failed.       exit 1
$ detoxrs -r .          # same tree, snapshot contains everything
  a b.txt  ->  a_b-3.txt
```

Nothing is clobbered — the apply-time guard holds, which is a real point in the design's
favour — but `--on-collision number` silently does not work outside a recursive walk, and the
diagnostic misdirects the user to look for a concurrent writer that does not exist.

---

### C-13 — MEDIUM — the stage-13 convergence bound is free

`crates/detoxrs-core/src/pipeline.rs:58` (`FIXED_POINT_BOUND`), `:240-263`.
Merges **O1-7** and **H2-2** (which reported the same gap at bound 2).

Consolidator-verified: setting `FIXED_POINT_BOUND` from 3 to **1** in an isolated copy leaves
the entire workspace suite green — **166 tests, 0 failures**. opus-1 measured what that
mutation actually changes: of 400 000 adversarial probe inputs, 19 845 come back
`Unrepresentable(NotConverged)` at bound 1 that resolve to a name at bound 2 or 3.

| bound | unrepresentable / 400 000 | NotConverged |
| ----- | ------------------------- | ------------ |
| 1     | 78 073                    | 19 845       |
| 2     | 58 673                    | 0            |
| 3     | 58 673                    | 0            |

So no test in the suite exercises a second iteration of the loop, and the constant that
decides how many iterations there are is unconstrained by the suite — while the second
iteration is load-bearing for roughly 5% of dirty names. Bound 3 is also unexercised: nothing
in 400 000 samples needs a third pass, so the comment's claim that "3 is never tight" is
supported but untested. `NotConverged` is reported and skipped rather than mis-renamed, so
this is a regression-risk defect, not a live one.

---

### C-14 — MEDIUM — `detoxrs *` is quadratic

`crates/detoxrs/src/walk.rs:172-183`. Reported as **O1-6**. Same root cause as C-6.

`real_entry_name` calls `fs::read_dir` and then `entry.metadata()` on every entry, for
**every** top-level argument, until it finds a matching inode. For `detoxrs *` in a directory
of _n_ files that is O(n²) `lstat` calls — and `detoxrs *` is the shell-native way to invoke
a non-recursive run. Consolidator-measured, release build, 3000 files named `f %04d.txt`:

```
$ time detoxrs -r . >/dev/null      # 0.02s user 0.01s system   0.260 total
$ time detoxrs * >/dev/null         # 0.84s user 7.90s system   9.599 total
```

37× slower for the same 3000 names, 7.9 s of it system time. Extrapolating the quadratic
term, 10 000 arguments is well over a minute.

---

### C-15 — MEDIUM — `--json` omits the batch id and journal path

`crates/detoxrs/src/report.rs:297-316`, `main.rs:148-154`. Reported as **O3-6**.

`report.rs`'s module doc calls `--json` "the only stable contract", and `applied()` explains
that the batch id goes in the human output because "a user who has just renamed 400 files
needs the one string that undoes it". The JSON document is built from `plan` and `outcomes`
only; `exec` has `batch` and `where_` in scope and passes neither. Consolidator-verified: the
document's top-level keys are `['applied', 'atomicity', 'items', 'schema', 'summary']` — no
`batch`, no `journal`, no `undo`. The one caller that cannot read the human report is exactly
the caller with no way to learn what to pass to `undo`. Its only recourse is `undo --list`
plus a guess, which is racy: opus-3 showed 12 concurrent runs producing 12 journals in the
same second.

---

### C-16 through C-29 — LOW

Accepted as reported; each is a real observation whose blast radius is small, a coverage gap
whose bypass is caught by a lower layer, or a diagnostic/documentation defect. Grouped
briefly.

**Test-coverage gaps** (real; all confirmed by a surviving mutation)

- **C-16** (O2-6) — the Order-safety property test _cannot_ fail for C-2, for two independent
  reasons: `build_snapshot` hardcodes one canonical spelling per level (`t`, `t/a b`,
  `t/a b/c d`) — never a `..`, never an absolute path — and the assertion is textual
  (`prop_assert!(!later.dir.starts_with(&container))`), so even with the failing input
  `"de ep/d ir".starts_with("de ep/../de ep")` is `false`. The property is written in the same
  units as the bug. Separately, `Entry::depth`'s documented safety claim is unguarded:
  substituting `Reverse(e.depth as usize)` for the derivation leaves every target green. Only
  the sort _direction_ is covered.
- **C-21** (H2-3) — `is_free`'s respell check `all(|h| h == owner)` mutated to `any(...)`
  survives the suite. **Downgraded from haiku-2's HIGH**; adjudicated in §4.6.
- **C-26** (H2-1) — `split_extension`'s `dot - prev - 1 <= 4` mutated to `< 4` survives:
  `split_extension_matches_the_documented_rule` covers 3-byte and 5-byte inner segments and
  never exactly 4, so `report.abcd.gz` is untested.
- **C-27** (H2-4) — `FIRST_NUMBER..=LAST_NUMBER` mutated to `..` survives: the constants test
  asserts the count is 998 but nothing asserts `-999` is reachable.

**Reasoned-only safety gaps** (PLAUSIBLE; none reproduced by anyone)

- **C-20** (O1-10) — `if let Ok(occupant) = ops.ident_at(...)` at `apply.rs:187` and
  `is_ok() && …` at `fallback.rs:243` treat _any_ error as an unoccupied destination. On
  tier-1 `RENAME_NOREPLACE` still refuses, so the cost is a worse message. On the demoted rung
  (`check_then_rename`, reached on `EINVAL`/`ENOSYS`/`EOPNOTSUPP`, and the whole Windows tier)
  there is no kernel guard, and an `lstat` failing for a reason other than absence falls
  through to `rename_plain`, which replaces. Matching on error kind closes it. The demoted
  rung's documented promise is "still never clobbers"; this is the one shape where it might.
- **C-22** (O2-7) — the dirfd pin is real _within_ one item, but it is established by resolving
  `item.dir` as a path at apply time, `open_dir` deliberately omits `O_NOFOLLOW`, and the
  walk's `dir_ident` is never copied into `PlanItem`, so `apply` cannot compare the directory
  it opened against the one previewed. Confinement rests on the entry's own `(dev, ino)`
  recheck, which any second hardlink satisfies. The module doc's "a descriptor resolved once
  is one directory, permanently" is true from step 0 onward, not from the walk onward. Fix is
  one `u64` pair in `PlanItem` plus one comparison.
- **C-23** (O1-9) — `check_no_sibling_chains` builds `vacated` from `entries[i].dir.as_path()`
  while layer 1's `wants` and the `Allocator` deliberately key on `dir_ident`, precisely
  because `.`, `""` and `./x` are three spellings of one directory (C8). Fails open only. Same
  spelling-vs-identity confusion as C-2; worth fixing in the same pass.
- **C-24** (O2-10) — `main.rs:141-146`: `j.finish()` failing means the batch has no `end`
  record and `undo` will warn and exit non-zero _later_, but the forward run prints a stderr
  warning and returns `s.exit_code()` unchanged — 0 if every rename succeeded. Same class as
  C-8: the run knows its own safety net is incomplete and does not say so where a script
  reads.

**Diagnostics, reporting, permissions, docs** (all CONFIRMED)

- **C-17** (O2-9) — an undo writes its own journal, which is then the newest, so a second
  `undo --last` replays it, restores the cleaned names, and reports `1 reverted, 0 refused`,
  exit 0. "An undo is itself undoable" is a stated design goal, so the mechanism is intended;
  the word `reverted` for a redo is not, and a user who types the command twice believing the
  first did not take silently re-dirties the tree. A one-line note when the selected batch is
  itself an undo would cost nothing.
- **C-18** (O3-5) — neither the journal directory nor the file sets a mode, so both take the
  umask; under the common `022` that is `drwxr-xr-x` / `-rw-r--r--`. Consolidator-verified.
  The journal holds the absolute path of every file the user renamed plus both names — a
  fairly complete map of a private tree, disclosed to every local account, never pruned.
  `$HOME/.local/state` is `0700` on many systems and masks this; the `XDG_STATE_HOME` path has
  no such protection. Also the enabling condition for C-10's practical attack route.
- **C-19** (O2-8) — the same `RenameErr` is returned by `open()` and `ident_at()`, so
  `Other(n) => "rename failed (errno {n})"` lands in messages about operations that are not
  renames, and `map_errno` has no `ENOTDIR` arm. Produces the self-contradicting
  `cannot open the containing directory: rename failed (errno 20)`. Behaviour is correct;
  only the diagnostic is wrong.
- **C-25** (O3-7) — `replay` deliberately ignores an unparseable _last_ line, correct for the
  `kill -9` case (a small `write(2)` to a regular file does not tear, so a cut `intent`
  implies the rename had not happened). Under a power-loss tear spanning a sector boundary —
  explicitly outside the threat model — the record can be cut _after_ the rename landed, and
  `replay` then reports neither an anomaly nor an `interrupted` item. The gap is that the
  warning's wording promises more than replay can deliver in that case.
- **C-28** (H1-2) — consolidator-verified: `undo --last` with no batches exits 2 while
  `undo --list` exits 0. `--help` defines exit 2 as "usage, walk, or plan error"; "no data to
  report" is none of those.
- **C-29** (H1-1, H1-4) — help says the journal goes to `$XDG_STATE_HOME/detoxrs/journal` and
  does not document the `$HOME/.local/state` fallback that actually applies; `-v` is a
  `clap::ArgAction::Count` documented as a boolean, and only `verbose > 0` changes anything,
  so either the docs or the type is wrong.

---

## 4. Adjudications

### 4.1 Terminal escaping — opus-1 (CONFIRMED) vs haiku-3 (PASS) vs opus-3 (PASS)

**Claims.** opus-1 O1-4: bidi / zero-width / `U+2028` reach the terminal unescaped.
haiku-3: "Terminal Injection — PASS, all control characters properly escaped … All Unicode
control chars (Cf, bidi, etc) → `<u+XXXX>`". opus-3: "`report::escape` injectivity — PASS".

**Ruling: opus-1 is right. haiku-3 is refuted. opus-3 is right about a different question.**

**Evidence.** `escape_text` is eleven lines and its predicate is `c.is_control() || c == '<'`.
Rust's `char::is_control()` is Unicode `Cc` and nothing else. The consolidator enumerated the
behaviour empirically against the real binary — table in C-7 — and every `Cf`, `Zl`, `Zp` and
`Zs` probe appears in the output as raw bytes with no token emitted, while both `Cc` probes
are escaped. haiku-3's specific assertion that `Cf` and bidi become `<u+XXXX>` is false about
the implementation.

haiku-3's error is instructive: **its own transcript contains the counter-evidence.** It
printed `./file‮‭name.txt         ->  filename.txt`, with the raw `U+202E` and `U+202D` in
the row, directly beneath the claim that all control characters were escaped as `<XX>`. It
read the escaped `<07>` and `<1b>` in the neighbouring rows, generalised, and did not check
the bidi row it had just produced. It also ran `od -c` on the output — the one tool that would
have shown it the raw bytes — and did not act on the result.

opus-3's PASS is a correct answer to a narrower question. Injectivity asks whether two
distinct names can render identically; since non-`Cc` characters pass through unmodified and
every literal `<` is escaped, distinct inputs do render distinctly, so `escape` _is_
injective. Injectivity and terminal safety are independent properties, and the code satisfies
the first while failing the second. Both verdicts stand as stated; only haiku-3's is wrong.

**Severity ruling.** Upgraded from opus-1's MEDIUM to HIGH. See C-7.

### 4.2 Directory ordering — opus-2 (FAIL) vs haiku-4 (PASS) vs opus-3 (PASS)

Forward-apply order and undo order are two questions. Separated and ruled on individually.

**Forward apply: opus-2 is right; haiku-4's PASS is true of its test case and false as
stated.** The consolidator reproduced O2-1 exactly (see C-2): with two arguments spelling one
subtree differently, the preview lists the parent first and `-x` half-applies with two ENOENT.
haiku-4's test used a single canonical argument (`-r` over `dirty dir/subdir/a b.txt`), and
the consolidator confirms that case does order children before parents. But haiku-4 wrote the
verdict as the general claim "Directory ordering: correct (children before parents) — PASS",
which its evidence does not support: it tested the one input class the defect cannot appear
in. A PASS that exercises only the canonical spelling cannot establish a property whose
counterexample requires a non-canonical spelling. **Verdict: FAIL in general, PASS for
single-canonical-spelling invocations, and haiku-4's verdict is unproven as written.**

**Undo: opus-3 and opus-2 are both right.** `replay` reverses the forward journal, so undo
order is the forward order inverted, which restores a parent before its children.
Consolidator-verified on a 3-level tree: `d_ir → d ir`, then `s_ub → s ub`, then
`f_ile.txt → f ile.txt`, `3 reverted, 0 refused`, and `find | sort` byte-identical to the
pre-run tree. **Verdict: PASS, and it is a real PASS** — this is a property, not a coincidence
of the test case, because the inversion is structural. Note the dependency: undo order is
correct _given_ a correctly ordered forward batch, so fixing C-2 does not endanger it, and
C-2 does not endanger undo (an O2-1-shaped batch only journals the renames that succeeded).

### 4.3 Truncation reachability — opus-2 O2-2 (HIGH) vs opus-3 O3-8 (LOW)

**Ruling: compatible, not contradictory. opus-3's analysis is correct and its conclusion is
wrong. opus-2's severity is right.**

O3-8 says the truncation stage is unreachable on ext4/APFS for single-byte names because
`transform` never lengthens a name and `M1_MAX_LEN == NAME_MAX`, so a name that exists
already fits. That reasoning is sound, and the consolidator confirms the boundary case: a
300-byte ASCII name cannot be created at all. O3-8 then correctly identifies the one class
that _does_ reach stage 11 — "multi-byte names on a filesystem that counts characters rather
than bytes" — and even reproduces it (200 × `é`, 406 bytes, created and truncated on APFS).

O2-2 is that exact class, taken one step further: it shows the class is not exotic (100 CJK
characters is an ordinary filename), and that within it the truncation _collides_ two legal
names and destroys the bytes that distinguished them, at exit 0 with `0 conflicts` and no
note. O3-8 stopped at "not a correctness bug"; the correctness bug is what happens after the
truncation, which O3-8 did not test.

So the two reviewers found the same code path and disagreed only about whether reaching it
matters. **O3-8 is the reachability proof for O2-2, and is folded into C-3 as such.** Its LOW
rating is over-generous and does not survive: a defect reachable with `漢`-named files on the
project's tier-1 target, that silently makes two files indistinguishable, is HIGH.

### 4.4 Exit codes — opus-2 O2-3 (FAIL, twice) vs haiku-4 (PASS) vs haiku-1 (LOW)

**Ruling: opus-2 is right. haiku-4's PASS is void. haiku-1 found a real but different, smaller
issue and missed this one.**

Both of opus-2's reproductions were re-reproduced verbatim by the consolidator (see C-8):
a conflicts-only `-x` run exits 0, and a run that could not read an entire subtree exits 0
while printing `0 failed`.

haiku-4's PASS rests entirely on unit tests it did not write and did not extend:
`a_read_only_filesystem_aborts_the_rest_of_the_batch` (EROFS → exit 1),
`a_permission_error_does_not_stop_the_batch` (EACCES on one item → batch continues), and
`a_broken_pipe_does_not_report_exit_2_after_renames_happened`. Every one of those covers a
case where an item was _attempted and failed_ — the case `Summary::exit_code` handles
correctly. Neither of the two failing classes involves an attempted item: conflicts are never
attempted, and an unreadable subtree is never planned. haiku-4 cited the tests that pass and
inferred the contract holds, without testing the contract. That is not evidence for the
verdict it wrote.

haiku-1's H1-2 (`undo --last` exits 2, `undo --list` exits 0 — kept as C-28) is a genuine
inconsistency, and the consolidator confirmed it. It is a much smaller one, and haiku-1's
"Exit code contract — PASS" row is refuted by the same two reproductions.

### 4.5 `NotConverged` and the other unexplained sightings — contamination, not defects

**Ruling: REFUTED as product defects. All three are artifacts of the shared working tree, and
the consolidator reproduced the mechanism for the hardest one.**

opus-2 recorded, as an explicit non-finding, that one preview run printed
`skipped (transform did not reach a fixed point)` for all three of `de ep`, `d ir` and
`fi le.txt` with `0 to rename, 3 skipped` — names that converge in one stage-13 iteration —
and that ~45 subsequent attempts produced `3 to rename`.

haiku-2's own report lists, in its "Mutations That Were Caught" table, a mutation of
`pipeline.rs:253`, `next == text` → `!=` — inverting the convergence check. The consolidator
applied exactly that mutation to an isolated copy, rebuilt, and ran opus-2's tree:

```
$ mut/target/debug/detoxrs -r .
./de ep/d ir
  fi le.txt  -   skipped (transform did not reach a fixed point)
./de ep
  d ir/  -   skipped (transform did not reach a fixed point)
.
  de ep/  -   skipped (transform did not reach a fixed point)

0 to rename, 0 unchanged, 3 skipped, 0 conflicts.
```

That is opus-2's observation character-for-character, including the same three names and the
same counts. opus-2 was running a binary built from haiku-2's mutated tree. The sighting is
fully explained, the mutation was reverted, and `NotConverged` is **not** reachable at HEAD.

opus-3's two sightings resolve the same way, and opus-3 said so itself: it observed that
`invisible.rs` "had changed between two reads", which accounts for
`assertion failed: is_invisible('\u{061c}')` for a character plainly present in the match arm,
and it verified that the `no_pre_existing_clobber` proptest failure does not reproduce at
HEAD. The consolidator's own isolated run is green over the full suite.

**Consequence.** `NotConverged` remains documented as "a bug report against us" and there is
now no field sighting of it. C-13 stands separately: `NotConverged` _is_ reachable if
`FIXED_POINT_BOUND` is ever lowered, and nothing in the suite would notice.

### 4.6 Allocator respell `all()` vs `any()` — haiku-2 H2-3 (HIGH) vs consolidator (LOW)

**Ruling: real coverage gap, over-severe rating. Downgraded HIGH → LOW.**

The mutation genuinely survives, so haiku-2 found something. Its severity reasoning does not
hold. haiku-2 wrote "HIGH RISK. An entry could clobber another entry's destination." It could
not. Tracing the consequence: with `any()`, an entry whose destination key is still held by
another snapshot entry is allowed to take it, so `plan()` emits a rename onto an occupied
name. Two independent later layers stop it — `apply`'s step-2 occupancy recheck
(`apply.rs:187`) refuses the item, and below that `rename_noreplace` refuses at the kernel.
The observable result is a per-item `AlreadyExists` failure and exit 1, not a clobber.

That no-clobber layer is not assumed; it is the best-evidenced property in the whole review.
opus-2 probed an occupied destination on disk in six shapes (plain file, directory, dangling
symlink, second hardlink to the source, case-folded collision on insensitive APFS, non-ASCII
case fold) with content byte-identical after each, and the consolidator watched step 2 fire
twice while reproducing other findings (C-12's false-race message, C-2's neighbour case).

The consolidator also tried to construct haiku-2's stated scenario and **could not on this
host**: APFS is normalization-insensitive, so an NFC and an NFD spelling of `café x.txt`
collapse to one directory entry, and the "multiple holders" state cannot exist. The scenario
is reachable on ext4 (byte-exact), which is a tier-1 target, so the gap is worth closing —
as a plan-layer coverage gap, at LOW, not as a data-loss risk.

### 4.7 Test-suite state and count

haiku-4 reported "173 tests pass"; haiku-2 reported 166; opus-1 enumerated 11 targets summing
to 166. The consolidator's isolated run at HEAD: **166 across 11 targets, 0 failures**, twice
(once clean, once with the C-1 guard removed). 173 is not a count HEAD produces. Whatever
haiku-4 measured, it was not this commit's suite — which is consistent with it having run
while another reviewer held mutations in `crates/`, and is one more reason its verdicts are
void.

---

## 5. Refuted / downgraded

| Claim                                                                      | By            | Disposition                                                                                                                                                                                                                                                                                                             |
| -------------------------------------------------------------------------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `NotConverged` for names that converge in one iteration                    | opus-2 (obs.) | **REFUTED** — reproduced exactly by haiku-2's `pipeline.rs:253` mutation; not reachable at HEAD (§4.5)                                                                                                                                                                                                                  |
| `assertion failed: is_invisible('\u{061c}')`                               | opus-3 (obs.) | **REFUTED** — contamination; opus-3 saw the file change between reads (§4.5)                                                                                                                                                                                                                                            |
| `no_pre_existing_clobber` proptest failure                                 | opus-3 (obs.) | **REFUTED** — does not reproduce at HEAD; 8 consecutive green `prop_plan` runs, plus consolidator's green suite                                                                                                                                                                                                         |
| "Terminal Injection — PASS, all control characters properly escaped"       | haiku-3       | **REFUTED** — only `Cc` is escaped; haiku-3's own transcript shows raw `U+202E`/`U+202D`. See C-7, §4.1                                                                                                                                                                                                                 |
| "Directory ordering: correct (children before parents) — PASS"             | haiku-4       | **REFUTED as written** — true only for single-canonical-spelling arguments; FAIL in general. See C-2, §4.2                                                                                                                                                                                                              |
| "Error propagation is correct (exit codes match specification)"            | haiku-4       | **REFUTED** — cited unit tests cover only attempted-and-failed items; both failing classes are unattempted. See C-8, §4.4                                                                                                                                                                                               |
| "Exit code contract — PASS"                                                | haiku-1       | **REFUTED** — same two reproductions                                                                                                                                                                                                                                                                                    |
| "Identity recheck at apply time … numbered the destination to `a_b-2.txt`" | haiku-4       | **REFUTED** — `apply` never renumbers; step 2 _refuses_. `apply.rs:28-39` says so explicitly, and `grep` finds no allocator in the CLI crate. The numbering haiku-4 saw came from `plan()`'s pre-existing-destination seed, i.e. before the walk, not between plan and apply — so its test did not test what it claimed |
| "Panics: PASS — all production code uses proper error handling"            | haiku-3       | **REFUTED** — `journal.rs:149` panics on a crafted journal filename (exit 101). Grepping for `unwrap`/`expect` does not find arithmetic overflow. See C-10                                                                                                                                                              |
| "No security hardening is required"                                        | haiku-3       | **REFUTED** — C-7 (falsifiable safety control), C-10 (panic + wrong-batch undo from a planted filename), C-18 (world-readable path map) are all security-relevant and all reproduced                                                                                                                                    |
| "173 tests pass"                                                           | haiku-4       | **REFUTED** — HEAD produces 166 across 11 targets (§4.7)                                                                                                                                                                                                                                                                |
| H2-3 allocator respell — HIGH, "could clobber"                             | haiku-2       | **DOWNGRADED to LOW (C-21)** — no-clobber is enforced independently at apply time and at the kernel; result is a per-item failure. Scenario also unconstructable on APFS (§4.6)                                                                                                                                         |
| O3-8 truncation "not a correctness bug" — LOW                              | opus-3        | **DOWNGRADED verdict, UPGRADED severity** — the analysis is right and is the reachability proof for C-3 (HIGH), which opus-3 did not test past the truncation itself (§4.3)                                                                                                                                             |
| O1-4 report escaping — MEDIUM                                              | opus-1        | **UPGRADED to HIGH (C-7)** — the preview is the tool's only pre-`-x` safety control, not a cosmetic surface                                                                                                                                                                                                             |
| H1-3 `--on-collision fail` exits 2 with no JSON                            | haiku-1       | **NOT A DEFECT** — haiku-1 said so itself; consolidator confirms exit 2 is correct and documented for a plan error. Dropped                                                                                                                                                                                             |
| "Prior C1 / C2 — UNKNOWN"; C4, C7, C9, C10 "not re-tested"                 | haiku-1       | **NOT ESTABLISHED, not PASS.** Six of haiku-1's eleven prior-defect rows are explicit non-tests. opus-3 does establish the C1 successor (journal record trust boundary) as a real PASS                                                                                                                                  |

A reviewer being wrong is a useful result twice over here: haiku-3's escaping PASS and
haiku-4's ordering and exit-code PASSes each named an area, gave it a green verdict, and would
have retired exactly the questions that turned out to hold reproducible defects. Three of the
seven most severe findings sit inside a Haiku PASS.

---

## 6. Process finding: shared-tree contamination

**Severity: HIGH, against the review process, not the code.**

All seven reviewers ran in one working tree, `/Users/kerry.hatcher/projects/detoxrs`. haiku-2's
assignment was mutation testing, and it held **live, unreverted mutations in `crates/`** while
other reviewers were building and running `cargo test` in the same checkout. haiku-2 reverted
each mutation eventually and its final `git status --short` was clean — the discipline was
correct _serially_ and meaningless _concurrently_.

**What the other reviewers saw.**

- opus-1: `cargo test --workspace` failed on first run with 5 failures in `truncate::tests`,
  traced to haiku-2's `split_extension` `<= 4` → `< 4` mutation. It moved to a
  `git clone --no-hardlinks` and did all subsequent work there.
- opus-3: "`cargo test` returned five mutually contradictory verdicts in six runs", including
  an assertion failure for a character plainly present in the match arm, and observed
  `invisible.rs` change between two reads. It moved to a `git archive` snapshot.
- opus-2: recorded a `NotConverged` sighting it could not reproduce in ~45 attempts. The
  consolidator has now shown this was haiku-2's convergence-inversion mutation, live in the
  shared `target/` binary (§4.5).
- opus-2 also deleted an untracked `crates/detoxrs-core/proptest-regressions/pipeline.txt`
  while cleaning up, which may have been another reviewer's artifact.

**Which verdicts this voids.** Every test-run-derived or binary-derived claim from haiku-1,
haiku-3 and haiku-4, because none of them detected the contamination, none used an isolated
copy, and none re-verified anything against a clean snapshot. Concretely:

- haiku-4's entire report. It is built on "173 tests pass" (a count HEAD does not produce),
  on unit tests it cited rather than exercised, and on synthesized runs against a binary of
  unknown provenance. Its thirteen PASS rows are **NOT ESTABLISHED**, and three are refuted on
  their own merits (§4.2, §4.4, §5).
- haiku-3's PASS rows for Terminal Injection, Panics, TOCTOU and Resource Exhaustion, all of
  which were derived from running a binary in the shared tree. The Terminal Injection and
  Panics rows are additionally refuted by code inspection, so contamination is not their only
  problem.
- haiku-1's PASS rows for the exit-code contract and the re-verified prior defects (C3, C5,
  C12). Its documentation findings (C-29) survive because they are read off `--help` text,
  which no mutation touched.

haiku-2's own findings survive, because a mutation-testing reviewer is the one participant
whose method is unaffected by its own mutations — but note that its 8 mutations were the
contamination source, and its `git status` check at the end of the review proves nothing about
the tree state _during_ it.

**Was the code ever at risk?** No. The consolidator verified `git diff HEAD -- crates/` is
empty and the isolated suite is green at 166 tests. All mutations were reverted. The damage
was entirely to the evidence.

**How to run the next multi-agent review.**

1. **One isolated checkout per reviewer, mandatory, before any reviewer starts.** A `git
worktree add` per reviewer, or `git clone --no-hardlinks` / `git archive HEAD | tar -x`
   into a per-reviewer scratchpad directory. `isolation: "worktree"` on the Agent call does
   this automatically.
2. **A separate `CARGO_TARGET_DIR` per reviewer.** Two reviewers sharing `target/` share a
   binary even with separate sources. opus-2 was reading a binary built from haiku-2's source.
3. **The shared checkout is read-only for reviewers.** Nobody builds, tests, or edits in it.
   The only writes are report files under `docs/reviews/`.
4. **Any mutation-testing reviewer gets a hard isolation requirement stated in its prompt**,
   not left to its own housekeeping discipline.
5. **A contradictory or flaky suite result is a stop-and-report event, not a finding.** opus-1
   and opus-3 both handled this correctly and both said so in their reports; that is the
   behaviour to require. A reviewer that observes a green suite it cannot reproduce should
   treat its own PASS verdicts as void.
6. **Require every PASS to name the evidence that would have caught the failure.** haiku-3 and
   haiku-4 both wrote PASS rows whose stated evidence, read carefully, does not bear on the
   claim. A template that forces "what input class did I test, and what would a failure have
   looked like" would have caught the escaping and ordering PASSes at authoring time.

---

## 7. Verdict table

Global. **PASS** means the stated evidence actually establishes the property.
**NOT ESTABLISHED** is preferred over any PASS that cannot be supported.

| Area                                                       | Verdict             | Evidence                                                                                                                                                                                                                                   |
| ---------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Undo round trip — names containing `\`**                 | **FAIL (CRITICAL)** | C-1, re-reproduced by the consolidator; permanently unrecoverable, reported as success                                                                                                                                                     |
| **Apply order (parent after its contents)**                | **FAIL**            | C-2, re-reproduced: preview lists the parent first, `-x` half-applies with 2 ENOENT                                                                                                                                                        |
| **Truncation / length limits**                             | **FAIL**            | C-3, re-reproduced on APFS: two legal 305-byte/105-UTF-16 names truncated to 255, collided, exit 0, `0 conflicts`, no note                                                                                                                 |
| **App-level idempotence (plan + apply)**                   | **FAIL**            | C-4, re-reproduced at the default limit; second `-x` renames again and kills batch 1's undo                                                                                                                                                |
| **Dotfile preservation**                                   | **FAIL**            | C-5, re-reproduced: `<ZWSP>.bashrc` → `bashrc`, `<BOM>.gitignore` → `gitignore`                                                                                                                                                            |
| **Argument → entry resolution (`walk.rs`)**                | **FAIL**            | C-6, re-reproduced: `detoxrs -x 'a b.txt'` renames `c d.txt`, and the preview names the wrong file                                                                                                                                         |
| **Report escaping / preview trustworthiness**              | **FAIL**            | C-7, exhaustively enumerated by the consolidator: 8 of 10 probe characters raw, 2 escaped; bidi reorder and row-splitting both demonstrated                                                                                                |
| **Exit-code fidelity**                                     | **FAIL**            | C-8, both classes re-reproduced: conflicts-only → exit 0; unreadable subtree → exit 0 with `0 failed`                                                                                                                                      |
| **Symlink integrity after a clean**                        | **FAIL**            | C-9, re-reproduced: `l ink -> t arget.txt` left dangling, `2 renamed, 0 failed`, exit 0                                                                                                                                                    |
| **Journal _filenames_ treated as untrusted**               | **FAIL**            | C-10, both re-reproduced: overflow panic at exit 101; `undo --last` reverts the wrong batch at exit 0                                                                                                                                      |
| **Undo accounting / reporting**                            | **FAIL**            | C-11, re-reproduced: `0 refused` for an item that was permanently lost                                                                                                                                                                     |
| **Collision numbering for non-recursive arguments**        | **FAIL**            | C-12, re-reproduced: `-2` chosen over an existing `a_b-2.txt`, refused with a false race message; `-r` picks `-3`                                                                                                                          |
| **Stage-13 convergence — test coverage**                   | **FAIL**            | C-13, consolidator-verified: bound 3 → 1 leaves all 166 tests green while changing 19 845 of 400 000 outcomes                                                                                                                              |
| **Performance of `detoxrs *`**                             | **FAIL**            | C-14, consolidator-measured, release build: 9.60 s vs 0.26 s for the same 3000 names                                                                                                                                                       |
| **`--json` contract completeness**                         | **FAIL**            | C-15, consolidator-verified: no `batch`, `journal` or `undo` key; and `note` is `null` even when a name was truncated                                                                                                                      |
| **Order-safety property test adequacy**                    | **FAIL**            | C-16: the generator emits one canonical spelling per level and the assertion is textual, so the property cannot fail for C-2                                                                                                               |
| **Journal location / permissions**                         | **FAIL (minor)**    | C-18, consolidator-verified: `drwxr-xr-x` / `-rw-r--r--` under `umask 022`, holding every renamed absolute path, never pruned                                                                                                              |
| **Undo ordering with nested directories**                  | **PASS**            | Structural, not incidental: `replay` inverts the forward journal. Consolidator-verified on a 3-level tree — `find \| sort` byte-identical, `3 reverted, 0 refused`. Also opus-3's 400 fuzz iterations                                      |
| **No-clobber rename (`RENAME_NOREPLACE`)**                 | **PASS**            | opus-2 probed six occupied-destination shapes on disk with content byte-identical after each; consolidator saw `apply`'s step 2 fire while reproducing C-12 and C-2                                                                        |
| **Round trip, general (apply → undo → byte-identical)**    | **PASS**            | opus-3, isolated snapshot: 400 randomized iterations with `\` excluded, 0 manifest diffs, inodes preserved; 78-entry hand-built nasty tree identical except C-1                                                                            |
| **Durability of `intent` before rename**                   | **PASS**            | `write_all` + `sync_data` before `rename_noreplace`; ordering asserted by a shared event log in `apply::tests`; opus-3's `kill -9` on a 4000-file run showed no rename without a preceding record                                          |
| **Journal _record_ trust boundary (traversal in from/to)** | **PASS**            | Two reviewers re-read it line by line; the existing test drives a forged journal through the real apply loop and asserts on disk. (This same check is C-1's cause — the mechanism is right, the character set is wrong)                    |
| **Undo of a partially applied batch (`kill -9`)**          | **PASS**            | opus-3: 4000-file run killed at 0.25 s, 20 renames done, the intended file verifiably not renamed, all 20 restored                                                                                                                         |
| **Concurrent `-x` runs on one journal directory**          | **PASS**            | opus-3: 12 simultaneous runs → 12 distinct journals, 480 renames reported and 480 observed, no interleaved records                                                                                                                         |
| **Undo clobbering a file created since the rename**        | **PASS**            | opus-3: original name recreated with different content, undo refused per item, both files byte-identical, exit 1                                                                                                                           |
| **Transform safety closure (pure core)**                   | **PASS**            | opus-1, isolated clone: 400 000 adversarial names × random limits 1–24 — no output ever held a control or separator character, was empty, `.`, `..`, over either limit, or non-NFC                                                         |
| **`transform` idempotence (pure core)**                    | **PASS**            | Same 400 000 inputs: `transform(transform(x)) == transform(x)` always, never `Unrepresentable` on the second pass. (App level is C-4 — a different question)                                                                               |
| **Grapheme integrity in truncation**                       | **PASS**            | Cluster count never rises across the pipeline once stage 4's deliberate exception is excluded (400 000 inputs); mutating to char boundaries fails 2 tests                                                                                  |
| **NFC / normalization smuggling**                          | **PASS**            | Stage 3 runs before stage 7, so `U+037E` (NFC → `;`) and `U+0338` compositions cannot smuggle a metacharacter past the safe map; asserted on 400 000 inputs containing both                                                                |
| **`decode.rs` — non-UTF-8 handling**                       | **PASS**            | `OsStr::to_str` only, no lossy path exists; `key_of_os`'s `0xFF` tagging makes the opaque and text key spaces provably disjoint. Non-UTF-8 names skipped as `NotUtf8` before any destination is computed                                   |
| **Collision engine — no clobber, no chain (intra-batch)**  | **PASS**            | opus-1: 40 000 random directories × {Sensitive, Insensitive} × {Number, Skip} — no two items ever on the same exact name, no two source keys merged, no `InternalInconsistency`                                                            |
| **Directory-fd pinning within one item**                   | **PASS**            | `CountingOps` asserts `opens == 1` deterministically; `fsops::tests::the_rename_follows_the_pinned_directory_not_the_path` asserts on disk after a real directory swap. Residual gap is C-22 (before step 0)                               |
| **`report::escape` injectivity**                           | **PASS**            | Distinct question from C-7: non-`Cc` characters pass through unmodified and every literal `<` is escaped as `<3c>`, so distinct names render distinctly. Traced by opus-3, re-checked by the consolidator                                  |
| **Symlink roots, loops, deep trees, non-descent**          | **PASS**            | Both `link` and `link/` spellings clean the link's own name without descending; `sub/u p -> ..` plus `se lf -> sub` neither descends nor hangs; 120-deep tree → 121 correct renames                                                        |
| **Hardlink handling in `fsops` (C5 machinery)**            | **PASS**            | `ln 'a b.txt' 'a_b.txt'; detoxrs -x 'a b.txt'` → `EEXIST`, exit 1, both names and the shared inode intact. (Distinct from C-6, which is a `walk.rs` argument-resolution defect)                                                            |
| **Read-only / EACCES directory**                           | **PASS**            | Per-item `EACCES`, batch continues, exit 1, sibling still renamed                                                                                                                                                                          |
| **No rename without a journal**                            | **PASS**            | `HOME` and `XDG_STATE_HOME` both unset, and unwritable `XDG_STATE_HOME`: exit 2, `nothing was renamed`, file untouched; preview still works                                                                                                |
| **Unsafe code**                                            | **PASS**            | `#![forbid(unsafe_code)]` in both crates — a compiler-enforced claim, the one haiku-3 assertion that needs no runtime evidence                                                                                                             |
| **Panic-freedom in production code**                       | **FAIL**            | C-10: `journal.rs:149` panics on a crafted filename. haiku-3's PASS grepped for `unwrap`/`expect` and missed arithmetic overflow                                                                                                           |
| **Allocator respell logic — coverage**                     | **FAIL**            | C-21: `all()` → `any()` survives. Downgraded from HIGH: no-clobber is enforced independently below the plan layer                                                                                                                          |
| **Truncation / numbering boundary coverage**               | **FAIL**            | C-26 (`<= 4` → `< 4` survives), C-27 (`..=999` → `..999` survives)                                                                                                                                                                         |
| **Plan→apply divergence**                                  | **NOT ESTABLISHED** | haiku-4's PASS is void: its evidence describes apply renumbering a destination, which apply never does (`apply.rs:28-39`; no allocator in the CLI crate). The apply-time rechecks _are_ separately established as load-bearing by mutation |
| **TOCTOU under an active attacker**                        | **NOT ESTABLISHED** | The deterministic guards are covered (mutations caught) but no live race was driven by anyone. haiku-3's TOCTOU PASS ran in the contaminated tree and describes a swap it did not verify timing for. C-22 records the reasoned gap         |
| **Power-loss durability**                                  | **NOT ESTABLISHED** | `sync_data` on the file only, no directory fsync, no `F_FULLFSYNC`. Documented as out of scope in `journal.rs`; untested. C-25 is the reporting consequence                                                                                |
| **Cross-device / mount points**                            | **NOT ESTABLISHED** | Renames are always intra-directory so cross-device rename is unreachable, but nothing was tested about renaming a mount point's own entry, and `walkdir` is not restricted to one filesystem                                               |
| **Non-UTF-8 _directory_ path through the journal**         | **NOT ESTABLISHED** | APFS rejects invalid UTF-8 filenames (`EILSEQ`), so the `dir_bytes` path could not be exercised end to end on this host. Needs a Linux run                                                                                                 |
| **Windows / non-Unix tier**                                | **NOT ESTABLISHED** | Not runnable here. The code documents its own degradation honestly (`ident_at_path` zeroes `dev`/`ino`, `dir_has_literal_entry` returns `false`), but "documented" is not "verified". C-20 is the specific concern                         |
| **Resource exhaustion**                                    | **NOT ESTABLISHED** | haiku-3's PASS ran in the contaminated tree at depth 20 with a 255-byte name — well inside every limit. C-14 shows one input shape that is 37× slower than it should be, which that test class would not have found                        |
| **Supply chain / advisories**                              | **NOT ESTABLISHED** | haiku-3 reports its audit tool timed out and rates its own conclusion PLAUSIBLE. Six mainstream direct dependencies, unverified advisory state                                                                                             |
| **Prior defects C1, C2, C4, C7, C9, C10 (from `04974e2`)** | **NOT ESTABLISHED** | haiku-1 explicitly did not test six of the eleven prior-defect rows it tabulated. C1's successor (the journal record trust boundary) _is_ established as PASS by opus-3, independently                                                     |

---

## 8. What a fix pass should tackle first

Recommendations only. Ordered by (harm × reachability), with cheap high-value fixes pulled
forward where they are prerequisites.

1. **C-1 — delete the `\` clause from `is_plain_basename`.** One line. opus-3 verified, and the
   consolidator re-verified, that removing `&& !bytes.contains(&b'\\')` fixes the round trip
   and leaves all 166 tests green. This is the only unrecoverable defect in the set and the
   cheapest fix in it. Add both missing tests while you are there: one asserting `/` and `.`
   and `..` are still rejected, one asserting a `\` name round-trips.

2. **C-11 — make `undo`'s summary count what it dropped.** Also small, and it is what turns
   C-1 (and any future record-rejection) from silent into loud. Compare the batch's `done`
   count against the number of items replayed and report the difference. Do this in the same
   pass as C-1 so that if another record class is ever rejected, the user is told.

3. **C-2 — order by containment, not by string length.** Sort on `dir_ident` ancestry, or
   canonicalise `dir` once in the walk before it reaches `plan`. The information is already
   computed and thrown away. Fix **C-16** in the same change or the fix is unguarded: teach
   `build_snapshot` to emit a `..` spelling and an absolute spelling of one directory, and
   replace the textual `starts_with` assertion with one in identity units. Consider **C-23**
   here too — it is the same spelling-vs-identity confusion, one function over.

4. **C-3 — stop applying a byte budget as a UTF-16 budget, and report truncation when it
   happens.** Two separable pieces, and the _reporting_ piece is the urgent one: a truncation
   that the user sees is an annoyance, a truncation that silently collides two files is data
   loss. Plumb `Outcome.truncated` into `PlanItem`, `report::line` and `report::json` first —
   it is already computed, already property-tested, and currently dropped one line after it is
   produced. Then split the byte limit from the UTF-16 limit so APFS's 255-code-unit limit is
   what governs on APFS. A truncation that creates a collision should arguably refuse rather
   than number.

5. **C-7 — escape the whole invisible set in `report.rs`, not just `Cc`.** The set to escape
   already exists in `detoxrs-core/src/invisible.rs`, so there is no new classification work:
   route `escape_text`'s predicate through it, and add `Zl`/`Zp`. Do this before shipping any
   of the above, because the preview is how a user is supposed to catch every other defect
   here, and right now the input can falsify it. Add a test asserting a `U+202E` name and a
   `U+2028` name each render on one line with a token.

6. **C-4 — make `numbered()` return a fixed point of `transform`, or reject.** The invariant
   §5.3 depends on is that every destination is a fixed point; assert it. Turning the existing
   `debug_assert!(fits(...))` into a check that also verifies `transform(candidate) ==
candidate` would have caught this at development time and costs nothing in release.

7. **C-6 and C-14 together — stop resolving arguments by inode scan.** One root cause,
   two symptoms (a wrong entry renamed, and O(n²) `lstat`). `real_entry_name` exists to
   recover the on-disk spelling of an argument; inode identity cannot do that job because it is
   not unique per directory entry. Use the argument's own basename and verify identity, rather
   than searching for identity and adopting whatever name it finds. This is the fix that
   removes a quadratic term and a trust violation in the same diff.

8. **C-5 — count `leading_dots` after stage 4, not before.** Small, and the doc comment already
   promises the behaviour the fix would deliver.

9. **C-8 and C-24 — make the exit code carry "I could not do what you asked."** Conflicts and
   unread subtrees both need to reach `exit_code()`. `WalkError` already exists for the second
   class and is deliberately unused; using it, or threading an `incomplete` flag, is the
   minimum. C-24 (unclosed journal) is the same shape and belongs in the same change.
   `detoxrs -x -r . && echo clean` must not print `clean` when a subtree was never inspected.

10. **C-10 — validate journal filenames.** Reject any filename whose leading token is not
    exactly six digits, and use `checked_add`. Two guards, and they close a panic and a
    wrong-batch undo together. Pair with **C-18** (create the journal directory `0700` and the
    file `0600`), which is the condition that makes C-10's planted-file route practical.

11. **C-9 — warn when a batch breaks a symlink it can see.** The information needs no extra
    I/O: the link's target is a name, and the batch already knows every name it is about to
    change in that directory. A per-item note in the preview is enough; refusing would be
    wrong.

12. **C-12 — seed the numbered candidates, or plan non-recursive arguments through the same
    snapshot path as `-r`.** The second is the smaller change and removes the divergence rather
    than patching it. Fixing this also removes a false diagnostic that sends users hunting for
    a concurrent writer.

13. **C-13, C-21, C-26, C-27 — close the four surviving mutations.** Cheap, mechanical, and
    each one is a guard that is currently free to change. C-13 is the most valuable: a test
    that genuinely needs a second stage-13 iteration.

14. **C-15 — put `batch` and `journal` in the `--json` document.** Two fields, and without them
    the declared stable contract cannot reach `undo` at all.

15. **Remaining LOW items** (C-17, C-19, C-20, C-22, C-25, C-28, C-29) as convenient. **C-22**
    is the one worth pulling forward if any of the security-adjacent items get attention: one
    `u64` pair in `PlanItem` plus one comparison closes the gap between the walk and step 0,
    and `O_NOFOLLOW` on discovered subdirectories (as distinct from command-line arguments) is
    a one-flag change that matches what the walk already refused to do.

**Before any of it: fix the review process (§6).** The next pass should give each reviewer its
own worktree and its own `CARGO_TARGET_DIR`. Two of seven reviews in this round produced
verdicts that cannot be trusted, and three of the seven most severe findings sit inside an
area another reviewer had already marked PASS.
