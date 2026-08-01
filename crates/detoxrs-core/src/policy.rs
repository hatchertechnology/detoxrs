//! The resolved transform policy (proposal §3.1, plan §5.1).
//!
//! Every field is concrete by the time it reaches `transform`: there is no
//! `0 = auto` sentinel in this type, because resolving `auto` into a number is a
//! walk-time concern in the binary crate and this crate has no filesystem.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Policy {
    /// Replacement for separator-class characters (§3.7). `_` in M1;
    /// `--separator` arrives with the config file at M3.
    pub separator: char,
    /// Maximum name length in UTF-8 bytes. M1 hardcodes 255 (ext4's limit);
    /// `statfs`-derived per directory from M5.
    pub max_len_bytes: usize,
    /// Maximum name length in UTF-16 code units. M1 hardcodes 255 (APFS's
    /// limit, empirically established in doc 06 Test 1).
    pub max_len_utf16: usize,
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
}
