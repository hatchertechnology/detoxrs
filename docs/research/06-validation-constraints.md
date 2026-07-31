# Adversarial Validation of `03-prior-art-and-constraints.md`

Environment: macOS Darwin 25.5.0 (arm64), APFS. Tests re-run independently in
`/private/tmp/.../scratchpad/validate-03/`, on three volumes: the real boot volume
(case-insensitive APFS), a freshly created case-insensitive APFS disk image (`CITest`,
via `hdiutil create -fs "APFS"`), and a freshly created case-sensitive APFS disk image
(`CSTest`, via `hdiutil create -fs "Case-sensitive APFS"`). Neither disk image touched
any real user volume. All test programs are inlined below with raw output.

## Claim / Verdict / Evidence

| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | APFS component limit is 255 UTF-16 code units, not bytes/codepoints | **CONFIRMED, and more rigorously than the original test** | See "Test 1" below. Discriminated against codepoint-limit and byte-limit hypotheses using CJK (3B/1cp/1u16) and precomposed é (2B/1cp/1u16) in addition to emoji, on *both* case-sensitive and case-insensitive APFS. |
| 2 | APFS is normalization-preserving, not normalization-sensitive | **CONFIRMED** | See "Test 2". Verified on both volume types with `os.listdir`, `open()`, `O_CREAT\|O_EXCL`, and raw `ls \| xxd`. |
| 3 | Case-only rename needs a two-step dance through a temp name | **REFUTED** | See "Test 3". `os.rename("CaseTest.txt","casetest.txt")` and a raw C `rename(2)` call both succeed **directly**, no EEXIST, on the real boot volume, a fresh case-insensitive APFS image, and (control) a case-sensitive image. The kernel resolves same-inode case variants correctly in one syscall; no temp-name step needed. |
| 4a | `std::fs::rename` has no no-clobber option; `rust-lang/libs-team#131` exists and proposes `rename_noreplace` | **CONFIRMED** | Fetched `https://api.github.com/repos/rust-lang/libs-team/issues/131` directly: title "Add `std::fs::rename_noreplace`", state `open`, created 2022-10-30, body proposes exactly `renameat2`+`RENAME_NOREPLACE` on Linux / `renameatx_np`+`RENAME_EXCL` on macOS / omit `MOVEFILE_REPLACE_EXISTING` on Windows. Matches the doc verbatim. |
| 4b | `renamex_np`/`RENAME_EXCL` and `RENAME_SWAP` exist on macOS with claimed semantics | **CONFIRMED** | Local `man renamex_np` on this machine (Darwin 25.5) shows exactly: `RENAME_EXCL` → EEXIST if destination exists, gated on `getattrlist` `VOL_CAP_INT_RENAME_EXCL`; `RENAME_SWAP` → atomic swap, gated on `VOL_CAP_INT_RENAME_SWAP`; the two flags are mutually exclusive (EINVAL if OR'd together). |
| 4c | `renameat2`/`RENAME_NOREPLACE` (Linux) exists with claimed semantics | **UNVERIFIABLE locally** (no Linux box here) but **plausible** — Linux man pages are third-party-cited in the original doc, not independently re-fetched by me either. Treat as medium confidence, not re-verified. |
| 4d | `rustix` exposes `RENAME_NOREPLACE`/`RENAME_EXCHANGE` (Linux) | **CONFIRMED** | `docs.rs/rustix/latest/rustix/fs/struct.RenameFlags.html` lists `NOREPLACE`, `EXCHANGE`, `WHITEOUT`. Current version **1.1.4** (updated 2026-02-22 per crates.io). |
| 4e | `rustix`/`nix` expose the macOS `renamex_np` `RENAME_EXCL`/`RENAME_SWAP` flags | **REFUTED (gap in the doc)** | `rustix`'s public `fs` module function list is only `rename`, `renameat`, `renameat_with` — no macOS-specific `renamex_np` wrapper found in current docs.rs output. `nix`'s `fcntl` module similarly only surfaces the Linux `renameat2` flag set (`NOREPLACE`/`EXCHANGE`/`WHITEOUT`), not a macOS `RENAME_EXCL` equivalent. **The doc implies both crates cover both platforms' flag semantics; today neither does for macOS — a Rust successor will need raw `libc`/manual FFI to `renamex_np` on macOS**, not just "small platform-conditional wrapper" as if the crates already had it. |
| 5a | `sanitize-filename` truncates to "255 bytes," check grapheme-safety before trusting | **CONFIRMED, and worse than implied** | Fetched actual source (`kardeiz/sanitize-filename` `src/lib.rs`). Truncation is `if name.len() > 255 { ... while !name.is_char_boundary(end) { end -= 1 } }` — i.e. **byte length**, snapped only to a **codepoint** boundary, not a **grapheme-cluster** boundary. It can and will split combining-character sequences (e.g. base+diacritic, ZWJ emoji sequences) even though it won't produce invalid UTF-8. Latest version **0.6.0** (2025-10-01), MIT-style, not abandoned. |
| 5b | `sanitise-file-name` "fewer allocations" claim, `unicode-normalization`, `unicode-segmentation`, `unicode-width`, `deunicode`, `slug` versions | **CONFIRMED versions, allocation claim UNVERIFIED** | `sanitise-file-name` 1.0.0 (2022-01-05, stale ~4.5 yrs but crate is tiny/complete, not necessarily "abandoned"); `unicode-normalization` 0.1.25 (2025-10-30); `unicode-segmentation` 1.13.3 (2026-06-01); `unicode-width` 0.2.2 (2025-10-06); `deunicode` 1.6.2 (2025-04-27). All actively maintained. Did not benchmark allocation counts — doc's "worth benchmarking" framing was already appropriately hedged. |
| 5c | `unicode-security`, `unicode_skeleton`, `confusables` are usable UTS #39 building blocks | **REFINED — flag staleness** | `unicode-security` 0.1.2 (2024-09-12) — reasonably fresh. **`unicode_skeleton` 0.1.1, last updated 2017-10-08 — nearly 9 years stale, de facto unmaintained.** `confusables` 0.1.0, last updated 2023-08-23 — also low-version/low-activity. The doc lists these as if interchangeable "core" building blocks; `unicode_skeleton` in particular should not be relied on without an abandonment check at implementation time. |
| 5d | `chardetng`, `trash`, `jwalk`, `figment` maintained | **REFINED** | `chardetng` 1.0.0 (2026-03-30) — active. `trash` 5.2.6 (2026-05-03) — active. **`jwalk` 0.8.1, last updated 2022-12-15 — over 3.5 years stale as of 2026-07-31.** Not archived on GitHub, but no recent releases; treat as "usable but not actively maintained," contradicts doc's unqualified recommendation. `figment` 0.10.19 (2024-05-17) — over 2 years since last release, moderate staleness, still the de facto pick for its niche. |
| 6a | `rnr` cannot represent non-UTF-8 filenames (doc cites its README) | **CONFIRMED, and the doc undersells the severity** | Fetched actual source `src/renamer.rs` line 70: `let file_name = path.file_name().unwrap().to_str().unwrap();` — this **panics** (`unwrap()` on `to_str()`, which returns `None` for non-UTF-8 `OsStr`) rather than gracefully erroring. `rnr` doesn't just "not represent" such a filename, it crashes when it encounters one. Confirmed via source, not just README skim. |
| 6b | `f2` dry-run-by-default, undo log via history | **CONFIRMED** | Fetched `raw.githubusercontent.com/ayoisaiah/f2/master/README.md` directly: "Dry Run by Default... defaults to a dry run", "Undo Functionality: Any renaming operation can be easily undone", with a doc link to `f2.freshman.tech/guide/undoing-mistakes.html`. Latest tag `v2.2.2`, published 2025-11-10 (doc's "did not confirm exact date" caveat is now resolved). GitHub `pushed_at` 2026-06-22, 2427 stars (matches doc's ~2.4k). |
| 7a | Windows reserved names (CON/PRN/AUX/NUL/COM1-9/LPT1-9 + superscript variants), regardless of extension, "in every directory" | **CONFIRMED against current MS Learn doc** | Fetched `learn.microsoft.com/.../naming-a-file` (page dated `ms.date: 2024-08-28`, `updated_at: 2025-04-11`). Verbatim: superscript digits ¹²³ "reserved in every directory"; "avoid these names followed immediately by an extension; for example, NUL.txt and NUL.tar.gz are both equivalent to NUL" — matches the doc's Win32-namespace-rules framing exactly. |
| 7b | "Windows 11 relaxed this for some contexts but the bare name is still reserved," implying `NUL.txt` is *still* invalid | **REFUTED — doc has the direction of the Windows 11 change backwards** | Per CPython core-dev discussion `python/cpython#95486` (eryksun, ChrisDenton — both recognized Windows API experts): **on Windows 11, path normalization no longer special-cases a DOS device name if it has an extension** — `con.txt`, `aux.c`, `nul.txt` (as a non-leaf-qualified path) are **no longer reserved**; the bare name (`CON`, `AUX`, `NUL`) as the leaf component is still reserved, and `NUL` is extra-special-cased even further. So the doc's implicit claim that `NUL.txt` "is still invalid" on current Windows is **wrong for Windows 11**; what actually changed is the opposite of what the doc describes. Caveat: two secondary sources (Meziantou's blog, a Microsoft Q&A thread) assert the old universal rule with no Windows-11 carve-out and conflict with the cpython thread — this is a genuinely contested point online, not a clean-cut fact; treat with medium confidence and re-verify empirically on real Windows 11 before hard-coding either behavior. |
| 7c | Trailing dots/spaces silently stripped by Windows path normalization | **CONFIRMED, wording refined** | MS Learn's own text is softer than "silently stripped": "Do not end a file or directory name with a space or a period. Although the underlying file system **may support** such names, the Windows shell and user interface **does not**." This is consistent with the doc's practical implication but the mechanism is UI/shell-layer inconsistency, not a hard filesystem-level strip in all cases — worth the nuance in the design doc. |
| 7d | MAX_PATH 260, long-path opt-in via registry/manifest or `\\?\` | **CONFIRMED** | Same MS Learn page: "In editions of Windows before Windows 10 version 1607, the maximum length for a path is MAX_PATH... In later versions... changing a registry key or using the Group Policy tool is required to remove the limit," plus the `\\?\` prefix section. Matches doc exactly. |
| 7e | Windows illegal characters `< > : " / \ \| ? *` + control chars 0-31 | **CONFIRMED** | MS Learn page lists exactly this reserved-character set plus "Integer value zero" (NUL byte) and control chars 1-31 (with an alt-data-stream exception the doc doesn't mention but doesn't need to for a rename tool). |

