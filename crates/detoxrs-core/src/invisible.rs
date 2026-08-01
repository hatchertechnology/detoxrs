//! Stage 4's character set: the named one (§3.2, plan's "named set at M1").
//!
//! M4 replaces this body with the UCD-generated `Cf`/`Cs`/`Co` closure behind an
//! unchanged `is_invisible` signature, and builds that generator once for both
//! this table and `scripts.rs`'s Script table. Until then the named set covers
//! the whole CVE-2021-42574 (Trojan Source) class this stage exists for, which is
//! what stage 4 is *for* rather than a superset of it.
//!
//! Surrogates (`Cs`) cannot appear here: `char` cannot hold one, and a byte
//! sequence encoding one is not valid UTF-8, so it never reaches text at all
//! (§3.7). Private use (`Co`) waits for M4 deliberately -- a private-use
//! character is not invisible in every font, so deleting it is a judgement the
//! generated closure should make all at once, not a hand-listed range.

/// Is this character stage 4's business?
///
/// Bidi controls, zero-width characters, and Unicode Tags. Named ranges only, no
/// UCD table.
#[must_use]
pub const fn is_invisible(c: char) -> bool {
    matches!(
        c,
        // Bidi embedding/override controls (CVE-2021-42574).
        '\u{202a}'..='\u{202e}'
        // Bidi isolates.
        | '\u{2066}'..='\u{2069}'
        // Bidi marks.
        | '\u{200e}' | '\u{200f}'
        // Zero-width: ZWSP, ZWNJ, ZWJ, word joiner, BOM/ZWNBSP.
        | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}' | '\u{feff}'
        // Unicode Tags, including the deprecated language tag (detox #120).
        | '\u{e0000}'..='\u{e007f}'
    )
}

#[cfg(test)]
mod tests {
    use super::is_invisible;

    #[test]
    fn the_named_set_is_invisible() {
        for c in [
            '\u{202a}',
            '\u{202b}',
            '\u{202c}',
            '\u{202d}',
            '\u{202e}',
            '\u{2066}',
            '\u{2067}',
            '\u{2068}',
            '\u{2069}',
            '\u{200e}',
            '\u{200f}',
            '\u{200b}',
            '\u{200c}',
            '\u{200d}',
            '\u{2060}',
            '\u{feff}',
            '\u{e0000}',
            '\u{e0041}',
            '\u{e007f}',
        ] {
            assert!(is_invisible(c), "{c:?}");
        }
    }

    #[test]
    fn visible_and_control_characters_are_not_stage_fours_business() {
        for c in [
            'a',
            '.',
            ' ',
            '\u{a0}',
            '\u{202f}',
            '\u{0}',
            '\u{1f600}',
            '\u{301}',
            '中',
        ] {
            assert!(!is_invisible(c), "{c:?}");
        }
    }
}
