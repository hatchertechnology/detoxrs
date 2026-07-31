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
//! Unsafe-code policy: `deny`, not `forbid`. `detoxrs-core` is pure and
//! forbids `unsafe` outright, but this binary crate will eventually contain
//! a hand-written macOS `libc` FFI shim (proposal §5.4, §7.1
//! `fsops/macos.rs`: `renamex_np` + a `getattrlist`/`VOL_CAP_INT_RENAME_EXCL`
//! probe, because neither `rustix` nor `nix` expose it). `forbid` cannot be
//! downgraded by any later `#[allow(unsafe_code)]`, however narrow and
//! well-justified -- it would have to be deleted wholesale the day that shim
//! lands. `deny` gets the same "unsafe requires a deliberate, reviewed
//! opt-in" default today, without that future flag-day: when `fsops/macos.rs`
//! is written, it gets a module- or function-scoped
//! `#[allow(unsafe_code)]` with a comment naming the syscall and the safety
//! argument, and every other module keeps rejecting `unsafe` by default.
#![deny(unsafe_code)]

fn main() {
    println!(
        "detoxrs {} -- not yet implemented",
        detoxrs_core::placeholder_version()
    );
}
