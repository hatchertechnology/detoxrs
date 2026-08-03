# Adversarial review — reviewer `opus-3`

Whole application, emphasis on the undo journal and round-trip integrity.

## Method

All experiments were run against an **isolated snapshot of `HEAD` (a144fe9)** copied into the
scratchpad with `git archive HEAD | tar -x`, not against the working tree. Reason: while other
reviewers were running mutation tests in the shared checkout, `cargo test --workspace` in the
real repo produced **contradictory results across consecutive runs** — including
`assertion failed: is_invisible('\u{061c}')` for a character that is plainly in the match arm,
and a `no_pre_existing_clobber` proptest failure. Re-reading `crates/detoxrs-core/src/invisible.rs`
showed the file had changed between two reads. In the isolated snapshot the suite is green and
stable (8 consecutive `prop_plan` runs, 3 consecutive full-workspace runs, 0 failures), so
**none of that flakiness is a product defect** and none of it is reported below. It is recorded
only as a warning to the coordinator: any reviewer who ran the suite in the shared tree got
untrustworthy output.

Journals were isolated with `XDG_STATE_HOME` pointed at the scratchpad; `-x` was never run
outside the scratchpad. Two mutations were performed, both **only in the scratchpad copy**, so
nothing under `crates/` in the real repo was ever modified.

Empirical work performed:

- Full byte-for-byte round trip on a 78-entry hand-built nasty tree (controls, bidi, ZWSP,
  newline, quotes, shell metacharacters, NFC/NFD pair, dotfiles, a 406-byte multi-byte name, a
  symlink, a hardlink, three nesting levels) — snapshot manifest of `(relative path bytes, inode,
content hash)` before `-x` and after `undo --last`.
- A randomized round-trip fuzzer over a nasty alphabet (**400 iterations**, random nested
  directories, real binary, real journal, manifest diff each time).
- `kill -9` mid-batch on a 4000-file run, then `undo --last`.
- Hand-truncated journal mid-`intent`.
- 12 concurrent `-x` runs sharing one journal directory.
- Reoccupied-destination undo, double undo, journal-unavailable paths, crafted journal filenames.

---

## O3-1 — CRITICAL — any filename containing `\` is renamed but can never be undone

`crates/detoxrs/src/journal.rs:433` (`is_plain_basename`), against
`crates/detoxrs-core/src/classes.rs:22`

### What's wrong

`\` is in the **separator class**, so `transform` rewrites it and `-x` renames every name that
contains one. `is_plain_basename`, which gates every `intent` record at replay time, rejects any
`from`/`to` whose bytes contain `\`:

```rust
fn is_plain_basename(name: &OsStr) -> bool {
    let bytes = name.as_encoded_bytes();
    !bytes.is_empty()
        && bytes != b"."
        && bytes != b".."
        && !bytes.contains(&b'/')
        && !bytes.contains(&b'\\')      // <-- rejects a legal Unix filename
}
```

So the tool deliberately renames a class of names that its own journal deliberately refuses to
restore. The rename is **permanently unrecoverable**: the journal record is well-formed and
durable, and `undo` will reject it identically on every future attempt. This defeats the single
guarantee the whole design is staked on ("an unjournaled rename is the one thing `undo` cannot
reverse" — the record exists and is still unusable).

The stated rationale (C1: "a journal is portable text that can be replayed on a different
platform than the one that wrote it") does not hold up: on Windows `\` cannot appear in a
filename at all, so a Windows-written record can never contain one, and a Unix-written record
containing one is exactly the record this check destroys.

### Concrete failure scenario (reproduced)

```
$ printf 'data' > "m1/back\\slash.txt"; printf 'x' > "m1/a b.txt"
$ detoxrs -x -r m1
m1/a b.txt  ->  a_b.txt
m1/back\slash.txt  ->  back_slash.txt

2 renamed, 1 unchanged, 0 skipped, 0 conflicts, 0 failed.
Undo with: detoxrs undo 000001-20260803T212517Z
$ detoxrs undo --last
detoxrs: journal problem: line 4 has a 'from' that is not a plain filename (a path separator,
  `.`, `..`, or empty); refusing rather than risking a rename outside the pinned directory
detoxrs: journal problem: line 5: a 'done' with no intent before it
/.../m1/a_b.txt  ->  a b.txt

