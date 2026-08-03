# Full-app adversarial review — reviewer opus-1

Emphasis: core transform correctness (`crates/detoxrs-core`), whole app in scope.
Read-only review. No production code was changed; every mutation listed below was
made in a **clone of HEAD** under the session scratchpad, never in the repo.

## How this was done

- `cargo test --workspace` in the working tree **failed** on first run (5 failures in
  `truncate::tests`). That was not a real defect: another reviewer was live
  mutation-testing `crates/detoxrs-core/src/truncate.rs` in the shared working
  directory. Everything below was therefore run against
  `git clone --no-hardlinks` of `a144fe9` in the scratchpad, where
  `cargo test --workspace` is fully green (39 + 19 + 6 + 14 + 8 + 56 + 3 + 8 + 11 + 1 + 1 passed,
  0 failed). Anyone re-running the suite in the shared tree should check
  `git status` first.
- Hands-on probes: a throwaway integration test (`tests/probe_opus1.rs` in the clone)
  driving `transform` and `plan` over 400 000 random names built from a 47-character
  adversarial alphabet (combining marks, ZWJ/ZWSP/BOM, RTL override, Tags, astral
  emoji, NUL/CR/DEL, NBSP, U+2028, U+037E, U+0338, U+212A, U+FB01) at randomised
  byte/UTF-16 limits from 1 to 24, plus 40 000 random multi-entry directories run
  through `plan` under both `VolumeCase` values and both `Number` and `Skip`.
- End-to-end runs of the built `detoxrs` binary against real temporary trees.
- Four mutations of specific guards, each applied to the clone and reverted immediately.

---

## Findings

### O1-1 — HIGH — `numbered()` can emit a destination the pipeline itself would rename again, breaking idempotence end to end

`crates/detoxrs-core/src/plan.rs:593-597`

`numbered()` builds a collision candidate as `truncate_graphemes(stem, budget) + "-N" + ext`
and never checks that the result is a fixed point of `transform`. When the stem is
truncated to make room for the suffix, the kept prefix can end in `-`, and the appended
`-N` then produces a `--` run — which stage 9 (`collapse`) would squeeze away. The plan's
whole safety argument in §5.3 rests on `transform` being idempotent, and this path
manufactures a name that is not a fixed point of it. `debug_assert!(fits(...))` checks the
length but nothing checks safety closure.

**Concrete failure scenario** (reproduced, real output). Two files whose cleaned names
collide at the default M1 255-byte limit; `a` × 248 abbreviated below:

```
$ ls
aaa…aaa-_b.txt      # 255 bytes, already clean
aaa…aaa- b.txt      # 255 bytes, dirty; cleans to the same name

$ detoxrs -x -r .
…aaa- b.txt  ->  …aaa--2.txt
1 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.

$ detoxrs -r .          # second run on the tree detoxrs just produced
  …aaa--2.txt  ->  …aaa-2.txt
1 to rename, 1 unchanged, 0 skipped, 0 conflicts.
```

The tool renames its own output. Consequences: `detoxrs -x` is not a fixed point, so a
second `-x` produces a third name and a second journal batch; and the first batch's undo
is invalidated by the second run (its recorded `current` name no longer exists), so
`detoxrs undo <batch-1>` after two forward runs refuses every item.

Minimal pure-core form, also reproduced:

```
Policy::new('_', 9, 9); entries ["ab-cd .txt", "ab-cd.txt"]
  "ab-cd .txt" -> "ab--2.txt"   Rename
  transform("ab--2.txt")  ==  "ab-2.txt"     // not a fixed point
```

The 40 000-directory fuzz found the same class independently at other limits
(`"é\"]-" -> "é_--2"`, and `transform("é_--2") == "é_-2"`): 4 non-fixed-point
destinations in that sample.

Confidence: **CONFIRMED**.

---

### O1-2 — HIGH — an invisible character in front of a dotfile destroys its dotfile status

`crates/detoxrs-core/src/pipeline.rs:196` (with `trim`, `:173-186`)

