# Agent instructions

## Formatting is part of finishing

After writing or editing files, check the ones you touched:

```
just fmt-check-file path/to/file.md ...     # verify
just fmt-file      path/to/file.md ...      # fix
```

A task is not done until that passes. Do not hand back work that leaves the
tree dirty — the next agent will inherit the failure and won't know whose it is.

Prefer the `-file` variants over the repo-wide `just fmt-check` / `just fmt`.
Other agents may be editing other files at the same time, and reformatting a
file you weren't asked to touch will clobber their work in progress.

If you do run the repo-wide check and it reports drift in a file you did **not**
touch, report that in your final message rather than fixing it.

## Keep the checkers current

`just fmt` / `just fmt-check` currently cover markdown only, via prettier.

When you introduce a file type the justfile does not yet check, add a checker
for it in the same change that introduces the file type — not later. Wire it
into both `fmt` (writes) and `fmt-check` (verifies, non-zero exit on drift), so
the two stay symmetrical.

Conventions for new checkers:

- Prefer the ecosystem-standard tool: `cargo fmt` for Rust, `taplo fmt` for
  TOML, `shfmt` for shell, `prettier` for anything it already handles (YAML,
  JSON, CSS, HTML).
- Prefer a tool invocable without a local install (`npx --yes`, `uvx`,
  `cargo fmt` from the toolchain) over anything requiring a new manifest or
  lockfile to maintain.
- Pin the major version so a tool release cannot silently reformat the repo.
- Don't add a checker for a file type that isn't in the repo yet.

## Commits

Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).
Formatting-only changes are `style:`.