1 reverted, 0 refused. This undo is itself batch 000002-20260803T212517Z.
$ ls m1
a b.txt        back_slash.txt        # back\slash.txt is gone for good
```

Scale check on the 78-entry tree: `-x` reported `70 renamed`, `undo --last` reported
`66 reverted, 0 refused`, and the manifest diff showed exactly the 4 `back\slash.txt` entries
(one per directory level) still bearing their cleaned names — inode-identical, so the files
survive, only their names are unrecoverable.

### Mutation evidence

Deleting the single line `&& !bytes.contains(&b'\\')` in the scratchpad copy makes the same
round trip succeed (`back_slash.txt -> back\slash.txt`, `1 reverted, 0 refused`) **and leaves the
entire workspace test suite green** — 165 tests, 0 failures. The guard has no test asserting its
purpose and no test asserting its false-positive cost. `grep -rn backslash crates/` finds nothing
outside the research docs.

### Fuzz corroboration

400 randomized round-trip iterations with `\` **excluded** from the name alphabet: **0
failures**. The same fuzzer with `\` included fails whenever a `\` is generated (4 of the first
40 iterations). `\` is the unique cause; nothing else in the tested space breaks the round trip.

Confidence: **CONFIRMED**

---

## O3-2 — MEDIUM — `undo`'s closing tally hides items that were dropped before the apply loop

`crates/detoxrs/src/main.rs:270-276`

### What's wrong

The closing line is built only from `Summary::renamed` / `Summary::failed`, which count items
that reached `apply::attempt`. An `intent` record rejected by `parse_intent` never becomes an
`UndoItem`, so it is in **neither** bucket. The user is told `0 refused` for a batch in which
items were permanently lost. The batch's own forward count is never compared against the number
of `done` records replayed, so "this batch had 70 renames and I could only address 66" is
information the program has and does not say.

The anomaly lines do go to stderr and the exit code is 1, but the _summary_ — the line a user
actually reads — actively contradicts them, and on a large batch the anomalies scroll away.

### Concrete failure scenario (reproduced)

The O3-1 transcript above: forward run `2 renamed`; undo prints `1 reverted, 0 refused`.
Correct output would be `1 reverted, 0 refused, 1 could not be undone at all`.

Confidence: **CONFIRMED**

---

## O3-3 — MEDIUM — `next_seq` overflows on a crafted journal filename; the run panics

`crates/detoxrs/src/journal.rs:149`

### What's wrong

`next_seq` parses the first `-`-delimited token of **every** filename in the journal directory as
a `u64` and returns `Ok(max + 1)` with no overflow guard. A file whose name begins with
`18446744073709551615-` drives `max` to `u64::MAX`. In a debug build this panics; in a release
build (overflow checks off) it wraps to 0 silently.

`journal.rs`'s own docs anticipate hostile journal _contents_ ("a shared or attacker-influenced
`XDG_STATE_HOME`, or a hand-edited journal, can put anything in this JSON") and harden
`parse_intent` accordingly, but journal _filenames_ are parsed with no validation at all. Nothing
in the crate catches the panic, and `main.rs` documents exit 2 for "failures where nothing was
attempted" — a panic is exit 101 instead.

### Concrete failure scenario (reproduced)

```
$ touch "$XDG_STATE_HOME/detoxrs/journal/18446744073709551615-20260101T000000Z.jsonl"
$ detoxrs -x -r m7
thread 'main' (21144011) panicked at crates/detoxrs/src/journal.rs:149:12:
attempt to add with overflow
$ echo $?
101
```

No renames happened (the failure is before the loop), so this is availability, not data loss.

Confidence: **CONFIRMED**

---

## O3-4 — MEDIUM — `undo --last` reverts the wrong batch once the sequence changes digit width

`crates/detoxrs/src/journal.rs:101` (`format!("{seq:06}")`), `journal.rs:537` (`out.sort()`),
`crates/detoxrs/src/main.rs:328` (`resolve_last`)

### What's wrong

`journal.rs:119-131` argues at length that the sequence counter is "what makes `undo --last`
correct" and that "a counter read from the directory cannot" pick the wrong batch, "whatever the
clock does", with `{seq:06}` justified as "fixed width so a lexical sort is a numeric sort". The
width is fixed only below 1 000 000. `list()` sorts the filenames lexically, so as soon as a
seven-digit sequence appears, `"1000000-…"` sorts **before** `"999999-…"` and `resolve_last`'s
`batches.iter().rev()` hands back the older batch. It then reverts it, exit 0, no warning.

Two routes in: a million real batches, or a single stray/planted filename in the journal
directory whose leading number is ≥ 999999 — `next_seq` adopts any such number as the high-water
mark without question (same missing validation as O3-3).

### Concrete failure scenario (reproduced)

```
$ touch "$XDG_STATE_HOME/detoxrs/journal/999998-20200101T000000Z.jsonl"
$ detoxrs -x m9/"one 1.txt"   # -> Undo with: detoxrs undo 999999-20260803T212754Z
$ detoxrs -x m9/"two 2.txt"   # -> Undo with: detoxrs undo 1000000-20260803T212754Z
$ ls m9
one_1.txt   two_2.txt
$ detoxrs undo --last
/.../m9/one_1.txt  ->  one 1.txt          # <-- the FIRST run, not the last

