# Adversarial review — reviewer `opus-2`

Whole application, emphasis on filesystem safety and data loss.
Baseline: `a144fe9`, `cargo build` clean, `cargo test` = 166 tests, 0 failures.
All probes ran against throwaway trees under the session scratchpad with
`XDG_STATE_HOME` redirected there. Every mutation was reverted; `git status` is
clean for tracked files.

## Verdict table

| Area                                                        | Verdict              | Evidence                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------------------------- | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Apply order (parent renamed after its contents)             | **FAIL**             | O2-1, reproduced: parent directory renamed first, two items then fail `ENOENT`, tree left half-cleaned                                                                                                                                                                                                               |
| Truncation / length limits                                  | **FAIL**             | O2-2, reproduced: two legal 305-byte APFS names truncated to 255, collide, distinguishing bytes destroyed; O2-5, truncation never reported                                                                                                                                                                           |
| Exit code fidelity                                          | **FAIL**             | O2-3, reproduced twice: conflicts → exit 0; unreadable subtree → exit 0, `0 failed`                                                                                                                                                                                                                                  |
| Symlink integrity after a clean                             | **FAIL**             | O2-4, reproduced: `l ink -> t arget.txt` left dangling, reported `2 renamed, 0 failed`, exit 0                                                                                                                                                                                                                       |
| No-clobber rename (`RENAME_NOREPLACE`)                      | **PASS**             | Occupied destination refused on disk in every shape probed: plain file, directory, dangling symlink, second hardlink to the source, case-folded collision on insensitive APFS, non-ASCII case fold. Content byte-identical after each.                                                                               |
| Hardlink handling (C5)                                      | **PASS**             | `ln 'a b.txt' 'a_b.txt'; detoxrs -x 'a b.txt'` → `EEXIST`, exit 1, both names and the shared inode intact                                                                                                                                                                                                            |
| Directory-fd pinning (rename anchored to the checked dirfd) | **PASS (partial)**   | `fsops` unit test swaps the directory out from under the pin and the rename still lands in the pinned inode; `apply` opens exactly once per item (`CountingOps`); mutation of both apply-time rechecks is caught by tests. Residual gap: O2-7 (dir identity itself is never verified, `open()` has no `O_NOFOLLOW`). |
| Symlink roots (`link` vs `link/`)                           | **PASS**             | Both spellings produce identical output, own name cleaned, no descent (probed on disk)                                                                                                                                                                                                                               |
| Symlink loops / deep trees                                  | **PASS**             | `sub/u p -> ..` plus `se lf -> sub`: no descent, no hang. 120-deep tree: 121 renames, all correct.                                                                                                                                                                                                                   |
| Read-only / EACCES directory                                | **PASS**             | Per-item `EACCES`, batch continues, exit 1, sibling still renamed                                                                                                                                                                                                                                                    |
| Undo round trip incl. nested directories                    | **PASS**             | 6-item nested apply then `undo --last` restored the tree byte-for-byte; undo order is the reverse of the forward order, so each recorded `dir` resolves when its turn comes                                                                                                                                          |
| Partial undo failure                                        | **PASS (behaviour)** | Squatting the parent's original name refuses both items, destroys nothing, exit 1. Message quality is O2-8.                                                                                                                                                                                                          |
| Journal sequencing / `undo --last` selection                | **PASS**             | `next_seq` produced 000001..000003 in order in a fresh state dir; `resolve_last` skips empty batches. Footgun: O2-9.                                                                                                                                                                                                 |
| Journal record trust boundary (traversal in `from`/`to`)    | **PASS**             | Existing test drives a forged journal through the real apply loop and asserts on disk; re-read line by line, `is_plain_basename` + absolute-`dir` check are both applied before an `UndoItem` exists                                                                                                                 |
| Cross-device / mount points                                 | **NOT ESTABLISHED**  | Renames are always intra-directory so cross-device rename is unreachable, but nothing was tested about renaming a mount point's own entry, and `walkdir` is not restricted to one filesystem                                                                                                                         |
| TOCTOU between plan and apply under an active attacker      | **NOT ESTABLISHED**  | The deterministic guards are covered (mutations caught); no live race was driven. O2-7 records the reasoned gap.                                                                                                                                                                                                     |
| Power-loss durability                                       | **NOT ESTABLISHED**  | `sync_data` on the file only, no directory fsync — documented in `journal.rs`, not tested here                                                                                                                                                                                                                       |

