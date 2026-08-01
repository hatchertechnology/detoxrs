//! Pipeline stage 1: `OsStr` -> text, or nothing (proposal §3.4, owner decision
//! 2026-07-31).
//!
//! Valid UTF-8 passes through untouched, and that is the whole stage. Anything
//! else is `Opaque`: skipped and reported, never repaired, never guessed at,
//! never lossily converted. There is no legacy decoder in this binary, so
//! detox's `café.txt -> cafÃ©.txt` bug class is unreachable rather than merely
//! discouraged -- the code does not exist.

use std::ffi::OsStr;

/// The only two outcomes there are.
///
/// The `Repaired` variant the earlier design had is gone by owner decision, and
/// its absence is what makes the decode property (§8.1) an iff rather than a
/// two-of-three case analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoded {
    /// The input was valid UTF-8. The `String` is byte-identical to the input.
    Utf8(String),
    /// The input was not valid UTF-8. No text, no guess, no lossy conversion.
    Opaque,
}

/// Decode a raw name.
///
/// No `Policy` parameter (plan §7.2, amendment 17): with repair dropped there is
/// no field for this function to read, and a dead parameter is not fidelity to
/// the design.
///
/// `OsStr::to_str` is the whole implementation on every platform: it yields
/// `Some` exactly when the underlying bytes are valid UTF-8 on Unix, and exactly
/// when the WTF-8 encoding contains no unpaired surrogate on Windows. Either way
/// `None` means "not representable as text", which is `Opaque`.
#[must_use]
pub fn decode(raw: &OsStr) -> Decoded {
    raw.to_str()
        .map_or(Decoded::Opaque, |s| Decoded::Utf8(s.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Decoded, decode};
    use std::ffi::OsStr;

    #[test]
    fn valid_utf8_round_trips_byte_for_byte() {
        let name = "café.txt";
        assert_eq!(decode(OsStr::new(name)), Decoded::Utf8(name.to_owned()));
    }

    #[test]
    fn empty_name_is_utf8_not_opaque() {
        assert_eq!(decode(OsStr::new("")), Decoded::Utf8(String::new()));
    }

    /// The CP1252 fixture from §8.3, which is the one that used to be routed to
    /// the deleted repair path.
    #[cfg(unix)]
    #[test]
    fn cp1252_bytes_are_opaque_never_repaired() {
        use std::os::unix::ffi::OsStrExt as _;
        let raw = OsStr::from_bytes(b"Bj\xf6rk - Vespertine.mp3");
        assert_eq!(decode(raw), Decoded::Opaque);
    }
}
