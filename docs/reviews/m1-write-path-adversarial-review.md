# M1 write path — adjudicated adversarial review

**Subject:** detoxrs @ `04974e2` (`fix(cli): seven defects from a six-reviewer adversarial pass`)
**Adjudicator method:** every ruling below was re-derived in an isolated pinned checkout
(`git archive 04974e2 | tar -x`), verified file-by-file against `git show 04974e2:` by md5
(6/6 load-bearing sources identical), built there, and run there. Mutations were applied with
`os.utime()` after both apply *and* revert, built with `cargo test --workspace --no-run`, and the
**compiled test binaries were executed directly** — see §5 for why that matters, and for the one
place I fell into the trap myself and caught it.
Baseline in the pinned checkout: **138 tests, 0 failures.**
Real working tree confirmed clean before and after (`git status --porcelain` empty; only this file added).

Inputs adjudicated: three full Opus reviews (A, B, C) and a digest of six earlier Sonnet-tier
reviewers (R1–R6). Every claim was treated as an allegation.

---

## 1. Adjudicated verdict: **FAIL**

Not a close call, and not for the reasons the majority weighted highest.

The safety core is genuinely strong and I did not manage to destroy a byte either. Nine reviewers
plus this pass have now failed to break no-clobber, atomicity, the identity recheck, the crash
protocol, ordering, or the transform's safety closure, across roughly 1.6 million fuzzed transform
cases, ~20 000 attempted concurrent renames, and dozens of `kill -9` runs. **The dirfd pin is real**
— I reproduced B's discriminating race and got 0/30 wrong-file renames at HEAD versus 5/30 with 14
false journal successes at a one-line mutant. That is the strongest positive evidence in the run and
it should not be re-litigated.

It fails on four things, any one of which is disqualifying for a tool whose entire pitch is "preview
by default, and you can always undo":

1. **The undo path takes `from`/`to` verbatim out of a journal record and hands them to
   `statat`/`renameat`.** A record with `../..` or an absolute path moves a file clean out of the
   pinned directory, prints a normal-looking revert, and exits 0. This falsifies the exact anchor
   guarantee the dirfd refactor exists to establish (`fsops.rs:111-112`, §5.2), on the one file the
   project itself calls a trust boundary. Verified both relative and absolute.
2. **One invalid UTF-8 byte anywhere in a journal makes the entire batch un-undoable** (`exit 2`,
   nothing recovered), because `replay` reads the whole file with `fs::read_to_string`. The same
   corruption at the same offset with a valid-UTF-8 byte recovers 5 of 5 items. This is a
   data-recoverability defect and it defeats the per-line fault tolerance the file documents. **No
   Opus reviewer found it**; it came from R2 alone and nobody corroborated it until now.
3. **`detoxrs -x -r 'somelink/'` descends a symlinked directory and renames files outside the named
   tree**, falsifying `walk.rs:20-22`'s absolute, flag-free guarantee, on a spelling shell tab
   completion produces by default.
4. **The regression guard for the previous pass's worst defect is in the wrong layer.** Re-introducing
   that defect inside `apply::attempt` leaves `cargo test` fully green while producing wrong-file
   renames with false journal `done` records under an ordinary directory-swap race.

On top of that: a hardlinked destination reports `1 renamed` / exit 0 having renamed nothing; a
broken pipe turns a partially-applied batch into exit 2, the code documented as "nothing was
attempted at all"; and an undo that reverts nothing still writes a journal that shadows `undo --last`
and makes the next one exit 0 on a no-op.

### What would have to change to reach a pass

Necessary and, I believe, sufficient:

- **D1** `parse_intent` rejects any `dir`/`from`/`to` that is not a single non-`..` component
  (`from`/`to`) or is not absolute (`dir`), as a `Replay::anomaly`.
- **D2** `replay` reads bytes and decodes per line, so one bad byte costs one record, not the batch.
- **D3** `snapshot` treats a walk root whose `lstat`-with-trailing-slash differs from its
  `lstat`-without as a symlink (or strips trailing separators before the `lstat`), so
  `follow_links(false)` governs the root too.
- **D4** rung 1 / `apply::attempt` step 2 distinguishes "same inode, same entry" (a respell) from
  "same inode, different entry" (a hardlink) and returns `AlreadyExists` for the latter.
- **D5** a reporting/flush failure after `apply::run` returns must not promote to exit 2, and a
  per-item progress write failure must not abort the batch.
- **D6** the undo journal is created only once `replay.items` is non-empty *and* something has
  actually been reverted; the forward path's `main.rs:97-99` guard applied to the undo path.
- **D7** the walk/plan dedup and collision keys use `Ident` (dev, ino), not the path spelling.
- **D8** tests: an `apply`-layer assertion that one `Dir` handle serves all three operations; a
  `replay` mismatched-outcome test; and a decision recorded in code about the three unpinned
  `sync_data()` calls (either an assertion via a `JournalWrite` double, or a comment stating the
  guard is deliberately behavioural-only).
- **D9** `README.md` and `CHANGELOG.md` stop describing a program that does not exist, and
  `docs/HANDOFF.md:60-61` stops contradicting `:141-146`.

Everything else below is real but not blocking.

---

## 2. Confirmed defects, ranked

Line numbers are as of `04974e2`. "Re-verified" means I ran it in the pinned checkout and pasted the
actual output.

---

### C1 — HIGH — `undo` replays journal names without basename validation; the rename escapes the pinned directory

**Where:** `crates/detoxrs/src/journal.rs:385-407` (`parse_intent` copies `dir`/`from`/`to` verbatim
into `UndoItem`), consumed at `crates/detoxrs/src/apply.rs:169-204` (`ops.open(&item.dir)`,
`ops.ident_at(&dir, &item.from)`, `ops.rename_noreplace(&dir, &item.from, &item.to)`).

**Defect.** `rustix::fs::statat`/`renameat` take a relative *path*, not a name. `..` components are
resolved against the dirfd and walk out of it; an absolute path makes the kernel ignore the dirfd
entirely. `journal::path_of` (`:444-452`) validates the batch **id** for precisely this reason
("a trust boundary is a trust boundary"); the file's *contents* got no such treatment.

**Failure scenario.** Any write access to `$XDG_STATE_HOME/detoxrs/journal` (a shared or
attacker-influenced `XDG_STATE_HOME`, a multi-user state directory, a synced dotfile tree) becomes an
arbitrary-file-move primitive executing with the victim's privileges, reported as a normal revert with
exit 0. No-clobber still holds on this path, so it is relocation and confinement escape, not
destruction. Claims falsified: `fsops.rs:111-112` "Rename within one directory, never across
directories (§5.2)"; `fsops.rs:19-25`'s directory-pin argument; `journal.rs:248-249` "Renames never
cross directories (§5.2), so this is the same for both names".

**Found independently by:** A (A-2), C (F1, as violated invariants I4/I5). R5 examined only the
forward path and declared the class impossible — see §3.

**Re-verified.** Forged record with a relative escape:

```
$ cat .../e2/st/detoxrs/journal/000001-20260803T000000Z.jsonl
{"op":"intent","dev":16777229,"ino":...,"kind":"file","mtime":0,
 "dir":".../exp/e2/inner","from":"../../ESCAPED.txt","to":"victim.txt"}
$ XDG_STATE_HOME=.../e2/st detoxrs undo --last
.../exp/e2/inner/victim.txt  ->  ../../ESCAPED.txt

1 reverted, 0 refused. This undo is itself batch 000002-20260803T184645Z.
exit=0
$ ls .../exp/ESCAPED.txt
.../scratchpad/adjudicator/exp/ESCAPED.txt          <-- two levels outside the pinned dir
```

And the absolute form, which leaves the subtree altogether:

```
$ # record with "from":"/tmp/ABS_PWNED_ADJ.txt"
$ XDG_STATE_HOME=.../e2b/st detoxrs undo --last
.../exp/e2b/inner/v.txt  ->  /tmp/ABS_PWNED_ADJ.txt
1 reverted, 0 refused.
exit=0
$ ls -l /tmp/ABS_PWNED_ADJ.txt
-rw-r--r--@ 1 kerry.hatcher  wheel  3 Aug  3 14:46 /tmp/ABS_PWNED_ADJ.txt
```

**Fix direction:** one guard in `parse_intent` — reject `from`/`to` that are not a single component
(`Path::components().count() != 1` or `Component::ParentDir`/`RootDir` present), and `dir` that is not
absolute; push the rejection into `Replay::anomalies` so the existing reporting path shows it.

---

### C2 — HIGH — one invalid UTF-8 byte makes an entire journal un-undoable

**Where:** `crates/detoxrs/src/journal.rs:319` — `let text = fs::read_to_string(path)?;`

**Defect.** `read_to_string` fails on the whole file for one bad byte. The surrounding doc
(`:307-315`) promises per-line fault tolerance: "A truncated final line is expected rather than
exceptional … Anything else that does not add up goes in `Replay::anomalies` and is reported."
Non-UTF-8 bytes bypass that entirely and reach `main.rs`'s `Err(String)` → exit 2.

**Failure scenario.** A single flipped bit or short write in the middle of a journal converts a
recoverable batch into an unrecoverable one. The journal is the *only* record of what happened; this
is the one defect in the set that costs the user recoverability rather than truthfulness. Note the
journal legitimately carries non-UTF-8 filename bytes (`dir_bytes`/`put_os`), so the file is not
guaranteed UTF-8 by construction in the first place.

**Found independently by:** R2 only. Not reported by A, B or C.

**Re-verified.** Control first — same corruption position, valid UTF-8:

```
$ # 5 files renamed, then 'GARBAGE' (valid UTF-8, not JSON) inserted as line 4
$ detoxrs undo <ID>
detoxrs: journal problem: line 4 is not valid JSON and was ignored
... 5 reverted, 0 refused.        exit=1        <-- per-line tolerance works
```

Then one byte:

```
$ # same journal, byte 10 of line 4 set to 0xFF
$ ls
f_1.txt f_2.txt f_3.txt f_4.txt f_5.txt
$ detoxrs undo --last
detoxrs: cannot read .../000001-20260803T184801Z.jsonl: stream did not contain valid UTF-8
exit=2
$ ls
f_1.txt f_2.txt f_3.txt f_4.txt f_5.txt          <-- nothing recovered
```

**Fix direction:** `fs::read(path)`, `split(|b| *b == b'\n')`, `str::from_utf8` per line, and a
non-UTF-8 line becomes the existing `line N is not valid JSON and was ignored` anomaly.

---

### C3 — HIGH — a trailing slash makes `-r` descend a symlinked directory and rename outside the tree

**Where:** `crates/detoxrs/src/walk.rs:84-89` (`fs::symlink_metadata(path)` then
`if recursive && md.is_dir()`) and `:100-101` (`WalkDir::new(root).follow_links(false)`).
**Claim falsified:** `walk.rs:20-22` — "**A symlinked directory is never descended, and there is no
flag for it.** `follow_links(false)` is that guarantee".

**Defect.** POSIX trailing-slash resolution makes `lstat("link/")` *follow* the link, so `md.is_dir()`
is true and `walk_into` runs. `follow_links(false)` governs only entries the walk *discovers*, never
its root. Bash and zsh tab completion append `/` to a directory-like name, so this is the ordinary
spelling.

