# Governance and security-policy pass

Written against `wiki/rust/ideal-project-setup`'s `governance.md`,
`security-policy.md`, `secure-development.md`, `local-dev.md`, and
`enterprise-readiness.md`. Scope: `SECURITY.md`, `CONTRIBUTING.md`,
`CODE_OF_CONDUCT.md`, `.github/CODEOWNERS`, `.github/ISSUE_TEMPLATE/**`,
`.github/PULL_REQUEST_TEMPLATE.md`. No workflow, `deny.toml`, `Cargo.toml`,
`crates/`, `README.md`, `AGENTS.md`, `LICENSE`, or justfile changes were made.

## Files created

- `SECURITY.md`
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `.github/CODEOWNERS`
- `.github/ISSUE_TEMPLATE/bug_report.yml`
- `.github/ISSUE_TEMPLATE/feature_request.yml`
- `.github/ISSUE_TEMPLATE/config.yml` (disables the blank-issue path from
  bypassing the security-report contact link; enables reporting a security
  vulnerability via GitHub Security Advisories instead of a form)
- `.github/PULL_REQUEST_TEMPLATE.md`

## Placeholders a human must fill in (do not treat as done)

1. **Security-response contact/timeline (`SECURITY.md`).** GitHub private
   vulnerability reporting is wired up as the real, working channel
   (`https://github.com/hatchertechnology/detoxrs/security/advisories/new`).
   The response-time table (`security-policy.md`'s acknowledgement/
   assessment/fix targets) is left as explicit placeholders — this project
   has no named security-response owner and no resourcing to promise a
   number honestly. A fallback email contact is also left as a placeholder;
   none was invented.
2. **Code of Conduct enforcement contact (`CODE_OF_CONDUCT.md`).** The
   standard Contributor Covenant 2.1 template is used verbatim per
   `governance.md`'s "don't customize it" guidance, but the enforcement
   contact section and the "who enforces this" section are both left as
   explicit placeholders. No email or individual's name was fabricated.
3. **Branch protection claim (`CONTRIBUTING.md`).** The PR-process section
   says branch protection on `main` is a GitHub setting this document
   cannot assert and flags it as something to verify/fill in — I did not
   check the live GitHub repository settings (no GitHub API access in this
   task), so I did not claim protection exists.
4. **Maintainer roster / GOVERNANCE.md.** `governance.md` recommends a
   `GOVERNANCE.md` "once the project has more than one active maintainer or
   intends to be a long-lived dependency," with named maintainer roles,
   decision-making process, a release owner, and a security-response owner.
   None of that exists yet and I did not invent it — `.github/CODEOWNERS`
   is a single-line catch-all to `@hatchertechnology` (the repo owner)
   rather than named per-area reviewers, because there is no maintainer
   team to name for release/dependency/unsafe-FFI review areas.
5. **SLA/compliance claims (`enterprise-readiness.md`).** Nothing in this
   pass makes an SSDF/compliance attestation, OpenSSF Scorecard/Badge claim,
   or SLA commitment — those require a real assessment this task can't
   perform, and `enterprise-readiness.md` itself says legal/compliance review
   is needed before claiming eligibility.

## License conflict and its concrete consequence for `CONTRIBUTING.md`

Per `docs/rust-setup-notes.md`, the repo `LICENSE` is BSD-3-Clause; the guide
(`governance.md`) recommends the Rust-ecosystem convention of dual
MIT/Apache-2.0. This agent did not relicense, did not add
`LICENSE-MIT`/`LICENSE-APACHE`, and did not touch `LICENSE`, per instructions.

Concrete effect on `CONTRIBUTING.md`'s "Licensing of contributions" section:
the guide's suggested text ("shall be dual-licensed as above") does not
apply. Today's `CONTRIBUTING.md` instead states contributions are licensed
under BSD-3-Clause, matching the actual `LICENSE` file, and calls out that
this is a deliberate departure from ecosystem convention with a pointer back
to this document. If the license conflict is later resolved by adding
`LICENSE-MIT`/`LICENSE-APACHE` and moving to dual licensing, both
`CONTRIBUTING.md`'s licensing section and its `LICENSE` link need to be
updated to match — they are not currently written to auto-adapt, and doing so
now would describe a licensing arrangement that doesn't exist.

## Project-specific threat model used for `SECURITY.md`