## Test 1 — APFS length limit, full transcript

Script (`len_test.py`) binary-searches, per character class, the max repeat count that
still fits in one path component, using `os.open(O_CREAT|O_WRONLY)` + `os.unlink`.
Four char classes chosen specifically to separate the byte/codepoint/UTF-16 hypotheses:
ASCII `a` (1B/1cp/1u16), precomposed `é` U+00E9 (2B/1cp/1u16), CJK `漢` U+6F22 (3B/1cp/1u16),
astral emoji `\U0001F600` (4B/1cp/2u16 via surrogate pair).

```
=== Case-insensitive APFS volume (fresh hdiutil image) ===
ASCII 'a' (1B,1cp,1u16): max N = 255  -> total bytes=255 codepoints=255 utf16_units=255
precomposed e-acute U+00E9 (2B,1cp,1u16): max N = 255  -> total bytes=510 codepoints=255 utf16_units=255
CJK Han U+6F22 (3B,1cp,1u16): max N = 255  -> total bytes=765 codepoints=255 utf16_units=255
astral emoji U+1F600 (4B,1cp,2u16): max N = 127  -> total bytes=508 codepoints=127 utf16_units=254

bytes-at-limit per char:       [255, 510, 765, 508]   <- NOT constant, refutes "255 bytes"
codepoints-at-limit per char:  [255, 255, 255, 127]   <- NOT constant, refutes "255 codepoints"
utf16-units-at-limit per char: [255, 255, 255, 254]   <- constant (254/255), CONFIRMS "255 UTF-16 units"

=== Case-sensitive APFS volume (fresh hdiutil image) ===
[identical results — the limit is filesystem-format-level, not affected by case-sensitivity mode]
```

