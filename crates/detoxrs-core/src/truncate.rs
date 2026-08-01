//! Stage 12: grapheme-safe, extension-preserving truncation (§3.10).
//!
//! Boundaries are **grapheme cluster** boundaries, never `is_char_boundary`.
//! `sanitize-filename` 0.6.0 truncates at `is_char_boundary`, i.e. it splits a
//! base+combining-mark pair or a ZWJ emoji sequence; that documented bug is the
//! specific reason this module exists rather than a dependency.

use unicode_segmentation::UnicodeSegmentation as _;

/// A resolved length budget, in both units at once (plan §5.1).
///
/// Both fields are always checked. One scalar cannot express "255 bytes on ext4
/// AND 255 UTF-16 units on APFS", and 130 astral emoji is the input that proves
/// it: 260 UTF-16 units, 520 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Maximum length in UTF-8 bytes.
    pub bytes: usize,
    /// Maximum length in UTF-16 code units.
    pub utf16: usize,
}

/// Length in UTF-16 code units, without encoding anything.
#[must_use]
pub fn utf16_len(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Does `s` satisfy both limits?
#[must_use]
pub fn fits(s: &str, limits: &Limits) -> bool {
    s.len() <= limits.bytes && utf16_len(s) <= limits.utf16
}

/// Largest prefix of `s` ending on a grapheme cluster boundary that satisfies
/// both limits.
///
/// Called from both §3.10 step 2 (the stem path) and step 3 (the whole-name
/// fallback). One function, two callers: two hand-written loops would drift, and
/// the fallback is exactly where an implementer would reach for
/// `is_char_boundary`.
#[must_use]
pub fn truncate_graphemes<'a>(s: &'a str, limits: &Limits) -> &'a str {
    if fits(s, limits) {
        return s;
    }
    let mut end = 0;
    let mut bytes = 0;
    let mut units = 0;
    for g in s.graphemes(true) {
        let (b, u) = (g.len(), utf16_len(g));
        if bytes + b > limits.bytes || units + u > limits.utf16 {
            break;
        }
        bytes += b;
        units += u;
        end += b;
    }
    &s[..end]
}

/// Truncate `stem` + `ext` to fit both limits, preserving the extension when
/// there is room for it.
///
/// Returns the new name and whether anything was dropped. Falls back to §3.10
/// step 3 -- truncating the whole name as one unit, same grapheme algorithm --
/// when the extension does not fit, or when keeping it would leave no stem at
/// all. detox's behavior here is "print a warning and give up unchanged", which
/// leaves an overlong name in place.
#[must_use]
pub fn truncate(stem: &str, ext: &str, limits: &Limits) -> (String, bool) {
    let mut whole = String::with_capacity(stem.len() + ext.len());
    whole.push_str(stem);
    whole.push_str(ext);
    if fits(&whole, limits) {
        return (whole, false);
    }

    let (ext_bytes, ext_units) = (ext.len(), utf16_len(ext));
    if ext_bytes < limits.bytes && ext_units < limits.utf16 {
        let reduced = Limits {
            bytes: limits.bytes - ext_bytes,
            utf16: limits.utf16 - ext_units,
        };
        let kept = truncate_graphemes(stem, &reduced);
        if !kept.is_empty() {
            let mut out = String::with_capacity(kept.len() + ext.len());
            out.push_str(kept);
            out.push_str(ext);
            return (out, true);
        }
        // An empty stem would turn `abcdef.txt` into `.txt`, i.e. manufacture a
        // dotfile out of a name that was not one -- which §8.1's Dotfile
        // preservation forbids in both directions. Step 3 handles it instead.
    }
    (truncate_graphemes(&whole, limits).to_owned(), true)
}

