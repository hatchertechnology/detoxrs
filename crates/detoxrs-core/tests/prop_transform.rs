//! §8.1's properties against `transform`, minus the two that live in-crate.
//!
//! Two scoping rules from §8.1 apply throughout and are not negotiable:
//! every property whose subject is "the output name" is quantified over the
//! `Name(_)` branch only (the `Unrepresentable` branch produces no name, so it
//! cannot violate a property about names), and every property is quantified over
//! **resolved** policies, both length fields concrete.
//!
//! Stage independence is not here: per plan §5.2 it lives in `pipeline.rs`'s own
//! `#[cfg(test)]` module, because the `StageMask` seam is `pub(crate)` and
//! nothing should become `pub` for a test.

mod support;

use detoxrs_core::classes::{CharClass, classify};
use detoxrs_core::pipeline::{TransformResult, Unrepresentable, transform};
use detoxrs_core::policy::Policy;
use detoxrs_core::truncate::{Limits, split_extension, truncate, truncate_graphemes};
use proptest::prelude::*;
use support::policy_strategy;
use unicode_segmentation::UnicodeSegmentation as _;

proptest! {
    /// **Safety closure.** For `Name(o)`, `o` contains no delete-class
    /// character, no separator-class character, no leading `-`, and no trailing
    /// dot or space.
    ///
    /// The case clause of §8.1's statement ("and is entirely in the requested
    /// case") is **vacuous in M1**: stage 8 is M4 work, so the only case policy
    /// is `keep`. Stated here rather than silently dropped.
    #[test]
    fn safety_closure(input in support::nasty_name(), p in policy_strategy()) {
        if let TransformResult::Name(o) = transform(&input, &p) {
            for c in o.text.chars() {
                prop_assert_ne!(classify(c), CharClass::Delete, "delete-class {:?} survived in {:?}", c, o.text);
                prop_assert_ne!(classify(c), CharClass::Separator, "separator-class {:?} survived in {:?}", c, o.text);
            }
            prop_assert!(!o.text.starts_with('-'), "leading dash in {:?}", o.text);
            prop_assert!(!o.text.ends_with('.'), "trailing dot in {:?}", o.text);
            prop_assert!(!o.text.ends_with(' '), "trailing space in {:?}", o.text);
        }
    }

    /// **Non-empty.** For `Name(o)`, `o` is never `""`, `"."` or `".."`. Those
    /// cases are exactly what `Unrepresentable` carries instead (§3.14).
    #[test]
    fn non_empty(input in support::nasty_name(), p in policy_strategy()) {
        if let TransformResult::Name(o) = transform(&input, &p) {
            prop_assert!(!o.text.is_empty());
            prop_assert_ne!(o.text.as_str(), ".");
            prop_assert_ne!(o.text.as_str(), "..");
        }
    }

    /// **Dotfile preservation.** For `Name(o)`, `x` starts with exactly one `.`
    /// if and only if `o` does.
    ///
    /// The "vice versa" half is the load-bearing one: it forbids *manufacturing*
    /// a dotfile, which is why a leading dot run is preserved verbatim rather
    /// than collapsed (`..weird..name..` keeps both leading dots).
    #[test]
    fn dotfile_preservation(input in support::nasty_name(), p in policy_strategy()) {
        if let TransformResult::Name(o) = transform(&input, &p) {
            prop_assert_eq!(
                exactly_one_leading_dot(&input),
                exactly_one_leading_dot(&o.text),
                "input {:?} -> output {:?}", input, o.text
            );
        }
    }
}