The CJK and é rows are the discriminating cases the original doc's transcript lacked: they
prove the limit tracks UTF-16 code units, not codepoints (emoji is capped at 127, i.e. half of
255, specifically because each emoji costs 2 UTF-16 units) and not bytes (three very different
byte totals — 255, 510, 765 — all hit the *same* wall). This is now a clean four-way discriminated
result, not just a two-point emoji/ASCII data point, and it holds on both volume formats.

## Test 2 — NFC/NFD, full transcript

Script (`nfc_nfd_test.py`) creates a file with NFC-spelled `café.txt`, then checks
`os.path.exists` on both spellings, `os.listdir` raw bytes, opening by the NFD spelling,
and whether `O_CREAT|O_EXCL` with the NFD spelling collides.

```
NFC bytes: b'caf\xc3\xa9'
NFD bytes: b'cafe\xcc\x81'

--- After creating with NFC name ---
os.path.exists(NFC path): True
os.path.exists(NFD path): True
os.listdir raw entries: [('café.txt', b'caf\xc3\xa9.txt')]     <- stored exactly as given (NFC bytes)
Opened via NFD path successfully, content: hello
O_CREAT|O_EXCL with NFD name -> EEXIST (confirms NFC and NFD resolve to the SAME directory entry)
Final directory listing count: 1
```//
Identical on the case-sensitive image. Raw `ls | xxd` confirms the on-disk bytes are the NFC
form given at creation time (`6361 66c3 a92e 7478 74` = `caf` + `c3 a9` (é, NFC) + `.txt`), not
silently converted to NFD (which would be the classic HFS+ myth some blogs still repeat).