**Failure scenario.** `detoxrs -x -r 'somelink/'` where `somelink -> $HOME` renames inside the home
directory. Secondary symptom (A-6, C's F4 second half): `push` records the *target directory's*
`(dev, ino)` and `EntryKind::Dir` for an entry that is a symlink, so the preview drops the `[symlink]`
note, renders a trailing `/`, and `-x` then refuses the link's own rename with a false "changed since
the preview (a different file now has this name)" — `ident_at` correctly uses `AT_SYMLINK_NOFOLLOW`
and sees the link. That half fails safe; the escape does not.

**Found independently by:** A (A-1, with the correct mechanism), C (F4, I15). R6 tested sibling-dir
links, `/etc` links, dangling links and loops and found no escape — see §3, scope-limited not wrong.

**Re-verified.**

```
$ mkdir -p e1/outside e1/tree && echo secret > e1/outside/'victim file.txt'
$ ln -s ../outside e1/tree/dirlink && cd e1/tree
$ detoxrs -x -r 'dirlink'            # WITHOUT the slash
0 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.       exit=0
$ ls -1 ../outside
victim file.txt                                                  <-- correctly untouched
$ detoxrs -x -r 'dirlink/'           # WITH the slash
dirlink/victim file.txt  ->  victim_file.txt
1 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.        exit=0
$ ls -1 ../outside
victim_file.txt                                                  <-- outside the named tree
```

And the mis-kinding / false mismatch, same fixture:

```
$ detoxrs -r -v 'dir link'      ->   dir link  ->  dir_link [symlink]
$ detoxrs -r -v 'dir link/'     ->   dir link/  ->  dir_link/      (no [symlink], rendered as a dir)
$ detoxrs -x -r 'dir link/'
dir link/f g.txt  ->  f_g.txt
detoxrs: dir link: changed since the preview (a different file now has this name); not renamed
1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 1 failed.        exit=1
```

**Fix direction:** in `snapshot`, strip trailing separators from each argument before the `lstat` (or
compare `symlink_metadata(p)` against `symlink_metadata(p_without_trailing_slash)` and take the
latter as authoritative). Fixing the root's `lstat` fixes both symptoms at once.

---

### C4 — HIGH (structural / test-layer) — the pin's regression guard cannot observe the layer the defect lived in

**Where:** guard at `crates/detoxrs/src/fsops.rs:440`
(`fsops::tests::the_rename_follows_the_pinned_directory_not_the_path`); defect site at
`crates/detoxrs/src/apply.rs:169-204` (`attempt`).

**Defect.** `docs/HANDOFF.md:111-119` names this the worst finding of the previous pass: `apply`
checked identity, then `fsops` re-resolved `dir` *by path* inside the rename. The fix moved the handle
into the `RenameOps` signature. But the guard pins a `Dir` the **test** created and asserts a property
of `fsops` alone; nothing asserts that `attempt` uses **one** handle for all three operations. One
line re-opening `item.dir` before the rename reinstates the original defect with the suite green.

**Found independently by:** B (B-1). A and C both verified the pin *behaviourally* (A: 0 false
successes in 448 journalled renames under a dir-swap race; C: I3 "HOLDS, structurally") and neither
noticed the guard was in the wrong layer. This is B's single best contribution.

**Re-verified — mutation survival:**

```
apply_reopen_dir: SURVIVED  (recompiled=True passed=138 failed=0 failing_bins=[])
```

**Re-verified — discriminating race.** 30 iterations; each builds `d/` with 60 files `f K.txt`
(content `REAL-K`) and `imp/` with the same names (`IMP-K`), rotates `d`↔`imp` from a background
thread, runs `-x -r d`, then checks the pin invariant from the journal: for every `intent`+`done`, the
entry with the recorded inode must now be named `to`.

```
HEAD:    iterations with wrong-file renames = 0/30, total bad records = 0
MUTANT:  iterations with wrong-file renames = 5/30, total bad records = 14
   WRONG-FILE RENAME: ('f 1.txt',  'f_1.txt',  'but inode is named', 'f 1.txt')
   WRONG-FILE RENAME: ('f 35.txt', 'f_35.txt', 'but inode is named', 'f 35.txt')
```

So the failure state is reachable at ~17 % per iteration on this machine, the journal records a
success against an inode that was never renamed, and the suite is silent. B measured 21/50 and 28 bad
records; I measured 5/30 and 14 with a smaller batch and a shorter swap window. Same phenomenon,
different rate — B's number is not exaggerated, my fixture is just gentler.

**Fix direction:** a `RenameOps` double in `apply::tests` that records the identity of the `Dir` it is
handed on each call and asserts `ident_at`×2 and `rename_noreplace` all got the same one. Deterministic;
fails every time, unlike the race.

---

### C5 — MEDIUM-HIGH — a hardlinked destination reports `1 renamed`, exit 0, and renames nothing

**Where:** `crates/detoxrs/src/fsops.rs:187-190` (rung 1: `EEXIST` + `same_inode` ⇒ `renameat_plain`),
`crates/detoxrs/src/fsops/fallback.rs:86-91` (`same_inode` answers `true` for two hardlinks by design),
`crates/detoxrs/src/apply.rs:187-194` (step 2 waives occupancy for a same-inode occupant).

**Defect.** When the planned destination is a second hardlink to the source's own inode and is **not
in the snapshot** (single-file argument, or a non-recursive directory argument), step 2 waives the
occupancy check, `renameat(NOREPLACE)` returns `EEXIST`, rung 1 sees `same_inode` and falls through to
plain `renameat`, and POSIX requires that call to "return successfully and perform no other action".
Net: nothing moves, `done` is journalled, the report says `1 renamed`, exit 0, and the dirty name is
still on disk. `fallback.rs:78-84` argues the rung is safe because neither name is destroyed — true,
and it says nothing about the false success. `fallback.rs:9-11`'s "not expected to fire on either
tier-1 platform" is false: one `ln` on stock APFS reaches it.

**Found independently by:** B (B-2), C (F5, I17). R6 tested two hardlinked *sources* both in the batch
and found correct handling — a different scenario, see §3.

**Re-verified.**

```
$ printf 'CONTENT' > 'a b.txt'; ln 'a b.txt' 'a_b.txt'; ls -li
154100977 -rw-r--r--@ 2 ... a b.txt
154100977 -rw-r--r--@ 2 ... a_b.txt
$ detoxrs -x 'a b.txt'
detoxrs: warning: this filesystem reported an existing destination for a rename of a name onto
itself; using a plain rename for that item. Please report this: it is a filesystem behaviour
detoxrs has not measured.
a b.txt  ->  a_b.txt

1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.       exit=0
$ ls -li                      # nothing happened
154100977 -rw-r--r--@ 2 ... a b.txt
154100977 -rw-r--r--@ 2 ... a_b.txt
$ cat journal
{"dev":16777229,"dir":".../hl","from":"a b.txt","ino":154100977,...,"op":"intent","to":"a_b.txt"}
{"ino":154100977,"op":"done"}
{"op":"end"}
$ detoxrs undo --last     ->  1 reverted, 0 refused.   exit=0   (also having done nothing)
```

Control (R6's scenario, both links in the snapshot) is correct:

```
$ detoxrs -r .
  a b.txt  ->  a_b.txt [hardlink, nlink=2]
  c d.txt  ->  c_d.txt [hardlink, nlink=2]
$ detoxrs -x -r .   ->  2 renamed;  ls -li shows a_b.txt and c_d.txt, one inode
```

Secondary: the warning text asks the user to report an unmeasured filesystem behaviour when what
happened is an ordinary hardlink, so this rung will generate spurious bug reports.

**Fix direction:** distinguish "same inode, same directory entry" (a genuine respell — source and
destination are one entry) from "same inode, two entries" (a hardlink) before treating `EEXIST` as
success; the latter is `AlreadyExists`.

---

### C6 — MEDIUM-HIGH — a broken pipe reports exit 2 after renames have happened, and truncates the batch

**Where:** `crates/detoxrs/src/apply.rs:210-216` (a per-item progress `writeln!` failure is
`Fail::Batch`) and `crates/detoxrs/src/main.rs:133-143` (`report::applied(...).and_then(flush)
.map_err(...)?`, and `main.rs:45-53` turns any `Err` into `ExitCode::from(2)`).
**Contract violated:** `main.rs:13-15` and `--help` — "`2` usage, walk, or plan error — **which are the
failures where nothing was attempted at all**".

**Failure scenario.** `detoxrs -x -r . | head` — or into `less`, or any consumer that closes early.
A wrapper that branches on `2 → nothing happened, no cleanup needed` skips the undo it needs, and the
batch is abandoned partway with no summary.

**Found independently by:** B (B-3), C (F2 + F7 as two symptoms of one root, I11).

**Re-verified.**

```
$ # 200 files 'k N.txt'
$ bash -c 'XDG_STATE_HOME=... detoxrs -x -r . 2>err | head -1 >/dev/null; echo detoxrs_exit=${PIPESTATUS[0]}'
detoxrs_exit=2
$ ls | grep -c '^k_' ; ls | grep -c '^k '
2
198
$ head -2 err
detoxrs: cannot write output: Broken pipe (os error 32)
detoxrs: cannot write output: Broken pipe (os error 32)
$ # journal: intents 2, done 2, end 1
```

Two of 200 renamed and journalled, under an exit code that means nothing was attempted.

**Fix direction:** in `exec`, a reporting/flush failure after `apply::run` returns warns on stderr and
returns `s.exit_code().max(1)`; and a progress-write failure in `attempt` should be `Fail::Item` (or
better, stop writing progress and keep going) rather than `Fail::Batch`.

---

### C7 — MEDIUM-HIGH — journals that describe nothing shadow `undo --last`, and the shadowed call exits 0

Three routes to one defect; they should be fixed together.

**Where:** `crates/detoxrs/src/main.rs:232-234` (the undo's journal is created *before* `apply::undo`
runs), `main.rs:100-107` (the forward-path guard skips the journal only when no plan item has
`Resolution::Rename`, not when no rename *succeeded*), and `main.rs:181-186` + `journal::list()`
(`--last` is `batches.last()` over every `*.jsonl`, including one another process is still writing).
`main.rs:224-227` returns `Ok(u8::from(suspect))`, and a complete-but-empty journal has
`suspect == false`, hence exit 0.

The forward path already carries this exact guard, with the reasoning spelled out at `main.rs:97-99`:
"an empty batch would be the newest one, so `undo --last` would stop meaning 'undo what I just did'".
The undo path never got it, and two other routes reach the same state.

**Found independently by:** C (F3, I12) for the all-refused-undo route; A (A-7) for the
all-items-failed forward route; R6 for the concurrent-`undo --last` route. Nobody connected them.

**Re-verified — route 1 (C's F3): an undo that reverts nothing still leaves a journal.**

```
$ detoxrs -x -r .            # batch 000001
$ detoxrs undo --last        # batch 000002, reverts it
$ detoxrs undo 000001-...    # already undone
detoxrs: .../c_d.txt: no longer readable since the preview: no longer there (ENOENT)
detoxrs: .../a_b.txt: no longer readable since the preview: no longer there (ENOENT)
0 reverted, 2 refused. This undo is itself batch 000003-...      exit=1
$ cat .../000003-*.jsonl
{"batch":"000003-...","cwd":"...","policy":{...},"v":1}
{"op":"end"}
$ detoxrs undo --last
nothing to undo in that batch.
exit=0                       # batch 000002 is now unreachable via --last
```

**Re-verified — route 2 (A's A-7): a forward `-x` in which every rename fails still leaves a journal.**

```
$ detoxrs -x 'a b.txt'       # fails per C9
0 renamed, 0 unchanged, 0 skipped, 0 conflicts, 1 failed.        exit=1
$ detoxrs undo --list  ->  000001-20260803T184835Z
$ detoxrs undo --last  ->  nothing to undo in that batch.        exit=0
```

**Re-verified — route 3 (R6): `--last` resolves to a journal another run is still writing.**

```
$ detoxrs -x -r .            # real batch 000001, complete
$ # plant a header-only journal, as a concurrent run would have mid-flight
$ detoxrs undo --last
detoxrs: warning: batch 000002-... has no completion record, so it either crashed or is still
running. If a detoxrs run is still in progress, its remaining items will not be reverted and the
tree will be left half-cleaned.
nothing to undo in that batch.
exit=1
$ ls   ->  g_1.txt g_2.txt g_3.txt      # real batch never reverted, and now unreachable via --last
```

Route 3 at least warns and exits 1 — R6's characterisation ("misresolution and misleading diagnosis,
not data loss") is accurate. Routes 1 and 2 exit **0** on a no-op the user asked to be an undo, which
is the worse outcome.

**Fix direction:** create the undo journal lazily, on the first successful revert; extend the
forward-path emptiness guard to "no rename succeeded"; and treat "the newest journal has no `end`
record" as "do not pick it for `--last`" rather than as something to warn about after picking it.

---

### C8 — MEDIUM — snapshot dedup and collision keys use the path *spelling*, not identity

**Where:** `crates/detoxrs/src/walk.rs:81` and `:158` (`seen: HashSet<(PathBuf, OsString)>` keyed on
the textual parent), `crates/detoxrs-core/src/plan.rs:235-263` (both `wants` and `occupied` keyed on
`&Path`). `walk.rs:76-80` states the dedup exists precisely so overlapping arguments do not "put one
directory entry in the snapshot twice".

**Failure scenario.** `.` / `` / `./x` are different keys for one directory, so one file is planned
twice, previewed twice, counted twice, and `-x` exits 1 on a phantom ENOENT; and the two collision
universes are blind to each other, so the preview can promise two entries onto one destination while
reporting `0 conflicts`. No clobber results — the apply-time recheck and the kernel both catch it —
but §5.3's argument is that "every destination is decided before anything is written", and here it
is not.

**Found independently by:** A (A-4), B (B-6), C (F6a/b/c, I13/I14/I16). All three; none could turn it
into a clobber, and neither could I.

**Re-verified.**

```
$ mkdir sub && printf x > 'sub/f g.txt'
$ detoxrs -r . sub
./sub
  f g.txt  ->  f_g.txt
sub
  f g.txt  ->  f_g.txt
2 to rename, 2 unchanged, 0 skipped, 0 conflicts.        # one file
$ detoxrs -x -r . sub
./sub/f g.txt  ->  f_g.txt
detoxrs: sub/f g.txt: no longer readable since the preview: no longer there (ENOENT)
1 renamed, 2 unchanged, 0 skipped, 0 conflicts, 1 failed.        exit=1
```

Two entries onto one destination, `0 conflicts`:

```
$ ls 'x y'      # 'a b.txt' (ONE) and 'a  b.txt' (TWO), both transform to a_b.txt
$ detoxrs 'x y/a b.txt' './x y/a  b.txt'
./x y
  a  b.txt  ->  a_b.txt
x y
  a b.txt  ->  a_b.txt
2 to rename, 0 unchanged, 0 skipped, 0 conflicts.        # the plan promises a clobber
$ detoxrs -x 'x y/a b.txt' './x y/a  b.txt'
./x y/a  b.txt  ->  a_b.txt
detoxrs: x y/a b.txt: a_b.txt appeared since the preview; not renamed
1 renamed, ..., 1 failed.        exit=1
$ ls -l 'x y'   ->  'a b.txt' (ONE) intact, a_b.txt == TWO      # no loss
```

**Fix direction:** key `seen` on the `Ident` the walk already collects; plan-side, key `wants` and
`occupied` on `(dev_of_dir, key)` or on a canonicalised dir. C's note is right that this also repairs
the aliased-depth ordering inversion (I16) for free.

---

### C9 — MEDIUM — collision layer 2 is blind to names outside the snapshot: the preview promises an impossible rename and the error blames a race that did not happen

**Where:** `crates/detoxrs-core/src/plan.rs:257-263` (occupancy built only from snapshot entries);
diagnostic at `crates/detoxrs/src/apply.rs:187-194`.

**Failure scenario.** `detoxrs 'a b.txt'` where `a_b.txt` already exists — the single-file invocation,
the most common one. The preview shows `0 conflicts` and a destination that cannot be taken; `-x` then
says `a_b.txt appeared since the preview`, which is a false statement about what happened. Under
`--on-collision number` the user is entitled to `a_b-2.txt` and does not get it.

**Found independently by:** A (A-3). B flagged the mechanism as a stated-architecture consequence
rather than a defect (B §3) and C did not separate it from C8.

**Re-verified.**

```
$ printf 1 > 'a b.txt'; printf 2 > 'a_b.txt'
$ detoxrs 'a b.txt'
  a b.txt  ->  a_b.txt
1 to rename, 0 unchanged, 0 skipped, 0 conflicts.        exit=0
$ detoxrs --on-collision number 'a b.txt'
  a b.txt  ->  a_b.txt                                    # not a_b-2.txt
1 to rename, 0 unchanged, 0 skipped, 0 conflicts.        exit=0
$ detoxrs -x 'a b.txt'
detoxrs: a b.txt: a_b.txt appeared since the preview; not renamed
0 renamed, 0 unchanged, 0 skipped, 0 conflicts, 1 failed. exit=1
```

**Ruling on B's framing.** B is right that an I/O-free `plan()` is the stated architecture, and I am
not asking for that to change. But two things here are defects independent of the architecture: the
diagnostic *asserts* a race that provably did not occur (`a_b.txt` predates the preview), and nothing
in `--help` or the preview footer says a shown destination is advisory. Fixing the message and adding
one `lstat` of the destination at preview time for the non-recursive/single-file case is a small,
architecture-preserving fix.

**Fix direction:** distinguish "occupied at plan time" from "appeared after the walk" in the apply-time
message (the walk already has the data to tell them apart), and seed layer 2's occupancy with an
`lstat` of each candidate destination when the argument is a single file or non-recursive directory.

---

### C10 — MEDIUM — `report::escape` is not injective, yet `--json` reports `"utf8": true`

**Where:** `crates/detoxrs/src/report.rs:326-341`, `:366-381`, `"utf8"` field at `:273`.
**Claims falsified:** `report.rs:270-272` — "A consumer that needs the raw bytes must not be able to
mistake one for the other"; `report.rs:325-326` — "nothing is lost, because the escapes are reversible".

**Failure scenario.** Control characters are escaped even when the name is valid UTF-8, so
`utf8: true` does not mean `from` is the name; and the escaping is not injective, so two distinct
files are indistinguishable in the only stable contract the project offers (`report.rs:3`).

**Found independently by:** A (A-5).

**Re-verified.**

```
$ python3 -c "open('a\nb.txt','w').close(); open('a<0a>b.txt','w').close()"
$ ls -b   ->  a\nb.txt   a<0a>b.txt
$ detoxrs -r .
.
  a<0a>b.txt  ->  ab.txt
  a<0a>b.txt  ->  a_0a_b.txt
$ detoxrs --json -r . | python3 -c '...'
True 'a<0a>b.txt' -> 'ab.txt'
True 'a<0a>b.txt' -> 'a_0a_b.txt'
```

Two different files, one rendering, `utf8: true` on both.

**Fix direction:** either make the escape injective (escape `<` as well, e.g. `<3c>`) or add a
distinct field for "this rendering is escaped" separate from "the name is valid UTF-8" — the doc
comment already promises the latter distinction, so this is a doc-vs-code gap, not a preference.

---

### C11 — LOW-MEDIUM — U+061C is not stripped, against `invisible.rs`'s "whole CVE-2021-42574 class" claim

**Where:** `crates/detoxrs-core/src/invisible.rs:20-34`.
**Claim falsified:** `invisible.rs:5-7` — "the named set covers the whole CVE-2021-42574 (Trojan
Source) class this stage exists for".

**Defect.** U+061C ARABIC LETTER MARK is a bidi formatting control and is part of the Trojan Source
character set. It classifies `Keep` and survives verbatim. U+2028/U+2029 (LINE/PARAGRAPH SEPARATOR)
and U+180E also survive; those are `Zl`/`Zp`/`Cf` and land squarely inside M4's promised UCD closure,
so they are scope-deferred (§7) — but U+061C is inside the class the current doc comment claims to
cover completely, which makes it a real defect at LOW severity.

**Found independently by:** R4.

**Re-verified.**

```
$ # files x<CP>y_NAME.txt for U+2028, U+2029, U+061C, U+180E, U+202E, U+200B
$ detoxrs -r .
.
  x<ZWSP>y_ZWSP.txt  ->  xy_ZWSP.txt
  x<RLO>y_RLO.txt    ->  xy_RLO.txt

2 to rename, 4 unchanged, 0 skipped, 0 conflicts.
```

Four of six survive; U+202E and U+200B are handled.

**Fix direction:** add `'\u{061c}'` to `is_invisible`'s bidi-marks arm now; leave U+2028/2029/180E to
M4, and narrow the doc comment to what the table actually covers.

---

### C12 — LOW — `--json` emits zero bytes on every exit-2 path

**Where:** `crates/detoxrs/src/main.rs:45-53` — every `Err(String)` short-circuits before any JSON is
written. `--help` says `--json` means "JSON on stdout, diagnostics on stderr".

**Re-verified.**

```
$ detoxrs --json --on-collision fail -r .      # 'a b.txt' + 'a_b.txt' present
exit=2 stdout_bytes=0
$ detoxrs --json nonexistent
exit=2 stdout_bytes=0
```

A machine consumer cannot distinguish a refusal from a crash. Valid JSON *is* produced on every
non-exit-2 path including per-item failures (A, B and I all verified this with a real parser).

**Found independently by:** B (B-10), C (§4, listed as suspected because no document promises it).
**Ruling:** LOW defect rather than suspected — `--json`'s own help text promises JSON on stdout
without carving out error paths.

**Fix direction:** emit a minimal `{"schema":…,"error":…}` document on the refusal paths, or state in
`--help` that exit 2 produces no JSON.

---

### C13 — LOW — the batch-id rejection message names an id format the tool cannot produce, and `HANDOFF.md` contradicts itself

**Where:** `crates/detoxrs/src/journal.rs:448` —
`"{id:?} is not a batch id; ids look like 20260801T142233Z-a91c"`. Real ids are `{seq:06}-{stamp}`
(`journal.rs:101`). `docs/HANDOFF.md:60-61` carries the same stale claim ("the suffix is now the
subsecond clock scaled to four fixed-width hex digits") and is contradicted by `:141-146` in the same
document ("Batches were named `<UTC-stamp>-<subsecond-hex>` and …").

**Found independently by:** B (B-7), A (A-8 row 4). **Re-verified** by reading both files at the pinned
commit and listing a real journal directory (`000001-20260803T184819Z.jsonl`).

**Why it matters beyond cosmetics:** `HANDOFF.md` is what a fresh session resumes from, and it
currently instructs that session to preserve an ordering invariant that was deliberately deleted.

**Fix direction:** one string, one paragraph.

---

### C14 — LOW — `README.md` and `CHANGELOG.md` describe a program that does not exist

**Re-verified** against the pinned tree and binary:

| Claim | Where | Reality |
|---|---|---|
| "**Status: pre-implementation.** … the tool does not do anything yet. `main.rs` is a placeholder." | `README.md:10-12` | Full preview + write + undo path ships and works. |
| "percent-escapes decoded" as a shipped behaviour | `README.md:5-8` | M2 work; no `percent` module (`lib.rs:12-13`); `%20` is left alone. |
| "no CLI, transform pipeline, or filesystem operations exist" | `CHANGELOG.md:10-14` | All three exist. |
| "Every -x run writes an undo journal to `$XDG_STATE_HOME/detoxrs/journal`" | `--help` after_help | A no-op `-x` deliberately writes none, and the path falls back to `$HOME/.local/state`. |

**Found independently by:** A (A-8), B (B-8), C (F8), R1. Four reviewers; unanimous; trivially true.

---

### C15 — LOW — `detoxrs-core`'s `Policy` has public unvalidated fields, and `separator: '/'` breaks the transform's safety closure

**Where:** `crates/detoxrs-core/src/policy.rs:19-29` — `pub separator: char` with no invariant stated
or enforced. `transform` with `Policy { separator: '/' }` maps `"a b.txt"` to `"a/b.txt"`, falsifying
the no-separator property the pipeline claims.

**Not reachable through the shipped CLI** — `main.rs:65` and `:233` hardcode `Policy::default()`, and
`--separator` is M3 — so this is a library-API footgun, not a CLI defect. `tests/support/mod.rs:16-19`
already pins `'_'` because otherwise "Safety closure false by construction", which is the project
noticing the hazard in its own test harness without closing it in the type.

**Found independently by:** R4, who correctly scoped it as not-CLI-reachable.

**Fix direction:** either document the precondition on the field, or make the field private behind a
`Policy::with_separator(c) -> Result<…>` that rejects `/`, `\`, NUL and `.`. Cheap either way; do it
before M3 wires a flag to it.

---

### C16 — informational, not a defect — renaming a directory breaks inbound relative symlinks with no note

B-9. This is `rename(2)` semantics, `undo` restores it, and no document promises otherwise. B's
argument is that the tool already volunteers `[hardlink, nlink=N]` for a strictly less dangerous
aliasing case. That is a reasonable enhancement request and I record it as such; it is **not** a
defect and should not gate a pass.

---

## 3. Adjudicated conflicts

### 3.1 Verdict split — A: FAIL, C: FAIL, B: CONDITIONAL PASS

**Ruling: FAIL. B under-tested rather than judged differently.**

B did not report C1 or C3 because B never ran the experiments that produce them. B's §6 does not list
either, and B's method sections show why: B attacked the journal as a *crash artifact* (12 `kill -9`
runs, torn lines, mismatched `done.ino`, two-intents-one-done) and as an *injection* surface
(filenames with newlines), and hand-forged journals only to test `replay`'s anomaly reporting — never
to test whether `apply` would execute a hostile `from`/`to`. Likewise B tested symlinks extensively
(dangling, self-referential, to `/etc`, to a directory) but never with a trailing slash. So B's
CONDITIONAL PASS is a verdict over a smaller evidence set, not a disagreement about the same evidence.

Both defects were reproduced independently by two reviewers and now a third. Neither is arguable. B's
own bar — "Fix those and close the test-coverage holes and this is a PASS" — is not met, because two
HIGH contract violations sit outside the list B was reasoning over.
**Confidence: high.**

### 3.2 Mutation survival rates — A 14/36 (39 %), B 8/33 (24 %), C 3/25 (12 %)

**Ruling: A's figure is closest to right. C's 12 % is materially understated and C's mutation log
should not be used. B's 24 % is close but contains one false survivor.**

Not a numbers dispute — a verdict dispute. I re-derived every contested mutation with a harness that
builds and then executes the compiled test binaries directly, checks that a recompile actually
happened (`recompiled=True`), and revalidates the 138/0 baseline after every revert. Three of C's
"caught" verdicts are refuted, and one of B's "survived" verdicts is refuted:

| Mutation | A | B | C | My re-derivation |
|---|---|---|---|---|
| `walk::push` dedup removed | M16 SURVIVED | M21 SURVIVED | **M19 caught** | **SURVIVED** (138/0, recompiled) — C wrong |
| `volume_case` always `Sensitive` | M22 SURVIVED | — | **M22 caught** | **SURVIVED** (138/0, recompiled) — C wrong |
| non-`Rename` items renamed too | M21 SURVIVED | — | **M17 caught** | **SURVIVED** (138/0, recompiled) — C wrong |
| `numbered` uses untruncated stem when nothing fits | — | **M13 SURVIVED** | — | **KILLED** by `prop_plan::bounded_renumbering`, 7/12 runs — B wrong |
| `next_seq` → always 1 | M32 SURVIVED (equivalent) | M24 SURVIVED | — | **SURVIVED**, 0/15 kills on repeat (1 flaky kill in 16 total) — A and B right |

For the non-`Rename` case I also confirmed the mutant is behaviourally observable, so it is a genuine
coverage gap and not an equivalent mutant:

```
HEAD:    1 renamed, 1 unchanged, 1 skipped, 0 conflicts, 0 failed.
MUTANT:  ./***  ->  ***
         ./a b.txt  ->  a_b.txt
         ./clean.txt  ->  clean.txt
         3 renamed, 1 unchanged, 1 skipped, 0 conflicts, 0 failed.     # journal intents: 3
```

**Why C's log is wrong, mechanically.** C's three false "caught" verdicts are the mirror image of trap
#2: restore a file from a backup whose mtime predates the last build and cargo does not rebuild, so
the *previous* mutant's binary runs against the *next* mutation's label and the tests fail for the
wrong reason. I reproduced that artifact in my own first pass (§5) — `shutil.move` preserves the
backup's mtime — and it produced exactly this signature. C used `cargo test` in an isolated copy,
which is consistent.

**Reconciled true survival rate.** Over the union of distinct mutation sites tested across the run
(≈45), I confirm **14 real coverage gaps**, i.e. **≈30 %**. A's headline 39 % includes four mutants A
itself flagged as equivalent or unreachable; A's own adjusted figure (31 %) is within noise of mine.
**Confidence: high** for the individual verdicts, **medium** for the aggregate rate (the union is not
a random sample and different reviewers weighted different modules).

### 3.3 How many `sync_data()` calls, and how many are unguarded?

**Ruling: there are THREE, and all three are unguarded. A undercounted by one; C and R2/R3 by two.**

```
crates/detoxrs/src/journal.rs:178   finish()  — closes the batch with the `end` record
crates/detoxrs/src/journal.rs:196   header()  — "A journal whose header never reached disk is a
                                                 journal `undo` cannot identify."
crates/detoxrs/src/journal.rs:229   intent()  — "The one fsync the design depends on."
```

Re-derived, each independently:

```
intent_fsync: SURVIVED  (recompiled=True passed=138 failed=0)
header_fsync: SURVIVED  (recompiled=True passed=138 failed=0)
finish_fsync: SURVIVED  (recompiled=True passed=138 failed=0)
```

A flagged two (`intent`, `header`); C, R2 and R3 flagged one (`intent`). **Nobody tested `finish`'s.**
All three carry a doc comment asserting a durability property, and none is pinned by any test.
**Confidence: high.**

Note the correct framing (§7): the *absence of a directory fsync* is documented and deliberately
deferred and is not a defect. The *absence of any test* on the three fsyncs that do exist is a
coverage gap, and the author already names it in `HANDOFF.md`. The reason it matters is not power loss
— it is that a live mutation of `journal.rs:229` sat in the shared working tree during this very
review and no test noticed (C observed exactly that).

### 3.4 Basename validation / separator injection — R5 says impossible, A and C reproduce an escape

**Ruling: R5's claim is scope-limited to the forward path and false as stated. The undo-path escape is
real; see C1.**

R5's reasoning is *correct for the forward path*: `from` comes from `Path::file_name()` and `to` from
`transform` over a basename, and POSIX names cannot contain `/`. I re-checked and could not break the
forward path either. R5 simply never looked at `apply::undo`, which routes journal-supplied strings
through the same `*_at` calls — and the digest's own annotation flags this correctly.

The lesson is about the shape of the claim, not the work: "impossible" without a named scope is the
kind of assertion that suppresses a HIGH. R5 should have written "impossible on the forward path;
undo not examined". **Confidence: high** (verified relative *and* absolute escapes, exit 0 both times).

### 3.5 Hardlinks — R6 finds correct handling, B and C find a false success

**Ruling: two different scenarios. Both results are correct. No contradiction.**

- R6 tested two hardlinked **sources** both present in the snapshot. Layer 2 sees the second link
  occupying its name and renumbers. I reproduced this: `a b.txt -> a_b.txt [hardlink, nlink=2]` and
  `c d.txt -> c_d.txt [hardlink, nlink=2]`, both applied, one inode, correct.
- B and C tested a hardlinked **destination** that is *not* in the snapshot (single-file argument).
  That waives step 2's occupancy check and lands on rung 1's `same_inode` carve-out. I reproduced the
  false success exactly (C5).

`fallback.rs:82-84` even states the boundary — "The planner will not normally produce such an item
anyway — a hardlink present in the snapshot occupies its name at collision layer 2" — which is
precisely R6's scenario. The bug lives in the case that sentence excludes. **Confidence: high.**

### 3.6 Symlink escape — R6 finds none, A and C find one requiring a trailing slash

**Ruling: R6 is scope-limited, not wrong. The trailing-slash escape is real; see C3.**

I ran both spellings back-to-back on the same fixture. Without the slash, `0 renamed, 1 unchanged`,
canary untouched — R6's result, reproduced. With the slash, `1 renamed`, exit 0, a file outside the
tree renamed — A's and C's result, reproduced. A's mechanism (`lstat("link/")` follows the link, so
`md.is_dir()` is true and `follow_links(false)` never gets a say because it only governs discovered
entries) is exactly right.

R6's list of cases (sibling-dir link, `/etc` link, dangling link, self-referential loop) is a good
list that happens to miss the one input that matters. **Confidence: high.**

### 3.7 R2's `replay()` whole-file UTF-8 finding, corroborated by nobody

**Ruling: real, HIGH, and the most under-reported finding in the run. See C2.**

Verified independently with a matched control (same corruption offset, valid-UTF-8 byte → 5/5 items
recovered, exit 1, per-line anomaly reported; invalid byte → exit 2, zero recovery). This belongs at
or near the top of the fix list because it is the only defect in the set that costs recoverability
rather than truthfulness, and because the journal legitimately contains non-UTF-8 filename bytes, so
the file is not UTF-8 by construction in the first place. **Confidence: high.**

Process note: that three Opus reviewers each ran extensive corrupt-journal suites and all three
corrupted *within* UTF-8 is a real blind spot worth remembering. Everyone reached for "truncate the
last line" and "insert garbage"; nobody reached for "flip a bit".

### 3.8 B-1 — the pin's guard is in the wrong layer

**Ruling: confirmed, exactly as B described. See C4.** Mutation survives (138/0, recompile verified);
HEAD 0/30 wrong-file renames versus mutant 5/30 with 14 false journal successes. My rate is lower than
B's 21/50 because my fixture uses 60 files and a shorter swap window; the phenomenon is identical.
B's claim is not exaggerated. **Confidence: high.**

This is the highest-leverage item in the report for a next session: it is the only finding that is
about the *test suite's* structure rather than the code's behaviour, and it concerns exactly the
refactor `HANDOFF.md` flags as unreviewed.

---

## 4. Rejected and demoted allegations

| Allegation | Source | Ruling |
|---|---|---|
| `numbered`'s `kept.is_empty()` guard is untested (M13) | B-11 | **REJECTED.** Killed by `prop_plan::bounded_renumbering` in 7 of 12 runs with persistence off. B hit a single unlucky run of a randomized property test. |
| `walk` dedup mutation is caught (M19) | C | **REJECTED.** Survives (138/0, recompile verified). A and B are right. |
| `volume_case` mutation is caught (M22) | C | **REJECTED.** Survives. |
| non-`Rename` items mutation is caught (M17) | C | **REJECTED.** Survives, and is behaviourally observable (`3 renamed` vs `1 renamed`, 3 journal intents), so it is a real gap and not an equivalent mutant. |
| Separator injection into `*_at` is impossible | R5 | **DEMOTED to scope-limited.** True of the forward path only; the undo path escapes (C1). |
| No symlink escape under `-r` | R6 | **DEMOTED to scope-limited.** True of every spelling R6 tested; false with a trailing slash (C3). |
| Two hardlinked sources are mishandled | — | **Never alleged, and correct.** Recorded here because it reads like a refutation of C5 and is not (§3.5). |
| No directory fsync ⇒ power-loss data loss | A §3, R2, R6 | **NOT A DEFECT.** `journal.rs:16-24` states the limit precisely ("survives `kill -9`, not power loss") and names the upgrade. A documented, deliberately-scoped guarantee that the code delivers. See §7. |
| `is_vcs_dir → false` survives (A M28) | A | **DEMOTED to equivalent mutant**, as A itself said: `.git` is still caught by `is_dotfile`. |
| `"." \| ".."` output guard survives (A M29), walk `.`/`..` name guard survives (A M30) | A | **DEMOTED to unreachable/equivalent mutants**, as A itself said. Note C claims M25 (the walk guard) *is* caught; I did not adjudicate this one — low value either way. Labelled UNVERIFIED. |
| `!converged` guard survives (A M26) | A | **DEMOTED.** A's own note: the non-convergent state was never produced by 800 000 fuzz cases. Defensive assertion, not a coverage gap. |
| `list()`/`--last` ordering breaks past 999 999 batches | B §3, C §4 | **ACCEPTED as SUSPECTED, not confirmed.** Mechanism is real (`{seq:06}` + lexical sort); nobody produced a million journals and neither did I. Fix is a one-line numeric sort; not blocking. |
| Renaming a directory breaks inbound relative symlinks | B-9 | **DEMOTED to informational** (C16). `rename(2)` semantics; no document promises otherwise. |
| `report::items` column widths count `chars`, not display width | A §6 | **REJECTED as a defect.** Cosmetic; A did not pursue it either. |
| `preview.rs::json_goes_to_stdout_and_parses` checks brace balance instead of parsing | C §6 | **ACCEPTED as a minor test-quality nit, not a defect.** Worth one line to fix since `serde_json` is already a dependency. |
| `--json -q` emits the full document | B-10 note | **DEMOTED.** `-q` is documented as "errors only" for the human report; that `--json` overrides it is defensible. Worth a `--help` sentence. |
| **`prop_plan` fails at HEAD with "numbered candidate over the limit: a-2"** | *my own first pass* | **REJECTED — this was trap #2 biting me.** I observed six deterministic `prop_plan` failures at a tree that was md5-identical to HEAD, and chased it as a HEAD defect. It was a stale test binary: my harness restored files with `shutil.move`, which preserves the backup's mtime, so cargo did not rebuild and I was running the previous mutant. After adding `os.utime()` on both apply and revert, HEAD is 0 failures in 20 runs at default cases and 0 in 5 runs at `PROPTEST_CASES=2048`. **There is no such HEAD defect. Do not re-litigate this.** |

---

## 5. Test-suite assessment

Baseline: **138 tests, 0 failures**, ~30 s, in a pinned checkout verified identical to `04974e2`.

### 5.1 Method, and the trap

Every verdict below came from: write mutation → `os.utime(file)` → `cargo test --workspace --no-run
--message-format=json` → assert `Compiling detoxrs` appeared in cargo's stderr → **execute each
compiled test binary directly** → restore file → `os.utime(file)` → revalidate the 138/0 baseline.

Three controls confirm the harness discriminates: `same_entry → true` (KILLED, 4 tests),
`follow_links(true)` (KILLED, 2), `renameat_noreplace → renameat_plain` (KILLED, 1).

**Trap #2 is worse than the brief describes, and it cuts both ways.** The reverted-source failure mode
makes a mutation look *survived*. But the same staleness one step later makes the *next* mutation look
*caught*, because the previous mutant's binary is still on disk. That second form is what produced C's
three false "caught" verdicts, and it produced a phantom HEAD defect in my own first pass (§4, last
row). The fix is one line — `os.utime(path)` after **both** the apply and the revert — and any future
mutation run in this repo should assert `Compiling` in cargo's output per mutation.

**A second, unrelated trap in this repo: `prop_plan` and `prop_transform` are randomized with no fixed
seed.** A mutation that only some inputs expose gets a non-deterministic verdict — `numbered_empty`
above is killed 7 times in 12. Any mutation touching `plan.rs` or `pipeline.rs` needs N repetitions
before "SURVIVED" means anything. B's single-run M13 verdict is the casualty. Worth setting
`PROPTEST_CASES` higher in CI, or seeding deterministically and keeping a separate randomized job.

Also: mutant runs **write to the tracked `*.proptest-regressions` files**. Mine polluted
`crates/detoxrs-core/tests/prop_plan.proptest-regressions` with three counterexamples; I restored it
from `git show`. Future runs should either set `PROPTEST_FAILURE_PERSISTENCE=off` or restore those
files, and reviewers should check them in `git status`.

### 5.2 Reconciled mutation table

Verified rows were re-derived by me in the pinned checkout. "UNVERIFIED" rows are reported kills I did
not re-run; I list them because ≥2 reviewers independently agree, which is decent but not proof.

| # | Mutation | Site | Expected catcher | My result | A | B | C | R2/R3/R5 |
|---|---|---|---|---|---|---|---|---|
| 1 | `intent()` drops `sync_data()` | `journal.rs:229` | none exists | **SURVIVED** ✔ | M3 S | M4 S | M4 S | R2 S, R3 S |
| 2 | `header()` drops `sync_data()` | `journal.rs:196` | none exists | **SURVIVED** ✔ | M34 S | — | — | — |
| 3 | `finish()` drops `sync_data()` | `journal.rs:178` | none exists | **SURVIVED** ✔ (new) | — | — | — | — |
| 4 | `replay` outcome↔intent inode match always accepts | `journal.rs:355` | none exists | **SURVIVED** ✔ | M20 S | M11 S | M7 S | R3 S |
| 5 | `walk::push` dedup removed | `walk.rs:81` | its own doc claim | **SURVIVED** ✔ | M16 S | M21 S | *M19 caught* ✘ | — |
| 6 | `volume_case` always `Sensitive` | `walk.rs:256` | none exists | **SURVIVED** ✔ | M22 S | — | *M22 caught* ✘ | — |
| 7 | `attempt` re-opens `item.dir` before the rename | `apply.rs:204` | `the_rename_follows_the_pinned_directory_not_the_path` | **SURVIVED** ✔ | — | M30 S | — | — |
| 8 | `check_no_sibling_chains` result discarded in `plan()` | `plan.rs:230` | none exists | **SURVIVED** ✔ | M5 S | M8 S | M23 S | — |
| 9 | `create_new(true)` → `create(true)` | `journal.rs:103` | its own doc claim | **SURVIVED** ✔ | M31 S | M23 S | — | — |
| 10 | `next_seq` → always 1 | `journal.rs:149` | `last_means_most_recently_created` | **SURVIVED** ✔ (1 flaky kill in 16) | M32 S (equiv) | M24 S | — | — |
| 11 | `open_dir` drops `O_DIRECTORY` | `fsops.rs:296` | its own doc claim | **SURVIVED** ✔ | M27 S | M15 S | — | — |
| 12 | `resolution != Rename` guard deleted | `apply.rs:107` | apply tests | **SURVIVED** ✔ | M21 S | — | *M17 caught* ✘ | — |
| 13 | `same_entry` drops the `dev` half | `apply.rs:245` | identity tests | **SURVIVED** ✔ | — | — | — | R2 S |
| 14 | `check_then_rename` drops the `!same_inode` carve-out | `fallback.rs:122` | none exists | **SURVIVED** ✔ | — | — | — | R5 S |
| 15 | `numbered` keeps the untruncated stem | `plan.rs:566` | length bounds | **KILLED** (7/12, `prop_plan::bounded_renumbering`) | — | *M13 S* ✘ | — | — |
| 16 | `same_entry` → always true | `apply.rs:245` | identity tests | **KILLED** (4) — control | M2 K | M1 K | M1 K | — |
| 17 | `follow_links(false)` → `true` | `walk.rs:101` | symlink tests | **KILLED** (2) — control | M17 K | M16 K | M13 K | — |
| 18 | `renameat_noreplace` → `renameat_plain` | `fsops.rs:183` | `an_occupied_destination_is_refused` | **KILLED** (1) — control | M1 K | M3 K | M3 K | — |
| 19 | `finish()` writes no `end` record | `journal.rs:177` | undo tests | **KILLED** (7) | M9 K | M10 K | M12 K | — |
| 20 | rename before `intent` (the flagship inversion) | `apply.rs` | `the_intent_is_recorded_before_the_rename_not_after` | UNVERIFIED (3 agree) | M35 K | M5 K | — | — |
| 21 | batch-id validation removed | `journal.rs:445` | `a_batch_id_cannot_escape_the_journal_directory` | UNVERIFIED (3 agree) | M6 K | M6 K | M5 K | — |
| 22 | step-2 occupancy check deleted | `apply.rs:187` | TOCTOU test | UNVERIFIED (3 agree) | M7 K | M2 K | M2 K | — |
| 23 | `Allocator::is_free` → true | `plan.rs:510` | collision tests | UNVERIFIED (3 agree) | M8 K | M12 K | M9 K | — |
| 24 | `aborts_batch` → false | `fsops.rs` | EROFS abort test | UNVERIFIED (3 agree) | M18 K | M7 K | M6 K | — |
| 25 | grapheme truncation → char/byte boundary | `truncate.rs` | ZWJ/combining tests | UNVERIFIED (3 agree) | M11 K | M9 K | M14 K | — |
| 26 | `escape` → `to_string_lossy` | `report.rs` | escape tests | UNVERIFIED (3 agree) | M12 K | M14 K | M21 K | — |
| 27 | `same_inode` → true (rung 1 clobbers) | `fallback.rs:86` | no-clobber tests | UNVERIFIED (2 agree) | M14 K | — | M18 K | — |
| 28 | journal `dir` recorded relative | `journal.rs:222` | `undo_works_from_a_different_working_directory` | UNVERIFIED (2 agree) | M25 K | M33 K | — | — |
| 29 | `is_vcs_dir` → false | `walk.rs` | `vcs_metadata_is_never_touched` | **equivalent mutant** (A's own call) | M28 S | — | — | — |
| 30 | `"." \| ".."` output guard deleted | `pipeline.rs` | pipeline tests | **unreachable mutant** (A's own call) | M29 S | — | — | — |
| 31 | walk `.`/`..` name guard deleted | `walk.rs:157` | walk tests | **UNVERIFIED — A and C disagree**, low value | M30 S | — | M25 K | — |
| 32 | `!converged` ignored | `pipeline.rs` | pipeline tests | **unreachable mutant** (A's own call) | M26 S | — | — | — |

(Rows 20–28 are reported kills with independent multi-reviewer agreement; ~15 further single-reviewer
kill claims across A/B/C are omitted as uncontested and low-risk.)

### 5.3 True survival rate

**≈30 % (14 confirmed coverage gaps over ≈45 distinct mutation sites tested across the run).**

- A's 39 % → 31 % after A's own equivalent/unreachable discount: **closest to right**.
- B's 24 %: close, one false survivor (row 15), and B's set omitted `header`/`finish` fsync,
  `same_entry`'s `dev` half and the `check_then_rename` carve-out.
- C's 12 %: **do not use.** Three of 22 "caught" verdicts refuted, all consistent with the
  stale-binary artifact. C's *findings* are solid; C's mutation log is not.

### 5.4 Coverage gaps that matter, in priority order

1. **No test asserts `apply::attempt` uses one directory handle** (row 7). Highest priority: it is the
   guard for the defect `HANDOFF.md` calls the worst of the previous pass.
2. **No test asserts `replay` matches outcomes to intents by inode** (row 4) — the fix for the
   previous pass's silent-data-loss defect. Behaviour is correct (A, B, C and I all verified by hand);
   the guard is unpinned.
3. **All three `sync_data()` calls are unpinned** (rows 1–3). `kill -9` cannot discharge them, but a
   `JournalWrite` double that counts syncs can.
4. **`walk`'s dedup guard is both wrong (C8) and unpinned** (row 5) — the rare case where the missing
   test and the live defect are the same line.
5. **`volume_case` has no test at all** (row 6). It is the input that keeps a case-insensitive volume
   from being handed a colliding destination; the whole function can be neutered silently. A verified
   the fallout is bounded by the kernel's `EEXIST`, so it is exit-1 noise rather than loss.
6. **`create_new` is unpinned** (row 9) — the only thing stopping two same-second runs from
   interleaving records into one journal file.
7. **`next_seq`'s regression guard is timing-dependent** (row 10). `last_means_most_recently_created`
   only pins ordering because its three batches land in the same UTC second and the `AlreadyExists`
   retry loop renumbers them. It needs batches with *different* stamps.
8. **The `resolution != Rename` guard is unpinned** (row 12) despite being behaviourally observable.
9. **`same_entry`'s `dev` comparison is unpinned** (row 13) — inode numbers are not unique across
   devices.
10. **The demoted rung's `!same_inode` carve-out is unpinned** (row 14). The atomic path has
    `a_case_only_respell_succeeds`; the demoted path has no equivalent, so every case-only rename on a
    filesystem lacking `renameat2` could break unnoticed. R5's finding; fails closed, which is why it
    is not higher.
11. **`plan()`'s call to `check_no_sibling_chains` is unpinned** (row 8) — the function has unit
    tests, the wiring does not.
12. **`prop_plan`/`prop_transform` are unseeded**, so plan-side mutation verdicts are probabilistic
    (§5.1).

---

## 6. Verified-sound list — what genuinely withstood attack

Do not re-litigate these. Each line names the strongest evidence in the run.

1. **The dirfd pin is real and load-bearing.** Strongest evidence: HEAD 0/30 wrong-file renames vs a
   one-line mutant's 5/30 with 14 false journal successes, under an active directory-swap race
   (mine); A's 0 false successes in 448 journalled renames over 30×400 files; B's 0/50 vs 21/50.
   Structurally, on Unix `Dir` carries only an `OwnedFd`, so no path exists inside the rename to
   re-resolve (C's I3). The *guard* is misplaced (C4); the *code* is correct.
2. **No-clobber holds on every reachable path.** Occupied destination on the atomic rung, on the
   demoted `check_then_rename` rung, with a *dangling symlink* as occupant (`lstat`, not `stat`),
   directory-onto-empty-directory (the case plain `rename(2)` allows), case-only destination on
   case-insensitive APFS, truncation-induced collisions, and **inside the C1 escape** (A verified a
   forged record whose escaped destination exists is refused and the victim survives byte-identical).
   Three reviewers attacked this ~8 ways each.
3. **The transform pipeline's safety closure.** ~1.55 million `(input, policy)` pairs across A
   (800 000) and B (746 200) over hostile alphabets (bidi overrides, ZWJ, ZWSP, BOM, Tags, NUL, CR,
   TAB, combining marks, astral emoji, dotted-I, `/`, `*`, `&`, `$`) with byte and UTF-16 limits down
   to 1: **zero violations** of non-empty, never `.`/`..`, no `/` in output, no NUL, no leading `-`,
   no trailing dot or space, both limits respected, dotfile preservation in both directions,
   idempotence, NFC, and no re-truncation on the second pass. No panics from byte-index slicing in
   `split_extension` or `truncate_graphemes`. R4 confirmed the same live through the binary.
4. **Grapheme-safe truncation.** Mutating to `is_char_boundary` is caught by 2–3 tests in every
   reviewer's run; ZWJ families, regional indicators and stacked combining marks are never split;
   608-byte CJK names truncate to 253/255 bytes with correct `-2` renumbering.
5. **Crash consistency.** A total of 18 `kill -9` runs across three reviewers on batches of 400–4000
   items: at most one dangling `intent`, **no rename ever happened without a journal record**, entry
   counts preserved, and `undo` reverted exactly the completed prefix while naming and leaving alone
   the interrupted item. R6 hit item 586/4000 and got exactly 585 reverted.
6. **Corrupt-journal handling, within UTF-8.** Torn final line ignored by design; garbage mid-file
   reported with a line number and ignored; two intents / one `done` reported as an orphan anomaly
   with inode and line; a `done` naming the wrong inode → "neither is trusted and neither will be
   undone"; empty, header-only and directory-as-journal all handled without panic. Every anomaly path
   drives a non-zero exit. (The gap is non-UTF-8 bytes — C2.)
7. **Journal injection is not reachable.** `serde_json` throughout; a filename containing a literal
   newline is recorded as `\n`, one record per line, and round-trips byte-exactly through `undo`.
   Verified with `xxd` by R2, and independently by A, B and C. No reviewer forged a record from a
   filename.
8. **Concurrency.** ~20 000 attempted renames across A (4 runs × 60 iters), B (3 runs × 60 iters), C
   and R6: nothing lost, duplicated or clobbered; entry counts always exact; 12 simultaneous runs
   produce 12 distinct journals. Concurrent `undo`s of one batch split cleanly (`283/117` and
   `117/283`) with no loss. Double-undo refuses every item and exits 1.
9. **Preview is pure.** Full `stat`-level censuses (names, inodes, mtimes to ns, sizes, modes) across
   17 read-ish invocations including every error path, byte-identical; and **the journal directory is
   never even created** by a non-`-x` run (A and B both checked this; the shipped
   `preview_never_writes_anything` does not).
10. **`undo` from a different working directory works** — `absolute()` in the record. Memory note
    20653's defect is genuinely fixed; A, C and R1 each verified independently, including with a
    same-named decoy tree adjacent.
11. **NFD→NFC respell round-trips byte-exactly**, including the exact original NFD bytes after undo,
    on case-insensitive APFS, with no spurious collision and no fallback warning.
12. **Hostile filesystem.** FIFOs, unix sockets and device nodes are renamed as entries and **never
    opened** (no hang, verified with timeouts); dangling links, self-referential links and links to
    `/etc` are renamed as links and never followed; `chmod 000` files and directories and a read-only
    parent produce per-item non-aborting `EACCES` and the batch continues; 188–227-level nesting to
    `PATH_MAX` with no stack overflow; 20 000 files in 0.25 s; 6 000 renames under `ulimit -n 64` and
    100 under `ulimit -n 20` with **no descriptor leak** despite one `open_dir` per item.
13. **No copy-based fallback rung exists anywhere** (R6 checked every rung: `renameat_plain` /
    `fs::rename` only). This closes the permission-leak, ownership-leak and double-existence vectors
    by construction. `EXDEV` is unreachable for the same reason.
14. **Exit-code matrix and CLI contract.** All 12+ documented cases reproduced by A, B, C and R1
    independently, including `-x` with neither `HOME` nor `XDG_STATE_HOME` → exit 2 with **nothing
    renamed** (verified by directory listing). Batch-id path traversal (`../secret`, `/etc/passwd`,
    `sub/../../secret`, `..%2fsecret`) rejected on all four vectors.
15. **`--json` is valid, parseable JSON on every non-exit-2 path**, including the per-item failure
    path, verified with real parsers by three reviewers. (The exit-2 gap is C12; the escaping gap is
    C10.)
16. **No structurally vacuous tests.** Three reviewers hunted specifically and found none: assertions
    compare against the filesystem rather than derived values, the interleaving test threads one event
    log through the journal double and the rename, the crash test asserts it was really interrupted,
    and the preview-purity census includes `mtime`. Several tests carry comments naming the mutation
    that proved an earlier version vacuous, and those strengthened versions do fail correctly. The
    suite is honest; it is *incomplete*, which is a different problem.
17. **`prop_plan` and `prop_transform` pass at HEAD.** 20 runs at default cases and 5 at
    `PROPTEST_CASES=2048`, zero failures. Explicitly recorded because I briefly believed otherwise
    (§4).

---

## 7. Scope-deferred gaps vs. real defects

The line: **a documented, deliberately-deferred gap is not a defect. A doc comment claiming a
guarantee the code does not deliver is.**

| Item | Where deferred | Ruling |
|---|---|---|
| No directory fsync on journal creation | `journal.rs:16-24` states the limit exactly — "survives `kill -9`, not power loss" — and names the upgrade (`File::open(dir)?.sync_all()` + `F_FULLFSYNC`) | **DEFERRED GAP, not a defect.** The doc is honest and the code matches it. |
| The three `sync_data()` calls are untested | `HANDOFF.md` "Still open: the fsync no-op survives the whole suite" | **COVERAGE GAP, acknowledged.** Not a behaviour defect. But the count is 3, not 1 (§3.3), and the live-mutation incident during this review shows why it matters. |
| Windows reserved names (`CON.txt`, `NUL`, `COM1`) pass unchanged | `lib.rs:12-13` (`reserved` is M5); `owner-decisions.md:60` explicitly leaves it open | **DEFERRED GAP.** |
| `invisible.rs` misses U+2028, U+2029, U+180E | `invisible.rs:3-13` — M4 replaces the body with the UCD `Cf`/`Cs`/`Co` closure; self-described stopgap | **DEFERRED GAP** for these three (`Zl`/`Zp`/`Cf`). |
| `invisible.rs` misses **U+061C** | same module, but `:5-7` claims "the named set covers **the whole** CVE-2021-42574 (Trojan Source) class" | **REAL DEFECT (C11).** U+061C is inside the class the comment claims to cover completely. Either add it or narrow the claim. |
| `policy.rs` has no separator validation | `--separator` is M3; `main.rs:65` hardcodes `Policy::default()` | **DEFERRED for the CLI; REAL but LOW for the public core API (C15)** — the field is `pub` with no stated invariant, and the project's own test harness pins `'_'` because otherwise the safety closure is false by construction. |
| `Other(i32)` / `NameTooLong` errno arms, demoted rung in production, non-UTF-8 end-to-end | project's own admissions; APFS never returns `EOPNOTSUPP`/`EINVAL` for `RENAME_EXCL` and refuses non-UTF-8 names at the syscall level | **DEFERRED / untestable on this hardware.** Not defects. |
| One `volume_case` probe for the whole batch; ASCII-only | `walk.rs:249-253`, an explicit `ponytail:` comment naming the ceiling and the upgrade path (a probe per `dev`, with M5's `statfs` work) | **DEFERRED GAP.** Correctly marked. A verified the fallout is bounded by the kernel's `EEXIST`. |
| `{seq:06}` breaks past 999 999 batches | not documented | **SUSPECTED defect, LOW.** Undocumented ceiling; one-line fix (numeric sort). Not blocking. |
| `walk.rs:20-22`'s absolute symlink guarantee | stated as absolute with no exception | **REAL DEFECT (C3).** The strongest possible form of doc-claims-what-code-does-not. |
| `fsops.rs:111-112` "never across directories (§5.2)" | stated as absolute | **REAL DEFECT (C1).** |
| `main.rs:13-15` "2 = the failures where nothing was attempted at all" | stated as absolute, repeated in `--help` | **REAL DEFECT (C6).** |
| `fallback.rs:9-11` "not expected to fire on either tier-1 platform" | stated as an expectation | **REAL DEFECT (C5).** One `ln` on stock APFS fires it. |
| `report.rs:270-272` / `:325-326` reversibility and `utf8` semantics | stated as absolutes | **REAL DEFECT (C10).** |
| `README.md`, `CHANGELOG.md`, `HANDOFF.md:60-61` | — | **REAL DEFECTS (C13, C14).** Not deferred; just wrong. |

---

## 8. Coverage holes across all nine reviewers

Nobody tested these. Listed so the next session can decide what to buy rather than rediscovering the
list.

- **Linux, entirely.** All nine reviewers ran macOS 25.5 / APFS / case-insensitive. Untouched:
  `renameat2` on ext4/btrfs/overlayfs, `Errno::INVAL`-driven demotion, `EMFILE` in `walk`, `EXDEV`,
  `dir_bytes` end to end, and byte-limit truncation against a real 255-*byte* ceiling rather than a
  synthetic small one.
- **Windows / the `#[cfg(not(unix))]` best-effort tier.** Never compiled, never run.
  Worth flagging: on that tier `Dir` is a `PathBuf`, so **C1's escape also applies there** via
  `d.join(from)` — and `fallback::ident_at_path` reports `dev: 0, ino: 0`, so `same_entry`
  degenerates to "the name still exists". A noted the first half; nobody tested any of it.
- **Non-UTF-8 filenames end to end.** APFS refuses to create them (`errno 92`, reproduced by two
  reviewers), so `SkipReason::NotUtf8`, `key_of_os`'s `0xFF` tagging, `put_os`'s `_unrepresentable`
  branch, `escape`'s WTF-8 path and the `dir_bytes` journal encoding are exercised only by unit tests.
  Nobody built a FAT/exFAT image. This is directly relevant to C2 — the journal legitimately holds
  non-UTF-8 bytes and `replay` cannot read them.
- **The demoted `check_then_rename` rung in production.** No filesystem available refuses
  `RENAME_EXCL`/`RENAME_NOREPLACE`, so `fallback::demote()` never fires in a real run and its TOCTOU
  window is untested outside unit tests. Combined with row 14's coverage gap, this rung is the least
  validated code in the crate.
- **Real power loss.** Only `kill -9`, which does not evict the page cache. This is a genuine ceiling,
  not an oversight — but it means the three `sync_data()` calls cannot be validated by any test this
  project can run, which is an argument for pinning them structurally rather than behaviourally.
- **Live `EROFS` / `ENOSPC` / `EDQUOT`.** No read-only mount, no full filesystem, no quota. Only real
  `EACCES` and the fault-injecting `AlwaysFails` double. C's suspected finding — a failed `done` write
  demoting a completed, reversible rename to "check by hand" — needs exactly this to reproduce.
- **Inode reuse as a live race.** `same_entry` compares `(dev, ino)`; nobody built a delete/recreate
  loop to try to land a reused inode inside the check-to-rename window.
- **`-x` racing `undo`** — B ran it once and found the mitigation fires; nobody ran it at volume.
  Nobody raced `undo` against `undo --last` batch resolution at volume either (R6 hit it once by
  accident, which is how C7 route 3 surfaced).
- **Cross-mount `volume_case`.** Nobody had two mounts with opposite case behaviour; nobody built a
  disk image.
- **>999 999 batches**, and **>227-level nesting** (`PATH_MAX` was the limit, not the tool).
- **Memory behaviour on very large trees.** `main.rs:117` buffers all progress lines in a `Vec<u8>`
  on the `--json`/`-q` path. C noted it; nobody measured it.
- **`just gate`** — clippy, rustfmt, MSRV, the dependency budget, `cargo-deny`, `cargo-vet`. Every
  reviewer ran `cargo build` and `cargo test` only. A gate failure would be a cheap, embarrassing miss.
- **`--json` against a written schema.** There is no schema document; reviewers checked emitted keys
  against `report.rs`'s own doc comments. The `schema` field in the output has nothing to point at.
- **`docs/research/` and `docs/plans/`** — the 22-file corpus and the proposal's §-numbering. Every
  reviewer checked only the §-references that were load-bearing for a specific finding. Given that
  `README`, `CHANGELOG` and `HANDOFF` all contained falsehoods, the corpus is likely to as well.
- **Proptest determinism** (§5.1) — nobody noticed the unseeded property tests make mutation verdicts
  probabilistic, which is a live methodology hazard for the next pass.

---

## 9. Reviewer reliability notes

Blunt, for calibration.

**Opus A — highest overall value. Weight: high.**
Broadest and most honest. Found both HIGH escapes (C1, C3) with correct mechanisms, and A's
explanation of the trailing-slash escape (`lstat("link/")` follows; `follow_links(false)` governs only
discovered entries) is the single best piece of root-cause analysis in the run. A voluntarily labelled
four of its own 14 survivors as equivalent or unreachable — the only reviewer who discounted its own
headline number downward. A's one error is an undercount: it found two `sync_data()` calls when there
are three. A also flagged the working-tree contamination early and moved to an isolated copy. Its
mutation log survived my re-derivation intact.

**Opus B — best single experiment in the run; weakest verdict. Weight: high on the pin and
concurrency, medium on breadth.**
B-1 is the most valuable finding here and B's discriminating race is the right instrument, correctly
built and correctly interpreted; I reproduced it and the numbers are not inflated. B's concurrency and
crash work (12 `kill -9` runs on 1 500-item batches, 4 500-rename concurrency sweeps, 746 200 transform
pairs, 120 000 randomized `plan()` calls) is the most systematic in the run. But B's CONDITIONAL PASS
is wrong, for a diagnosable reason: B attacked the journal as a crash artifact and an injection
surface, never as a *hostile input to `apply`*, and tested symlinks in five shapes but not with a
trailing slash. One false survivor (M13) from a single run of an unseeded property test. Trust B's
experiments; do not trust B's overall verdict without checking what it covered.

**Opus C — excellent findings, unusable mutation log. Weight: high on findings, low on §6.**
C's invariant table (I1–I19) is the best organising device any reviewer produced and I would keep that
format. C found both HIGH escapes independently, including the absolute-path variant of C1 that A did
not test, and C's method note — discovering another reviewer's live mutation in the shared tree,
proving it with `git show`, discarding the would-be finding, and re-basing on an md5-verified pinned
checkout — is exactly right and should be the template. But **three of C's 25 mutation verdicts are
refuted**, all in the same direction (false "caught"), all consistent with the stale-binary artifact
C's own colleague R3 had warned about. C's 12 % survival rate is the most misleading number in the run
and would have led a next session to conclude the suite was strong where it is weakest.

**R1 spec/UX — reliable, low yield. Weight: medium.**
Everything R1 said held up under re-verification. Found only the README drift, but its "HOLDS" list was
accurate and independently corroborated. Honest about what was untestable.

**R2 journal + crash — highest value per unit of effort. Weight: high.**
Found the one finding no Opus reviewer found and the one that costs recoverability (C2), with the right
control experiment attached (same offset, valid UTF-8) — which is exactly what made it adjudicable in
minutes rather than hours. Also found the `same_entry` `dev`-half survivor that nobody else tested.
Both of R2's mutation verdicts re-derived correctly. Take R2's findings at face value.

**R3 test vacuity — the most valuable methodological contribution in the run. Weight: high.**
R3 discovered trap #2 and wrote it down. That warning is the reason this adjudication caught its own
stale-binary artifact instead of shipping a phantom HEAD defect (§4). Both R3 survivors re-derived
correctly. R3's "no fourth structurally-vacuous test" conclusion matches B's and C's independent hunts.
If future runs keep one thing from this pass, keep R3's method note.

**R4 core transforms — accurate and correctly scoped. Weight: high.**
Every R4 finding re-verified: U+2028/2029/061C/180E all survive (I confirmed 4 of 6 unchanged), and
`Policy{separator:'/'}` does break the closure. R4's discipline in labelling the separator issue "NOT
reachable via shipped CLI" is exactly the scoping A/B/C should have applied to their own absolutes.

**R5 dirfd refactor — one real find, one dangerous overclaim. Weight: medium; discount categorical
claims.**
R5 found a genuine, otherwise-unreported survivor (the `check_then_rename` `!same_inode` carve-out —
re-derived, survives) and its "HELD" list about the pin is accurate. But R5's "CLAIMED IMPOSSIBLE" on
separator injection is true only of the forward path, and stated without that scope it is the kind of
assertion that suppresses a HIGH — it would have been read as refuting A and C had the digest not
annotated it. R5 reviewed the refactor and did not read the second consumer of the API it was
reviewing.

**R6 concurrency + hostile FS — broad, honest, nearly misleading. Weight: medium-high.**
Enormous coverage (~150 real invocations of the dir-swap TOCTOU, 200 concurrent applies, a real `kill
-9` at item 586/4000, 20 000 files, `PATH_MAX` nesting) and every positive result held. R6 also
established the genuinely reassuring architectural fact that **no copy-based fallback rung exists
anywhere**, which closes a whole vector class. Two of R6's "HELD" entries — no symlink escape, correct
hardlink handling — are true of what R6 tested and read as refutations of A/B/C findings. R6 also
surfaced C7's third route by accident and reported it accurately (including that the data was fine).

**The digest compiler — deserves credit.** The three `*** ... Adjudicate. ***` annotations in
`sonnet-digest.md` (R5's scope limit, R6's untested trailing slash, R6's different hardlink scenario)
each pointed at a genuine conflict and each turned out to be correctly characterised. Every one of
those three annotations resolved the way the annotation implied it would. That triage saved this
adjudication real time and is worth keeping as a step.

**Structural lesson for future runs.** Nine agents mutating one working tree concurrently produced at
least one finding that was another reviewer's mutation, forced three reviewers to abandon and re-base
mid-pass, and left the mutation logs of two reviewers partly wrong. Give every reviewer its own
`git archive` checkout up front, mandate build-then-execute-the-binary for mutations with an
`os.utime()` on both apply and revert, mandate an assertion that the crate actually recompiled, seed
the property tests, and restore `*.proptest-regressions` when done. That is a five-line harness change
that would have removed most of the noise this pass had to adjudicate.
