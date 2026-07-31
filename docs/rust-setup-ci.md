# Rust project setup notes: CI/CD

CI/Dependabot pass. Written against `wiki/rust/ideal-project-setup/ci-cd.md`
and `msrv.md` (primary sources), `dependencies.md`, `README.md`,
`local-dev.md`, skimmed `code-quality.md`/`testing.md`, and
`docs/research/00-proposal-rust-detox-successor.md` §6.4. Builds on
`docs/rust-setup-notes.md` (Agent A's foundation pass) without contradicting
it. Scope: `.github/workflows/ci.yml`, `.github/dependabot.yml`.

## Job design

Separate jobs per the guide's "a failure names itself" principle:

- **`test`** (matrix: `ubuntu-latest`, `macos-latest`, `macos-15-intel`) —
  `cargo build --workspace --locked` then `cargo test --workspace --locked`.
  `fail-fast: false` so one OS failing doesn't cancel the others.
- **`test-windows`** — same steps, `windows-latest`, its own job (not folded
  into the matrix) with `continue-on-error: true`. Best-effort per the
  proposal, not tier 1; kept separate so a Windows-only failure is visible
  but never blocks merge.
- **`cross-build`** (matrix: `aarch64-unknown-linux-gnu`,
  `x86_64-unknown-linux-musl`) — build-only verification (no test execution)
  on `ubuntu-latest`, using `rustup target add` (via `dtolnay/rust-toolchain`'s
  `targets:` input) plus an apt-installed cross linker.
- **`lint`** — `cargo fmt --all --check` + `cargo clippy --workspace
--all-targets --locked -- -D warnings`, Ubuntu only.
- **`msrv`** — `just msrv` (build check on the pinned 1.93.0 toolchain) plus
  `cargo +1.93.0 test --workspace --locked` (full test suite), per
  `msrv.md`'s explicit guidance to run tests, not just `cargo check`, on the
  MSRV toolchain.

## Matrix chosen vs. the guide vs. the proposal

The guide's own example matrix is `[ubuntu-latest, macos-latest,
windows-latest]` — one native OS each, tests run everywhere. The proposal
(§6.4) names a _different_ tier-1 set: Linux x86_64/aarch64 (gnu **and**
musl) and macOS x86_64/aarch64, with Windows explicitly demoted to
best-effort (§6.4, §6.5 — the Windows reserved-name behavior tier 1 would
need to assert is contested and unresolved). **These two disagree on
Windows's tier**, and I followed the proposal (project-specific, reasoned
about the actual code) over the guide's generic example.

Reconciling the proposal's tier-1 list against what GitHub-hosted runners can
actually do:

| Proposal tier-1 target       | How it's covered                              | Why                                                                                                                                                                                                                                                                                                                |
| ---------------------------- | --------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Linux x86_64 gnu             | `test` job, `ubuntu-latest`                   | Native runner; full test execution.                                                                                                                                                                                                                                                                                |
| macOS aarch64                | `test` job, `macos-latest`                    | Native runner (GitHub's `macos-latest`/14+ line is aarch64-only); full test execution.                                                                                                                                                                                                                             |
| macOS x86_64                 | `test` job, `macos-15-intel`                  | Native Intel runner label (see below); full test execution.                                                                                                                                                                                                                                                        |
| Linux aarch64 gnu            | `cross-build` job                             | GitHub-hosted runners are x86_64 or (paid/beta) aarch64; cross-compiling with `gcc-aarch64-linux-gnu` is a plain `apt-get install`, but the resulting binary can't run on an x86_64 host without emulation, so this is build-only, not test.                                                                       |
| Linux x86_64 musl            | `cross-build` job                             | Same reasoning; `musl-tools` is a plain `apt-get install`. Build-only.                                                                                                                                                                                                                                             |
| Linux aarch64 musl           | **Deferred**                                  | Needs a musl **cross** C toolchain (targeting aarch64 from an x86_64 host), which isn't a simple apt package — it needs a prebuilt musl-cross toolchain or tooling like `cross-rs`/`cargo-zigbuild`. Adding that is a real piece of tooling work, not a one-line CI change; deferred rather than half-implemented. |
| Windows x86_64 (best-effort) | `test-windows` job, `continue-on-error: true` | Matches the proposal's own tier for Windows.                                                                                                                                                                                                                                                                       |

**On `macos-13`:** I initially wrote the matrix as
`[ubuntu-latest, macos-latest, macos-13]` (the conventional x86_64 label at
the time the guide text and much prior art were written). `actionlint`
caught that `macos-13` is no longer a valid runner label — GitHub has
retired that image line — and listed `macos-15-intel` as the current Intel
macOS label, which is what's in the committed workflow. This is exactly the
kind of drift `actionlint` exists to catch; worth a periodic re-check as
GitHub continues rotating runner images.

Nothing GitHub-hosted-runner-shaped was excluded beyond aarch64-musl above.
A true multi-arch, multi-libc test _matrix_ (running tests, not just builds,
on aarch64 and musl) would need either paid/beta GitHub-hosted ARM runners
or QEMU-based emulation (e.g. via `docker run --platform` or
`uraimo/run-on-arch-action`) — both are real additions, not reconciliations
of what's "reasonable" on the free hosted fleet, so left for whoever adds
release cross-builds (this agent's scope excludes `release.yml`).

## Actions pinned to commit SHA

Per `ci-cd.md`'s pinning guidance (reproducible CI inputs; a compromised
action release can't inject code if the ref is an immutable commit).
Every SHA below was confirmed to resolve to a real commit via
`git ls-remote --tags <repo>` (the `gh` CLI on this machine points at a
GitHub Enterprise host and 404s on public `github.com` repos; the GitHub
REST API was rate-limited without a token, so `git ls-remote` — no API,
no auth needed — was used instead):

| Action                   | Version | SHA                                        |
| ------------------------ | ------- | ------------------------------------------ |
| `actions/checkout`       | v4.4.0  | `11d5960a326750d5838078e36cf38b85af677262` |
| `dtolnay/rust-toolchain` | v1      | `e97e2d8cc328f1b50210efc529dca0028893a2d9` |
| `Swatinem/rust-cache`    | v2.9.1  | `23869a5bd66c73db3c0ac40331f3206eb23791dc` |
| `taiki-e/install-action` | v2.85.5 | `6a1bd70eaac3c8bdf093356838d7ee09fda951cf` |

(The `dtolnay/rust-toolchain@v1` SHA happens to match the guide's own
worked example exactly — confirmed independently via `git ls-remote`, not
copied from the guide text.) Dependabot's `github-actions` ecosystem entry
(below) will open PRs to bump these SHAs going forward, per `ci-cd.md`.

## Caching and concurrency

- `Swatinem/rust-cache` on every job that builds, per the guide.
- Top-level `concurrency: { group: ${{ github.workflow }}-${{ github.ref }},
cancel-in-progress: true }` so a superseded push cancels the in-flight run
  for the same branch/PR, per the guide.
- `permissions: contents: read` at the workflow level (no job publishes or
  releases anything, so nothing needs elevated permissions — matches the
  guide's default-read-only principle).

## Where `just` recipes were used, and where direct `cargo` was used instead

Agent A's justfile already has `build`, `test`, `clippy`, `fmt-check`,
`msrv`, `gate` — enough to _mostly_ run CI through `just`, but not cleanly
in every job:

- **`msrv` job** uses `just msrv` for the build-check step (it already does
  exactly `cargo +1.93.0 build --workspace --locked`), then adds a bare
  `cargo +1.93.0 test --workspace --locked` step directly, because `just
