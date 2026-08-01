//! detoxrs: make filenames sane. See
//! `docs/research/00-proposal-rust-detox-successor.md`.
//!
//! Scaffolding only. None of the CLI, transform pipeline, or filesystem ops
//! described in proposal §7.1 (`cli.rs`, `config.rs`, `walk.rs`, `fsops.rs`,
//! `journal.rs`, `report.rs`) exist yet.
//!
//! TODO(proposal §7.1): add `cli`, `config`, `walk`, `fsops` (+ per-platform
//! submodules), `limits`, `journal`, and `report` modules as that logic is
//! written -- not as empty stubs ahead of time.
//!
//! Unsafe-code policy: `forbid`, same as `detoxrs-core`. An earlier version of
//! this crate used `deny` to leave room for a hand-written macOS `libc` FFI
//! shim, on the premise that neither `rustix` nor `nix` exposed
//! `renamex_np`/`RENAME_EXCL`. That premise was false and is withdrawn:
//! `rustix::fs::renameat_with` with `RenameFlags::NOREPLACE`/`EXCHANGE` wraps
//! `renameatx_np` under `#[cfg(apple)]` and `renameat2` under
//! `#[cfg(linux_kernel)]`, so both no-clobber rename paths are reachable from
//! safe code (proposal §5.4, §7.2). docs.rs hid the Apple items because its
//! default render target is Linux. With no FFI shim planned, there is no
//! future `#[allow(unsafe_code)]` to leave room for, and `forbid` -- which
//! cannot be downgraded by a later `allow` -- is the honest attribute.
#![forbid(unsafe_code)]

fn main() {
    println!(
        "detoxrs {} -- not yet implemented",
        detoxrs_core::placeholder_version()
    );
}
