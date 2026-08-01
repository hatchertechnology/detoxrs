# Rust project setup notes

Foundation-only pass: workspace scaffolding, no application logic. Written against
`wiki/rust/ideal-project-setup` and `docs/research/00-proposal-rust-detox-successor.md`
(§5.4, §7.1, §7.2, §8).

## Decisions

- **Edition: 2024.** Stable as of the installed toolchain (1.96.1); the guide allows
  2021 or 2024-once-stable, and 2024 is stable, so there is no reason to start on 2021.
- **MSRV: 1.93.0.** Proposal policy is "stable at least 6 months old." Today is
  2026-07-31; 1.93.0 released 2026-01-19 (>6 months); 1.94.0 released 2026-03-02
  (~5 months, too new). Declared in `[workspace.package] rust-version`, mirrored in
  `clippy.toml`'s `msrv` key, and verified cheaply: `just msrv` builds with
  `cargo +1.93.0 --locked` against the actual 1.93.0 toolchain (installed locally via
  rustup) — passes.
- **Toolchain file pins 1.96.1** (the latest verified-working stable, matching the
  environment), separate from the MSRV. `rust-toolchain.toml` documents this split.
- **Unsafe-code policy: `forbid` in both crates (asymmetry removed 2026-07-31;
  the original justification for it is withdrawn).**
  The core crate is pure by design and has no future need for `unsafe`. The binary
  crate was set to `deny` rather than `forbid` on the premise that it would
  eventually need a hand-written macOS `libc` FFI shim for `renamex_np` (proposal
  §5.4, §7.1 `fsops/macos.rs`) "because neither `rustix` nor `nix` expose it."
  **That premise is false for `rustix`.** `rustix` does expose macOS
  `renameatx_np` through the safe wrapper `rustix::fs::renameat_with` with
  `RenameFlags::NOREPLACE`/`EXCHANGE`, gated on `#[cfg(apple)]` — established by
  reading `rustix` 1.1.4's source (`src/fs/at.rs`,
  `src/backend/libc/fs/syscalls.rs`) and by compiling and running a
  `#![forbid(unsafe_code)]` program against it on APFS. docs.rs hides these items
  because its default build target is Linux, which is how the original reading went
  wrong. `nix` genuinely does not expose it.
  Consequences:
  - The stated reason for choosing `deny` over `forbid` in `crates/detoxrs` **does
    not hold**. No `renamex_np` FFI shim is needed; `rustix` covers it from safe
    code, so both crates could carry `#![forbid(unsafe_code)]` today.
  - The only remaining shim candidate was the `getattrlist` /
    `VOL_CAP_INT_RENAME_EXCL` capability probe, which `rustix` does not wrap. The
    propagation pass dropped that probe too: an unsupported flag is detected from
    the error `renameat_with` returns, which is already the design's Linux
    demotion path, so the probe bought a dependency and an `unsafe` block to learn
    at open time what the rename call reports anyway (proposal §5.4).
  - **Applied 2026-07-31 (propagation pass):** `crates/detoxrs/src/main.rs` now
    declares `#![forbid(unsafe_code)]` with the withdrawn rationale replaced;
    `CONTRIBUTING.md`, `SECURITY.md`, and the proposal were corrected in the same
    sweep.
- **Lints: `[workspace.lints.clippy] all/pedantic/nursery = "warn"`**, inherited by both
  crates via `[lints] workspace = true`; CI-equivalent enforcement is `-D warnings` on
  the command line (`just clippy`), not baked into the lint table, so local `cargo
clippy` still just warns.
- **License: dual MIT OR Apache-2.0.** Originally BSD-3-Clause, which conflicted with
  the guide's Rust-ecosystem convention. **Resolved 2026-07-31 by owner decision**:
  relicensed while the project still had a single copyright holder and no external
  contributors, which is the only cheap moment to do it. `LICENSE` was replaced by
  `LICENSE-MIT` + `LICENSE-APACHE` (Apache text fetched from apache.org, not
  reproduced from memory); `Cargo.toml`, `deny.toml`, `CONTRIBUTING.md` and
  `README.md` updated to match. Apache-2.0 is what supplies the express patent
  grant that BSD-3-Clause lacks.
- **Layout matches proposal §7.1 exactly**: `crates/detoxrs-core` (no I/O, no clap, no
  `std::fs`) and `crates/detoxrs` (binary). Each crate has one placeholder file
  (`lib.rs` / `main.rs`) with a `TODO` naming every module §7.1 plans, rather than an
  empty-file module tree.

## Verification (run 2026-07-31)

- `cargo build --workspace` — passes.
- `cargo test --workspace` — passes (1 unit test in `detoxrs-core`, 0 in `detoxrs`).
- `cargo clippy --workspace --all-targets -- -D warnings` — passes after adding
  `#[must_use]` and `const` to `placeholder_version` (pedantic/nursery caught both;
  fixed for real rather than suppressed).