1 reverted, 0 refused. This undo is itself batch 1000001-20260803T212755Z.
$ ls m9
one 1.txt   two_2.txt                     # the batch the user meant is untouched
```

Exit 0. The user believes they have undone their most recent run; they have instead undone an
older one and left the recent one applied.

Confidence: **CONFIRMED**

---

## O3-5 — LOW — the journal is created world-readable, with no mode override

`crates/detoxrs/src/journal.rs:93` (`fs::create_dir_all`), `journal.rs:103` (`OpenOptions`)

### What's wrong

Neither the directory nor the file sets a mode, so both take the process umask. Under the common
`umask 022` the result is `drwxr-xr-x` / `-rw-r--r--`. The journal holds the absolute path of
every file the user renamed, plus both names — which is a fairly complete map of a private
directory tree, disclosed to every local account. Journals are never pruned, so this accumulates.

`$HOME/.local/state` happens to be `0700` on many systems, which masks the problem there; the
`XDG_STATE_HOME` path has no such protection, and the tool does not require it to be private.

### Concrete failure scenario (reproduced)

```
$ umask; detoxrs -x -r mz >/dev/null; stat -f '%Sp %N' "$XDG_STATE_HOME"/detoxrs/journal{,/*.jsonl}
022
drwxr-xr-x  .../stz/detoxrs/journal
-rw-r--r--  .../stz/detoxrs/journal/000001-20260803T213248Z.jsonl
```

Poisoning is a separate matter and is mostly handled: `parse_intent` validates record contents
and the identity recheck gates every rename, so a hostile _record_ is refused. Hostile
_filenames_ are not validated — see O3-3 and O3-4.

Confidence: **CONFIRMED**

---

## O3-6 — MEDIUM — `--json`, the declared stable contract, omits the batch id and journal path

`crates/detoxrs/src/report.rs:297-316`, `crates/detoxrs/src/main.rs:148-154`

### What's wrong

`report.rs`'s module doc calls `--json` "the only stable contract", and `applied()` explains that
the batch id goes in the human output because "a user who has just renamed 400 files needs the
one string that undoes it, and making them go and look for it is how `undo` ends up unused". The
JSON document is built from `plan` and `outcomes` only — `exec` has `batch` and `where_` in scope
and passes neither. So the machine consumer, the one caller that cannot read the human report, is
exactly the caller with no way to learn what to pass to `undo`. Its only recourse is
`undo --list` plus a guess about which entry is its own, which is racy under concurrency (12
concurrent runs produce 12 journals in the same second — see the PASS row below).

### Concrete failure scenario (reproduced)

```
$ detoxrs -x -r --json mz | python3 -c "import json,sys; print(sorted(json.load(sys.stdin)))"
['applied', 'atomicity', 'items', 'schema', 'summary']
```

No `batch`, no `journal`, no `undo` key anywhere in the document.

Confidence: **CONFIRMED**

---

## O3-7 — LOW — a torn final `intent` degrades the "name the interrupted item" promise to a generic warning

`crates/detoxrs/src/journal.rs:345-352`

### What's wrong

`replay` deliberately ignores an unparseable **last** line, which is correct for the `kill -9`
case (a small `write(2)` to a regular file does not tear, so a cut `intent` implies the rename
had not happened). Under a power-loss tear that spans a sector boundary — the case the module doc
explicitly places outside the threat model — the record can be cut _after_ the rename landed.
`replay` then reports neither an anomaly nor an `interrupted` item; the user gets only the
generic "no completion record" warning and is never told which name to check.

### Concrete failure scenario (reproduced by hand-truncating the journal)

```
$ # 3 files renamed, journal truncated mid-way through the 3rd intent record
$ detoxrs undo --last
detoxrs: warning: batch 000001-… has no completion record, so it either crashed or is still running…
/.../ma/f_2.txt  ->  f 2.txt
/.../ma/f_1.txt  ->  f 1.txt

2 reverted, 0 refused. This undo is itself batch 000002-…
$ ls ma
f 1.txt   f 2.txt   f_3.txt      # f_3 unrecoverable, and never named to the user
```

Listed as LOW because the reachable-by-`kill -9` variant is provably safe and the power-loss
variant is a declared non-goal; the gap is that the _warning wording_ promises more than replay
can deliver in that case.

Confidence: **CONFIRMED** (as a behaviour; the triggering crash is out of declared scope)

---

## O3-8 — LOW — the truncation stage is unreachable on the filesystems the project targets

`crates/detoxrs-core/src/policy.rs:101` (`M1_MAX_LEN = 255`)

`transform` never lengthens a name, and the default byte limit is 255 — exactly `NAME_MAX` on
ext4/APFS. A name that already exists therefore already fits, so stage 11 can only fire for names
whose _byte_ length exceeds 255 while the filesystem still accepted them, i.e. multi-byte names on
a filesystem that counts characters rather than bytes. Confirmed empirically: a 406-byte
(200 × `é`) name on APFS is created successfully and _is_ truncated; a 300-byte ASCII name cannot
be created at all (`ENAMETOOLONG`). Not a correctness bug — worth stating because the truncation
tests are all unit-level and the end-to-end path has one narrow trigger.

Confidence: **CONFIRMED**

---

## Verdict table

| Area                                                       | Verdict                    | Evidence                                                                                                                                                                                                                                                                                                                                                                                             |
| ---------------------------------------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Round trip, general (apply → undo → byte-identical)        | **PASS**                   | 400 randomized iterations (nested dirs, controls, bidi, ZWSP, newline, quotes, metacharacters, NFC/NFD, CJK, astral emoji, NBSP/ideographic space) with `\` excluded: 0 manifest diffs. 78-entry hand-built tree: identical except O3-1. Inodes preserved throughout.                                                                                                                                |
| Round trip, names containing `\`                           | **FAIL**                   | O3-1, reproduced minimally and at scale, plus a mutation that fixes it with the suite green.                                                                                                                                                                                                                                                                                                         |
| Undo ordering with nested directories                      | **PASS**                   | 3-level tree: forward journal is deepest-first, `replay` reverses it, parent restored before its children, `find` output identical to pre-run. Also exercised in 400 fuzz iterations with random nesting.                                                                                                                                                                                            |
| Undo clobbering a file created since the rename            | **PASS**                   | Recreated the original name with different content; undo refused per item (`a b.txt appeared since the preview; not renamed`), both files byte-identical afterwards, exit 1.                                                                                                                                                                                                                         |
| Undo run twice                                             | **PASS**                   | Second `undo <same-id>`: `0 reverted, 3 refused`, every item refused with `ENOENT` at the identity recheck, tree unchanged.                                                                                                                                                                                                                                                                          |
| Undo of a partially applied batch (`kill -9`)              | **PASS**                   | 4000-file run killed at 0.25 s: 20 renames done, journal ended on an `intent` with no outcome, the intended file was verifiably _not_ renamed, `undo --last` restored all 20 and the tree returned to 4000 dirty names.                                                                                                                                                                              |
| Durability of `intent` before rename                       | **PASS**                   | `intent` is `write_all` + `sync_data` before `rename_noreplace` (`journal.rs:225-229`, `apply.rs:197-204`); ordering is asserted by a shared event log in `apply::tests::the_intent_is_recorded_before_the_rename_not_after`, and the `kill -9` run showed no rename without a preceding record. Power-loss durability is explicitly out of scope (no directory fsync, no `F_FULLFSYNC`).            |
| Journal escaping / delimiter injection                     | **PASS**                   | Records are built with `serde_json`; names containing `\n`, `\r`, `\t`, `"`, `'`, `<`, `>`, `$`, `;`, `&`, `                                                                                                                                                                                                                                                                                         | `, `*`, `?`, control bytes and astral emoji all round-tripped byte-for-byte through the JSONL format in the fuzz and the hand-built tree. |
| Concurrent `-x` runs on one journal directory              | **PASS**                   | 12 simultaneous runs: 12 distinct journals (`create_new` + seq retry), 480 renames reported and 480 observed on disk, zero errors, zero interleaved records.                                                                                                                                                                                                                                         |
| Batch-id collision / ambiguity                             | **PASS**                   | `create_new` plus the 64-attempt seq bump makes same-second collisions impossible; verified by the 12-way concurrent run.                                                                                                                                                                                                                                                                            |
| `--last` picks the right batch                             | **FAIL**                   | O3-4: wrong batch, exit 0, no warning, once the sequence crosses six digits. The all-failed / all-refused / no-op skip logic in `resolve_last` is otherwise correct — verified against a stray empty journal that sorted newest and was correctly skipped.                                                                                                                                           |
| No rename without a journal                                | **PASS**                   | `HOME` and `XDG_STATE_HOME` both unset: exit 2, `cannot open an undo journal (…); nothing was renamed`, file untouched. Unwritable `XDG_STATE_HOME`: exit 2, same, file untouched. Preview still works.                                                                                                                                                                                              |
| Journal contents treated as untrusted                      | **PASS**                   | `parse_intent` refuses non-basename `from`/`to` and relative `dir`; the existing `undo_refuses_a_traversal_record_instead_of_escaping_the_pinned_directory` test asserts on disk, not on `replay`'s own report. (The same check is the cause of O3-1 — the mechanism is right, the character set is wrong.)                                                                                          |
| Journal _filenames_ treated as untrusted                   | **FAIL**                   | O3-3 (overflow panic) and O3-4 (width poisoning). No validation at all in `next_seq`.                                                                                                                                                                                                                                                                                                                |
| Journal location / permissions                             | **FAIL (minor)**           | O3-5: default umask, world-readable, holds every renamed path.                                                                                                                                                                                                                                                                                                                                       |
| Undo accounting / reporting                                | **FAIL**                   | O3-2: dropped items counted as neither reverted nor refused.                                                                                                                                                                                                                                                                                                                                         |
| `--json` contract completeness                             | **FAIL**                   | O3-6: no batch id, so the machine path cannot reach `undo`.                                                                                                                                                                                                                                                                                                                                          |
| Non-UTF-8 _directory_ path through the journal             | **NOT ESTABLISHED**        | APFS rejects invalid UTF-8 filenames (`EILSEQ`), so the `dir_bytes` path could not be exercised end-to-end on this host. `put_os`/`get_os` round-trip in isolation is unit-tested, and `Path::is_absolute` on a byte-reconstructed `OsString` is a leading-`/` byte test that holds by inspection — but no evidence was gathered for the full `intent` → `replay` → `open` chain. Needs a Linux run. |
| Non-UTF-8 _file_ names                                     | **PASS (by construction)** | `desired_for` returns `Skip(NotUtf8)` before any destination is computed, so such a name never reaches a rename and never reaches the journal. Could not be exercised on APFS.                                                                                                                                                                                                                       |
| `report::escape` injectivity                               | **PASS**                   | Every `<` is escaped as `<3c>`, so every `<` in the output opens a genuine token; the three token shapes (`<3c>` for a literal `<`, `<hh>` for an invalid byte, `<u+xxxx>` for a non-ASCII control) have disjoint preimages because 0x3c and 0x0a are valid UTF-8 and so never reach the invalid-byte arm. Traced line by line; `distinct_inputs_render_distinctly` covers the C10 case.             |
| Directory pin (`Dir` fd) reuse                             | **PASS**                   | `attempt` opens once and threads one handle through `ident_at`/`ident_at`/`rename_noreplace`; `CountingOps` asserts `opens == 1` deterministically, and `fsops::tests::the_rename_follows_the_pinned_directory_not_the_path` asserts on disk after a real directory swap.                                                                                                                            |
| Plan collision engine (no clobber, no chain, order safety) | **PASS**                   | 8 proptests green over 8 consecutive runs in the isolated snapshot, plus 400 end-to-end fuzz iterations with zero unexpected failures. The `no_pre_existing_clobber` failure observed early in this review came from another reviewer's live mutation in the shared checkout and does **not** reproduce at `HEAD`.                                                                                   |
| Test-suite trustworthiness in the shared checkout          | **N/A — process warning**  | `cargo test` in `/Users/kerry.hatcher/projects/detoxrs` returned five mutually contradictory verdicts in six runs while other reviewers held mutations in `crates/`. Any finding derived from a suite run in the shared tree during this review should be re-verified against a clean snapshot.                                                                                                      |

## Cleanliness

Two mutations were performed, both confined to the scratchpad copy
(`…/scratchpad/repo/crates/detoxrs/src/journal.rs`), and both restored from `HEAD` afterwards
(`grep` confirms the `\\` guard is back at line 433 and the copy rebuilds). No file under
`crates/` or `docs/` in the real repository was modified by this reviewer; the only file written
is this report.
