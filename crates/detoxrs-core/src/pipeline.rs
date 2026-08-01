//! The transform pipeline (§3.2), and plan §5.2's `StageMask` seam.
//!
//! M1's stage set is 1, 3, 4, 7, 9, 10, 12, 13. Stage 1 is `decode`, which runs
//! before this module sees text at all; stages 2, 5, 6, 8 and 11 belong to later
//! milestones and are not represented here even as no-ops.
//!
//! Each linear stage is its own named `pub(crate)` function and one internal
//! `run_with` composes them. That is a testability requirement, not a style
//! preference: without the seam, the Stage-independence property can only
//! reimplement the pipeline, which tests nothing (plan §5.2). A disabled stage is
//! skipped, which is identity by definition -- there is no second pipeline to
//! keep in sync.

use crate::classes::{CharClass, classify};
use crate::invisible::is_invisible;
use crate::policy::Policy;
use crate::truncate::{Limits, split_extension, truncate};
use unicode_normalization::UnicodeNormalization as _;

/// A successful transform.
///
/// No `Vec<StageDelta>` in M1: the per-stage trace arrives at M2, when `-vv`
/// makes one worth reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The cleaned name.
    pub text: String,
    /// Whether stage 12 shortened the name.
    pub truncated: bool,
}

/// Why there is no representable output (§3.14).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unrepresentable {
    /// Nothing safe remained (`***`).
    ReducesToEmpty,
    /// The result would have been `.` or `..`.
    ReducesToDotOrDotDot,
    /// Stage 13's loop was still moving after its bound.
    NotConverged,
}

/// `transform`'s two outcomes.
///
/// There is deliberately no third one, and no fallback to the original text:
/// falling back would reintroduce exactly the characters the pipeline exists to
/// remove, which is what §3.14 fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformResult {
    /// A name that satisfies Safety closure and Non-empty.
    Name(Outcome),
    /// No name at all. The planner skips the entry, unchanged, and reports it.
    Unrepresentable(Unrepresentable),
}

/// Stage 13's iteration bound (§3.2, §3.14). Spike 12 measures whether 3 is
/// ever tight; §3.14 makes non-convergence safe either way.
const FIXED_POINT_BOUND: u8 = 3;

/// Transform a decoded name. Pure: no path, no directory, no other file.
#[must_use]
pub fn transform(input: &str, p: &Policy) -> TransformResult {
    run_with(input, p, StageMask::NONE)
}

/// Which linear stages to skip. Values are the §3.2 stage numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StageMask(u16);

impl StageMask {
    /// Every stage on: what `transform` uses.
    pub(crate) const NONE: Self = Self(0);
    /// Stage 3, NFC normalization.
    pub(crate) const NORMALIZE: Self = Self(1 << 3);
    /// Stage 4, invisible stripping.
    pub(crate) const INVISIBLE: Self = Self(1 << 4);
    /// Stage 7, the safe map.
    pub(crate) const SAFE_MAP: Self = Self(1 << 7);
    /// Stage 9, same-character run collapsing.
    pub(crate) const COLLAPSE: Self = Self(1 << 9);
    /// Stage 10, trimming.
    pub(crate) const TRIM: Self = Self(1 << 10);
    /// Every maskable (linear) stage off. Stages 12 and 13 are not maskable:
    /// they are tested directly against their own signatures (plan §5.2).
    /// Test-only: the shipping pipeline never disables a stage.
    #[cfg(test)]
    pub(crate) const ALL_LINEAR: Self = Self(
        Self::NORMALIZE.0 | Self::INVISIBLE.0 | Self::SAFE_MAP.0 | Self::COLLAPSE.0 | Self::TRIM.0,
    );

    const fn has(self, stage: Self) -> bool {
        self.0 & stage.0 != 0
    }
}

/// Stage 3: NFC. Comparison inside the planner is always NFC regardless.
pub(crate) fn normalize(s: &str) -> String {
    s.nfc().collect()
}

