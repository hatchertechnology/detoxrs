//! §3.7's safe-character policy as code: three classes, defined by rule rather
//! than by a shipped table.

/// What stage 7 does with a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Removed with no replacement. Control characters only.
    Delete,
    /// Each run becomes one `Policy::separator`.
    Separator,
    /// Kept verbatim, including every Unicode letter, mark and digit.
    Keep,
}

/// §3.7's separator class: ASCII space plus the shell-metacharacter and path set.
///
/// `[` and `]` are here and `{` `}` are not, which looks arbitrary and is not: a
/// bracket expression is the one glob construct that can silently match a
/// *different* file, while brace expansion is a bash/zsh extension and not
/// filename globbing at all.
const SEPARATOR_CLASS: &[char] = &[
    ' ', '"', '\'', '`', '$', '!', '*', '?', '[', ']', '<', '>', '|', ';', '&', ':', '\\', '/',
    '(', ')',
];

/// Classify one character.
///
/// The delete class is Unicode `Cc` (which `char::is_control` reports, and which
/// includes NUL and DEL) and nothing else. It deliberately does **not** include
/// stage 4's invisibles (`Cf`, bidi, zero-width, Tags, `Cs`, `Co`): those are
/// stage 4's business alone, and if the delete class duplicated them then
/// `--no-invisible-strip` would be a dead flag and the Stage-independence
/// property would be false. That was an ACCEPTED finding in the stage-3 review,
/// so it is asserted by a test below rather than left as a comment.
#[must_use]
pub fn classify(c: char) -> CharClass {
    if c.is_control() {
        CharClass::Delete
    } else if SEPARATOR_CLASS.contains(&c) {
        CharClass::Separator
    } else {
        CharClass::Keep
    }
}

#[cfg(test)]
mod tests {
    use super::{CharClass, classify};
    use crate::invisible::is_invisible;

    #[test]
    fn controls_including_nul_and_del_are_deleted() {
        for c in ['\0', '\r', '\n', '\t', '\u{7f}', '\u{1b}', '\u{85}'] {
            assert_eq!(classify(c), CharClass::Delete, "{c:?}");
        }
    }

    #[test]
    fn shell_metacharacters_and_space_are_separators() {
        for c in [' ', '&', '*', '[', ']', '(', ')', '/', '\\', ':', '?'] {
            assert_eq!(classify(c), CharClass::Separator, "{c:?}");
        }
    }

    #[test]
    fn keep_class_holds_the_contested_members() {
        for c in [
            '.', ',', '-', '_', '+', '=', '~', '#', '%', '@', '^', '{', '}',
        ] {
            assert_eq!(classify(c), CharClass::Keep, "{c:?}");
        }
    }

    #[test]
    fn non_ascii_letters_and_marks_are_always_kept() {
        for c in ['é', 'ü', '中', 'ß', '\u{301}', '\u{1f600}'] {
            assert_eq!(classify(c), CharClass::Keep, "{c:?}");
        }
    }

    /// The stage-3 review's ACCEPTED finding, as a test: the delete class must
    /// not re-include stage 4's set, or stage 4 becomes unobservable.
    #[test]
    fn delete_class_does_not_overlap_stage_fours_invisibles() {
        for c in [
            '\u{200b}',
            '\u{200c}',
            '\u{200d}',
            '\u{2060}',
            '\u{feff}',
            '\u{202e}',
            '\u{2066}',
            '\u{200e}',
            '\u{e0041}',
        ] {
            assert!(is_invisible(c), "{c:?} should be stage 4's business");
            assert_eq!(
                classify(c),
                CharClass::Keep,
                "{c:?} must not be deleted by stage 7"
            );
        }
    }
}