## Test 3 — Case-only rename, full transcript

`case_rename_test.py` (Python `os.rename`) and `rename_syscall.c` (raw `rename(2)` via
a compiled C binary, to rule out any Python-level cleverness):

```
=== Case-INSENSITIVE APFS (fresh hdiutil image) ===
os.stat('CaseTest.txt').st_ino = 43
os.stat('casetest.txt').st_ino = 43 (same file, case-insensitive lookup)
Same inode via both spellings: True
--- Attempting direct rename(2): CaseTest.txt -> casetest.txt ---
os.rename succeeded directly (no EEXIST/ENOTEMPTY).
Listing after rename: ['casetest.txt']
content: marker-content-12345

=== Real boot volume (case-insensitive APFS) ===
[identical: os.rename succeeded directly]

$ clang -O2 -o rename_syscall rename_syscall.c
$ touch UPPER.txt && ./rename_syscall UPPER.txt upper.txt
rename(2) SUCCEEDED directly (raw syscall, not libc wrapper magic)
$ ls
upper.txt
```

This **flatly refutes** claim #3 in the original doc. `rename(2)` on a case-insensitive-but-
case-preserving filesystem is specifically designed to detect that source and destination
resolve to the same inode and just re-case the directory entry in place — no EEXIST, no
ENOTEMPTY, no two-step temp-name dance required, confirmed at both the Python `os.rename`
level and the raw C `rename(2)` syscall level. (For the record, this is also true on Linux
ext4 with `rename2`-family semantics, and is why tools like `git mv A.txt a.txt` work directly
on case-insensitive filesystems — but that wasn't re-tested here since no Linux box was
available; flagging as corroborating background knowledge, not independently re-verified.)

## Corrections Required (edit `03-prior-art-and-constraints.md`)

1. **Delete constraint #2's two-step-temp-name requirement entirely.** `rename(2)`/
   `os.rename` on APFS (case-sensitive or -insensitive) handles a pure case change in one
   atomic call. The design must NOT special-case "differs only by case" as needing a temp
   intermediate — that's solving a problem that doesn't exist on APFS or (by general Unix
   `rename(2)` semantics) on any case-insensitive-but-preserving filesystem. If a
   cross-platform edge case exists where this genuinely fails (some network filesystems?),
   it needs its own citation — don't keep the current unsourced claim.