`leading_dots` is counted from `run_with`'s **original input**, before stage 4 deletes
invisibles. `trim` then strips every leading dot and restores exactly `leading_dots` of
them. If the input begins with an invisible character followed by dots, the count is 0,
stage 4 removes the invisible, and `trim` strips the now-leading dots and restores none.
A hidden file becomes a visible one, and a config file stops being read by whatever
expects a dotfile. The doc comment claims Dotfile preservation "in both directions".

**Concrete failure scenario** (reproduced, real output):

```
$ ls -b
\342\200\213..hidden        # U+200B ZWSP + "..hidden"
\342\200\213.bashrc         # U+200B ZWSP + ".bashrc"
\357\273\277.gitignore      # U+FEFF BOM  + ".gitignore"

$ detoxrs -r .
  ​..hidden    ->  hidden
  ​.bashrc     ->  bashrc
  ﻿.gitignore  ->  gitignore
```

`.bashrc` -> `bashrc`, `.gitignore` -> `gitignore`. BOM-prefixed names are not exotic:
they come out of Windows-authored zips and CSV/text tooling routinely.

The 200 000-input probe found this is one-directional: dots are only ever **lost**, never
manufactured, so no non-dotfile becomes hidden. Every failing case has the same shape
(`("\u{200b}.B", "B", want 1, got 0)`).

Confidence: **CONFIRMED**.

---

### O1-3 — HIGH — a hardlink in the same directory makes detoxrs rename an entry the user never named

`crates/detoxrs/src/walk.rs:172-183` (`real_entry_name`, called from
`corrected_top_level_path`, `:161`)

For every top-level argument, `real_entry_name` lists the containing directory and returns
the name of the **first** entry whose `(dev, ino)` matches the argument's. Inode identity is
not a unique key for a directory entry: two hardlinks in one directory share it. `readdir`
order then decides which name detoxrs believes it was given.

**Concrete failure scenario** (reproduced, real output):

```
$ ls -bi
154799347 a b.txt
154799347 c d.txt       # hardlink to the same inode

$ detoxrs 'a b.txt'
  c d.txt  ->  c_d.txt [hardlink, nlink=2]
1 to rename, 0 unchanged, 0 skipped, 0 conflicts.

$ detoxrs -x 'a b.txt'
c d.txt  ->  c_d.txt
1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.

$ ls -b
a b.txt
c_d.txt
```

The user asked for `a b.txt`; detoxrs previewed and renamed `c d.txt`, and left the named
file dirty. No data is lost (the inode is shared), but the tool acted on a directory entry
outside the argument it was given, and its own preview named the wrong file — so the
preview cannot be used to catch it. The `#[cfg(unix)]` fallback path (`:185`) does not have
this problem because it does not do the lookup at all.

Confidence: **CONFIRMED**.

---

### O1-4 — MEDIUM — the preview does not escape bidi, zero-width or line-separator characters

`crates/detoxrs/src/report.rs:380-397` (`escape_text`; `escape`, `:333`)

`escape_text` escapes `char::is_control()` (Unicode `Cc`) and `<`. It does not escape `Cf`
bidi controls, zero-width characters, or `Zl`/`Zp`. The preview is the tool's primary safety
control — "preview by default", and the CVE-2021-42574 advisory is cited as the reason
stage 4 exists — yet the preview renders exactly those characters raw to the terminal,
so the line a user reads before typing `-x` can be visually reordered by the filename
being reviewed.

**Concrete failure scenario** (reproduced, real output). A file named
`invoice<U+202E>gpj.exe` and one named `safe<U+200B>name.txt`:

```
$ detoxrs -r . > out.txt 2>&1
$ cat out.txt
.
  invoice‮gpj.exe  ->  invoicegpj.exe
  safe​name.txt    ->  safename.txt

$ python3 -c "d=open('out.txt','rb').read(); print(b'\xe2\x80\xae' in d, b'\xe2\x80\x8b' in d, b'<u+202e>' in d)"
True True False
```

The RTL override and the ZWSP are in the output byte-for-byte; no escape token is emitted.
The same hole covers `U+2028 LINE SEPARATOR`, which many terminals and editors treat as a
line break — `escape_text` passes it through, so one report row can render as two (also
reproduced; see O1-8).

Confidence: **CONFIRMED**.

---

### O1-5 — MEDIUM — collision numbering is blind past `-2` for single-file and non-recursive arguments, and reports a race that did not happen

