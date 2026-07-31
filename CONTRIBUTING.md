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

These are the recipes that exist today. There is no `just ci` or `just gate`
alias beyond what's listed — `just gate` below **is** the full local gate;
there is no wider aggregate recipe to reach for.

| Recipe                        | Command                                     | What it protects                                                |
| ----------------------------- | ------------------------------------------- | --------------------------------------------------------------- |
| `just build`                  | `cargo build` (workspace)                   | The workspace compiles                                          |
| `just fmt`                    | `cargo fmt` + `prettier` on Markdown        | Formatting, fixed in place                                      |
| `just fmt-check`              | Check-only form of `fmt`                    | Formatting, unmodified (CI-equivalent)                          |
| `just fmt-file <files>`       | Format only the given files                 | Formatting a specific file without touching others mid-edit     |
| `just fmt-check-file <files>` | Check-only form of `fmt-file`               | Same, without writing                                           |
| `just clippy`                 | `cargo clippy --all-targets -- -D warnings` | Lints — `all`/`pedantic`/`nursery`, warnings promoted to errors |
| `just test`                   | `cargo test` (workspace)                    | The test suite                                                  |
| `just msrv`                   | Builds against the pinned MSRV toolchain    | MSRV drift from dependency or code changes                      |
| `just gate`                   | `fmt-check`, `clippy`, `test`, `msrv`       | The full local gate — run this before opening a PR              |

Markdown formatting is checked with `prettier`; there is currently no checker
for anything else beyond `cargo fmt` for Rust — see `AGENTS.md` for the rule
that a new file type gets a checker added in the same change that introduces
it.

There is no `just audit`/`deny`/`trivy`/`vet`/`geiger` yet. Supply-chain and
security-scanning tooling (`cargo-deny`, `cargo-audit`, `cargo-vet`, Trivy,
`cargo-geiger`) is not wired into this repository yet — see
`docs/rust-setup-notes.md`'s deferred-work list. Don't reference those
recipes as if they exist.

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
- **Tests come with the change.** There is no test-layout convention
  established yet beyond what ships with the workspace scaffolding (one unit
  test in `detoxrs-core`); follow the project's testing design once real
  logic lands (`docs/research/00-proposal-rust-detox-successor.md` §8) —
  unit tests beside the code, property tests for transform invariants,
  snapshot and CLI-driving tests for behavior.
- **Never panic on untrusted input** (filenames, CLI arguments, config
  files). Invalid or hostile input must degrade to a reported skip, not a
  crash — this is a hard requirement for a tool whose entire job is
  processing attacker-influenced filenames (see `SECURITY.md`).
- **Keep the dependency tree small.** A PR adding a new dependency should
  say why it's needed; there is no `cargo vet` policy in this repo yet to
  formally require an entry for it, but that is a gap, not a green light —
  don't add a dependency casually.
- Format Markdown you touch with `just fmt-check-file <path>` /
  `just fmt-file <path>` rather than the repo-wide `fmt`/`fmt-check`, to
  avoid reformatting files someone else is mid-edit on.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) —
`feat:`, `fix:`, `docs:`, `chore:`, `ci:`, `refactor:`, `test:`,
`style:` for formatting-only changes. This is required by `AGENTS.md` today;
whether release automation consumes these prefixes to derive versions is not
yet decided (no release tooling exists in this repo yet).

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

No release process exists yet — there is no version-bump automation, no
`CHANGELOG.md`, and `detoxrs` is not published to any registry (see
`docs/research/00-proposal-rust-detox-successor.md` §7.1: it is not intended
for independent publish). Do not hand-edit a version field in anticipation of
a release process that hasn't been built.

## Licensing of contributions

The repository is licensed under **BSD-3-Clause** (see [`LICENSE`](./LICENSE)).
By submitting a contribution, you agree it is licensed under those same
terms, unless you state otherwise.

This is a deliberate departure from the Rust-ecosystem convention of
dual-licensing under MIT OR Apache-2.0. That convention is unresolved here —
`docs/rust-setup-governance.md` documents the conflict and its consequences.
If the project relicenses in the future, this section and the contribution
terms above must be updated to match whatever `LICENSE`/`LICENSE-*` files
exist at the time; until then, BSD-3-Clause is what you are actually agreeing
to, not MIT/Apache-2.0.