- `cargo fmt --check` — passes (Rust files only; this check is scoped to `*.rs`).
- `cargo tree --workspace` — **4 lines / effectively 2 unique crates**
  (`detoxrs` -> `detoxrs-core`, plus `detoxrs-core` listed standalone since it's also a
  workspace root). **Budget correction (2026-07-31):** the only dependency ceiling
  that exists is **<= 11 direct dependencies** (proposal §7.2), enforced by
  `just dep-budget` (currently reports `0/11 direct dependencies: (none)`). The
  earlier "<= 45 total crates" figure stated here was wrong: proposal §7.2
  deliberately **struck** the total-transitive cap as unmeasured ("a number
  asserted without measuring is not a budget"). Do not reintroduce it.
- `just --list` — the justfile has grown since this pass. It now lists 21 recipes:
  `audit`, `build`, `ci`, `clippy`, `dep-budget`, `deny`, `fmt`, `fmt-check`,
  `fmt-check-file`, `fmt-check-md`, `fmt-check-rust`, `fmt-file`, `fmt-md`,
  `fmt-rust`, `gate`, `geiger`, `msrv`, `sbom`, `test`, `trivy`, `vet`.
- `just gate` (`fmt-check`, `clippy`, `test`, `msrv`, `dep-budget`) — **passes**,
  re-run 2026-07-31 during the stage-3 review (exit 0). At the time this note was
  first written it failed on `fmt-check-md` prettier drift in `docs/research/*.md`
  that other agents were concurrently editing; that drift has since been formatted
  away. Note that `fmt-check` covers `**/*.{md,yml,yaml,json}`, not just `**/*.md`.
  **Design question raised, not unilaterally changed:** `gate` is documented as
  "what a developer runs before pushing," yet it fails on Markdown/YAML/JSON
  formatting anywhere in the repo — so a Rust change can be blocked by an unrelated
  docs edit. `fmt-check-rust` already exists as the Rust-only half.
  **Recommendation for the justfile owner:** make `gate` depend on
  `fmt-check-rust` and leave repo-wide `fmt-check` to `ci` / a docs recipe. The
  justfile is shared, so this is a recommendation, not a change made here.
- `just fmt-check-file crates/detoxrs-core/README.md crates/detoxrs/README.md` — passes.

## Deferred guide recommendations (status re-verified 2026-07-31)

- ~~`cargo-deny`, `cargo-vet`, `cargo-audit`, `cargo-geiger`, Trivy: no `deny.toml`/CI
  wiring added.~~ **DONE.** `deny.toml`, `supply-chain/`, `.cargo/audit.toml`,
  `.github/workflows/security.yml` and `.github/workflows/supply-chain.yml` all exist;
  see `docs/rust-setup-supply-chain.md`. `cargo-msrv` itself is still **not** used:
  `just msrv` uses `rustup`/`cargo +<version>` directly, which needs only the
  toolchain already installed. That remains a deliberate choice, not a gap.
- ~~`just ci`/`gate` does not chain `audit`, `deny`, `trivy`, `vet`, `geiger`.~~
  **DONE.** `just ci` is `gate audit deny vet geiger trivy`; `gate` is
  `fmt-check clippy test msrv dep-budget`. `just sbom` and `just dep-budget` also
  exist now.
- Dev-dependencies from `testing.md`/proposal §8 (`proptest`, `insta`, `trycmd`,
  `assert_cmd`, `criterion`) are **not** added. There is no transform/plan logic yet to
  test against; adding them now would be dependencies with nothing exercising them.
- `LICENSE-MIT` / `LICENSE-APACHE` files: not added, since the license conflict above is
  unresolved at the time. **Since resolved**: `LICENSE-MIT` and `LICENSE-APACHE` now
  exist and `LICENSE` was removed.
- `.cargo/config.toml`: left absent (removed an empty placeholder directory from the
  partial prior run). Nothing in scaffolding needs a cargo config override yet — no
  target-specific flags, no registry mirror, no build script. Add it when a concrete
  need appears.
- `CONTRIBUTING.md`, `CLAUDE.md` additions for Rust invariants: not written; `local-dev.md`
  describes them but they read as governance/contributor-doc territory for another agent.

## Proposal conflicts flagged (not silently resolved)

- License: guide wanted dual MIT/Apache-2.0; repo was BSD-3-Clause. **No longer a
  conflict** — relicensed to dual MIT OR Apache-2.0 on 2026-07-31, so the project now
  matches the guide.

## Checklist for later agents (status re-verified 2026-07-31)

- [x] CI: GitHub Actions workflows (build matrix, clippy, fmt, msrv job pinned to
      1.93.0). **DONE** — `.github/workflows/ci.yml`, `docs/rust-setup-ci.md`.
- [x] Supply chain: `deny.toml`, `cargo-vet` policy/audits, `cargo-audit` wiring,
      Trivy scan; `just gate`/`ci` chaining. **DONE** —
      `docs/rust-setup-supply-chain.md`.
- [x] Security scanning: `cargo-geiger`. **DONE** — `just geiger` (needs an absolute
      `--manifest-path`; `-p` does not work against a virtual manifest).
- [x] Release automation: `release-please` config, `cargo publish` posture (still
      "do not publish", per §7.1), cross-platform binary builds. **DONE** —
      `.github/workflows/release.yml`, `docs/rust-setup-release.md`. Note the
      **tooling conflict**: the guide prescribes `release-please`, proposal §9.4
      chose `cargo-dist` + `release-plz`. `release-please` is the deliberate interim
      choice and `docs/rust-setup-release.md` recommends switching before the v1.0
      packaging milestone — the choice was not uncontested, and a human still owes
      that decision.
- [x] Governance: `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`,
      dual-license files. **DONE** — dual MIT OR Apache-2.0 as of 2026-07-31. The
      contact and response-time placeholders in `SECURITY.md` /
      `CODE_OF_CONDUCT.md` are **not** done and are real obligations before the
      first public release (see `docs/owner-decisions.md`).
- [x] Dependency management: Dependabot config. **DONE** — `.github/dependabot.yml`.
- [ ] Add `proptest`/`insta`/`trycmd`/`assert_cmd`/`criterion` dev-dependencies and the
      actual transform/plan modules they will test, per proposal §8 — still
      **OUTSTANDING**, deliberately: there is still no application logic to test.
- [ ] TOML formatting checker (`taplo` or equivalent). Still **OUTSTANDING** and
      still in tension with `AGENTS.md`'s "add a checker in the same change" rule —
      five TOML files now exist unchecked. See `docs/rust-setup-supply-chain.md`.

## Review record (stage 3)

Adjudicated 2026-07-31. "Ran" = command executed during this pass; "read" = verified
by reading the file/source.

| Finding                                                                                   | Reviewer | Verdict | Action / reason                                                                                                                                                                                                                                                                                    |
| ----------------------------------------------------------------------------------------- | -------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `deny`-over-`forbid` rationale rests on "neither `rustix` nor `nix` expose `renamex_np`"  | L1, L2   | ACCEPT  | Rewrote the unsafe-policy entry. `rustix` **does** expose `renameatx_np` via `renameat_with`/`RenameFlags` under `#[cfg(apple)]` (read: `rustix` 1.1.4 source; a `forbid(unsafe_code)` program was compiled and run on APFS). `nix` does not. Rationale withdrawn; both crates could use `forbid`. |
| `main.rs` repeats the same withdrawn rationale and drives `#![deny(unsafe_code)]` from it | L1       | ACCEPT  | Out of this document's scope (code file). Recorded here and listed for the propagation pass; not silently left unmentioned.                                                                                                                                                                        |
| "Budget is <= 45" total transitive crates                                                 | L2       | ACCEPT  | Struck. Proposal §7.2 (read) removed the transitive cap as unmeasured; the only cap is <= 11 direct, enforced by `just dep-budget` (ran: `0/11`).                                                                                                                                                  |
| `just --list` recipe list stale (9 recipes claimed, 20 exist)                             | L1       | ACCEPT  | Updated from `just --list` (ran).                                                                                                                                                                                                                                                                  |
| `just gate` failure attributed to three named research docs that are now clean            | L1       | ACCEPT  | Re-ran `just gate`: **passes, exit 0**. Recorded as passing today with the historical failure kept as context, rather than deleting the record.                                                                                                                                                    |
| Prettier scope stated as `**/*.md`                                                        | L1       | ACCEPT  | Corrected to `**/*.{md,yml,yaml,json}` (read: `justfile` `prettier_glob`).                                                                                                                                                                                                                         |
| Deferred section and checklist describe finished work as pending                          | L1, L2   | ACCEPT  | Both sections re-verified item by item against `just --list` and the repo tree, and marked DONE / OUTSTANDING.                                                                                                                                                                                     |
| Release-automation checklist item silent on the release-please vs cargo-dist conflict     | L2       | ACCEPT  | Added the pointer; the interim choice is now visibly an interim choice.                                                                                                                                                                                                                            |
| `gate` couples a Rust pre-push check to repo-wide Markdown formatting                     | L1 (obs) | ACCEPT  | Recorded as a design question with a concrete recommendation (`gate` -> `fmt-check-rust`). Not applied: the justfile is shared and this is not this document's call.                                                                                                                               |
| MSRV, edition, layout, license, build/test/clippy/fmt claims                              | L1       | CONFIRM | Re-ran `just gate` (build, test, clippy, fmt-rust, msrv, dep-budget all pass). No change needed.                                                                                                                                                                                                   |
| Placeholders should be tidied into commitments                                            | —        | REJECT  | Not a reviewer request, and explicitly refused: L3 found 30 placeholders all correctly guarded. Nothing here was converted from "placeholder" to "done", and no workflow is described as having run.                                                                                               |

**Files outside this document that still carry the withdrawn `rustix` rationale**
(handled by a separate propagation pass, not edited here): `CONTRIBUTING.md`,
`SECURITY.md`, `crates/detoxrs/src/main.rs`,
`docs/research/00-proposal-rust-detox-successor.md`.
