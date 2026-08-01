//! Shared test support. Not a test target itself (it lives in a subdirectory),
//! so each test file pulls it in with `mod support;`.
//!
//! `dead_code` is allowed because different test targets use different parts of
//! it; the alternative is a per-target `#[allow]` on every unused helper.
#![allow(dead_code)]

pub mod corpus;

use detoxrs_core::policy::{M1_MAX_LEN, Policy};
use proptest::prelude::*;

/// Resolved policies for the §8.1 properties.
///
/// `separator` is fixed at `_`: M1 has no `--separator` flag (it arrives with the
/// config file at M3), so a generated separator would quantify over policies that
/// cannot exist -- and a separator-class separator would make Safety closure
/// false by construction rather than by a bug.
///
/// Both length fields are always concrete (plan §5.1). The generator deliberately
/// includes limits far below M1's 255 as well as 255 itself: a limit that never
/// bites makes the Length-bound property vacuous, which is the failure mode risk
/// 9 watches.
pub fn policy_strategy() -> impl Strategy<Value = Policy> {
    (1usize..=300, 1usize..=300).prop_map(|(bytes, utf16)| Policy {
        separator: '_',
        max_len_bytes: bytes,
        max_len_utf16: utf16,
    })
}

/// The default (M1, 255/255) policy plus generated ones, for properties that
/// should also be pinned on the shipping configuration.
pub fn policy_or_default() -> impl Strategy<Value = Policy> {
    prop_oneof![
        1 => Just(Policy::default()),
        1 => Just(Policy { separator: '_', max_len_bytes: M1_MAX_LEN, max_len_utf16: M1_MAX_LEN }),
        4 => policy_strategy(),
    ]
}

/// Arbitrary names, biased toward the hazards the pipeline exists for.
///
/// `any::<char>()` covers astral planes, combining marks, bidi controls, unpaired
/// -- no, `char` cannot be a surrogate -- and every control character; the hazard
/// pool makes short adversarial inputs (`***`, `..`, `-`) frequent instead of
/// astronomically unlikely.
pub fn nasty_name() -> impl Strategy<Value = String> {
    let hazards: Vec<char> = "...--__**  &()[]<>|;:!?\'\"`$\\/\r\n\t\0.\u{200b}\u{200d}\u{202e}\u{e0041}\u{301}e\u{1f600}aZ9"
        .chars()
        .collect();
    prop_oneof![
        // Mostly hazards: short, adversarial, shrinks well.
        3 => proptest::collection::vec(proptest::sample::select(hazards), 0..12)
            .prop_map(String::from_iter),
        // Anything at all, including long astral runs.
        1 => proptest::collection::vec(any::<char>(), 0..40).prop_map(String::from_iter),
    ]
}

/// Build an `OsString` from raw name bytes without a `&str` step.
///
/// Unix only: `OsStringExt::from_vec` is the only lossless byte -> `OsString`
/// route, and it does not exist on Windows. Tier-1 is Linux and macOS
/// (`docs/owner-decisions.md`), so tests that need invalid UTF-8 are
/// `#[cfg(unix)]`.
#[cfg(unix)]
#[must_use]
pub fn os_string_from_bytes(bytes: &[u8]) -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(bytes.to_vec())
}

/// `<hh>`-escaped rendering of raw bytes, for snapshotting names that are not
/// valid UTF-8 without ever printing raw bytes at a terminal (§3.4, §6.1).
///
/// The reporter's own version lives in the binary crate (work package 5a); this one exists
/// so a core-crate snapshot can show what it is asserting about.
#[must_use]
pub fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(s) => {
                push_escaped(&mut out, s);
                break;
            }
            Err(e) => {
                let good = e.valid_up_to();
                // `valid_up_to()` is a valid UTF-8 boundary by definition.
                push_escaped(
                    &mut out,
                    std::str::from_utf8(&rest[..good]).unwrap_or_default(),
                );
                let bad = e.error_len().unwrap_or(rest.len() - good);
                for b in &rest[good..good + bad] {
                    push_hex(&mut out, *b);
                }
                rest = &rest[good + bad..];
            }
        }
    }
    out
}

/// Append one byte as `<hh>`.
fn push_hex(out: &mut String, b: u8) {
    use std::fmt::Write as _;
    // Writing to a String is infallible; the Result exists only for the trait.
    let _ = write!(out, "<{b:02x}>");
}

/// Append `s`, escaping control characters so a snapshot file never carries a
/// raw CR (the `Icon\r` fixture) or a raw NUL.
fn push_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        if c.is_control() {
            for b in c.to_string().bytes() {
                push_hex(out, b);
            }
        } else {
            out.push(c);
        }
    }
}
