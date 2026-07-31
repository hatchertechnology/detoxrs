# Rust project setup notes: release automation

Release-only pass. Written against `wiki/rust/ideal-project-setup/releases.md`
(primary source), `README.md`'s non-negotiable release baseline,
`ci-cd.md`'s release section, and `enterprise-readiness.md`'s release
evidence packet, cross-checked against
`docs/research/00-proposal-rust-detox-successor.md` sections 6.4, 7.1, and
9.4. No release has shipped: version is `0.1.0`, `main.rs` is a placeholder.
Everything here is machinery for a future release, not a release itself.

## Files added (this agent's scope only)

- `release-please-config.json`
- `.release-please-manifest.json`
- `.github/workflows/release.yml`
- `CHANGELOG.md` (seeded, no fabricated history)
- this file

Did not touch: `justfile`, `Cargo.toml`, anything under `crates/`,
`README.md`, `ci.yml`, `dependabot.yml`, `deny.toml`, `supply-chain/**`.

## The tooling conflict: release-please vs. cargo-dist/dist + release-plz

The guide (`releases.md`) prescribes `release-please`. The project's own
design proposal (section 9.4) instead chose `cargo-dist` (rebranded `dist`,
still installed as the `cargo-dist` crate because the bare `dist` name is
squatted) plus `release-plz`, specifically for packaging reasons: prebuilt
static binaries for x86_64/aarch64 across linux-musl, linux-gnu, macOS, and
Windows; a Homebrew tap; Nix and AUR; Debian/Fedora later once the dependency
tree is small enough for debcargo. This is not a stylistic disagreement --
the two toolchains solve overlapping but different problems.

**What release-please does well here:** it is purely a _version and
changelog_ engine driven by Conventional Commits, which the repo already
requires (`AGENTS.md`). It answers "what version is this and what changed,"
and gates every release behind a human-reviewed, human-merged release PR --
the "explicit, accountable approval" the guide's opening paragraph asks for.
It does nothing about cross-compiling or packaging; that has to be hand-built
in a separate workflow (what `release.yml` in this repo now does).

**What cargo-dist/release-plz would do well here:** `cargo-dist` _is_ a
cross-platform packaging engine purpose-built for exactly proposal 9.4's
target list -- it generates the GitHub Actions build matrix, the archive
naming convention, checksums, an installer script, and (as of recent
versions) can emit SBOM and provenance/attestation steps itself, rather than
those being hand-assembled YAML. `release-plz` overlaps with release-please
(version bump + changelog from Conventional Commits, opens a release PR) but
is Rust/Cargo-native: it understands `[workspace.package]` version
inheritance and `Cargo.lock` directly instead of parsing `Cargo.toml` as a
generic-language plugin the way release-please's "rust" strategy does. Using
`cargo-dist`, the ~200-line `build` job in `release.yml` below would likely
collapse to a generated config file plus a few lines of glue.

**Cost of switching later:** moderate, not high, _if_ done before the first
real release. `release-please-config.json` / `.release-please-manifest.json`
would be deleted and replaced by `release-plz.toml`; `release.yml`'s
`release-please` job would be replaced by a `release-plz` job; the `build`,
`checksums-and-provenance` jobs would either be replaced by `cargo-dist`'s
generated workflow or kept and re-pointed at `release-plz`'s tag output --
the gating logic (`release_created` equivalent) carries over conceptually.
`CHANGELOG.md`'s seed content is tool-agnostic. The switch gets expensive
only _after_ real releases exist, because at that point there is a version
history, existing tags, and possibly published Homebrew/Nix artifacts whose
naming and update mechanism would need to migrate too -- exactly why this
decision is better made now, before v0.1 ships, than retrofitted.

**Recommendation for a human:** the guide's `release-please` is what got
implemented below, per this task's brief, and it works for the "is this
release accountable and versioned correctly" question today with zero new
external tooling risk. But proposal 9.4 is the more specific, more
load-bearing source for this project -- it was written with this project's
actual packaging destinations in mind (Homebrew tap before detox's
2027-07-28 Homebrew disable date, Nix, AUR, Debian/Fedora once the dependency
budget allows). **I recommend the human decide to switch to
`release-plz` + `cargo-dist` before the v1.0 packaging milestone**
(proposal roadmap section 10, "packaging items 1-4 from 9.4"), and keep
`release-please` only as the interim mechanism that produces the first few
`0.x` version bumps while the CLI itself is being built. Do not run both;
they will fight over `Cargo.toml`'s version field and both want to own the
release PR.