2. **Constraint #3 (Windows reserved names) needs a Windows-11-specific carve-out**, sourced
   to the `python/cpython#95486` discussion (or better, a live Windows 11 test): a reserved
   name *with an extension* and not as the sole leaf component is generally no longer blocked
   on Windows 11; only the bare device name as the leaf component remains reserved (`NUL`
   even more so). The doc's current text asserts the opposite ("`NUL.txt` is still invalid").
   Given conflicting secondary sources, mark this itself as re-verify-on-real-hardware, not
   settled — but stop stating the old universal rule as current fact.
3. **Crate table: demote `unicode_skeleton` and flag `jwalk`.** `unicode_skeleton` (last
   released 2017) should not be recommended without an explicit "verify not abandoned before
   use" flag equal to what's given to `detox` itself. `jwalk` (last released Dec 2022) should
   get the same flag; `ignore`'s own parallel-walk support (`WalkParallel`) may already cover
   the need without a second, staler crate.
4. **`sanitize-filename` note should say "byte-and-codepoint-boundary-safe, NOT grapheme-
   safe"** — confirmed from source, not just "check before trusting." A base+combining-mark
   pair or ZWJ emoji sequence can be split by its current truncation logic.
5. **Crate note for `rustix`/`nix` should say macOS `renamex_np`/`RENAME_EXCL`/`RENAME_SWAP`
   are NOT currently exposed by either crate's public API** (only the Linux `renameat2` flag
   set is). The implementation plan needs a raw FFI/`libc` shim for macOS, not just "a small
   platform-conditional wrapper" as if the crates already handled both OSes symmetrically.

## Load-Bearing Uncertainties (the Rust design must not assume these)

- **Windows 11 reserved-name behavior is contested even among people who study Windows
  internals for a living** (two respected CPython core devs vs. two blog/Q&A sources with
  the opposite claim). Don't hard-code either behavior into the sanitizer without a live
  Windows 11 empirical test; the safe engineering choice is to keep the conservative
  (old-Windows) reserved-name check as the *default*, since files may travel to older
  systems or SMB shares regardless of what the local Windows 11 build permits — but log this
  as an assumption, not a verified fact.
- **exFAT and NTFS component-length behavior was not independently tested** (no such volume
  available here either) — still an open item exactly as the original doc flagged.
- **Linux `renameat2`/`RENAME_NOREPLACE` filesystem-version support matrix (ext4 3.15+,
  btrfs/tmpfs/cifs 3.17+, xfs 4.0+, most others by 4.9) was not independently re-verified** —
  no Linux box in this environment either. Treat as inherited, not re-confirmed, risk.
- **Crate staleness is a moving target.** `unicode_skeleton`, `confusables`, `jwalk`, and
  `figment` all show meaningful gaps since last release as of this snapshot (2026-07-31).
  Re-check immediately before any dependency is pinned in `Cargo.toml` — don't trust this
  table's version numbers past the design phase.
- **No macOS crate currently wraps `renamex_np`/`RENAME_EXCL`/`RENAME_SWAP` cleanly** (per the
  docs.rs surface checked here). Budget real implementation time for a small `libc`-based FFI
  shim rather than assuming `rustix` or `nix` already has it "for free."

## Reproduction

All scripts live in
`/private/tmp/claude-502/-Users-kerry-hatcher-projects/144be503-4a3a-4fb6-8eda-b4b656539f0b/scratchpad/validate-03/`:
`len_test.py`, `nfc_nfd_test.py`, `case_rename_test.py`, `rename_syscall.c`. Case-sensitive/
case-insensitive test volumes were ephemeral `hdiutil` disk images (`cstest.dmg`/`citest.dmg`,
64 MB, since detached and deleted) — reproduce with:
```
hdiutil create -size 64m -fs "Case-sensitive APFS" -volname CSTest cstest.dmg
hdiutil create -size 64m -fs "APFS" -volname CITest citest.dmg
hdiutil attach cstest.dmg && hdiutil attach citest.dmg
```
