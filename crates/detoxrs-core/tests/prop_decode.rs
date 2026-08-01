//! §8.1's "Decode is total and never re-interprets" property.
//!
//! For every byte string `x`, `decode` returns `Utf8` if and only if `x` is
//! valid UTF-8 and `Opaque` otherwise -- no third outcome, no panic -- and
//! `Utf8(s)` round-trips to exactly `x`. This is P2 as an executable assertion
//! and the regression test for detox's `café.txt -> cafÃ©.txt`.

mod support;

use detoxrs_core::decode::{Decoded, decode};
use proptest::prelude::*;

proptest! {
    /// The property over arbitrary bytes. `#[cfg(unix)]` because
    /// `OsStringExt::from_vec` is the only lossless byte -> `OsString` route and
    /// it is Unix-only; tier-1 is Linux and macOS.
    #[cfg(unix)]
    #[test]
    fn decode_is_total_over_arbitrary_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let os = support::os_string_from_bytes(&bytes);
        match decode(&os) {
            Decoded::Utf8(s) => {
                prop_assert!(std::str::from_utf8(&bytes).is_ok());
                prop_assert_eq!(s.as_bytes(), bytes.as_slice());
            }
            Decoded::Opaque => prop_assert!(std::str::from_utf8(&bytes).is_err()),
        }
    }

    /// Valid UTF-8 always decodes, unchanged, on every platform.
    #[test]
    fn decode_never_rewrites_valid_utf8(s in ".{0,64}") {
        let os = std::ffi::OsString::from(s.clone());
        match decode(&os) {
            Decoded::Utf8(out) => prop_assert_eq!(out, s),
            Decoded::Opaque => prop_assert!(false, "valid UTF-8 decoded as Opaque"),
        }
    }
}

/// Every corpus entry decodes according to its own bytes, with no third outcome.
#[cfg(unix)]
#[test]
fn decode_agrees_with_utf8_validity_on_the_corpus() {
    for e in support::corpus::all() {
        let os = support::os_string_from_bytes(&e.bytes);
        let valid = std::str::from_utf8(&e.bytes).is_ok();
        match decode(&os) {
            Decoded::Utf8(s) => {
                assert!(valid, "{}: Utf8 for invalid bytes", e.id);
                assert_eq!(s.as_bytes(), e.bytes.as_slice(), "{}: not byte-exact", e.id);
            }
            Decoded::Opaque => assert!(!valid, "{}: Opaque for valid UTF-8", e.id),
        }
    }
}