Rejected generic boilerplate ("read files outside expected paths," "execute
arbitrary code") as the primary framing because `detoxrs` is a batch filename
renamer with no network exposure and no plugin/scripting surface. Instead the
in-scope list was built from the actual safety architecture described in
`docs/research/00-proposal-rust-detox-successor.md`:

- **Data loss / unintended overwrite** — from §5.3/§5.4 (no-clobber renames,
  no `overwrite` mode, three-layer collision detection).
- **Path traversal / scope escape via a crafted filename** — from §5.2
  (basename-only renames, no cross-directory moves).
- **Symlink following** — from §5.6 (`lstat` always, no recursion into
  symlinked directories, no flag to re-enable it; cites the project's own
  citation of upstream `detox` issue #23 as the hazard evidence).
- **TOCTOU between plan and apply** — from §5.1 and §5.3 layer 3 (frozen
  snapshot before any I/O, kernel-level no-clobber as the last defense).
- **Terminal/output injection via hostile filenames** — from §3.12 (Trojan
  Source / CVE-2021-42574 bidi and invisible-character stripping) and §6.1
  (`<hh>`-escaped display of undecodable names so a terminal can't be driven
  by a filename).
- **Undo-journal integrity** — from §5.5 (replay refuses when the recorded
  `(dev, ino)` no longer matches).

This is intentionally a design-target threat model, not a verified-secure
claim: the application code implementing these guarantees does not exist yet
(per `docs/rust-setup-notes.md`, `main.rs` is a placeholder). `SECURITY.md`
says "designed to" throughout rather than asserting the properties hold
today, and the RustSec/`cargo audit` cross-reference is marked as
not-yet-wired-up rather than described as active.

## Guide recommendations deferred (with reason)

- **`GOVERNANCE.md`** — not created; `governance.md` scopes it to
  multi-maintainer or long-lived-dependency projects, which this single
  agent pass cannot establish. Flagged as a gap in `.github/CODEOWNERS` and
  above instead of writing an empty-role document.
- **Named CODEOWNER areas per `governance.md`** (release workflows,
  dependency policy, `unsafe`/FFI, security policy) — collapsed to one
  catch-all owner; no maintainer team exists to split these out to honestly.
- **`SUPPORT.md`** (`enterprise-readiness.md`'s table) — out of this agent's
  file scope (not listed among the paths I own) and would duplicate
  `SECURITY.md`'s supported-versions table with no additional information
  to add; not created.
- **Branch-protection/ruleset configuration** (`governance.md`) — this is a
  GitHub repository setting, not a file in this repo; not something this
  task can apply, and `CONTRIBUTING.md` says so explicitly rather than
  describing protection that may not be turned on.
- **Response-time commitments and a real security contact** — deliberately
  left as placeholders rather than invented, per the task's honesty rules.
- **Everything CI/supply-chain/release-related** (`cargo-deny`, `cargo-vet`,
  `cargo-audit`, Trivy, SBOM, provenance, release-please) — explicitly out of
  this agent's scope per the task brief (owned by other agents/humans);
  `CONTRIBUTING.md` and `SECURITY.md` both state plainly that these are not
  wired up yet rather than describing them as available checks.

## Verification performed

- `just --list` confirmed the only recipes referenced in `CONTRIBUTING.md`
  (`build`, `clippy`, `fmt`, `fmt-check`, `fmt-check-file`, `fmt-file`, `msrv`,
  `gate`, `test`) actually exist; no reference to `ci`, `audit`, `deny`,
  `trivy`, `vet`, or `geiger` was made, since those recipes don't exist.
- `just fmt-file` / `just fmt-check-file` run on every markdown file created
  (`SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `.github/PULL_REQUEST_TEMPLATE.md`) — passes after one `fmt-file` pass
  (prettier reformatted two tables/headings in `SECURITY.md` and
  `CONTRIBUTING.md`; `CODE_OF_CONDUCT.md` and the PR template were already
  clean). The repo-wide formatter was not run.
- `python3 -c "import yaml,sys; yaml.safe_load(...)"` run against all three
  YAML files under `.github/ISSUE_TEMPLATE/` — all parse.
- `.github/CODEOWNERS` checked by hand against GitHub's documented syntax
  (pattern, then space-separated `@user`/`@org/team` owners per line,
  `#`-comments): the file has one active rule, `* @hatchertechnology`,
  which is valid syntax and matches the stated repo owner. No automated
  CODEOWNERS linter was available in this environment, so this was a manual
  read against the documented grammar rather than a tool run.