proptest! {
    /// **Totality.** For every input and every resolved policy, `transform`
    /// returns either `Name(o)` -- with `o` satisfying Safety closure and
    /// Non-empty -- or `Unrepresentable(r)`. It never returns an unsafe name and
    /// never panics. This is what makes §8.1's `Name(_)` scoping honest rather
    /// than a hole, so it re-asserts the other properties' conclusions here.
    #[test]
    fn totality(input in support::nasty_name(), p in support::policy_or_default()) {
        match transform(&input, &p) {
            TransformResult::Name(o) => {
                prop_assert!(!o.text.is_empty());
                prop_assert_ne!(o.text.as_str(), ".");
                prop_assert_ne!(o.text.as_str(), "..");
                prop_assert!(o.text.chars().all(|c| classify(c) == CharClass::Keep));
                prop_assert!(!o.text.starts_with('-'));
                prop_assert!(!o.text.ends_with('.') && !o.text.ends_with(' '));
                prop_assert!(o.text.len() <= p.max_len_bytes);
                prop_assert!(utf16_len(&o.text) <= p.max_len_utf16);
            }
            TransformResult::Unrepresentable(
                Unrepresentable::ReducesToEmpty
                | Unrepresentable::ReducesToDotOrDotDot
                | Unrepresentable::NotConverged,
            ) => {}
        }
    }

    /// **Idempotence.** For `Name(o)`, `transform(o) == Name(o)`.
    ///
    /// The generator is biased toward inputs that approach stage 13's 3-iteration
    /// bound (plan §5.2): long separator/dot runs next to an extension dot, and
    /// small limits so truncation itself feeds the loop.
    #[test]
    fn idempotence(input in support::nasty_name(), p in support::policy_or_default()) {
        if let TransformResult::Name(o) = transform(&input, &p) {
            // The fixed point is over the *name*. `truncated` is a note about
            // what happened to this input (§3.1's `Outcome.notes`), not part of
            // the name, so re-running on an already-short name reports
            // `truncated: false` -- which is asserted here rather than papered
            // over, because a second pass that DID truncate again would mean the
            // first pass produced an over-long name.
            match transform(&o.text, &p) {
                TransformResult::Name(again) => {
                    prop_assert_eq!(&again.text, &o.text, "not a fixed point: {:?}", input);
                    prop_assert!(!again.truncated, "re-truncated {:?}", o.text);
                }
                TransformResult::Unrepresentable(r) => {
                    prop_assert!(false, "{:?} -> {:?} -> Unrepresentable({:?})", input, o.text, r);
                }
            }
        }
    }

    /// **Length bound**, against BOTH fields (plan §5.1). A one-axis
    /// implementation fails here rather than on ext4 in the field (risk 9).
    #[test]
    fn length_bound(input in support::nasty_name(), p in support::policy_strategy()) {
        if let TransformResult::Name(o) = transform(&input, &p) {
            prop_assert!(
                o.text.len() <= p.max_len_bytes,
                "{} bytes > {}: {:?}", o.text.len(), p.max_len_bytes, o.text
            );
            prop_assert!(
                utf16_len(&o.text) <= p.max_len_utf16,
                "{} UTF-16 units > {}: {:?}", utf16_len(&o.text), p.max_len_utf16, o.text
            );
        }
    }

    /// **No grapheme splitting**, at the stage where it is decidable: every
    /// cluster `truncate_graphemes` keeps is a complete cluster of its input, in
    /// order, and the result is a grapheme-prefix of the input.
    ///
    /// The pipeline-level form of this property lives in `pipeline.rs`'s own test
    /// module, because it must disable stage 4 through the `pub(crate)` mask --
    /// see the note there: stripping a ZWJ *raises* the cluster count by design.
    #[test]
    fn no_grapheme_splitting(input in support::nasty_name(), limits in limits_strategy()) {
        let kept = truncate_graphemes(&input, &limits);
        prop_assert!(input.starts_with(kept));
        let want: Vec<&str> = input.graphemes(true).collect();
        let got: Vec<&str> = kept.graphemes(true).collect();
        prop_assert!(got.len() <= want.len());
        for (i, g) in got.iter().enumerate() {
            prop_assert_eq!(*g, want[i], "cluster {} was split: {:?}", i, kept);
        }
        prop_assert!(kept.len() <= limits.bytes);
        prop_assert!(utf16_len(kept) <= limits.utf16);
    }

    /// `truncate` obeys both limits and never splits a cluster either, on the
    /// stem path and on §3.10 step 3's whole-name fallback alike.
    #[test]
    fn truncate_obeys_both_limits(input in support::nasty_name(), limits in limits_strategy()) {
        let (stem, ext) = split_extension(&input);
        let (out, truncated) = truncate(stem, ext, &limits);
        prop_assert!(out.len() <= limits.bytes, "{:?}", out);
        prop_assert!(utf16_len(&out) <= limits.utf16, "{:?}", out);
        prop_assert_eq!(truncated, out.len() != input.len() || utf16_len(&out) != utf16_len(&input));
    }
}

