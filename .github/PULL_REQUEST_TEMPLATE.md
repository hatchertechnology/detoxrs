<!--
Thanks for the PR. A few things that make review faster for this project:
-->

## What does this change?

<!-- One or two sentences. Link an issue with "Fixes #123" if there is one. -->

## Why?

<!-- The motivation, not just the mechanics of the diff. -->

## How was this tested?

<!--
If this touches filename handling: what filenames (including any
non-ASCII/control/bidi cases) did you test against, and on which OS and
filesystem? "I ran the existing tests" is fine if nothing new needed manual
testing.
-->

## Checklist

- [ ] `just gate` passes locally (`fmt-check`, `clippy`, `test`, `msrv`).
- [ ] Commit messages follow [Conventional
      Commits](https://www.conventionalcommits.org/en/v1.0.0/).
- [ ] Tests were added or updated for the behavior this changes, if any exists to test yet.
- [ ] If this adds `unsafe` code, the safety invariant is documented at the
      `#[allow(unsafe_code)]` site and this PR explains why it's needed (see
      `CONTRIBUTING.md`'s unsafe policy).
- [ ] If this adds a dependency, this PR explains why it's needed.
