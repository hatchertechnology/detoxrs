//! The resolved transform policy (proposal §3.1, plan §5.1).
//!
//! Every field is concrete by the time it reaches `transform`: there is no
//! `0 = auto` sentinel in this type, because resolving `auto` into a number is a
//! walk-time concern in the binary crate and this crate has no filesystem.

use crate::classes::{CharClass, classify};

/// A fully resolved policy.
///
/// The two length fields are the plan's §5.1 adjudication: one scalar cannot
/// express "255 bytes on ext4 AND 255 UTF-16 units on APFS" simultaneously, so
/// the Length-bound property (§8.1) would be vacuous on one axis -- exactly the
/// failure it was written to catch. Both fields are always concrete and both are
/// always checked.
///
/// `on_collision` deliberately does not live here: it is a plan-time concern
/// (§5.3), and this struct's contract is "every field maps 1:1 to a flag AND is
/// read by some stage".
///
/// `#[non_exhaustive]` blocks struct-literal construction from outside this
/// crate, but that alone is not enough: a `pub` field can still be *assigned*
/// after construction (`let mut p = Policy::default(); p.separator = '/';`),
/// which reaches `transform` exactly as broken as the literal did (C15). So
/// `separator` is `pub(crate)` instead of `pub` -- readable and writable from
/// anywhere inside this crate (the plan-time and pipeline-time modules that
/// already pin it to `'_'` or a proptest-generated `Keep`-class value), but
/// neither from outside it. [`Policy::new`] and [`Policy::default`] are the
/// only construction paths a library consumer has, and both already enforce
/// the invariant; [`Policy::separator`] is the read-only way out. This closes
/// the hole for future callers, in particular M3's `--separator` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Policy {
    /// Replacement for separator-class characters (§3.7). `_` in M1;
    /// `--separator` arrives with the config file at M3.
    ///
    /// `pub(crate)`, not `pub` -- see the struct doc comment. Read it from
    /// outside this crate with [`Policy::separator`].
    ///
    /// ```compile_fail
    /// let mut p = detoxrs_core::policy::Policy::default();
    /// p.separator = '/'; // cannot assign to private field
    /// ```
    pub(crate) separator: char,
    /// Maximum name length in UTF-8 bytes. M1 hardcodes 255 (ext4's limit);
    /// `statfs`-derived per directory from M5.
    pub max_len_bytes: usize,
    /// Maximum name length in UTF-16 code units. M1 hardcodes 255 (APFS's
    /// limit, empirically established in doc 06 Test 1).
    pub max_len_utf16: usize,
}

impl Policy {
    /// Build a policy, rejecting a `separator` that would break `transform`'s
    /// safety closure.
    ///
    /// `separator` must classify as [`CharClass::Keep`] (§3.7). A `Delete`-class
    /// separator (a control character) would vanish instead of separating, and
    /// a `Separator`-class separator -- ' ', '/', and the rest of the
    /// shell-metacharacter set -- would manufacture a fresh separator-class
    /// character inside a single filename component. `Policy { separator: '/',
    /// .. }` is exactly how `transform("a b.txt")` used to become `"a/b.txt"`:
    /// a path separator built inside one component, falsifying the no-separator
    /// guarantee the pipeline exists to provide. Checking it once here, rather
    /// than in every caller, means M3's `--separator` flag inherits the
    /// guarantee instead of having to re-derive it.
    ///
    /// # Errors
    ///
    /// Returns `separator` unchanged if it is not `CharClass::Keep`.
    pub fn new(separator: char, max_len_bytes: usize, max_len_utf16: usize) -> Result<Self, char> {
        if classify(separator) == CharClass::Keep {
            Ok(Self {
                separator,
                max_len_bytes,
                max_len_utf16,
            })
        } else {
            Err(separator)
        }
    }

    /// The resolved separator character.
    ///
    /// `separator` the field is `pub(crate)` (see the struct doc comment), so
    /// this getter is the read side for callers outside this crate -- the
    /// journal header (`detoxrs::journal`) is the one that needs it today.
    #[must_use]
    pub const fn separator(&self) -> char {
        self.separator
    }
}

/// M1's hardcoded limit, in both units.
///
/// Both tier-1 platforms are exactly 255 in their own unit (ext4: bytes; APFS:
/// UTF-16 code units), so this constant is wrong only on filesystems nobody is
/// running yet, and only in the over-truncating direction. M5 deletes it in the
/// same commit that adds `statfs` detection.
pub const M1_MAX_LEN: usize = 255;

impl Default for Policy {
    fn default() -> Self {
        Self {
            separator: '_',
            max_len_bytes: M1_MAX_LEN,
            max_len_utf16: M1_MAX_LEN,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{M1_MAX_LEN, Policy};

    #[test]
    fn default_is_the_m1_shape() {
        let p = Policy::default();
        assert_eq!(p.separator, '_');
        assert_eq!(p.max_len_bytes, M1_MAX_LEN);
        assert_eq!(p.max_len_utf16, M1_MAX_LEN);
    }

    #[test]
    fn a_keep_class_separator_is_accepted() {
        let p = Policy::new('_', M1_MAX_LEN, M1_MAX_LEN).expect("'_' is Keep-class");
        assert_eq!(p.separator, '_');
    }

    /// C15: `separator: '/'` used to be constructible directly and broke
    /// `transform`'s no-separator guarantee (`transform("a b.txt")` became
    /// `"a/b.txt"`, a path separator manufactured inside one component).
    /// `Policy::new` must refuse it -- and everything else in the
    /// `Separator`/`Delete` classes -- rather than silently accepting it.
    #[test]
    fn a_separator_class_separator_is_rejected() {
        for c in ['/', '\\', ' ', '*', ':'] {
            assert_eq!(Policy::new(c, M1_MAX_LEN, M1_MAX_LEN), Err(c), "{c:?}");
        }
    }

    #[test]
    fn a_delete_class_separator_is_rejected() {
        for c in ['\0', '\n', '\u{7f}'] {
            assert_eq!(Policy::new(c, M1_MAX_LEN, M1_MAX_LEN), Err(c), "{c:?}");
        }
    }
}
