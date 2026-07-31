# Owner decisions

Decisions made by the project owner that the research could not settle. These
override the design proposal (`docs/research/00-proposal-rust-detox-successor.md`)
where they conflict. Each entry records the date, the decision, and what it
changes, so a later reader can tell an owner's call from an agent's inference.

---

## 2026-07-31 — License: dual MIT OR Apache-2.0

Relicensed from BSD-3-Clause. Done while the project had a single copyright
holder and no external contributors. Apache-2.0 supplies the express patent
grant BSD-3-Clause lacks; MIT preserves GPLv2 compatibility.

Also directed: **credit `detox` for the concept.** `README.md` gains an
Acknowledgments section crediting Doug Harple for the idea, the problem framing,
and twenty years of user reports. `CONTRIBUTING.md` makes the corresponding rule
enforceable — study upstream behavior, never copy upstream expression — which is
what keeps the dual license clean, since upstream is BSD-3-Clause.

**Applied.** Commit `5bfb271`.

---

## 2026-07-31 — Test hardware: Linux and macOS only

Available: **Linux (any distro)** and **macOS**. Not available: Windows, and no
NTFS or exFAT volume.

Consequences for the proposal's gating spikes (§11):

| Spike                                               | Status                                                                                                                  |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| 2. `renameat2(RENAME_NOREPLACE)` across filesystems | **Closeable.** Run the ext4/xfs/btrfs/tmpfs/overlayfs matrix on Linux and record the errno per filesystem.              |
| 6. Legacy-encoding repair on real mis-encoded names | **Moot** — see the encoding decision below. The subsystem is being dropped.                                             |
| 3. Windows 11 reserved names                        | **Stays open.** The conservative pre-Windows-11 rule remains a documented assumption, not a fact.                       |
| 4. NTFS / exFAT length limits                       | **Stays open.** Both the byte limit and the UTF-16 limit continue to be enforced simultaneously as the safe conjecture. |

Windows therefore stays a **best-effort tier**: it must compile and unit-test in
CI, but no filesystem behavior is asserted. Do not promote Windows to tier 1, and
do not write documentation that implies verified Windows behavior.

---

## 2026-07-31 — Ambition: a real, publicly packaged tool

This is intended as the successor detox users migrate to, not an internal
utility. So:

- The governance and release machinery already built **stays** — provenance,
  checksums, SBOM, supported-version policy, packaging roadmap.
- `SECURITY.md`'s response-time table and fallback contact, and
  `CODE_OF_CONDUCT.md`'s enforcement contact, are **real commitments to real
  third parties** and must be filled in by a human before the first public
  release. They remain marked placeholders until then. Do not invent them.
- The name/trademark spike (§11 spike 1) matters and is partly closed below.
- Packaging order in §9.4 stands, with the caveat that upstream's footprint is
  frozen (archived 2026-07-12) and can only shrink.

### Name availability, checked 2026-07-31 (crates.io API)

| Name      | Status                              | Note                                                             |
| --------- | ----------------------------------- | ---------------------------------------------------------------- |
| `detoxrs` | **available**                       | The chosen name. Claim it early.                                 |
| `detoxr`  | available                           | First fallback.                                                  |
| `detox`   | taken (v0.1.2, 2019-05-01, 5284 dl) | Unusable regardless; §9.2 already forbids taking this name.      |
| `dtx`     | taken (v0.1.1, 2024-01-06, 3007 dl) | Does not block shipping a `dtx` **binary** from our own package. |

**New concern for the `dtx` short alias:** an unrelated `dtx` crate exists. If it
installs a binary of the same name, `cargo install` users could end up with a
collision. Verify what binary that crate ships before committing to the alias.
Trademark clearance is still unrun and is still not something a web search
satisfies.

---

## 2026-07-31 — Drop legacy encoding repair from v1.0

Non-UTF-8 filenames are **skipped and reported, never repaired**.

Removes: the CP1252/Latin-1 decode tables, `--legacy-encoding`, the `Repaired`
decode outcome, and the entire highest-risk untested subsystem in the design —
spike 6 was never validated in any research pass because APFS refuses to create
such a filename at the syscall level.

Pipeline stage 1 simplifies to: **valid UTF-8, or skip with a report.** The
`Decoded` type loses its `Repaired` variant, keeping `Utf8` and `Opaque`.

What is explicitly _retained_: `OsStr`-at-the-boundary discipline (§6.1). Refusing
to repair is not permission to panic, lossily convert, or print raw invalid bytes
to a terminal. An undecodable name must still be handled safely and displayed
with escapes.

This can return post-1.0 as an opt-in `--repair-encoding` flag once there is
Linux hardware to measure its false-positive rate against a real corpus.

---

## 2026-07-31 — Collision default: auto-number

Owner accepted either auto-numbering or fail-the-batch, and delegated the choice
to the research. **Decision: auto-number** (`IMG_0042.JPG` -> `IMG_0042-2.JPG`),
with `--on-collision skip|fail` remaining available, and `fail` recommended in the
`paranoid` profile.

Evidence, and it points at "don't over-engineer this":

- Collision demand in the upstream tracker is **tiny — two items** (`#122`,
  `#130`), and doc 02's revised theme 6 states plainly that demand exists and is
  small, the maintainer's technical objection is credible, and a safe design is
  _not_ established by that evidence.
- `user_feedback_online.md` found the rejected overwrite flag has **zero
  independent echo** anywhere online: no forum, blog, or Q&A post asks for it. So
  there is no evidence that real users hit collisions often enough for the default
  to be a major ergonomic question.
- The best-evidenced actual use case is **bulk cleanup of imported trees**, often
  inside a multi-tool pipeline (`convmv` -> `detox -r` -> `rename` -> `mogrify`),
  documented from multiple independent sources. A default that aborts the entire
  batch on one conflict is actively hostile to that workflow — one odd pair would
  block hundreds of good renames in a scripted pass.
- Preview-by-default already removes the main argument for `fail`: conflicts are
  shown, with their resolution, before anything is written. The user sees
  `IMG_0042-2.JPG` coming and can re-run with `--on-collision fail` if they
  disagree.

Neither option risks data loss, so this was decided on workflow fit rather than
safety. The standing counterargument is unchanged and worth keeping in the docs:
auto-numbering implies a relationship between unrelated files, and `Report-2.pdf`
reads like a second version of `Report.pdf`.
