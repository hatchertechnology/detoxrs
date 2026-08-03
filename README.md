# detoxrs

Tames problematic filenames.

Point it at a file or a directory tree and get sane, unix-safe names back:
spaces and shell metacharacters replaced, invisible and bidirectional control
characters stripped, overlong names truncated on grapheme boundaries. Preview
by default; nothing is renamed until you ask.

> **Status: M1 complete.** Preview, `-x` to apply, the undo journal and `undo`
> all work end to end, on both real filesystems detox users care about (ext4,
> APFS). This is not yet a finished tool: an independent adversarial review of
> M1 found fifteen defects (several high-severity) that are being worked
> through, and later milestones (`docs/plans/unified-draft-plan.md`) still owe
> percent-escape decoding, config-file rules, reserved-name handling and a
> `--separator` flag. Treat it as a working preview, not a hardened release.

## Usage

```sh
detoxrs path...          # preview only; nothing is renamed
detoxrs -x path...       # rename, and write an undo journal
detoxrs undo <batch-id>  # revert one batch
detoxrs undo --last      # revert the most recent batch
```

Add `-r` to recurse into directories and `--json` for a machine-readable
report. Run `detoxrs --help` for the full flag list.

## Acknowledgments

The idea for this project is not ours. It comes from
[**detox**](https://github.com/dharple/detox) by **Doug Harple**, first
released in 2004 and maintained for over twenty years — the tool that
established that "filenames are a mess and something should just fix them" is a
problem worth a dedicated utility. Its problem framing, its vocabulary, and two
decades of user reports on its issue tracker are what made this design possible.
`detoxrs` exists because `detox` was a good idea, and because its author
archived it on 2026-07-12 with the project needing a rewrite he did not have
time for.

Credit where it is due: if you have been using `detox` and it worked, that was
Doug Harple's work, not ours.

`detoxrs` is an **independent implementation**, not a fork or a port. No
upstream code, character table, or `.tbl` data has been copied. What we took was
the concept and the accumulated evidence of what users needed — both cited
throughout [`docs/research/`](./docs/research/), which reads the upstream C
source closely and links every claim to it. `detox` is BSD-3-Clause; since none
of its code is present here, that license imposes no obligation on this
repository. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for why keeping it that
way matters.

This is also not a drop-in replacement. The defaults differ deliberately — most
visibly, `detoxrs` previews by default and renames only when asked, where
`detox` renames unless you pass `-n`. The binary is deliberately not named
`detox`, so it cannot silently change the behavior of anyone's existing scripts.

## License

Dual-licensed under either of

- [MIT license](./LICENSE-MIT)
- [Apache License, Version 2.0](./LICENSE-APACHE)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this work by you, as defined in the Apache-2.0 license, shall
be dual-licensed as above, without any additional terms or conditions.
