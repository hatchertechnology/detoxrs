//! Decode snapshot over the whole corpus (plan §7.1 WP1).
//!
//! One reviewable table of `id`, decode outcome, `<hh>`-escaped rendering and
//! the `disk_constructible_everywhere` flag. Any change to what decode does, or
//! to the corpus, shows up as a diff rather than as a silent behavior change.

mod support;

use detoxrs_core::decode::{Decoded, decode};
use std::fmt::Write as _;

#[cfg(unix)]
#[test]
fn decode_corpus_snapshot() {
    let mut out = String::new();
    for e in support::corpus::all() {
        let os = support::os_string_from_bytes(&e.bytes);
        let outcome = match decode(&os) {
            Decoded::Utf8(_) => "Utf8",
            Decoded::Opaque => "Opaque",
        };
        let disk = if e.disk_constructible_everywhere {
            "disk:yes"
        } else {
            "disk:no "
        };
        // Writing to a String is infallible; the Result exists only for the trait.
        let _ = writeln!(
            out,
            "{:<20} {:<7} {disk} {}",
            e.id,
            outcome,
            support::escape_bytes(&e.bytes)
        );
    }
    insta::assert_snapshot!(out);
}
