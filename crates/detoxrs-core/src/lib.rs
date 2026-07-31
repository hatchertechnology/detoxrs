//! Pure, I/O-free filename-transform logic for detoxrs.
//!
//! This crate deliberately has no CLI parsing, no filesystem access, and no
//! `std::fs`: that split is what lets the transform pipeline be
//! property-tested against arbitrary strings without touching a filesystem.
//! See `docs/research/00-proposal-rust-detox-successor.md` §3 (transform
//! model) and §7.1 (module layout).
//!
//! Nothing here is implemented yet -- this is scaffolding, not the pipeline.
//!
//! TODO(proposal §7.1, §3): this crate will eventually hold `policy`,
//! `decode`, `percent`, `classes`, `invisible`, `rules`, `pipeline`,
//! `truncate`, `reserved`, and `plan` modules. They are not stubbed out here
//! as empty files; add each one when its logic is actually written.
//!
//! Never `unsafe`: this crate is pure by design, so `unsafe_code` is
//! `forbid`den outright (not just `deny`d) -- there is no future FFI need
//! here the way there is in the `detoxrs` binary crate.
#![forbid(unsafe_code)]

/// Returns this crate's own version.
///
/// Placeholder so `detoxrs`'s `main` has something honest to print before
/// any transform logic exists. Delete once the pipeline (proposal §3) needs
/// a real public API.
#[must_use]
pub const fn placeholder_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_version_is_not_empty() {
        assert!(!placeholder_version().is_empty());
    }
}