msrv` only builds and `msrv.md` is explicit that the MSRV job should run
  the full test suite. No justfile edit needed for this — it's an
  additional CI step, not a missing recipe.
- **`lint` job** deliberately does **not** call `just fmt-check`. That
  recipe runs `npx prettier --check "**/*.md"` over the _entire_ repo in
  addition to `cargo fmt --all --check`. Coupling the Rust lint gate to
  every Markdown file in the repo (including ones other agents are actively
  editing — see `docs/rust-setup-notes.md`'s note that `just gate` currently
  fails on `fmt-check` for exactly this reason) would make this CI job flaky
  for reasons that have nothing to do with Rust code quality. Used
  `cargo fmt --all --check` + `cargo clippy --workspace --all-targets
--locked -- -D warnings` directly instead.
- **`test`/`build` steps** use direct `cargo build/test --workspace
--locked` rather than `just build`/`just test`, because those two recipes
  don't pass `--locked` (`cargo test --workspace`, `cargo build
--workspace`). Without `--locked`, a CI run could silently succeed against
  a locally-mutated `Cargo.lock` instead of failing when the committed
  lockfile is out of sync with `Cargo.toml` — the guide's reproducibility
  concern for an application crate with a committed lockfile.

**Recipe requests for whichever agent owns the justfile next** (not
implemented here — out of this agent's scope per the task brief):

- `test`/`build` could gain `--locked` (`cargo test --workspace --locked`,
  `cargo build --workspace --locked`) to match what CI actually runs and
  keep local/CI identical, per `local-dev.md`'s stated goal.
- A Rust-only formatting recipe (e.g. `fmt-check-rust: cargo fmt --all
--check`) separate from the combined Markdown+Rust `fmt-check` would let
  CI's `lint` job call `just` too, instead of duplicating the bare `cargo