/// §3.10 step 1: split off the extension.
///
/// The last `.`-suffix, plus the preceding segment when that segment is <= 4
/// **bytes of UTF-8** and is itself preceded by a `.` (`.tar.gz`). The unit is
/// bytes because upstream's equivalent is pointer arithmetic over a `char *`, and
/// this is the one comparison in §3.10 a reader must not have to guess about.
///
/// A leading `.` is never an extension: `.bashrc` is a dotfile, not a bare
/// suffix.
#[must_use]
pub fn split_extension(name: &str) -> (&str, &str) {
    let Some(dot) = name.rfind('.') else {
        return (name, "");
    };
    if dot == 0 {
        return (name, "");
    }
    let head = &name[..dot];
    if let Some(prev) = head.rfind('.') {
        // `dot - prev - 1` is the inner segment's length in bytes.
        if prev > 0 && dot - prev - 1 <= 4 {
            return (&name[..prev], &name[prev..]);
        }
    }
    (head, &name[dot..])
}

#[cfg(test)]
mod tests {
    use super::{Limits, split_extension, truncate, truncate_graphemes, utf16_len};

    const fn limits(bytes: usize, utf16: usize) -> Limits {
        Limits { bytes, utf16 }
    }

    #[test]
    fn split_extension_matches_the_documented_rule() {
        assert_eq!(split_extension("report.tar.gz"), ("report", ".tar.gz"));
        assert_eq!(split_extension("archive.tar.bz2"), ("archive", ".tar.bz2"));
        assert_eq!(split_extension("photo.jpeg"), ("photo", ".jpeg"));
        assert_eq!(split_extension("no_extension"), ("no_extension", ""));
        assert_eq!(split_extension(".bashrc"), (".bashrc", ""));
        assert_eq!(split_extension("trailing."), ("trailing", "."));
        // The inner segment is 5 bytes, past the lookback, so no pair.
        assert_eq!(split_extension("a.abcde.gz"), ("a.abcde", ".gz"));
    }

    #[test]
    fn nothing_is_truncated_when_it_already_fits() {
        let (out, truncated) = truncate("report", ".txt", &limits(255, 255));
        assert_eq!(out, "report.txt");
        assert!(!truncated);
    }

    #[test]
    fn the_extension_survives_when_there_is_room_for_a_stem() {
        let (out, truncated) = truncate(&"a".repeat(300), ".txt", &limits(10, 255));
        assert_eq!(out, "aaaaaa.txt");
        assert!(truncated);
    }

    /// §3.10 step 3: when the extension alone does not fit, the whole name is one
    /// unit -- and the grapheme rule is not waived on that path.
    #[test]
    fn the_whole_name_is_truncated_when_the_extension_cannot_fit() {
        let (out, truncated) = truncate("report", ".verylongextension", &limits(6, 255));
        assert_eq!(out, "report");
        assert!(truncated);
    }

    #[test]
    fn a_zwj_emoji_sequence_is_never_split() {
        let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f466}";
        // 18 bytes, 8 UTF-16 units, one grapheme cluster: a byte-boundary
        // truncation to 17 would cut the last emoji in half.
        assert_eq!(family.len(), 18);
        assert_eq!(utf16_len(family), 8);
        assert_eq!(truncate_graphemes(family, &limits(17, 255)), "");
        assert_eq!(truncate_graphemes(family, &limits(18, 255)), family);
        // UTF-16 is the binding limit here, not bytes.
        assert_eq!(truncate_graphemes(family, &limits(255, 7)), "");
    }

    #[test]
    fn a_combining_mark_stays_with_its_base() {
        // "a", then the cluster "e\u{301}" (2 chars, 3 bytes), then "z".
        let s = "ae\u{301}z";
        // 3 bytes is one short of the cluster: it is dropped whole, not split.
        assert_eq!(truncate_graphemes(s, &limits(3, 255)), "a");
        assert_eq!(truncate_graphemes(s, &limits(4, 255)), "ae\u{301}");
    }

    #[test]
    fn both_limits_bind_independently() {
        let emoji = "\u{1f600}".repeat(128); // 512 bytes, 256 UTF-16 units
        let by_bytes = truncate_graphemes(&emoji, &limits(255, usize::MAX));
        assert_eq!(by_bytes.chars().count(), 63); // 63 * 4 = 252 <= 255
        let by_units = truncate_graphemes(&emoji, &limits(usize::MAX, 255));
        assert_eq!(by_units.chars().count(), 127); // 127 * 2 = 254 <= 255
    }
}