---

## O2-1 — HIGH — Order safety is decided by path _spelling_, so a parent directory can be renamed before its contents

`crates/detoxrs-core/src/plan.rs:414` (`deterministic_order`, `depth: Reverse(e.dir.components().count())`), with
`crates/detoxrs/src/walk.rs:118-145` supplying the `dir` spellings.

### What's wrong

`plan.rs`'s module doc calls order safety a data-loss property, and `Entry::depth`
carries the comment "Ordering does **not** trust this field: it is derived from
`dir` instead, so Order safety cannot be broken by a walker that miscounts."
The derivation is `e.dir.components().count()` — the number of components in the
_string_ that happens to name the containing directory. That string comes from
whichever command-line argument first reached the entry, and `walk.rs` never
canonicalises it. `Components` drops `.` but keeps `..`, and an absolute argument
has more components than a relative one, so two arguments naming the same tree
can give a parent entry a _larger_ component count than the entry for a directory
inside it. The sort then puts the parent first, and §5.3's invariant is inverted.

The `dir_ident` machinery already knows these are the same directory (it is used
to deduplicate `seen` and to key the collision engine), so the information needed
to order correctly is present and discarded.

### Concrete failure scenario (CONFIRMED, reproduced)

```
$ mkdir -p "de ep/d ir" && echo hi > "de ep/d ir/fi le.txt"
$ detoxrs -r "de ep/d ir" "de ep/../de ep"
de ep/..
  de ep/  ->  de_ep/
de ep/d ir
  fi le.txt  ->  fi_le.txt
de ep
  d ir/  ->  d_ir/

3 to rename, 0 unchanged, 0 skipped, 0 conflicts.
```

The parent is listed _first_ — the preview itself shows the inverted order.
Applying it:

```
$ detoxrs -x -r "de ep/d ir" "de ep/../de ep"
de ep/../de ep  ->  de_ep
detoxrs: de ep/d ir/fi le.txt: cannot open the containing directory: no longer there (ENOENT)
detoxrs: de ep/d ir: cannot open the containing directory: no longer there (ENOENT)

1 renamed, 0 unchanged, 0 skipped, 0 conflicts, 2 failed.
exit 1

$ find . | sort
./de_ep
./de_ep/d ir
./de_ep/d ir/fi le.txt
```

