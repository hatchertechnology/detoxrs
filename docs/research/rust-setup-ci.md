# Rust project setup notes: CI/CD

CI/Dependabot pass. Written against `wiki/rust/ideal-project-setup/ci-cd.md`
and `msrv.md` (primary sources), `dependencies.md`, `README.md`,
`local-dev.md`, skimmed `code-quality.md`/`testing.md`, and
`docs/research/00-proposal-rust-detox-successor.md` §6.4. Builds on
`docs/research/rust-setup-notes.md` (Agent A's foundation pass) without contradicting
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

**Owner-decision compliance (`docs/owner-decisions.md`, 2026-07-31 "Test
hardware"):** the owner has Linux and macOS only, no Windows machine and no
NTFS or exFAT volume. Windows therefore "must compile and unit-test in CI, but
no filesystem behavior is asserted," and must not be promoted to tier 1. This
matrix complies: the Windows job compiles and runs the unit tests on a
GitHub-hosted runner, is `continue-on-error: true`, and is deliberately kept
out of the tier-1 matrix. **Nothing in this document asserts verified Windows
filesystem behavior, and nothing here should be read as doing so** — a green
`test-windows` job means the code compiled and the unit tests passed on a
hosted runner, not that reserved-name or path-length behavior has been
validated on NTFS or exFAT (proposal §11 spikes 3 and 4 remain open per the
owner decision).

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
  recipe runs prettier over the _entire_ repo (`**/*.{md,yml,yaml,json}`) in
  addition to `cargo fmt --all --check`. Coupling the Rust lint gate to
  every Markdown/YAML/JSON file in the repo (including ones other agents are
  actively editing) would make this CI job flaky for reasons that have
  nothing to do with Rust code quality. Used `cargo fmt --all --check` +
  `cargo clippy --workspace --all-targets --locked -- -D warnings` directly
  instead.
- **`test`/`build` steps** use direct `cargo build/test --workspace
--locked` rather than `just build`/`just test`. **Status update
  (2026-07-31): the stated reason is obsolete** — `just build` and
  `just test` now both pass `--locked` (read: `justfile`), so the recipes
  and the CI steps are equivalent today. The CI steps were left as bare
  `cargo` rather than switched, which is a defensible choice (the workflow
  no longer depends on `just` being installed for these jobs) but it is now
  a choice, not a necessity. The original reproducibility concern stands
  either way: without `--locked`, CI could silently succeed against a
  mutated `Cargo.lock`.

**Recipe requests for whichever agent owns the justfile next — both now
satisfied** (re-verified 2026-07-31 against `just --list` and `justfile`):

- ~~`test`/`build` could gain `--locked`.~~ **DONE** — `cargo build
--workspace --locked`, `cargo test --workspace --locked`.
- ~~A Rust-only formatting recipe separate from the combined Markdown+Rust
  `fmt-check`.~~ **DONE** — `fmt-check-rust` (`cargo fmt --all --check`),
  with `fmt-check-md` as the prettier half. The `lint` job could now call
  `just fmt-check-rust` instead of duplicating the bare command; that is a
  small follow-up, not a gap.

**Related design question, recorded not acted on:** `just gate` (the
documented pre-push check) still depends on the combined `fmt-check`, so a
Rust developer's gate fails on unrelated Markdown drift anywhere in the
repo — the same coupling this job avoided. Recommendation for the justfile
owner: point `gate` at `fmt-check-rust` and leave repo-wide `fmt-check` to
`ci`. The justfile is shared, so this is a recommendation only. (`just gate`
was re-run during the stage-3 review and **passes** today, exit 0 — the
coupling is a fragility, not a current failure.)

No existing recipe was edited by this pass, and no new recipe was invented
and called as if it existed.

## Dependabot

`.github/dependabot.yml`: `cargo` and `github-actions` ecosystems, both
weekly, per `dependencies.md`'s batching/CI-cost/review-bandwidth rationale.
No dependency-group config, since there's nothing to group yet (zero
non-dev dependencies exist per `docs/research/rust-setup-notes.md`) — adding grouping
rules now would be speculative config for dependencies that don't exist.
Dependabot's `github-actions` ecosystem understands SHA-pinned actions and
will open PRs that bump the SHA (with an updated version comment), matching
`ci-cd.md`'s note that "Dependabot will open PRs to update the pinned SHAs."

## Deferred guide recommendations (explicitly out of scope this round)

- ~~`security` and `vet` jobs from `ci-cd.md` (cargo-audit, cargo-deny, Trivy,
  cargo-geiger, cargo-vet)~~ — **DONE by another agent**, as separate workflow
  files rather than jobs in `ci.yml`: `.github/workflows/security.yml` and
  `.github/workflows/supply-chain.yml`. See
  `docs/research/rust-setup-supply-chain.md`. Not folded into `ci.yml`; `ci.yml` is
  unchanged by that work.
- ~~Scheduled (cron) runs for RustSec/license checks~~ — **DONE** in
  `security.yml` (weekly `0 6 * * 1`). Evidence retention for release
  artifacts and org-level full-length-SHA policy remain **OUTSTANDING** and
  belong with release/org work, not this CI skeleton.
- ~~`release-please`/release workflow~~ — **DONE by another agent**:
  `.github/workflows/release.yml`, `docs/research/rust-setup-release.md`. Still out of
  this document's scope; noted so the deferral does not read as an open
  request.
- True aarch64-musl and any emulated/hosted-ARM _test_ execution (as
  opposed to build-only cross-compilation) — see the matrix table above.

## What remains unvalidated until a real push

**This workflow has not been executed.** `actionlint` found and let me fix
one real problem (the retired `macos-13` runner label) and otherwise reports
no issues, and every YAML file parses with `yaml.safe_load`, but none of the
following has been verified against a live GitHub Actions run:

- That `cargo build`/`test --locked` actually succeeds on each runner OS
  (only verified locally on this machine's toolchain, per
  `docs/research/rust-setup-notes.md`).
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

## Review record (stage 3)

Adjudicated 2026-07-31. "Ran" = command executed during this pass; "read" = verified
by reading the file.

| Finding                                                                                          | Reviewer | Verdict | Action / reason                                                                                                                                                                                                                                                                                                                        |
| ------------------------------------------------------------------------------------------------ | -------- | ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Stated reason for not using `just build`/`just test` (no `--locked`) is obsolete                 | L1, L2   | ACCEPT  | `justfile` now passes `--locked` on both (read). Marked the reason obsolete and reframed the bare-`cargo` steps as a choice, not a necessity. Did not change `ci.yml`.                                                                                                                                                                 |
| Recipe request "Rust-only fmt-check" already satisfied                                           | L1, L2   | ACCEPT  | `fmt-check-rust` exists (ran `just --list`). Both recipe requests marked DONE so they stop reading as open asks.                                                                                                                                                                                                                       |
| Cross-reference to `rust-setup-notes.md`'s "`just gate` currently fails" is stale                | L1       | ACCEPT  | Removed the stale cross-reference. Re-ran `just gate`: **passes, exit 0**.                                                                                                                                                                                                                                                             |
| Deferred `security`/`vet` jobs and `release-please` still framed as another agent's open work    | L2       | ACCEPT  | Marked DONE with the actual files (`security.yml`, `supply-chain.yml`, `release.yml`) named, while keeping them out of this document's scope.                                                                                                                                                                                          |
| Platform matrix contradicts owner-decisions on Windows                                           | L2       | REJECT  | L2 itself concluded "no contradiction" and this pass agrees: Windows is `continue-on-error`, outside the tier-1 matrix, and no filesystem behavior is claimed. Rather than change the matrix, added an explicit owner-decision compliance note so a future reader cannot mistake a green Windows job for verified NTFS/exFAT behavior. |
| Action SHA table (checkout v4.4.0, rust-toolchain v1, rust-cache v2.9.1, install-action v2.85.5) | L1       | CONFIRM | L1 re-verified all four against `git ls-remote`. No change.                                                                                                                                                                                                                                                                            |
| "What remains unvalidated" should be repeated in a final summary                                 | L3       | REJECT  | The section already exists, is titled unambiguously, and states "This workflow has not been executed." A duplicate summary adds no information and risks the two copies drifting. L3 itself recorded "no change to the document required."                                                                                             |
| Matrix `[ubuntu-latest, macos-latest, macos-15-intel]` matches `ci.yml`                          | L1       | CONFIRM | Read `ci.yml`. No change.                                                                                                                                                                                                                                                                                                              |

**Not claimed:** this workflow still has never run in GitHub Actions. Nothing in this
pass changed that, and no statement here should be read as a passing CI run. The local
`just gate` result above is a local result only.
