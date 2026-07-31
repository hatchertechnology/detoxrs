# Contributing to detoxrs

Thanks for taking the time to contribute. `detoxrs` renames files to sanitize
unsafe or unwieldy filenames — a successor to the archived C utility `detox`
— and it is still pre-1.0, so the codebase is small and there is plenty of
room to shape it.

## Code of Conduct

This project follows the [Contributor
Covenant](./CODE_OF_CONDUCT.md). By participating you agree to abide by it.

## Security

Do not open a public issue for a security vulnerability. See
[SECURITY.md](./SECURITY.md) for private reporting.

## Finding something to work on

There is no application logic yet — the workspace is scaffolding only (see
`docs/rust-setup-notes.md`). Check open issues, and look for a
`good first issue` label once the project starts labeling them (it does not
yet).

## Development setup

You need:

- The Rust toolchain pinned in `rust-toolchain.toml` (currently 1.96.1),
  installed via [rustup](https://rustup.rs/). `rustup` will pick it up
  automatically inside the repo.
- [`just`](https://just.systems/man/en/) as the command runner.

```bash
git clone https://github.com/hatchertechnology/detoxrs.git
cd detoxrs
just build
just test
```

Run `just --list` to see every available recipe with its description; the
table below is only a summary, and the recipe list is the source of truth if
they drift.

## The check suite

These are the recipes that exist today. `just gate` is the fast local gate;
`just ci` is the wider one that also runs the supply-chain tooling (and needs
those tools installed).

| Recipe                        | Command                                             | What it protects                                                |
| ----------------------------- | --------------------------------------------------- | --------------------------------------------------------------- |
| `just build`                  | `cargo build` (workspace)                           | The workspace compiles                                          |
| `just fmt`                    | `cargo fmt` + `prettier` on Markdown                | Formatting, fixed in place                                      |
| `just fmt-check`              | Check-only form of `fmt`                            | Formatting, unmodified (CI-equivalent)                          |
| `just fmt-file <files>`       | Format only the given files                         | Formatting a specific file without touching others mid-edit     |
| `just fmt-check-file <files>` | Check-only form of `fmt-file`                       | Same, without writing                                           |
| `just clippy`                 | `cargo clippy --all-targets -- -D warnings`         | Lints — `all`/`pedantic`/`nursery`, warnings promoted to errors |
| `just test`                   | `cargo test` (workspace)                            | The test suite                                                  |
| `just msrv`                   | Builds against the pinned MSRV toolchain            | MSRV drift from dependency or code changes                      |
| `just dep-budget`             | Counts direct dependencies against a cap            | The <= 11 direct-dependency budget (proposal §7.2)              |
| `just audit`                  | `cargo audit`                                       | Known advisories in the dependency tree                         |
| `just deny`                   | `cargo deny check`                                  | License, advisory, ban and source policy (`deny.toml`)          |
| `just vet`                    | `cargo vet check`                                   | Dependency review status                                        |
| `just geiger`                 | `cargo geiger -p detoxrs`                           | `unsafe` usage in the dependency tree (informational)           |
| `just trivy`                  | `trivy fs`                                          | Vulnerability/secret/misconfig scan                             |
| `just sbom`                   | `cargo cyclonedx`                                   | CycloneDX SBOM generation                                       |
| `just gate`                   | `fmt-check`, `clippy`, `test`, `msrv`, `dep-budget` | The fast local gate — run this before opening a PR              |
| `just ci`                     | `gate` + `audit`, `deny`, `vet`, `geiger`, `trivy`  | Everything, including supply chain                              |

`prettier` checks Markdown, YAML and JSON; `cargo fmt` checks Rust. TOML is
deliberately unchecked (see the comment in the `justfile`). See `AGENTS.md` for
the rule that a new file type gets a checker added in the same change that
introduces it.

The supply-chain recipes above require tools that are **not** installed by
`rustup`: `cargo-deny`, `cargo-audit`, `cargo-vet`, `cargo-geiger`,
`cargo-cyclonedx` and `trivy`. `just gate` deliberately excludes them so the
common path needs nothing extra; install them before running `just ci`.

### Tool installation

`just` itself: see the [installation instructions](https://just.systems/man/en/packages.html)
for your platform (e.g. `cargo install just`, or your package manager). The
pinned Rust toolchain installs itself via `rustup` the first time you build in
this repo.

## MSRV

The workspace MSRV is **1.93.0** (`rust-version` in the workspace
`Cargo.toml`), separate from the pinned toolchain (1.96.1, in
`rust-toolchain.toml`, used for day-to-day development). Do not use stdlib or
language features stabilized after 1.93.0, and do not raise the MSRV without
updating both `Cargo.toml` and `docs/rust-setup-notes.md`'s derivation. Run
`just msrv` to verify a change still builds on the MSRV toolchain.

## Code standards

- **`unsafe` is forbidden in `detoxrs-core`** (`#![forbid(unsafe_code)]`) and
  **denied in `detoxrs`** (`#![deny(unsafe_code)]`, overridable only with an
  explicit, reviewed `#[allow(unsafe_code)]` naming the syscall and the
  safety argument — see `crates/detoxrs/src/main.rs`). A PR introducing
  `unsafe` needs that comment and should expect close review; this is
  reserved for a planned macOS FFI shim that does not exist yet.
- **Clippy is not advisory.** `pedantic` and `nursery` are on
  workspace-wide, and `just clippy` treats warnings as errors. If a lint is
  genuinely wrong for a piece of code, `#[allow(...)]` it with a comment
  explaining why, rather than silencing it globally.
- **Tests come with the change.** No test-layout convention is established yet
  beyond the workspace scaffolding (one unit test in `detoxrs-core`); follow the project's testing design once real
  logic lands (`docs/research/00-proposal-rust-detox-successor.md` §8) —
  unit tests beside the code, property tests for transform invariants,
  snapshot and CLI-driving tests for behavior.
- **Never panic on untrusted input** (filenames, CLI arguments, config
  files). Invalid or hostile input must degrade to a reported skip, not a
  crash — this is a hard requirement for a tool whose entire job is
  processing attacker-influenced filenames (see `SECURITY.md`).
- **Keep the dependency tree small.** The budget is **<= 11 direct
  dependencies** (proposal §7.2), enforced by `just dep-budget`, because every
  transitive crate becomes a Debian source package. A PR adding a dependency
  must say why our own code will not do, and will need a `cargo vet` entry and
  a `deny.toml` license check to pass.
- Format Markdown you touch with `just fmt-check-file <path>` /
  `just fmt-file <path>` rather than the repo-wide `fmt`/`fmt-check`, to
  avoid reformatting files someone else is mid-edit on.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) —
`feat:`, `fix:`, `docs:`, `chore:`, `ci:`, `refactor:`, `test:`,
`style:` for formatting-only changes. Required by `AGENTS.md`, and
`release-please` parses these prefixes to derive the next version — so a
mislabelled commit produces a wrong version bump.

## Pull requests

1. Branch off `main`.
2. Write Conventional Commits (above).
3. Run `just gate` locally before opening the PR.
4. Open the PR against `main`.

> **Placeholder — human decision needed:** whether `main` has branch
> protection, required status checks, or required reviews configured is a
> GitHub repository setting, not something this document can assert. Verify
> in the repository's Settings → Branches/Rulesets and update this section
> once it reflects reality — do not describe protections that are not
> actually turned on.

## Releases

Release automation exists but has never run. `release-please` derives versions
from Conventional Commit prefixes (so the prefixes above are load-bearing), and
`.github/workflows/release.yml` builds cross-platform binaries with checksums
and SLSA provenance. It **fails closed**: nothing builds, tags or publishes
until a maintainer creates the `release` GitHub Environment and its secrets.

`detoxrs` is not published to crates.io — the binary is the product (see
`docs/research/00-proposal-rust-detox-successor.md` §7.1). Do not hand-edit a
version field; `release-please` owns versions and `CHANGELOG.md`. Full detail in
`docs/rust-setup-release.md`, including a recommended future move to
`release-plz` + `cargo-dist` before the v1.0 packaging milestone.

## Licensing of contributions

This project is dual-licensed under **MIT OR Apache-2.0**, the Rust-ecosystem
convention — see [`LICENSE-MIT`](./LICENSE-MIT) and
[`LICENSE-APACHE`](./LICENSE-APACHE). Users may choose either license.

Unless you state otherwise, any contribution you intentionally submit for
inclusion in this project, as defined in the Apache-2.0 license, shall be
dual-licensed under those same two licenses, with no additional terms or
conditions.

### Do not copy code from the upstream `detox`

`detoxrs` is an independent implementation, not a fork or a port. The upstream
C `detox` is BSD-3-Clause; because we copy none of its code, that license
imposes no obligation on this repository, and our dual MIT/Apache-2.0 licensing
is clean.

That property is easy to break by accident. Do **not** paste upstream code,
character-translation tables, or `.tbl` data into this project. Reading the
upstream source to understand its _behavior_ is fine and expected — the
research in `docs/research/` does exactly that, with citations. Reproducing its
_expression_ is not. If you believe a piece of upstream code genuinely must be
carried over, raise it in an issue first: it requires retaining Doug Harple's
copyright notice and the BSD-3-Clause terms, and that is a licensing decision,
not a code-review one.