fn limits_strategy() -> impl Strategy<Value = Limits> {
    (1usize..=300, 1usize..=300).prop_map(|(bytes, utf16)| Limits { bytes, utf16 })
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// §3.14's worked counterexample, as the named literal case the plan asks for
/// before the property that subsumes it.
#[test]
fn three_asterisks_are_unrepresentable() {
    assert_eq!(
        transform("***", &Policy::default()),
        TransformResult::Unrepresentable(Unrepresentable::ReducesToEmpty)
    );
}

/// Risk 9's fixture: 128 astral emoji is 512 bytes and 256 UTF-16 units, so it
/// is over BOTH of M1's 255 limits, and a one-axis truncation passes only one of
/// these two assertions.
#[test]
fn astral_emoji_corpus_entry_satisfies_both_limits() {
    let p = Policy::default();
    let bytes = support::corpus::repeated('\u{1f600}', 128);
    let input = std::str::from_utf8(&bytes).expect("generated fixture is UTF-8");
    assert_eq!(input.len(), 512);
    assert_eq!(utf16_len(input), 256);
    match transform(input, &p) {
        TransformResult::Name(o) => {
            assert!(o.truncated);
            assert!(o.text.len() <= p.max_len_bytes, "{} bytes", o.text.len());
            assert!(utf16_len(&o.text) <= p.max_len_utf16);
            // Grapheme-safe: every emoji is whole, none split into surrogates.
            assert!(o.text.chars().all(|c| c == '\u{1f600}'));
        }
        TransformResult::Unrepresentable(r) => panic!("Unrepresentable({r:?})"),
    }
}

/// Whether `s` starts with exactly one `.`, once leading invisible characters
/// (stage 4's own subject) are looked past.
///
/// C-5: an invisible character in front of a dot is not itself a dot, and it
/// is not visible content either -- `transform` deletes it. Comparing raw
/// literal first characters, as this helper did before, made the property
/// blind to C-5: for `"\u{200b}.bashrc"`, the literal check said "does not
/// start with a dot" both before and after the bug's fix removed the leading
/// dot, so the mismatch this property exists to catch never showed up. This
/// is not the property's subject changing -- "did we manufacture or destroy a
/// dotfile" was always about the name's real leading dot, and an invisible
/// character was never part of that name's real content.
fn exactly_one_leading_dot(s: &str) -> bool {
    let s = s.trim_start_matches(detoxrs_core::invisible::is_invisible);
    s.strip_prefix('.')
        .is_some_and(|rest| !rest.starts_with('.'))
}

/// Every corpus entry that decodes runs through `transform` without a panic, and
/// its `Name(_)` results satisfy the same closure the properties assert.
#[test]
fn corpus_transforms_safely() {
    let p = Policy::default();
    for e in support::corpus::all() {
        let Ok(text) = std::str::from_utf8(&e.bytes) else {
            continue; // Opaque: stage 1 skips it, there is nothing to transform.
        };
        if let TransformResult::Name(o) = transform(text, &p) {
            assert!(!o.text.is_empty(), "{}: empty name", e.id);
            assert!(
                o.text.chars().all(|c| classify(c) == CharClass::Keep),
                "{}: unsafe character survived in {:?}",
                e.id,
                o.text
            );
        }
    }
}