One rename applied, two refused, the tree permanently half-cleaned. Re-running
does clean the rest, but the guarantee the design is staked on ("a parent is
never renamed before its contents") is false, and the failure mode is a batch
that reports the right count for the wrong reason: the items did not fail on
their own merits, they failed because detoxrs pulled the directory out from under
them. Nothing here protects the ENOENT case from becoming a _wrong-directory_
case if anything recreates the vacated name between the two items — the identity
recheck would then be the only remaining guard, and it accepts any entry sharing
the recorded `(dev, ino)`, which a hardlink satisfies.

A second, more ordinary invocation shape hits the same key: mixing an absolute
and a relative spelling of the same subtree. `detoxrs -r "de ep/d ir" "$PWD/de ep"`
happens to come out safe only because the child's own entry is discovered under
the same long spelling; the ordering is decided by argument order and string
length, not by containment, so which invocations are safe is accidental.

Confidence: **CONFIRMED**.

---

## O2-2 — HIGH — Default length policy truncates legal macOS filenames and silently destroys the part that made them distinct

`crates/detoxrs-core/src/policy.rs:101` (`M1_MAX_LEN = 255` used for _both_ the byte and the UTF-16 budget),
consumed at `crates/detoxrs-core/src/pipeline.rs:218-222`.

### What's wrong

The comment claims the constant "is wrong only on filesystems nobody is running
yet, and only in the over-truncating direction". Both halves are wrong on the
project's own tier-1 macOS target. APFS/HFS+ limit names to 255 _UTF-16 code
units_, not 255 bytes. Applying a 255-_byte_ budget to a name that is legal on
the filesystem in front of you turns "clean this name" into "shorten this name",
and the shortening is applied to names that are otherwise already clean —
`desired_for` runs `transform` on every entry, so truncation alone is enough to
produce a `Rename`. Over-truncation is not a conservative direction when the
truncated prefixes of two different names are equal: the collision engine then
numbers them, and the bytes that told them apart are gone from the filesystem.

### Concrete failure scenario (CONFIRMED, reproduced on APFS)

Two files, 305 bytes / 105 UTF-16 units each — well inside APFS's limit, and
containing nothing the pipeline objects to except length:

```
$ python3 -c "open('漢'*100+'a.txt','w').write('A'); open('漢'*100+'b.txt','w').write('B')"
$ detoxrs -x -r .
...a.txt  ->  漢…漢.txt
...b.txt  ->  漢…漢-2.txt

2 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.
exit 0

$ python3 -c "
import os
for f in sorted(os.listdir('.')): print(len(f.encode()),'bytes',open(f).read())"
255 bytes B
253 bytes A
```

Exit 0, "0 conflicts", no warning. After the run, `…a.txt` and `…b.txt` are
`漢…漢.txt` and `漢…漢-2.txt`: the user cannot tell which is which, and there is
nothing in the output saying a name was shortened (see O2-5). The
`-2` is the tool inventing a distinction to replace the one it removed. Both files
were legal, both were readable, and nothing about them needed changing.

The `undo` journal does make this recoverable, which is the only reason this is
HIGH and not CRITICAL — but only until the journal is pruned, and only for a user
who notices.

Confidence: **CONFIRMED**.

---

## O2-3 — MEDIUM — A run that could not do what it was asked exits 0

`crates/detoxrs/src/apply.rs:76` (`Summary::exit_code`), `crates/detoxrs/src/main.rs:174`,
plus `crates/detoxrs/src/walk.rs:240-243` for the second reproduction.

### What's wrong

`--help` documents exit 1 as "one or more items could not be renamed". Two
distinct classes of "could not be renamed" produce exit 0 instead:

1. **Conflicts.** `Resolution::Conflict(_)` items are never attempted, so they
   never enter `Summary::failed`. A `-x` run whose every dirty name conflicts
   reports the conflicts and exits 0.
2. **Subtrees the walk could not read.** `walk_into` warns to stderr and
   continues, and nothing downstream records that the snapshot is incomplete.

Either way `detoxrs -x -r . && echo clean` prints `clean`.

### Concrete failure scenarios (CONFIRMED, reproduced)

```
$ echo a > "a b.txt"; echo b > "a_b.txt"
$ detoxrs -x --on-collision skip -r .
.
  a b.txt  !   conflict (that name is already taken)

0 renamed, 1 unchanged, 0 skipped, 1 conflicts, 0 failed.
exit 0
```

```
$ mkdir -p secret && echo x > "secret/h idden.txt" && echo y > "o k.txt"
$ chmod 000 secret
$ detoxrs -x -r .
detoxrs: warning: IO error for operation on ./secret: Permission denied (os error 13)
./o k.txt  ->  o_k.txt

1 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.
exit 0
```

The second is the worse of the two: `0 failed` is a positive claim, and an entire
subtree was never inspected. `WalkError` exists for exactly this class of "the
walk could not see what it was asked to see" and is deliberately not used here;
the exit code should still carry it.

Confidence: **CONFIRMED**.

---

## O2-4 — MEDIUM — Cleaning a tree silently breaks relative symlinks inside it, and reports success

`crates/detoxrs/src/apply.rs:204-228` (a symlink's own name is renamed; its target is never considered),
`crates/detoxrs/src/walk.rs:436-447` (`EntryKind::Symlink` is recorded and then treated as any other entry).

### What's wrong

detoxrs renames a symlink's directory entry and also renames the file that
symlink points at, in the same batch, and never notices that the second rename
invalidates the first link. §5.6's "renamed as the link itself, never followed"
is about _not dereferencing_, which is right; it is not an answer to "the link
now points at nothing". The preview shows two independent renames; there is no
note, no warning, and no exit-code signal that a working link became dangling.

This is the one case in the tool where a normal, non-adversarial run _creates_
breakage rather than failing to fix it.

### Concrete failure scenario (CONFIRMED, reproduced)

```
$ echo TARGET > "t arget.txt"; ln -s "t arget.txt" "l ink"
$ cat "l ink"
TARGET
$ detoxrs -x -r .
./t arget.txt  ->  t_arget.txt
./l ink  ->  l_ink

2 renamed, 0 unchanged, 0 skipped, 0 conflicts, 0 failed.
exit 0

$ readlink "l_ink"
t arget.txt
$ cat "l_ink"
cat: l_ink: No such file or directory
```

The link was valid before the run and is dead after it, and the tool reports
`0 failed`, exit 0. At minimum this needs to be a per-item note in the preview
(the information is available with no extra I/O: the link's target is a name, and
the batch knows every name it is about to change in that directory).

Confidence: **CONFIRMED**.

---

## O2-5 — MEDIUM — Truncation is computed, plumbed, and then thrown away; no output ever says a name was shortened

`crates/detoxrs-core/src/pipeline.rs:29` (`Outcome::truncated`), discarded at
`crates/detoxrs-core/src/plan.rs:355-359` (`desired_for` matches `TransformResult::Name(o)` and keeps only `o.text`).

### What's wrong

`Outcome.truncated` is maintained carefully through stage 12 and the stage-13
loop, has its own property test, and is then dropped on the floor: `Desired::Rename(o.text)`
keeps the text and nothing else. `PlanItem` has no truncation field, `report::line`
has no truncation note, and `report::json` has no truncation key. The one
transformation that destroys information rather than rearranging it is the one
transformation the report cannot mention.

```
$ grep -rn "truncated" crates --include="*.rs" | grep -v tests | grep -v truncate.rs
crates/detoxrs-core/src/pipeline.rs:29:    pub truncated: bool,
crates/detoxrs-core/src/pipeline.rs:222:    let (mut text, mut truncated) = apply_truncate(&text, &limits);
crates/detoxrs-core/src/pipeline.rs:252:        truncated |= cut;
crates/detoxrs-core/src/pipeline.rs:272:        _ => TransformResult::Name(Outcome { text, truncated }),
```

No consumer outside the crate's own tests.

### Concrete failure scenario (CONFIRMED)

The O2-2 reproduction is also this one: `--json` for that run emits
`"resolution": "rename"`, `"note": null` for both items. A machine consumer
auditing a batch has no way to distinguish "spaces became underscores" from
"50 characters were deleted from the end of the name".

Confidence: **CONFIRMED**.

---

## O2-6 — MEDIUM — The Order-safety property test cannot fail for O2-1, and the ordering key's stated safety argument is unverified

`crates/detoxrs-core/tests/prop_plan.rs:288-312` (the property), `:155-175` (`snapshot()` / `build_snapshot`).

### What's wrong

Two independent reasons the green suite says nothing about O2-1:

1. **The generator only ever produces one canonical spelling per level.**
   `build_snapshot` hardcodes `t`, `t/a b`, `t/a b/c d`. No `..`, no absolute
   path, no second spelling of one directory — i.e. exactly the input class the
   defect needs.
2. **The assertion is textual.** `prop_assert!(!later.dir.starts_with(&container))`.
   Even if the generator produced `de ep/../de ep`, `"de ep/d ir".starts_with("de ep/../de ep")`
   is `false`, so the property passes on the failing case. The property is
   written in the same units as the bug.

And the claim in `Entry::depth`'s doc ("Ordering does not trust this field … so
Order safety cannot be broken by a walker that miscounts") is not tested at all:

```
# mutation: use the untrusted field as the sort depth
-            depth: Reverse(e.dir.components().count()),
+            depth: Reverse(e.depth as usize),
$ cargo test -p detoxrs-core
test result: ok. 56 passed; 0 failed …   (every target green)
```

The two are interchangeable as far as the suite is concerned, so the deliberate
choice the comment defends is unguarded. (Reverting the sort direction _is_
caught — `Reverse(usize::MAX - count)` fails 1 test — so only the direction is
covered, not the derivation.)

Both mutations were reverted; `git status` clean.

Confidence: **CONFIRMED** (mutation run).

---

## O2-7 — LOW — `apply` re-resolves `item.dir` by path, follows a symlink to get there, and never checks the directory's recorded identity

`crates/detoxrs/src/apply.rs:169-171` (step 0), `crates/detoxrs/src/fsops.rs:300-314` (`open_dir`, no `O_NOFOLLOW`),
`crates/detoxrs-core/src/plan.rs:98` (`Entry::dir_ident`, recorded by the walk and never copied into `PlanItem`).

### What's wrong

The dirfd pin is real _within_ one item — everything after step 0 goes through the
one descriptor, and both the `CountingOps` test and my mutation run confirm the
apply-time rechecks are load-bearing. But the pin is established by resolving
`item.dir` as a path at apply time, after the walk has finished, and:

- `open_dir` sets `O_DIRECTORY | O_RDONLY | O_CLOEXEC` and deliberately not
  `O_NOFOLLOW`. The justification given is about command-line arguments ("a user
  who names one on the command line is pointing at its target on purpose"), but
  the same call is used for every _discovered_ subdirectory under `-r`, where the
  walk explicitly refused to descend through symlinks. A subdirectory replaced by
  a symlink between the walk and its item's turn is followed.
- The walk already computed `dir_ident` (`(dev, ino)` of the containing
  directory) and `plan()` keys its collision maps on it, but `PlanItem` does not
  carry it, so `apply` cannot and does not compare the directory it opened
  against the directory that was previewed. Confinement rests entirely on the
  entry's own `(dev, ino)` recheck in step 1 — which any second hardlink to the
  same inode satisfies, since `same_entry` compares only `dev`/`ino` by design.

Consequence, reasoned not driven: with a pre-existing hardlink to a victim file
under the same basename in an attacker-controlled directory, swapping the walked
subdirectory for a symlink to it during a batch makes the rename land on the
attacker's directory entry while the journal records the original path. The harm
is bounded (no clobber, same inode, the victim's own name simply stays dirty and
the journal line is misleading), which is why this is LOW — but the fix is one
`u64` pair in `PlanItem` plus one comparison, and the invariant the module doc
claims ("A descriptor resolved once is one directory, permanently") is only true
from step 0 onward, not from the walk onward.

Confidence: **PLAUSIBLE** (no live race driven).

---

## O2-8 — LOW — `RenameErr` renders as "rename failed" on paths where no rename was attempted, and `ENOTDIR` is unmapped

`crates/detoxrs/src/fsops.rs:83` (`Other(n) => "rename failed (errno {n})"`),
`crates/detoxrs/src/fsops.rs:317-334` (`map_errno` has no `ENOTDIR` arm).

The same `RenameErr` is returned by `open()` and `ident_at()`, so its Display
text lands in messages about operations that are not renames, and any errno
outside the eight mapped ones is reported by number.

### Concrete failure scenario (CONFIRMED, reproduced)

Squat the parent's original name, then undo (O2-1's neighbour case):

```
detoxrs: …/pu/a_b: a b appeared since the preview; not renamed
detoxrs: …/pu/a b/f_1.txt: cannot open the containing directory: rename failed (errno 20)
0 reverted, 2 refused.  exit 1
```

errno 20 is `ENOTDIR` — the path component `a b` is now a regular file. The
message says "cannot open the containing directory: rename failed", which is
self-contradicting, and leaves the user to look up 20. Behaviour is correct
(nothing was destroyed, exit 1); only the diagnostic is wrong.

Confidence: **CONFIRMED**.

---

## O2-9 — LOW — `detoxrs undo --last` run twice re-applies the renames it just reverted, and calls it "reverted"

`crates/detoxrs/src/main.rs:214-278` and `:328-344` (`resolve_last`).

An undo writes its own journal, and that journal is the newest, so a second
`undo --last` selects it and replays it — putting the cleaned names back and
reporting `1 reverted, 0 refused`, exit 0.

```
$ detoxrs -x "g h.txt" ; detoxrs undo --last   # -> "g h.txt"
$ detoxrs undo --last
…/du/g h.txt  ->  g_h.txt
1 reverted, 0 refused.   exit 0
$ ls
g_h.txt
```

"An undo is itself undoable" is a stated design goal, so this is intended
mechanically — but the word `reverted` is wrong for a redo, and a user who types
the command twice (believing the first did not take) silently re-dirties the
tree. A one-line note when the selected batch is itself an undo would cost
nothing.

Confidence: **CONFIRMED**.

---

## O2-10 — LOW — A batch whose journal could not be closed still exits 0

`crates/detoxrs/src/main.rs:141-146`.

`j.finish()` failing means the batch has no `end` record, and `undo` will
correctly warn "no completion record, so it either crashed or is still running"
and exit non-zero _then_. The forward run that produced that state prints a
stderr warning and returns `s.exit_code()` unchanged — 0 if every rename
succeeded. The same class as O2-3: the run knows its own safety net is
incomplete and does not say so in the one channel a script reads.

Confidence: **PLAUSIBLE** (read from control flow; not driven, since forcing
`finish()` to fail needs the fault injection `tests/apply.rs` already has for
`XDG_STATE_HOME`).

---

## Unexplained observation (not a finding)

One preview run of the O2-1 shape (`detoxrs -r "de ep/d ir" "$PWD/de ep"`)
printed `skipped (transform did not reach a fixed point)` —
`Unrepresentable::NotConverged` — for all three of `de ep`, `d ir` and
`fi le.txt`, and `0 to rename, 3 skipped`. Those names converge in one stage-13
iteration, so `NotConverged` should be unreachable for them, and the same command
on the same tree has produced `3 to rename` on every one of ~45 subsequent
attempts (including a verbatim replay of the original block). I could not
reproduce it and cannot explain it; recording it because `NotConverged` is
documented as "a bug report against us" and a single sighting of it in the field
would be worth this paragraph. **NOT ESTABLISHED.**

## Housekeeping

- Mutations applied and reverted: `plan.rs` ×2, `apply.rs` ×2, `walk.rs` ×1.
  `git checkout --` after each; `git status --short` shows no tracked
  modifications.
- Coverage confirmed by mutation (guard removed → test fails): `apply`'s step-1
  identity recheck (2 failures), `apply`'s step-2 occupancy check (1 failure),
  `walk::trim_trailing_slash` (2 failures), the sort direction in
  `deterministic_order` (1 failure).
- I removed an untracked `crates/detoxrs-core/proptest-regressions/pipeline.txt`
  while cleaning up after the mutation runs. If a concurrent reviewer created it,
  it needs regenerating; it is not in git.