## release-please configuration for the workspace

`Cargo.toml` already declares `[workspace.package] version = "0.1.0"`, and
both `crates/detoxrs/Cargo.toml` and `crates/detoxrs-core/Cargo.toml` use
`version.workspace = true`. That means the two crates are **already
version-locked** at the Cargo level -- there is exactly one version number in
this workspace, not two.

Given that, and given proposal 7.1's explicit statement that
`detoxrs-core` "is not published as a general-purpose crate in v1.0; the
binary is the product," `release-please-config.json` declares a **single**
package at `"."` (`release-type: "rust"`, `package-name: "detoxrs"`). It does
**not** add a second `packages` entry for `crates/detoxrs-core`. Adding one
would tell release-please to track that crate as an independently versioned,
independently changelogged component, which contradicts both the existing
Cargo-level version lock and the proposal's decision. If a human later
decides to publish `detoxrs-core` as a standalone crate with its own
versioning, that is the point to add a second `packages` entry (and probably
move to `release-plz`, which is more workspace-aware -- see the conflict
section above).

`bump-minor-pre-major: true` is set because the proposal's roadmap
(section 10) plans v0.1 -> v0.2 -> v0.3 -> v1.0 with breaking config/CLI
changes expected along the way pre-1.0; this keeps a `feat!:`/
`BREAKING CHANGE:` commit bumping minor (`0.x.0`) instead of jumping to
`1.0.0` by surprise before the project is actually ready to claim stability.

**Unverified, flagged rather than guessed:** whether release-please's `rust`
strategy correctly patches a `[workspace.package] version` key (as opposed to
a plain `[package] version`) has changed across release-please's history and
I could not run it to confirm current behavior for this exact layout. The
first real release PR release-please opens must be inspected by a human
before merge to confirm it bumped the right field in `Cargo.toml` and that
`Cargo.lock` is left for the workflow's `cargo update --workspace --locked`
step to reconcile (this is standard release-please behavior per
`releases.md`, not specific to this repo).

## The release workflow (`.github/workflows/release.yml`)

Three jobs:

1. **`release-please`** -- runs on every push to `main`. Only ever opens or
   updates the standing release PR _or_, when that PR is merged, creates the
   tag/GitHub Release. Guarded with
   `if: github.repository == 'hatchertechnology/detoxrs'` so a fork can't
   spend App-token secrets. Uses a GitHub App token
   (`actions/create-github-app-token`), not the default `GITHUB_TOKEN`,
   because the default token cannot trigger the downstream `build` job on
   the tag it creates -- this is `releases.md`'s stated reason and applies
   unchanged here.
2. **`build`** -- matrix job, gated on
   `needs.release-please.outputs.release_created == 'true'` (see "Safety"
   below), runs under the `release` GitHub Environment.
3. **`checksums-and-provenance`** -- downloads every built archive, produces
   `SHA256SUMS`, attests build provenance, uploads both to the GitHub
   Release. Also gated the same way, same environment.

No fourth job publishes to crates.io. See "cargo publish posture" below.

### Targets: what's included, what's excluded, and why

Reconciled the guide's example matrix (4 targets: linux-gnu x2, macOS
aarch64 only, Windows x64) against proposal 9.4's packaging list (x86_64 and
aarch64 for linux-musl, linux-gnu, macOS, Windows) and proposal 6.4's
platform tiers (Linux + macOS are tier 1; Windows is best-effort only;
FreeBSD/NetBSD/OpenBSD are compile-checked, not release targets).

