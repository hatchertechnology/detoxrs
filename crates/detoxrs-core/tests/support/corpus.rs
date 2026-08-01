//! The fixture corpus, per plan §6 and proposal §8.3.
//!
//! Byte-string constants, never checked-in files whose *name* is the payload:
//! several entries here are deliberately not valid UTF-8, and APFS refuses to
//! create such a filename at all (EILSEQ/errno 92, verified 2026-07-31 on both
//! `b"bad\xffname.txt"` and `b"Bj\xf6rk - Vespertine.mp3"`). A `b"..."` literal
//! is diffable, holds arbitrary bytes, and exists on every platform.
//!
//! Every entry carries `disk_constructible_everywhere`. Pure, property and
//! snapshot tests ignore the flag and never touch a filesystem; the
//! filesystem-matrix tests (work package 5b) filter on it and must log each skip.

/// One corpus entry.
///
/// `bytes` is owned rather than `&'static [u8]` so generated entries (the
/// 300-byte name, the 128 astral emoji) are ordinary function results. Plan §6:
/// a `const` plus `leak()` is a memory leak dressed as a constant.
pub struct Entry {
    /// Stable identifier, used as the snapshot key. Never a filename.
    pub id: &'static str,
    /// The raw name bytes, exactly as they would appear in a directory entry.
    pub bytes: Vec<u8>,
    /// False when some tier-1 platform refuses to create this name at all.
    pub disk_constructible_everywhere: bool,
    /// Why the flag is false, or what the entry is for when it is true.
    pub note: &'static str,
}

fn entry(id: &'static str, bytes: &[u8], constructible: bool, note: &'static str) -> Entry {
    Entry {
        id,
        bytes: bytes.to_vec(),
        disk_constructible_everywhere: constructible,
        note,
    }
}

/// `n` repetitions of `c`, UTF-8 encoded.
///
/// Plan §6's correction: the repeated-emoji fixture is generated here rather
/// than stored as leaked `const` data.
#[must_use]
pub fn repeated(c: char, n: usize) -> Vec<u8> {
    let mut s = String::new();
    for _ in 0..n {
        s.push(c);
    }
    s.into_bytes()
}

/// The whole corpus, in a stable order (the snapshot key depends on it).
#[must_use]
#[allow(clippy::too_many_lines)] // A fixture table. Splitting it would only hide entries.
pub fn all() -> Vec<Entry> {
    let mut v = vec![
        entry(
            "cafe_nfc",
            b"caf\xc3\xa9.txt",
            true,
            "cafe.txt with a precomposed U+00E9 (NFC)",
        ),
        entry(
            "cafe_nfd",
            b"cafe\xcc\x81.txt",
            true,
            "same name decomposed (e + U+0301); stage 3 must recompose it",
        ),
        entry(
            "bidi_rlo",
            b"invoice\xe2\x80\xae\x66dp.txt",
            true,
            "U+202E RIGHT-TO-LEFT OVERRIDE: the CVE-2021-42574 class",
        ),
        entry(
            "zero_width_space",
            b"in\xe2\x80\x8bvisible.txt",
            true,
            "U+200B ZERO WIDTH SPACE",
        ),
        entry(
            "zwj_emoji_family",
            "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f466}.png".as_bytes(),
            true,
            "one grapheme cluster held together by two U+200D joiners",
        ),
        entry(
            "unicode_tag",
            "hidden\u{e0041}.txt".as_bytes(),
            true,
            "U+E0041, a Unicode Tag character (detox #120)",
        ),
        entry("con_txt", b"CON.txt", false, "reserved stem on Windows"),
        entry("hidden_file", b".hidden file", true, "dotfile with a space"),
        entry(
            "weird_dots",
            b"..weird..name..",
            true,
            "leading, interior and trailing dot runs",
        ),
        entry(
            "pct20",
            b"100%20done.txt",
            true,
            "well-formed escape; stage 2 is M2, so M1 leaves the % literal",
        ),
        entry(
            "pct25",
            b"100%25 done.txt",
            true,
            "escaped percent plus space",
        ),
        entry(
            "pct_malformed",
            b"50%-70%.txt",
            true,
            "malformed escapes: stage 2's all-or-nothing case (M2)",
        ),
        entry(
            "libstdcpp",
            b"libstdc++.so",
            true,
            "+ is keep-class; plus_to_space is off",
        ),
        entry(
            "music_separator",
            b"a_-_b.mp3",
            true,
            "detox #121: no run here is longer than one character",
        ),
        entry(
            "icon_cr",
            b"Icon\r",
            false,
            "macOS-origin name with a literal CR; Windows rejects CR outright",
        ),
        entry(
            "lone_dash",
            b"-",
            true,
            "a name that is a single leading dash",
        ),
        entry(
            "all_punctuation",
            b"***",
            true,
            "§3.14's worked example: Unrepresentable(ReducesToEmpty)",
        ),
        entry("dot", b".", true, "the current directory's own name"),
        entry("dotdot", b"..", true, "the parent directory's own name"),
        entry(
            "shell_metachars",
            b"a & b (1985) [720p].mkv",
            true,
            "separator-class run coverage for stages 7 and 9",
        ),
        entry(
            "bjork_cp1252",
            b"Bj\xf6rk - Vespertine.mp3",
            false,
            "CP1252 bytes: not valid UTF-8, so Opaque. APFS refuses it (EILSEQ)",
        ),
        entry(
            "invalid_lone_ff",
            b"bad\xffname.txt",
            false,
            "lone 0xff: not valid UTF-8, so Opaque. APFS refuses it (EILSEQ)",
        ),
    ];

    // Generated entries. Both are `false` because their job is exercising
    // truncation in memory, not proving a filesystem accepts an oversized name.
    let mut ascii_300 = repeated('a', 296);
    ascii_300.extend_from_slice(b".txt");
    v.push(entry(
        "ascii_300",
        &ascii_300,
        false,
        "300 bytes of ASCII: over every tier-1 limit",
    ));
    v.push(entry(
        "astral_emoji_128",
        &repeated('\u{1f600}', 128),
        false,
        "128 astral emoji = 256 UTF-16 units and 512 bytes: over BOTH limits",
    ));
    v
}