`crates/detoxrs/src/walk.rs:290-320` (`seed_pre_existing_destination`)

The function seeds exactly one pre-existing entry: the **unnumbered** destination
`dir.join(&wanted.text)`. `plan()` is I/O-free, so if that name is taken it renumbers to
`-2` — a name nothing ever checked against the filesystem. `apply`'s step-2 recheck then
finds the occupant and refuses the item with "appeared since the preview", which is a false
statement: the file was there before the walk started.

**Concrete failure scenario** (reproduced, real output):

```
$ ls -b
a b.txt  a_b-2.txt  a_b.txt

$ detoxrs -x 'a b.txt'; echo "exit=$?"
detoxrs: a b.txt: a_b-2.txt appeared since the preview; not renamed
0 renamed, 1 unchanged, 0 skipped, 0 conflicts, 1 failed.
exit=1

$ detoxrs -r .          # same tree, snapshot contains everything
  a b.txt  ->  a_b-3.txt
```

`-r` gets it right (`-3`); the single-file argument gets it wrong and blames a race. Nothing
is clobbered — the apply-time guard holds — but `--on-collision number` silently does not
work outside a recursive walk, and the diagnostic misdirects the user to look for a
concurrent writer.

Confidence: **CONFIRMED**.

---

### O1-6 — MEDIUM — `detoxrs *` is quadratic: one full directory listing plus an `lstat` per entry, per argument

`crates/detoxrs/src/walk.rs:172-183`

`real_entry_name` calls `fs::read_dir` and then `entry.metadata()` on every entry, for
**every** top-level argument, until it finds a matching inode. For `detoxrs *` in a
directory of _n_ files that is O(n²) `lstat` calls. `detoxrs *` is the shell-native way to
invoke a non-recursive run.

**Concrete failure scenario** (reproduced, real timings, 3000 files named `f %04d.txt`):

```
$ time detoxrs -r . >/dev/null      # recursive discovery, no correction pass
0.02s user 0.01s system  0.036 total

$ time detoxrs * >/dev/null         # 3000 top-level arguments
0.86s user 8.39s system 10.466 total
```

290× slower for the same 3000 names, and 8.4 s of it is system time. Extrapolating the
quadratic term, 10 000 arguments is roughly two minutes.

Confidence: **CONFIRMED**.

---

### O1-7 — MEDIUM — the stage-13 convergence loop has no coverage above one iteration, though the second iteration is load-bearing for ~5% of dirty names

`crates/detoxrs-core/src/pipeline.rs:58` (`FIXED_POINT_BOUND`), `:240-263`

Mutating `FIXED_POINT_BOUND` from 3 to 1 leaves the **entire workspace suite green**
(`cargo test -p detoxrs-core --lib`: 56 passed, 0 failed; full workspace also green), yet
that mutation changes observable behaviour for a large fraction of inputs: with the bound at
1, 19 845 of my 400 000 probe inputs come back `Unrepresentable(NotConverged)` instead of a
name (78 073 unrepresentable vs 58 673 at bound 2 or 3). So no test in the suite exercises a
second iteration of the loop, and the constant that decides how many iterations there are is
free.

Measured (clone, each value reverted after):

| bound | unrepresentable / 400 000 | NotConverged |
| ----- | ------------------------- | ------------ |
| 1     | 78 073                    | 19 845       |
| 2     | 58 673                    | 0            |
| 3     | 58 673                    | 0            |

Examples that need the second pass: `"%>\u{7f}-\u{2028}\u{7f}"` at limit 2,
`"B\u{202e}!!B/"` at limit 2, `"%@=:😀!=\u{301}"` at limit 6.

A `NotConverged` is reported and skipped rather than mis-renamed, so this is a coverage and
regression-risk defect rather than a live one — but "the bound is never tight" is exactly the
claim the comment at `:57` makes, and it is untested in the direction that matters. Bound 3
is also unexercised: nothing in 400 000 samples needs a third pass.

Confidence: **CONFIRMED** (mutation, reverted; `git status` clean afterwards).

---

### O1-8 — LOW — `U+2028`/`U+2029` and the `Zs` whitespace set are declared "safe" and pass through