| Target                        | Runner                      | Included? | Why                                                                                                                                                                            |
| ----------------------------- | --------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `x86_64-unknown-linux-gnu`    | `ubuntu-latest`             | Yes       | Tier 1, native build                                                                                                                                                           |
| `aarch64-unknown-linux-gnu`   | `ubuntu-24.04-arm`          | Yes       | Tier 1, native build (GitHub-hosted arm64 runner)                                                                                                                              |
| `x86_64-unknown-linux-musl`   | `ubuntu-latest` via `cross` | Yes       | In proposal 9.4's packaging list; no native musl cross-toolchain on stock runner, so built with `cross` (Docker-based)                                                         |
| `aarch64-unknown-linux-musl`  | `ubuntu-latest` via `cross` | Yes       | Same as above, plus QEMU emulation under `cross` since the runner is x86_64                                                                                                    |
| `aarch64-apple-darwin`        | `macos-14`                  | Yes       | Tier 1, native (Apple Silicon runner)                                                                                                                                          |
| `x86_64-apple-darwin`         | `macos-14`                  | Yes       | Tier 1, cross-compiled from the same Apple Silicon runner (Xcode toolchain supports both Apple archs from one host)                                                            |
| `x86_64-pc-windows-msvc`      | `windows-latest`            | Yes       | Explicitly in proposal 9.4's list; Windows is best-effort (6.4), not tier 1, but still requested                                                                               |
| `aarch64-pc-windows-msvc`     | --                          | **No**    | Not in proposal 9.4's packaging list or 6.4's tiers. Windows itself is already best-effort; adding an architecture nobody asked for would be new, unrequested scope            |
| FreeBSD/NetBSD/OpenBSD        | --                          | **No**    | Proposal 6.4 calls these "compile-checked, community-supported" explicitly, not release-artifact targets. That's `ci.yml`'s job (another agent's scope), not a release job     |
| Debian/Fedora native packages | --                          | **No**    | Proposal 9.4 explicitly orders these last, "post-1.0, once the dep tree is stable" (roadmap section 10). Not release-workflow scope yet                                        |
| Homebrew tap / Nix / AUR      | --                          | **No**    | Proposal 9.4 items 2-4. Downstream of the GitHub Release existing at all; separate future work, likely alongside a `cargo-dist`/`release-plz` migration (see conflict section) |

**Known risk I could not resolve without running it:** `cargo-auditable`
(which the guide recommends to embed the dependency tree in the binary) is
only wired up for the five natively-built targets. Whether `cargo auditable
build` composes cleanly with `cross`'s Docker-wrapped `cargo` invocation for
the two musl targets is untested; the workflow currently skips
`cargo-auditable` for those two and builds them with plain `cross build`.
This is marked with a `TODO`/comment in the workflow itself.

### Provenance/signing choice: GitHub artifact attestations (SLSA-style), not Sigstore/cosign

Chose `actions/attest-build-provenance` over standalone Sigstore/`cosign`
signing because:

- It requires no key management at all -- it uses GitHub's OIDC identity
  (`id-token: write`) and Sigstore's public transparency log under the hood,
  but the repo never handles a signing key or a Sigstore `cosign` identity
  directly.
- Verification is a single `gh attestation verify` command a user already
  has if they have the GitHub CLI, rather than requiring `cosign` as an
  extra install.
- It is the mechanism the guide itself documents in `releases.md`
  ("provenance -- not a guarantee that the artifact has no vulnerability"),
  so following it introduces no new tool the guide didn't already vet.

This is provenance, not a cryptographic signature over the binary itself
that survives outside GitHub's attestation store -- a real limitation if a
future requirement is "verify this binary with no dependency on GitHub being
reachable or trusted." If that requirement appears, `cosign` keyless signing
(also OIDC-based, no long-lived key) is the natural upgrade; it wasn't
chosen now because the guide's own worked example is the attestation action,
and introducing a second signing mechanism without a stated need would be
unrequested scope.

### Verification procedure (user-facing; belongs in README, not written there this round)

Exact commands a skeptical downloader should run once a real release exists:

```bash
# 1. Download the archive and the checksum manifest from the GitHub Release.
curl -LO https://github.com/hatchertechnology/detoxrs/releases/download/vX.Y.Z/detoxrs-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz
curl -LO https://github.com/hatchertechnology/detoxrs/releases/download/vX.Y.Z/SHA256SUMS

# 2. Verify the checksum.
sha256sum --ignore-missing --check SHA256SUMS

# 3. Verify build provenance (requires the GitHub CLI, `gh`).
gh attestation verify detoxrs-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz \
  --owner hatchertechnology \
  --signer-workflow hatchertechnology/detoxrs/.github/workflows/release.yml
```

This is not yet runnable: no release exists, so there is nothing at those
URLs and no attestation to verify. The commands themselves match
`releases.md`'s documented pattern and `actions/attest-build-provenance`'s
current verification flow. **This is the content a human should paste into
`README.md`'s verification section once the first release actually ships**
-- not written into `README.md` in this pass, since that file isn't this
agent's to edit.

### What I need from agent C (SBOM)

`release.yml`'s `checksums-and-provenance` job has an explicit `TODO` block
(not a broken step -- deliberately left as a comment so the workflow stays
syntactically valid and doesn't fail on a missing artifact) asking for:

1. A CycloneDX JSON SBOM generated with `cargo cyclonedx --format json
--describe binaries` (or equivalent) against the _release build's_ lock
   file/feature set -- not a report that includes dev-only dependencies.
2. Produced as a GitHub Actions workflow artifact (or callable step) with a
   stable, documented name I can reference in a `download-artifact` step.
3. A decision from agent C on granularity: one SBOM per target, or one SBOM
   for the whole release. Whichever it is, this job's copy step and the
   verification docs above need to name it accordingly.

Until that lands, releases will ship checksums and provenance but no SBOM --
a real gap against the guide's non-negotiable baseline
(`README.md`: "a published SBOM, release provenance or signatures,
checksums, and a documented way to verify them"). Flagging it rather than
faking a step that references an artifact that doesn't exist yet.

### `cargo publish` posture: not wired up, deliberately

No `publish-crate` job exists in `release.yml`, and no `CARGO_REGISTRY_TOKEN`
secret is referenced anywhere. Decision, per proposal 7.1 ("the binary is
the product") and 9.4 (crates.io publish is not in the packaging order at
all -- GitHub Releases, Homebrew, Nix, AUR, then Debian/Fedora): **do not
publish to crates.io in v1.0.** `detoxrs-core`'s `[dependencies]` entry in
`crates/detoxrs/Cargo.toml` already uses a path dependency
(`{ path = "../detoxrs-core", version = "0.1.0" }`), which works fine without
either crate ever touching crates.io. If a human later wants `detoxrs-core`
publishable as a general-purpose crate (proposal 7.1 explicitly defers this
to "post-1.0"), that's a new decision with its own security posture
(publish credentials, OIDC trusted publishing) -- not something to
half-build now.

### Safety: why this cannot fire on a plain push to `main`

- The workflow trigger is `on: push: branches: [main]`, which the guide's
  own example uses too -- but the _only_ unconditional job is
  `release-please`, and running it does not tag, build, or publish anything
  by itself. It either updates a standing PR (no side effects outside that
  PR) or, on the one push event that is itself a release-PR merge, sets
  `release_created=true` and creates the tag/GitHub Release that
  `release-please` itself is designed to gate on human PR review+merge.
- `build` and `checksums-and-provenance` are both gated on
  `needs.release-please.outputs.release_created == 'true'`, which is false
  for every ordinary commit/merge to `main`. A feature branch merging to
  `main` never sets this.
- Both of those jobs additionally run under the `release` GitHub
  Environment. **That environment does not exist in this repository yet** --
  I cannot create GitHub Environments from files in the repo; a repo admin
  must create it via Settings -> Environments and should add required
  reviewers there, which becomes the second, independent human gate the
  enterprise-readiness guide asks for ("a protected release environment,
  restrict who may approve that environment").
- The `release-please` job is scoped to `github.repository ==
'hatchertechnology/detoxrs'` so a fork cannot spend this repo's secrets
  even if it copies the workflow file verbatim.
- No secrets exist yet for any of this to actually run: `secrets.RELEASE_APP_ID`
  and `secrets.RELEASE_APP_PRIVATE_KEY` (for the GitHub App token) are
  referenced but not created. Missing secrets make `create-github-app-token`
  fail closed -- the safe failure mode -- rather than silently falling back
  to a less-privileged token.

## Verification performed / not performed

- `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))"` on
  `.github/workflows/release.yml` -- **passes** (valid YAML).
- `python3 -m json.tool` on `release-please-config.json` and
  `.release-please-manifest.json` -- **both pass**.
- Every third-party action SHA pinned in `release.yml` was resolved two
  independent ways: the GitHub REST API (`commits/<ref>`) and
  `git ls-remote --tags` against the public repo (peeling annotated tags
  with the `^{}` suffix where present), and both agreed after I caught and
  fixed one transcription error (an off-by-one-character SHA for
  `actions/upload-artifact@v4.6.2`, corrected to
  `ea165f8d65b6e75b540449e92b4886f43607fa02`):
  - `actions/checkout@v4` -> `11d5960a326750d5838078e36cf38b85af677262` (lightweight tag)
  - `actions/create-github-app-token@v1` -> `d72941d797fd3113feb6b93fd0dec494b13a2547`
  - `googleapis/release-please-action@v5.0.0` -> `45996ed1f6d02564a971a2fa1b5860e934307cf7`
  - `actions/attest-build-provenance@v2` (= v2.4.0) -> `e8998f949152b193b063cb0ec769d69d929409be` (annotated tag, peeled)
  - `Swatinem/rust-cache@v2` -> `e18b497796c12c097a38f9edb9d0641fb99eee32` (annotated tag, peeled)
  - `taiki-e/install-action@v2` -> `6a1bd70eaac3c8bdf093356838d7ee09fda951cf`
  - `actions/upload-artifact@v4` (= v4.6.2) -> `ea165f8d65b6e75b540449e92b4886f43607fa02`
  - `actions/download-artifact@v4` (= v4.3.0) -> `d3f86a106a0bac45b974a628896c90dbdf5c8093`
- **Guide staleness found:** `releases.md`'s worked example pins
  `googleapis/release-please-action@v4`. Current upstream latest is
  **v5.0.0** (published 2026-04-22; v4's line stopped at v4.4.1,
  2026-04-13). v5 is primarily a Node 24 runtime bump per its release notes;
  no breaking config-schema change was found for `release-please-config.json`
  itself. Used v5 rather than following the guide's stale v4 pin, and
  reported this rather than silently reproducing the guide's version.
- `just fmt-check-file docs/rust-setup-release.md CHANGELOG.md` -- **passes**
  (prettier, via the existing recipe; repo-wide `fmt`/`fmt-check` was not run,
  since other agents are concurrently editing files outside this agent's
  scope).
- **Not done, and cannot be done without running the workflow for real:**
  the workflow has never executed. No push to `main` in this repo has
  triggered it. I did not, and do not, claim it works. What would validate
  it: (1) a real push to `main` to confirm the `release-please` job opens a
  release PR with the changelog/version bump it should; (2) merging that PR
  once the CLI has real commits, to confirm `release_created` flips to
  `true` and the `build` matrix actually produces seven working archives
  (musl targets under `cross` are the highest-risk step, per the
  `cargo-auditable` caveat above); (3) manually creating the `release`
  GitHub Environment and the `RELEASE_APP_ID`/`RELEASE_APP_PRIVATE_KEY`
  secrets before any of this can run at all; (4) running the verification
  commands above against a real released archive once one exists.

## Deferred / needs a human

- Create the GitHub App used for release-please's token, and add
  `RELEASE_APP_ID` / `RELEASE_APP_PRIVATE_KEY` as repository secrets.
  Scope it to contents + pull-requests write only.
- Create the `release` GitHub Environment (Settings -> Environments) with
  required reviewers, per the guide's "protected release environment"
  requirement. Nothing in `build`/`checksums-and-provenance` can run without
  it existing.
- Decide, before the v1.0 packaging milestone, whether to switch to
  `release-plz` + `cargo-dist` per proposal 9.4 (see the conflict section
  above) -- this is a real decision, not busywork; recommendation given.
- Wire in agent C's SBOM artifact (see "What I need from agent C" above).
- Once a release actually exists, paste the verification commands above into
  `README.md`'s release/verification section (not done this round; not this
  agent's file).
- Justfile recipes another agent should add (not added here -- not this
  agent's file to edit):
  - `release-dry-run`: something like
    `npx release-please release-pr --repo-url=... --dry-run --token=...`
    or the `release-plz` equivalent, so a maintainer can preview the next
    version bump/changelog locally before pushing.
  - `checksums`: `sha256sum target/release/detoxrs > SHA256SUMS` for local
    sanity-checking before trusting CI's output.
  - Whatever recipe wraps agent C's local SBOM generation
    (`cargo cyclonedx ...`), so `just gate`/`just ci` can eventually include
    it once real dependencies and a real release exist.
- Homebrew tap, Nix flake, AUR package (proposal 9.4 items 2-4): all
  downstream of a first real GitHub Release existing; not started.
