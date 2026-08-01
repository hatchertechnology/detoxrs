# Security Policy

`detoxrs` renames files on your filesystem based on their existing names. It
does not open a network port, does not run as a privileged service, and does
not execute file contents. The realistic security surface is data loss,
path/symlink handling during a batch rename, and hostile filenames used as an
injection vector (into a terminal, a shell, or a script that consumes
`detoxrs`'s output) — not remote code execution. This policy is scoped to that
surface; see [Scope](#scope) below.

## Reporting a vulnerability

**Please do not report security vulnerabilities through public GitHub
issues.**

Preferred channel: [GitHub private vulnerability
reporting](https://github.com/hatchertechnology/detoxrs/security/advisories/new)
for this repository. It keeps the report, the fix discussion, and the
resulting advisory in one place.

> **Placeholder — human decision needed:** this project does not currently
> publish a security-contact email address. If GitHub private reporting is
> unavailable to a reporter, an alternate contact channel (a role-account
> email, not an individual's) needs to be decided by a maintainer and added
> here. Do not add one until it is real.

A useful report includes:

- The `detoxrs` version or commit affected.
- The operating system and filesystem (e.g. macOS/APFS, Linux/ext4).
- The exact command line, including flags.
- The smallest filename or directory layout that reproduces the issue. If the
  filename contains non-printable, bidirectional, or otherwise unusual
  characters, include it as a hex/escaped dump (e.g. `xxd` output or a
  `printf '%b'` string) as well as the raw bytes, since the raw filename may
  not survive copy-paste intact.
- What you expected to happen versus what happened (e.g. a file was renamed
  unexpectedly, deleted, overwritten, or a symlink target was affected).

## Supported versions

| Version        | Supported                  |
| -------------- | -------------------------- |
| Latest release | Yes                        |
| Anything older | No — upgrade to the latest |

`detoxrs` has not reached a `1.0` release yet. Before `1.0`, only the most
recent published release receives fixes. This table will be revisited once a
real support window is defined (see the placeholder in
[What to expect](#what-to-expect)).

## What to expect

> **Placeholder — human decision needed:** the response-time and fix-timeline
> commitments below are not yet a real commitment; this project has no
> dedicated security-response owner today (see `docs/rust-setup-governance.md`
> and `governance.md`'s "security-response owner" requirement). Do not treat
> the table below as a promise until a maintainer confirms it can actually be
> met, or replace it with honest, currently-achievable numbers.

| Stage                              | Target                            |
| ---------------------------------- | --------------------------------- |
| Acknowledgement of your report     | _placeholder — not yet committed_ |
| Initial assessment and severity    | _placeholder — not yet committed_ |
| Fix released, or a plan with dates | _placeholder — not yet committed_ |

## Disclosure

We intend to follow coordinated disclosure: once a fix is available, publish
a [GitHub Security
Advisory](https://github.com/hatchertechnology/detoxrs/security/advisories)
on this repository. `detoxrs` is not currently published to crates.io (see
`docs/research/00-proposal-rust-detox-successor.md` §7.1); if that changes,
advisories affecting a published crate should also be submitted to
[RustSec](https://github.com/rustsec/advisory-db), since that is the database
[`cargo audit`](https://github.com/rustsec/rustsec) reads and RustSec entries
are in turn imported into the GitHub Advisory Database.

We will credit reporters by name only with explicit permission. Say nothing
and the advisory stays anonymous. Please give us a chance to ship a fix before
disclosing publicly.

## Threat model and scope

This is specific to what `detoxrs` actually does — a batch filename renamer —
rather than a generic template. It is derived from
`docs/research/00-proposal-rust-detox-successor.md` §3.12, §5, and §6.1;
consult those sections for the full design rationale.

**In scope:**

- **Data loss or unintended overwrite during a rename batch.** `detoxrs` is
  designed to never clobber an existing file (no-clobber renames, collision
  detection before any I/O, no `overwrite` mode — proposal §5.3, §5.4). A way
  to make it delete, truncate, or silently overwrite a file the user did not
  intend to touch is a security bug here, not just a correctness bug.
- **Path traversal or scope escape via a crafted filename.** `detoxrs` only
  ever changes a basename and never moves a file between directories
  (proposal §5.2), which is meant to make directory traversal through a
  filename structurally impossible. A filename (however encoded, however many
  `..` or path-separator-like characters it contains once decoded) that
  causes a write outside the directory it was found in is in scope.
- **Symlink following during recursion.** `detoxrs` is designed to always
  `lstat`/`symlink_metadata` rather than `stat`, to rename a symlink's own
  directory entry without touching its target, and to never recurse into a
  symlinked directory, with no flag to enable that (proposal §5.6). A path
  through which `detoxrs` follows a symlink into a directory outside the
  scanned tree — the failure mode documented against upstream `detox` in
  issue #23 — is in scope, as is any way to make it write through a symlink
  to affect the target file's contents instead of only the directory entry.
- **TOCTOU between planning and renaming.** `detoxrs` splits work into a
  read-only planning phase and a separate apply phase (proposal §5.1) and uses
  no-clobber, kernel-level rename primitives as the last line of defense
  against a collision introduced between the two (proposal §5.3 layer 3). A
  way to defeat that — for example, getting `detoxrs` to rename onto a path
  that changed identity after planning, or to follow a race to clobber an
  unrelated file — is in scope.
- **Terminal/output injection via a hostile filename.** Filenames are
  attacker-controlled strings from the perspective of anything that later
  displays or greps `detoxrs`'s output (a terminal, a log, a script consuming
  `--plan`/journal output). Bidirectional-override and invisible/tag
  characters (CVE-2021-42574-style "Trojan Source" tricks) and raw control
  bytes are meant to be stripped from filenames by default (proposal §3.12),
  and undecodable names are meant to be displayed with `<hh>` escapes rather
  than raw bytes so a terminal cannot be driven by a filename (proposal
  §6.1). A filename that reaches a terminal, log, or downstream tool without
  that escaping — including through the undo journal or a `--plan` file — is
  in scope.
- **Undo-journal integrity.** The undo journal (proposal §5.5) is meant to
  refuse to replay an item whose current name no longer resolves to the
  recorded `(dev, ino)`, rather than force it. A way to make `undo` act on
  the wrong file, or a journal entry that leaks or corrupts data it should
  not, is in scope.

**Explicitly out of scope for this policy** (report as an ordinary issue
instead, since these are not confidential):

- Vulnerabilities in third-party dependencies — tracked separately by the
  dependency scanning that is already wired into CI: `cargo audit` and
  `cargo deny check` (policy in `deny.toml`) plus `trivy` and `cargo geiger`
  run in `.github/workflows/security.yml`, `cargo vet` against the audit set in
  `supply-chain/`. If you spot one those have missed, an ordinary issue is
  fine.
- Behavior that is merely surprising or a UX papercut without a
  data-loss/escape/injection consequence — file those as regular bugs.
- Denial of service against your own machine by pointing `detoxrs` at an
  adversarial tree you control (e.g. deep symlink loops in a directory you
  own) — that is a robustness bug, not a vulnerability, unless it also causes
  data loss or scope escape.
- Anything that requires `detoxrs` to already run with elevated/unexpected
  privileges it was not granted (`detoxrs` has no privilege-escalation
  design; it runs as the invoking user and touches only what that user can
  already rename).

`detoxrs` has `unsafe_code = "forbid"` in **both** crates, and no `unsafe` code
exists in the codebase. No FFI shim is planned: the no-clobber rename syscalls
this tool needs (`renameat2` on Linux, `renameatx_np` on macOS) are reachable
from safe code through `rustix::fs::renameat_with` (proposal §5.4, §7.2;
correction recorded in `docs/rust-setup-notes.md`). Introducing `unsafe` would
therefore mean deliberately relaxing a `forbid` attribute, which is a
reviewable change in its own right — and if it ever happens, the safety
argument and the syscalls wrapped become part of this project's
security-relevant surface, per `secure-development.md`'s FFI policy guidance.