`crates/detoxrs-core/src/invisible.rs:23-39`, `crates/detoxrs-core/src/classes.rs:36-44`

`is_invisible` is a named set that stops at bidi controls, zero-width characters and Tags;
`classify` deletes only `Cc`. `U+2028 LINE SEPARATOR`, `U+2029`, `U+180E`, `U+00A0` and the
`U+2000`–`U+200A`/`U+202F`/`U+3000` spaces are all `Keep`. This is documented as an M4
deferral in `invisible.rs`'s module comment, so it is a known gap rather than an oversight —
recorded here because it interacts with O1-4: a name containing `U+2028` survives the
transform _and_ is printed raw, so the preview a user reads can gain a line break that hides
a row.

Reproduced: `a<U+2028>b .txt` -> `a<U+2028>b.txt` (declared clean and safe), and the raw
`e2 80 a8` bytes appear in the report.

Confidence: **CONFIRMED** (behaviour), deferral is documented.

---

### O1-9 — LOW — the sibling-chain assertion keys on `dir`'s spelling while both collision layers key on `dir_ident`

`crates/detoxrs-core/src/plan.rs:484-497`

`check_no_sibling_chains` builds `vacated: HashMap<(&Path, &str), usize>` from
`entries[i].dir.as_path()`. Layer 1's `wants` and the `Allocator` deliberately key on
`dir_ident` instead, precisely because `.`, `""` and `./x` are three spellings of one
directory (C8, `plan.rs:93-98`). A genuine rename chain between two entries the walk reached
under different spellings of the same directory would land in two buckets here and the
assertion would not fire. It can only fail _open_ (miss a chain), never closed, and the
chain it guards is argued unreachable — but the guard is weaker than the layers whose
proof it defends, for no reason a reader can see.

Confidence: **PLAUSIBLE** (reasoned from the code; the dedupe in `walk.rs` makes a
one-directory two-spelling snapshot hard to construct, so I did not reproduce it).

---

### O1-10 — LOW — an `ident_at` error at the destination is read as "vacant" on both the atomic and the demoted rung

`crates/detoxrs/src/apply.rs:187`, `crates/detoxrs/src/fsops/fallback.rs:243`

`if let Ok(occupant) = ops.ident_at(&dir, &item.to)` and
`if ops.ident_at(dir, to).is_ok() && …` both treat _any_ error — not just `ENOENT` — as an
unoccupied destination. On tier-1 the kernel's `RENAME_NOREPLACE` still refuses, so the
consequence is only a worse error message. On the demoted rung (`check_then_rename`,
reached on `EINVAL`/`ENOSYS`/`EOPNOTSUPP`, and on the whole Windows tier) there is no kernel
guard: an `lstat` that fails for a reason other than absence falls through to
`rename_plain`, which replaces. Matching on the error kind rather than `is_ok()` would close
it. Low likelihood — most ways for that `lstat` to fail also make the `renameat` fail — but
the demoted rung's documented promise is "still never clobbers", and this is the one shape
where it could.

Confidence: **PLAUSIBLE** (not reproduced; I could not construct a directory where
`fstatat` fails and `renameat` succeeds).

---

## Verdict table

