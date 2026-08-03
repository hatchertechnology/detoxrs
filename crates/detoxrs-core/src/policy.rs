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
    /// Maximum name length in UTF-8 bytes. `Policy::default` hardcodes 255
    /// (ext4's limit) on non-macOS targets and `usize::MAX` (not this
    /// platform's constraint, per C-3) on macOS; `statfs`-derived per
    /// directory from M5.
    pub max_len_bytes: usize,
    /// Maximum name length in UTF-16 code units. `Policy::default` hardcodes
    /// 255 (APFS's limit, empirically established in doc 06 Test 1) on
    /// macOS and `usize::MAX` (not this platform's constraint, per C-3)
    /// elsewhere.
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
/// UTF-16 code units). M5 deletes this constant in the same commit that adds
/// `statfs` detection.
pub const M1_MAX_LEN: usize = 255;

/// C-3: applying `M1_MAX_LEN` to *both* fields at once was the bug, not a
/// stand-in for it. ext4 does not look at UTF-16 units at all, and APFS does
/// not look at byte count at all -- each platform has exactly one binding
/// axis. Binding both unconditionally makes the byte cap fire on APFS for any
/// name with a multi-byte character well inside the filesystem's own 255-unit
/// limit, which is `Policy::default`'s whole job to not do: turning "clean
/// this name" into "shorten this name" for a name the filesystem in front of
/// the user would have accepted untouched, and -- because truncation is not a
/// conservative direction once two truncated prefixes coincide -- silently
/// colliding two legal, distinct files into one write.
///
/// The default is therefore per-target-OS rather than one number: the axis
/// that is not the current platform's constraint is set to `usize::MAX`,
/// which does not weaken the safety closure (`fits` still requires both
/// fields, and the platform's real limit is still the one enforced) -- it
/// just stops a limit that was never the filesystem's own from binding first.
/// M1 has no `statfs`, so `target_os` is the stand-in for "which filesystem
/// is this" until M5 replaces it with the real probe.
#[cfg(target_os = "macos")]
const fn default_max_len_bytes() -> usize {
    usize::MAX
}
#[cfg(not(target_os = "macos"))]
const fn default_max_len_bytes() -> usize {
    M1_MAX_LEN
}

/// See [`default_max_len_bytes`].
#[cfg(target_os = "macos")]
const fn default_max_len_utf16() -> usize {
    M1_MAX_LEN
}
#[cfg(not(target_os = "macos"))]
const fn default_max_len_utf16() -> usize {
    usize::MAX
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            separator: '_',
            max_len_bytes: default_max_len_bytes(),
            max_len_utf16: default_max_len_utf16(),
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
        // C-3: only one axis binds by default, and it is the axis this build's
        // target platform's own filesystem actually cares about. The other
        // axis is `usize::MAX`, not `M1_MAX_LEN` -- asserting both `== 255` is
        // exactly the bug this test used to enshrine.
        #[cfg(target_os = "macos")]
        {
            assert_eq!(p.max_len_bytes, usize::MAX);
            assert_eq!(p.max_len_utf16, M1_MAX_LEN);
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert_eq!(p.max_len_bytes, M1_MAX_LEN);
            assert_eq!(p.max_len_utf16, usize::MAX);
        }
    }

    /// C-3: the default limit must never let a name through that violates the
    /// platform's own real limit, even though the *other* axis no longer
    /// binds. This is the safety half of the fix -- relaxing the axis that
    /// was never the constraint must not turn into relaxing the axis that is.
    #[test]
    fn the_platform_axis_still_binds_by_default() {
        let p = Policy::default();
        #[cfg(target_os = "macos")]
        assert_eq!(p.max_len_utf16, 255);
        #[cfg(not(target_os = "macos"))]
        assert_eq!(p.max_len_bytes, 255);
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
