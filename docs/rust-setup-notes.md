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
- **Unsafe-code policy: `forbid` in `detoxrs-core`, `deny` (not `forbid`) in `detoxrs`.**
  The core crate is pure by design and has no future need for `unsafe`. The binary
  crate will eventually contain a hand-written macOS `libc` FFI shim (proposal §5.4,
  §7.1 `fsops/macos.rs`: `renamex_np` + a `getattrlist`/`VOL_CAP_INT_RENAME_EXCL` probe,
  because neither `rustix` nor `nix` expose it). `forbid` cannot be downgraded by any
  later `#[allow(unsafe_code)]`, however narrow — it would have to be deleted outright
  the day that shim lands. `deny` gets the same "unsafe requires deliberate, reviewed
  opt-in" default today, and when `fsops/macos.rs` is written it gets a scoped
  `#[allow(unsafe_code)]` with a comment naming the syscall and the safety argument.
  Documented in `crates/detoxrs/src/main.rs`.
- **Lints: `[workspace.lints.clippy] all/pedantic/nursery = "warn"`**, inherited by both
  crates via `[lints] workspace = true`; CI-equivalent enforcement is `-D warnings` on
  the command line (`just clippy`), not baked into the lint table, so local `cargo
clippy` still just warns.
- **License left untouched.** Repo `LICENSE` is BSD-3-Clause. The guide's Rust-ecosystem
  convention is dual MIT/Apache-2.0. **Conflict, not resolved here** — `Cargo.toml`
  declares `license = "BSD-3-Clause"` (matching the actual `LICENSE` file) rather than
  the guide's recommendation. A human needs to decide whether to relicense or keep
  BSD-3-Clause and simply note the ecosystem-convention deviation.
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
  workspace root). Budget is <= 45; nowhere close, as expected with zero non-dev
  dependencies added yet.
- `just --list` — lists `build`, `clippy`, `fmt`, `fmt-check`, `fmt-check-file`,
  `fmt-file`, `gate`, `msrv`, `test`.
- `just gate` (`fmt-check`, `clippy`, `test`, `msrv`) — **clippy/test/msrv pass**;
  `fmt-check` fails on `npx prettier --check "**/*.md"` reporting drift in
  `docs/research/00-proposal-rust-detox-successor.md`, `01-detox-current-behavior.md`,
  `02-detox-issues-and-demand.md`. Those are files another agent is concurrently
  editing (see `git status` at task start) and are outside this agent's scope —
  per `AGENTS.md`, reporting rather than fixing. Not caused by anything added here.
- `just fmt-check-file crates/detoxrs-core/README.md crates/detoxrs/README.md` — passes.

## Deferred guide recommendations (not implemented — no code exists yet to need them)

- `cargo-msrv`, `cargo-deny`, `cargo-vet`, `cargo-audit`, `cargo-geiger`, Trivy: no
  `deny.toml`/CI wiring added. `just msrv` uses `rustup`/`cargo +<version>` directly
  instead of `cargo msrv verify` — installing and configuring `cargo-msrv` is CI/tooling
  territory for another agent; today's recipe needs only the toolchain already
  installed.
- `just ci`/`gate` does not chain `audit`, `deny`, `trivy`, `vet`, `geiger` — those
  recipes don't exist because the tools aren't wired up yet (another agent's scope per
  the task brief: CI, supply-chain).
- Dev-dependencies from `testing.md`/proposal §8 (`proptest`, `insta`, `trycmd`,
  `assert_cmd`, `criterion`) are **not** added. There is no transform/plan logic yet to
  test against; adding them now would be dependencies with nothing exercising them.
- `LICENSE-MIT` / `LICENSE-APACHE` files: not added, since the license conflict above is
  unresolved and the existing `LICENSE` (BSD-3-Clause) was left untouched per instructions.
- `.cargo/config.toml`: left absent (removed an empty placeholder directory from the
  partial prior run). Nothing in scaffolding needs a cargo config override yet — no
  target-specific flags, no registry mirror, no build script. Add it when a concrete
  need appears.
- `CONTRIBUTING.md`, `CLAUDE.md` additions for Rust invariants: not written; `local-dev.md`
  describes them but they read as governance/contributor-doc territory for another agent.

## Proposal conflicts flagged (not silently resolved)

- License: guide wants dual MIT/Apache-2.0; repo `LICENSE` is BSD-3-Clause. Implemented
  per the actual `LICENSE` file (BSD-3-Clause) since changing the license is explicitly
  out of scope for this task.

## Checklist for later agents

- [ ] CI: GitHub Actions workflows (build matrix, clippy, fmt, msrv job pinned to
      1.93.0, per `msrv.md`'s job example).
- [ ] Supply chain: `deny.toml` (`cargo-deny`), `cargo-vet` policy/audits, `cargo-audit`
      wiring, Trivy scan; extend `just gate`/`ci` to chain them per `local-dev.md`'s
      `ci: fmt lint test audit msrv deny trivy vet geiger` pattern.
- [ ] Security scanning: `cargo-geiger` (informational; trivially zero right now since
      `unsafe_code` is forbidden/denied everywhere).
- [ ] Release automation: `release-please` config, Conventional Commits enforcement
      beyond the `AGENTS.md` note, `cargo publish` posture (crate is not intended for
      independent publish per §7.1 — confirm that stays true), cross-platform binary
      builds.
- [ ] Governance: `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CONTRIBUTING.md`, dual-license
      files or a documented decision on the BSD-3-Clause vs MIT/Apache-2.0 conflict
      above.
- [ ] Dependency management: Dependabot/Renovate config once real dependencies exist.
- [ ] Add `proptest`/`insta`/`trycmd`/`assert_cmd`/`criterion` dev-dependencies and the
      actual transform/plan modules they will test, per proposal §8 — deliberately not
      done here since there is no logic yet.