| Area                                                | Verdict         | Evidence                                                                                                                                                                                                                                            |
| --------------------------------------------------- | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `decode.rs` — non-UTF-8 handling                    | PASS            | `OsStr::to_str` only; no lossy path exists. Non-UTF-8 name skipped as `NotUtf8` and never renamed (repo test + `key_of_os`'s `0xFF` tagging makes the opaque and text key spaces provably disjoint).                                                |
| Transform safety closure                            | PASS            | 400 000 random adversarial names × random limits 1–24: no output ever contained a control character, a separator-class character, was empty, was `.`/`..`, exceeded either limit, or was non-NFC.                                                   |
| `transform` idempotence (pure core)                 | PASS            | Same 400 000 inputs: `transform(transform(x)) == transform(x)` in every case, and never `Unrepresentable` on the second pass.                                                                                                                       |
| **App-level idempotence (plan + apply)**            | **FAIL**        | O1-1: a numbered destination is not a fixed point; reproduced end to end at the default 255-byte limit, the second run renames again.                                                                                                               |
| Grapheme integrity in truncation                    | PASS            | Cluster count never rises across the pipeline once stage 4's deliberate exception is excluded (400 000 inputs). Mutating `truncate_graphemes` to char boundaries fails 2 tests, so the rule is genuinely covered.                                   |
| NFC / normalization smuggling                       | PASS            | Stage 3 runs before stage 7, so `U+037E` (NFC → `;`) and `U+0338` compositions cannot smuggle a shell metacharacter past the safe map; asserted on 400 000 inputs containing both.                                                                  |
| **Dotfile preservation**                            | **FAIL**        | O1-2: reproduced, `\u{200b}.bashrc` -> `bashrc`. Loss is one-directional (never manufactures a dotfile) — 200 000-input probe.                                                                                                                      |
| Stage-13 convergence (does it terminate correctly?) | PASS            | 0 `NotConverged` in 400 000 inputs at the shipped bound of 3.                                                                                                                                                                                       |
| **Stage-13 convergence — test coverage**            | **FAIL**        | O1-7: bound 3 → 1 keeps the whole suite green while changing 19 845 outcomes.                                                                                                                                                                       |
| Collision engine — no-clobber                       | PASS            | 40 000 random directories × {Sensitive, Insensitive} × {Number, Skip}: no two items ever end on the same exact name, and no two distinct source keys are ever merged onto one destination key. No `InternalInconsistency`.                          |
| Collision engine — guard coverage                   | PASS            | Mutating `Allocator::is_free` to always return `true` fails 5 named `plan::tests` (`a_pre_existing_destination_is_numbered_around`, `an_existing_numbered_name_is_respected`, `skip_leaves_both_sides_alone`, +2).                                  |
| **Collision numbering for non-recursive args**      | **FAIL**        | O1-5: reproduced, `-2` chosen over an existing `a_b-2.txt`, refused at apply time with a false race message; `-r` on the same tree picks `-3`.                                                                                                      |
| Apply-time TOCTOU / identity recheck                | PASS            | Repo tests are strong here (pinned dirfd, `CountingOps` open-count guard, intent-before-rename asserted through a shared event log). My own runs never saw a clobber; the O1-5 case proves the destination recheck fires.                           |
| **Argument → entry resolution (`walk.rs`)**         | **FAIL**        | O1-3: reproduced, `detoxrs -x 'a b.txt'` renamed `c d.txt`.                                                                                                                                                                                         |
| Undo round trip (incl. nested directories)          | PASS            | Ran `-x -r .` over `d ir/s ub/f ile.txt`, then `undo --last`: 3 reverted, tree byte-identical to the original, undo itself recorded as batch 2.                                                                                                     |
| **Report escaping**                                 | **FAIL**        | O1-4: raw `U+202E`, `U+200B`, `U+2028` bytes present in report output; no escape token emitted.                                                                                                                                                     |
| Performance of the common invocation                | **FAIL**        | O1-6: 3000 args = 10.5 s vs 0.036 s for `-r .`; quadratic.                                                                                                                                                                                          |
| Hardlink handling in `fsops`                        | NOT ESTABLISHED | The C5 machinery (`is_same_entry_not_hardlink`, `dir_has_literal_entry`) reads as correct and is directly tested on both rungs, but I only traced it; I did not fault-inject the demoted rung. See O1-10 for the one gap I do have a concern about. |
| `journal.rs` durability / crash recovery            | NOT ESTABLISHED | Not exercised beyond the happy-path round trip above. `917` lines unreviewed in depth; no claim either way.                                                                                                                                         |
| Windows / non-Unix tier                             | NOT ESTABLISHED | Cannot run it; the code documents the degradation honestly (`ident_at_path` zeroes `dev`/`ino`, `dir_has_literal_entry` returns `false`), but "documented" is not "verified".                                                                       |

## Note for the coordinator

`cargo test` in `/Users/kerry.hatcher/projects/detoxrs` was failing when I started because a
concurrent reviewer had an unreverted mutation in `crates/detoxrs-core/src/truncate.rs`
(`split_extension`'s `<= 4` changed to `< 4`). It has since been reverted. I made no edits
to the working tree; all my probes and mutations ran in a scratchpad clone.
