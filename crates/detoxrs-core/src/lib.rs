//! Pure, I/O-free filename-transform logic for detoxrs.
//!
//! This crate deliberately has no CLI parsing, no filesystem access, and no
//! `std::fs`: that split is what lets the transform pipeline be
//! property-tested against arbitrary strings without touching a filesystem.
//! See `docs/research/00-proposal-rust-detox-successor.md` §3 (transform
//! model) and §7.1 (module layout).
//!
//! Implemented so far (plan §7.1, M1 work packages 1-4): `policy`, `decode`,
//! `classes`, `invisible`, `truncate`, `pipeline`, `plan`.
//!
//! Later milestones add `percent` (M2), `rules` (M4) and `reserved` (M5). They
//! are not stubbed out here as empty files; add each one when its logic is
//! written.
//!
//! Never `unsafe`: this crate is pure by design, so `unsafe_code` is
//! `forbid`den outright (not just `deny`d) -- there is no future FFI need
//! here the way there is in the `detoxrs` binary crate.
#![forbid(unsafe_code)]

pub mod classes;
pub mod decode;
pub mod invisible;
pub mod pipeline;
pub mod plan;
pub mod policy;
pub mod truncate;

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