fmt` command.

No existing recipe was edited, and no new recipe was invented and called as
if it existed.

## Dependabot

`.github/dependabot.yml`: `cargo` and `github-actions` ecosystems, both
weekly, per `dependencies.md`'s batching/CI-cost/review-bandwidth rationale.
No dependency-group config, since there's nothing to group yet (zero
non-dev dependencies exist per `docs/rust-setup-notes.md`) — adding grouping
rules now would be speculative config for dependencies that don't exist.
Dependabot's `github-actions` ecosystem understands SHA-pinned actions and
will open PRs that bump the SHA (with an updated version comment), matching
`ci-cd.md`'s note that "Dependabot will open PRs to update the pinned SHAs."

## Deferred guide recommendations (explicitly out of scope this round)

- `security` and `vet` jobs from `ci-cd.md` (cargo-audit, cargo-deny, Trivy,
  cargo-geiger, cargo-vet) — another agent's scope (supply-chain/security
  workflows; this agent was explicitly told not to create
  `security.yml`/`supply-chain.yml`).
- Scheduled (cron) runs for RustSec/license checks, evidence retention for
  release artifacts, org-level policies requiring full-length SHA pins —
  `ci-cd.md`'s "Evidence retention and scheduled checks" section; belongs
  with the security/release work, not this CI skeleton.
- `release-please`/release workflow — explicitly out of scope
  (`release.yml` named as another agent's file).
- True aarch64-musl and any emulated/hosted-ARM _test_ execution (as
  opposed to build-only cross-compilation) — see the matrix table above.

## What remains unvalidated until a real push

**This workflow has not been executed.** `actionlint` found and let me fix
one real problem (the retired `macos-13` runner label) and otherwise reports
no issues, and every YAML file parses with `yaml.safe_load`, but none of the
following has been verified against a live GitHub Actions run:

- That `cargo build`/`test --locked` actually succeeds on each runner OS
  (only verified locally on this machine's toolchain, per
  `docs/rust-setup-notes.md`).
- That the `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-musl` cross
  builds succeed with just the apt packages named (cross-compilation is
  notorious for missing linker/pkg-config wrinkles that only show up on the
  actual runner image).
- That `dtolnay/rust-toolchain`'s `targets:` input and the `CARGO_TARGET_*_LINKER`
  environment variable actually wire up the way I expect end-to-end in a
  real run.
- That Dependabot's weekly schedule and PR generation behave as configured
  (this can only be observed once the config is live on GitHub).

A push to a branch (or a PR) against this repo is what would validate all of
the above.