/// Stage 4: delete the invisibles (§3.12).
pub(crate) fn strip_invisible(s: &str) -> String {
    s.chars().filter(|c| !is_invisible(*c)).collect()
}

/// Stage 7: the safe map. Character classes, not a table.
pub(crate) fn safe_map(s: &str, separator: char) -> String {
    s.chars()
        .filter_map(|c| match classify(c) {
            CharClass::Delete => None,
            CharClass::Separator => Some(separator),
            CharClass::Keep => Some(c),
        })
        .collect()
}

/// Stage 9: collapse a run of the *same repeated character* to one, for the
/// collapse set only (`.`, `-`, `_`, and the separator), then drop separators
/// adjacent to a `.`.
///
/// Never merges runs of *different* characters: `a_-_b` is unchanged, which is
/// detox #121's complaint fixed rather than reproduced.
pub(crate) fn collapse(s: &str, separator: char) -> String {
    let mut squeezed = String::with_capacity(s.len());
    let mut prev = None;
    for c in s.chars() {
        let collapsible = matches!(c, '.' | '-' | '_') || c == separator;
        if collapsible && prev == Some(c) {
            continue;
        }
        squeezed.push(c);
        prev = Some(c);
    }

    // `" & " -> "___"` next to an extension dot would otherwise leave
    // `Movie_1985_.mkv`; §3.7 defends `( )` being separator-class on exactly
    // this rule keeping the result readable.
    let chars: Vec<char> = squeezed.chars().collect();
    let mut out = String::with_capacity(squeezed.len());
    for (i, c) in chars.iter().copied().enumerate() {
        if c == separator && c != '.' {
            let after_dot = i > 0 && chars[i - 1] == '.';
            let before_dot = chars.get(i + 1) == Some(&'.');
            if after_dot || before_dot {
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// Stage 10: trim.
///
/// Strips a leading `-`, leading and trailing separators, leading and trailing
/// dots and spaces, and then restores exactly `leading_dots` leading dots --
/// `leading_dots` being the count from `transform`'s *original* input, not from
/// this stage's input.
///
/// Preserving the original run verbatim rather than "exactly one dot" is what
/// makes Dotfile preservation true in both directions: collapsing `..weird` to
/// `.weird` would manufacture a dotfile out of a name that was not one, and
/// §8.1's "and vice versa" clause forbids that.
///
/// `.!file.txt` is the worked example from §3.8: stage 7 gives `._file.txt`,
/// stage 9 drops the separator next to the dot, and this stage keeps the one
/// leading dot -> `.file.txt`, never `._file.txt`. The dot is a dotfile marker,
/// not a shield for whatever follows it.
///
/// Returns the empty string when nothing survives; the caller turns that into
/// `Unrepresentable` rather than reattaching a dot run that would be a name
/// ending in a dot.
pub(crate) fn trim(s: &str, separator: char, leading_dots: usize) -> String {
    let body = s
        .trim_start_matches(['.', '-', ' ', separator])
        .trim_end_matches(['.', ' ', separator]);
    if body.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(body.len() + leading_dots);
    for _ in 0..leading_dots {
        out.push('.');
    }
    out.push_str(body);
    out
}

/// Stage 12 over a whole name: split the extension off, then truncate.
fn apply_truncate(text: &str, limits: &Limits) -> (String, bool) {
    let (stem, ext) = split_extension(text);
    truncate(stem, ext, limits)
}

/// Plan §5.2's seam. `transform` is this with nothing disabled.
pub(crate) fn run_with(input: &str, p: &Policy, disabled: StageMask) -> TransformResult {
    let leading_dots = input.chars().take_while(|c| *c == '.').count();

    let mut text = input.to_owned();
    if !disabled.has(StageMask::NORMALIZE) {
        text = normalize(&text);
    }
    if !disabled.has(StageMask::INVISIBLE) {
        text = strip_invisible(&text);
    }
    if !disabled.has(StageMask::SAFE_MAP) {
        text = safe_map(&text, p.separator);
    }
    if !disabled.has(StageMask::COLLAPSE) {
        text = collapse(&text, p.separator);
    }
    if !disabled.has(StageMask::TRIM) {
        text = trim(&text, p.separator, leading_dots);
    }

    // Stage 12. Not maskable: it is tested directly against its own signature
    // (plan §5.2), and a name over the filesystem's limit cannot be renamed at
    // all, so "skip this stage" is not a state the pipeline can be in.
    let limits = Limits {
        bytes: p.max_len_bytes,
        utf16: p.max_len_utf16,
    };
    let (mut text, mut truncated) = apply_truncate(&text, &limits);

    // Stage 13: re-run to a fixed point, bounded at FIXED_POINT_BOUND
    // iterations, because truncation can itself create a trailing dot
    // (`report.tar.gz` -> `report.`).
    //
    // The loop re-runs stage **3** as well as 9, 10 and 12, which §3.2 does not
    // say and which is a deliberate correction found by the Idempotence property
    // at M1 WP3: stages 4 and 7 DELETE characters, and a deletion can bring a
    // base character and a combining mark together that were not adjacent
    // before. `e\r\u{301}` is the minimal case -- NFC cannot compose across the
    // CR, stage 7 deletes the CR, and the result `e\u{301}` is not NFC, so
    // `transform` was not a fixed point and the planner's NFC invariant was
    // broken by its own pipeline. Re-normalizing inside the loop fixes both.
    // Stage 12 is re-run with it because NFC is not length-preserving in bytes
    // (a singleton decomposition that does not recompose grows), so normalizing
    // after truncation without re-truncating could exceed the limit again.
    let mut converged = false;
    for _ in 0..FIXED_POINT_BOUND {
        let mut next = text.clone();
        if !disabled.has(StageMask::NORMALIZE) {
            next = normalize(&next);
        }
        if !disabled.has(StageMask::COLLAPSE) {
            next = collapse(&next, p.separator);
        }
        if !disabled.has(StageMask::TRIM) {
            next = trim(&next, p.separator, leading_dots);
        }
        let (next, cut) = apply_truncate(&next, &limits);
        truncated |= cut;
        if next == text {
            converged = true;
            break;
        }
        text = next;
    }
    if !converged {
        // §3.14: no silent non-idempotent output, no runtime-raised bound, no
        // panic. A NotConverged occurrence is a bug report against us.
        return TransformResult::Unrepresentable(Unrepresentable::NotConverged);
    }

    match text.as_str() {
        "" => TransformResult::Unrepresentable(if input == "." || input == ".." {
            Unrepresentable::ReducesToDotOrDotDot
        } else {
            Unrepresentable::ReducesToEmpty
        }),
        "." | ".." => TransformResult::Unrepresentable(Unrepresentable::ReducesToDotOrDotDot),
        _ => TransformResult::Name(Outcome { text, truncated }),
    }
}

#[cfg(test)]
mod tests {
    use super::{StageMask, TransformResult, Unrepresentable, run_with, transform};
    use crate::policy::Policy;
    use proptest::prelude::*;
    use unicode_segmentation::UnicodeSegmentation as _;

    fn name(input: &str) -> String {
        match transform(input, &Policy::default()) {
            TransformResult::Name(o) => o.text,
            TransformResult::Unrepresentable(r) => panic!("{input:?} was Unrepresentable({r:?})"),
        }
    }

    /// §3.8's worked example, spelled out there precisely because two
    /// implementations would otherwise differ.
    #[test]
    fn bang_dotfile_keeps_the_dot_and_drops_the_separator() {
        assert_eq!(name(".!file.txt"), ".file.txt");
    }

    #[test]
    fn named_cases_from_the_corpus() {
        assert_eq!(name("a_-_b.mp3"), "a_-_b.mp3"); // detox #121
        assert_eq!(name(".hidden file"), ".hidden_file");
        assert_eq!(name("..weird..name.."), "..weird.name");
        assert_eq!(name("libstdc++.so"), "libstdc++.so");
        assert_eq!(name("a & b (1985) [720p].mkv"), "a_b_1985_720p.mkv");
        assert_eq!(name("Icon\r"), "Icon");
        assert_eq!(name("cafe\u{301}.txt"), "caf\u{e9}.txt");
        assert_eq!(name("in\u{200b}visible.txt"), "invisible.txt");
        assert_eq!(name("invoice\u{202e}fdp.txt"), "invoicefdp.txt");
    }

    #[test]
    fn a_lone_dash_and_dots_are_unrepresentable() {
        let p = Policy::default();
        assert_eq!(
            transform("-", &p),
            TransformResult::Unrepresentable(Unrepresentable::ReducesToEmpty)
        );
        assert_eq!(
            transform(".", &p),
            TransformResult::Unrepresentable(Unrepresentable::ReducesToDotOrDotDot)
        );
        assert_eq!(
            transform("..", &p),
            TransformResult::Unrepresentable(Unrepresentable::ReducesToDotOrDotDot)
        );
    }

    // ---- Stage independence (§8.1, plan §5.2) --------------------------------
    //
    // The seam makes "the output with stage N off equals the pipeline with stage
    // N replaced by identity" true by construction, so asserting *that* would be
    // vacuous. What is not vacuous, and what caught detox's #40/#86 (the UTF-8
    // filter also doing safe-filter work), is the converse: stage N is the ONLY
    // stage that does stage N's job. Each test below disables exactly one stage
    // and asserts its effect is gone -- if some other stage had quietly grown the
    // same behavior, the effect would survive the mask.

    #[test]
    fn disabling_every_linear_stage_is_the_identity() {
        // A tame name so stages 12 and 13, which are not maskable, cannot move it.
        let p = Policy::default();
        let input = "A b*c\u{200b}--d\u{301}..";
        match run_with(input, &p, StageMask::ALL_LINEAR) {
            TransformResult::Name(o) => assert_eq!(o.text, input),
            TransformResult::Unrepresentable(r) => panic!("Unrepresentable({r:?})"),
        }
    }

    /// §8.1's "No grapheme splitting" says the output's cluster count is never
    /// greater than the input's. **Stage 4 makes that false on purpose**, and
    /// this test pins the conflict rather than hiding it: a ZWJ emoji sequence is
    /// ONE cluster held together by joiners that stage 4 deletes as invisible
    /// (§3.2, §3.12), so the family emoji becomes three clusters. Stripping a
    /// Trojan-Source-class character wins over preserving a cluster count; the
    /// property therefore holds at the truncation boundary (`truncate.rs`, where
    /// the `is_char_boundary` bug it was written against lives) and across every
    /// stage except 4, which is what the mask lets the next test assert.
    #[test]
    fn stage_four_raises_the_cluster_count_on_a_zwj_sequence_by_design() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f466}.png";
        assert_eq!(family.graphemes(true).count(), 5); // family + '.' + p + n + g
        let cleaned = name(family);
        assert_eq!(cleaned.graphemes(true).count(), 7);
        assert!(!cleaned.contains('\u{200d}'));
    }

    proptest! {
        /// The pipeline-level form of "No grapheme splitting": with stage 4
        /// masked off, no stage increases the grapheme cluster count. Stage 12 is
        /// the one that could split a cluster, and it is not maskable, so it is
        /// exercised here on every input.
        #[test]
        fn no_stage_but_four_increases_the_cluster_count(
            input in proptest::collection::vec(
                proptest::sample::select(
                    "aZ9.-_ *&()[]e\u{301}\u{1f600}\u{1f468}\u{200d}\u{1f469}\r\0\u{202e}"
                        .chars()
                        .collect::<Vec<char>>(),
                ),
                0..16,
            )
            .prop_map(String::from_iter),
            bytes in 1usize..=64,
            utf16 in 1usize..=64,
        ) {
            let p = Policy { separator: '_', max_len_bytes: bytes, max_len_utf16: utf16 };
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::INVISIBLE) {
                prop_assert!(
                    o.text.graphemes(true).count() <= input.graphemes(true).count(),
                    "{:?} -> {:?}", input, o.text
                );
            }
        }
    }

    proptest! {
        /// Stage 4 is the only stage that removes invisibles.
        #[test]
        fn only_stage_four_strips_invisibles(stem in "[a-z]{1,8}") {
            let p = Policy::default();
            let input = format!("{stem}\u{200b}\u{202e}\u{e0041}{stem}");
            let with = run_with(&input, &p, StageMask::NONE);
            let without = run_with(&input, &p, StageMask::INVISIBLE);
            if let TransformResult::Name(o) = with {
                prop_assert!(!o.text.contains('\u{200b}'), "{:?}", o.text);
                prop_assert!(!o.text.contains('\u{202e}'), "{:?}", o.text);
                prop_assert!(!o.text.contains('\u{e0041}'), "{:?}", o.text);
            }
            if let TransformResult::Name(o) = without {
                prop_assert!(o.text.contains('\u{200b}'), "{:?}", o.text);
                prop_assert!(o.text.contains('\u{202e}'), "{:?}", o.text);
                prop_assert!(o.text.contains('\u{e0041}'), "{:?}", o.text);
            }
        }

        /// Stage 7 is the only stage that maps separator-class characters, and
        /// the only stage that deletes control characters.
        #[test]
        fn only_stage_seven_runs_the_safe_map(stem in "[a-z]{1,8}") {
            let p = Policy::default();
            let input = format!("{stem}*{stem}\u{7}{stem}");
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::NONE) {
                prop_assert!(!o.text.contains('*'), "{:?}", o.text);
                prop_assert!(!o.text.contains('\u{7}'), "{:?}", o.text);
                prop_assert!(o.text.contains('_'), "{:?}", o.text);
            }
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::SAFE_MAP) {
                prop_assert!(o.text.contains('*'), "{:?}", o.text);
                prop_assert!(o.text.contains('\u{7}'), "{:?}", o.text);
                prop_assert!(!o.text.contains('_'), "{:?}", o.text);
            }
        }

        /// Stage 3 is the only stage that normalizes.
        #[test]
        fn only_stage_three_normalizes(stem in "[a-z]{1,8}") {
            let p = Policy::default();
            let input = format!("{stem}e\u{301}{stem}");
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::NONE) {
                prop_assert!(o.text.contains('\u{e9}'), "{:?}", o.text);
                prop_assert!(!o.text.contains('\u{301}'), "{:?}", o.text);
            }
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::NORMALIZE) {
                prop_assert!(o.text.contains('\u{301}'), "{:?}", o.text);
                prop_assert!(!o.text.contains('\u{e9}'), "{:?}", o.text);
            }
        }

        /// Stage 9 is the only stage that collapses runs.
        #[test]
        fn only_stage_nine_collapses(stem in "[a-z]{1,8}") {
            let p = Policy::default();
            let input = format!("{stem}___{stem}---{stem}");
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::NONE) {
                prop_assert!(!o.text.contains("__"), "{:?}", o.text);
                prop_assert!(!o.text.contains("--"), "{:?}", o.text);
            }
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::COLLAPSE) {
                prop_assert!(o.text.contains("___"), "{:?}", o.text);
                prop_assert!(o.text.contains("---"), "{:?}", o.text);
            }
        }

        /// Stage 10 is the only stage that trims edges.
        #[test]
        fn only_stage_ten_trims(stem in "[a-z]{1,8}") {
            let p = Policy::default();
            let input = format!("-{stem}. ");
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::NONE) {
                prop_assert!(!o.text.starts_with('-'), "{:?}", o.text);
                prop_assert!(!o.text.ends_with('.'), "{:?}", o.text);
                prop_assert!(!o.text.ends_with('_'), "{:?}", o.text);
            }
            if let TransformResult::Name(o) = run_with(&input, &p, StageMask::TRIM) {
                prop_assert!(o.text.starts_with('-'), "{:?}", o.text);
                // The trailing space became a separator (stage 7) and stage 9
                // dropped it as adjacent to the dot, so the dot is what survives.
                prop_assert!(o.text.ends_with('.'), "{:?}", o.text);
            }
        }
    }
}
