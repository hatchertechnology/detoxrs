//! The preview (proposal §2.2) and `--json` (plan §7.3).
//!
//! Data on stdout, diagnostics on stderr. `--json` is the only stable contract;
//! the human layout is fixed two columns with no `terminal_size` dependency
//! (decision recorded at M2).
//!
//! The rule this module exists to enforce: **an undecodable name is never
//! printed raw.** Invalid bytes become `<hh>`, and so does every character that
//! could mislead a reader of the preview — controls, bidi overrides, zero-width
//! characters, Tags, line/paragraph separators, and non-`U+0020` spaces (C7) —
//! so no filename can drive the terminal it is displayed on, reorder the text
//! around it, or forge an extra report row (§6.1). That applies to the JSON
//! output as well — a JSON string is not a safe place to put a raw control byte
//! either.

use crate::apply::{ItemResult, Summary};
use crate::fsops;
use detoxrs_core::pipeline::Unrepresentable;
use detoxrs_core::plan::{Conflict, EntryKind, Plan, PlanItem, Resolution, SkipReason};
use serde_json::json;
use std::ffi::OsStr;
// Both `Write`s: `io` for the output streams, `fmt` for building one line as a
// `String`. Writing to a `String` cannot fail, which is why those results are
// dropped rather than propagated.
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::Path;

/// Counts for the summary line.
#[derive(Debug, Default, Clone, Copy)]
struct Tally {
    rename: usize,
    unchanged: usize,
    skipped: usize,
    conflicts: usize,
    not_utf8: usize,
}

impl Tally {
    fn of(items: &[PlanItem]) -> Self {
        let mut t = Self::default();
        for i in items {
            match i.resolution {
                Resolution::Rename => t.rename += 1,
                Resolution::Unchanged => t.unchanged += 1,
                Resolution::Skipped(reason) => {
                    t.skipped += 1;
                    if matches!(reason, SkipReason::NotUtf8) {
                        t.not_utf8 += 1;
                    }
                }
                Resolution::Conflict(_) => t.conflicts += 1,
            }
        }
        t
    }
}

/// Print the whole preview: the per-directory item lines, the summary, and the
/// footer stating plainly that nothing was changed.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn preview(w: &mut impl Write, plan: &Plan, show_unchanged: bool) -> io::Result<()> {
    items(w, &plan.items, show_unchanged)?;
    let t = Tally::of(&plan.items);
    writeln!(
        w,
        "\n{} to rename, {} unchanged, {} skipped, {} conflicts.",
        t.rename, t.unchanged, t.skipped, t.conflicts
    )?;
    writeln!(w, "Nothing was changed. Re-run with -x to apply.")?;
    not_utf8_hint(w, t)
}

/// The closing report of an `-x` run.
///
/// The batch id goes in the output rather than only in the journal filename: a
/// user who has just renamed 400 files needs the one string that undoes it, and
/// making them go and look for it is how `undo` ends up unused. `journal` is
/// `None` when the run had nothing to rename and so deliberately opened no
/// journal — printing an undo command for a batch that does not exist would be
/// worse than printing nothing.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn applied(
    w: &mut impl Write,
    plan: &Plan,
    s: &Summary,
    journal: Option<(&str, &str)>,
) -> io::Result<()> {
    let t = Tally::of(&plan.items);
    // Items the plan never intended to rename are reported the same way the
    // preview reports them, because "why was this left alone" is the same
    // question after the fact as before it.
    let untouched: Vec<PlanItem> = plan
        .items
        .iter()
        .filter(|i| !matches!(i.resolution, Resolution::Rename | Resolution::Unchanged))
        .cloned()
        .collect();
    items(w, &untouched, false)?;

    writeln!(
        w,
        // Same field order as the preview's summary, with `failed` appended, so
        // the two lines read the same way (§2.2).
        "\n{} renamed, {} unchanged, {} skipped, {} conflicts, {} failed.",
        s.renamed, t.unchanged, t.skipped, t.conflicts, s.failed
    )?;
    if let Some(why) = &s.aborted {
        writeln!(w, "The batch stopped early: {why}")?;
    }
    if let Some((batch, path)) = journal {
        writeln!(w, "Undo with: detoxrs undo {batch}")?;
        writeln!(w, "Journal: {path}")?;
    }
    not_utf8_hint(w, t)
}

fn not_utf8_hint(w: &mut impl Write, t: Tally) -> io::Result<()> {
    if t.not_utf8 > 0 {
        writeln!(
            w,
            "{} name(s) were skipped as not-valid-UTF-8: fix the encoding with convmv, then re-run.",
            t.not_utf8
        )?;
    }
    Ok(())
}

/// The item lines, grouped under a header per directory.
///
/// Also used to show what an `--on-collision fail` batch refusal was about, so
/// the reason travels with the error instead of being described in prose.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn items(w: &mut impl Write, items: &[PlanItem], show_unchanged: bool) -> io::Result<()> {
    let shown: Vec<&PlanItem> = items
        .iter()
        .filter(|i| show_unchanged || i.resolution != Resolution::Unchanged)
        .collect();

    let mut current: Option<&Path> = None;
    let mut width = 0;
    for (i, item) in shown.iter().enumerate() {
        if current != Some(item.dir.as_path()) {
            current = Some(item.dir.as_path());
            // Width is the widest source name in this group, so the arrows line
            // up per directory rather than across the whole tree.
            width = shown[i..]
                .iter()
                .take_while(|o| o.dir == item.dir)
                .map(|o| display_name(&o.from, o.kind).chars().count())
                .max()
                .unwrap_or(0);
            let dir = item.dir.as_os_str();
            // A bare relative argument (`detoxrs file.txt`) has an empty parent.
            let header = if dir.is_empty() {
                ".".to_owned()
            } else {
                escape(dir)
            };
            writeln!(w, "{header}")?;
        }
        writeln!(w, "  {}", line(item, width))?;
    }
    Ok(())
}

/// One item line, without its indent.
fn line(item: &PlanItem, width: usize) -> String {
    let from = display_name(&item.from, item.kind);
    let pad = width.saturating_sub(from.chars().count());
    let (arrow, rest) = match item.resolution {
        Resolution::Rename => ("->", display_name(&item.to, item.kind)),
        Resolution::Unchanged => ("= ", "(unchanged)".to_owned()),
        Resolution::Skipped(reason) => ("- ", format!("skipped ({})", skip_note(reason))),
        Resolution::Conflict(c) => ("! ", format!("conflict ({})", conflict_note(c))),
    };
    let mut out = format!("{from}{:pad$}  {arrow}  {rest}", "", pad = pad);
    if let Some(kind) = kind_note(item.kind) {
        let _ = write!(out, " [{kind}]");
    }
    // O2-5: the one transformation that destroys information (stage 12
    // shortening the name) is the one the report used to have no way to
    // mention. `to` alone cannot tell a reader "spaces became underscores"
    // apart from "50 characters were deleted from the end of the name".
    if item.truncated {
        let _ = write!(out, " [name shortened to fit the length limit]");
    }
    // Files only: every directory has nlink >= 2 by construction (`.` and its
    // parent's entry), so noting it there would tag every directory in every
    // preview with a warning that means nothing.
    if item.kind == EntryKind::File && item.ident.nlink > 1 {
        // §5.6: renaming one link renames one directory entry; the other links
        // keep the old name. That is rename(2), not a choice, so it is a note.
        let _ = write!(out, " [hardlink, nlink={}]", item.ident.nlink);
    }
    out
}

/// Directories get a trailing `/`; everything else is shown as-is.
fn display_name(name: &OsStr, kind: EntryKind) -> String {
    let mut s = escape(name);
    if kind == EntryKind::Dir {
        s.push('/');
    }
    s
}

/// Entry kind, for anything that is not a regular file or directory (§5.6).
///
/// This is the information detox's `--special` flag was withholding: instead of
/// silently refusing to touch a symlink or a FIFO, we say what it is.
const fn kind_note(kind: EntryKind) -> Option<&'static str> {
    match kind {
        EntryKind::File | EntryKind::Dir => None,
        EntryKind::Symlink => Some("symlink"),
        EntryKind::Other => Some("special"),
    }
}

const fn skip_note(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::NotUtf8 => "name is not valid UTF-8; detoxrs does not guess encodings",
        SkipReason::Unrepresentable(Unrepresentable::ReducesToEmpty) => "nothing safe would remain",
        SkipReason::Unrepresentable(Unrepresentable::ReducesToDotOrDotDot) => {
            "would become . or .."
        }
        SkipReason::Unrepresentable(Unrepresentable::NotConverged) => {
            "transform did not reach a fixed point"
        }
    }
}

const fn conflict_note(c: Conflict) -> &'static str {
    match c {
        Conflict::IntraBatch => "another entry in this directory wants the same name",
        Conflict::PreExisting => "that name is already taken",
        Conflict::Unresolvable => "no free -N suffix below 1000",
        Conflict::TruncationCollision => {
            "shortening this name to fit the length limit made it collide with another entry"
        }
    }
}

/// The machine-readable form. Data on stdout; nothing else goes there.
///
/// `outcomes` is `None` for a preview and `Some` after an `-x` run, one entry per
/// plan item in the same order. The `applied` field is derived from which it is,
/// so a consumer can never read a preview as a completed run.
///
/// `journal` mirrors `applied`'s parameter of the same name: `Some((batch,
/// path))` for a run that opened a journal, `None` for a preview or a run with
/// nothing to rename. C15: the human report has always printed the batch id
/// because "a user who has just renamed 400 files needs the one string that
/// undoes it" (see `applied`) — a `--json` consumer that applies a batch is that
/// same user, and without this it had no way to learn what to pass to `undo`
/// short of `undo --list` plus a guess.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn json(
    w: &mut impl Write,
    plan: &Plan,
    outcomes: Option<&[ItemResult]>,
    journal: Option<(&str, &str)>,
) -> io::Result<()> {
    let t = Tally::of(&plan.items);
    let items: Vec<_> = plan
        .items
        .iter()
        .enumerate()
        .map(|(n, i)| {
            let outcome = outcomes.and_then(|o| o.get(n));
            json!({
                "result": outcome.map(|o| match o {
                    ItemResult::Renamed => "renamed",
                    ItemResult::NotAttempted => "not_attempted",
                    ItemResult::Failed(_) => "failed",
                }),
                "error": match outcome {
                    Some(ItemResult::Failed(why)) => Some(why.as_str()),
                    _ => None,
                },
                "dir": escape(i.dir.as_os_str()),
                "from": escape(&i.from),
                "to": escape(&i.to),
                // Whether the strings above are the exact name or an escaped
                // rendering of one. A consumer that needs the raw bytes must not
                // be able to mistake one for the other.
                "utf8": i.from.to_str().is_some(),
                "kind": match i.kind {
                    EntryKind::File => "file",
                    EntryKind::Dir => "dir",
                    EntryKind::Symlink => "symlink",
                    EntryKind::Other => "other",
                },
                "depth": i.depth,
                "nlink": i.ident.nlink,
                // O2-5: whether stage 12 shortened `to` to fit the length
                // limit. The only field that says information was destroyed
                // rather than rearranged; a consumer diffing `from`/`to`
                // cannot otherwise tell a rewrite from a truncation.
                "truncated": i.truncated,
                "resolution": match i.resolution {
                    Resolution::Rename => "rename",
                    Resolution::Unchanged => "unchanged",
                    Resolution::Skipped(_) => "skipped",
                    Resolution::Conflict(_) => "conflict",
                },
                "note": match i.resolution {
                    Resolution::Skipped(r) => Some(skip_note(r)),
                    Resolution::Conflict(c) => Some(conflict_note(c)),
                    Resolution::Rename | Resolution::Unchanged => None,
                },
            })
        })
        .collect();

    let doc = json!({
        "schema": 1,
        "applied": outcomes.is_some(),
        // C15: null unless this run actually opened a journal (a preview, and
        // an `-x` run with nothing to rename, both pass `None`) — printing an
        // undo command for a batch that does not exist would be worse than
        // omitting it, same rule `applied` follows for the human report.
        "batch": journal.map(|(batch, _)| batch),
        "journal": journal.map(|(_, path)| path),
        // Which guarantee this run actually had (§5.4): the atomic no-clobber
        // rename, or the demoted check-then-rename with its documented window.
        // Reported rather than assumed, because a consumer auditing a batch needs
        // to know which one it got.
        "atomicity": fsops::atomicity(),
        "summary": {
            "to_rename": t.rename,
            "unchanged": t.unchanged,
            "skipped": t.skipped,
            "conflicts": t.conflicts,
            "renamed": outcomes.map(|o| o.iter().filter(|r| **r == ItemResult::Renamed).count()),
            "failed": outcomes.map(|o| {
                o.iter().filter(|r| matches!(r, ItemResult::Failed(_))).count()
            }),
        },
        "items": items,
    });
    serde_json::to_writer_pretty(&mut *w, &doc).map_err(io::Error::other)?;
    writeln!(w)
}

/// Render a name for display: exact text where it is text, `<hh>` where it is
/// not, and `<hh>`/`<u+XXXX>` for anything that could mislead a reader either
/// way (see `needs_escape`, C7).
///
/// This is the *only* place a name becomes a printable string, and it is display
/// code — the one place `to_string_lossy`-shaped conversion is permitted, and
/// even here nothing is lost, because the escapes are reversible: a literal `<`
/// is escaped too (as `<3c>`), so every `<` in the output starts a genuine
/// escape token and none can originate from the name itself (C10). Without
/// that, a file named literally `a<0a>b.txt` and one named `a<NEWLINE>b.txt`
/// rendered identically, which made the mapping lossy in exactly the case the
/// paragraph above claims it is not.
#[must_use]
pub fn escape(name: &OsStr) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        escape_bytes(name.as_bytes())
    }
    #[cfg(not(unix))]
    {
        // Windows is best-effort (owner decision): `OsStr` there is WTF-8 with
        // no stable accessor for its bytes, so an unpaired surrogate renders as
        // U+FFFD rather than `<hh>`. It is still never raw.
        escape_text(&name.to_string_lossy())
    }
}

#[cfg(unix)]
fn escape_bytes(mut rest: &[u8]) -> String {
    let mut out = String::with_capacity(rest.len());
    loop {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                out.push_str(&escape_text(text));
                return out;
            }
            Err(e) => {
                let (good, bad) = rest.split_at(e.valid_up_to());
                out.push_str(&escape_text(&String::from_utf8_lossy(good)));
                let len = e.error_len().unwrap_or(bad.len());
                for b in &bad[..len] {
                    let _ = write!(out, "<{b:02x}>");
                }
                rest = &bad[len..];
            }
        }
    }
}

/// Escape the control characters — and the literal `<` — in text that is
/// already valid.
///
/// `<` is not a control character, but every escape produced below is
/// `<...>`-shaped, so a literal `<` in the name would look exactly like the
/// start of one and make two distinct names render identically (C10). Escaping
/// it here too keeps the mapping injective: a `<` can only ever appear in the
/// output as the start of a genuine escape token. `>` needs no such treatment —
/// it only becomes ambiguous in combination with a literal `<`, which is now
/// always escaped away.
fn escape_text(s: &str) -> String {
    if !s.chars().any(needs_escape) {
        return s.to_owned();
    }
    s.chars()
        .map(|c| {
            if c == '<' {
                "<3c>".to_owned()
            } else if !needs_escape(c) {
                c.to_string()
            } else if c.is_ascii() {
                format!("<{:02x}>", c as u32)
            } else {
                format!("<u+{:04x}>", c as u32)
            }
        })
        .collect()
}

/// C7: does displaying `c` raw risk misleading the reader of the preview?
///
/// The preview is the tool's only safety control before `-x` (§2.2), so this is
/// a stricter question than stage 4's "does the *transform* touch this". Three
/// ways a character can mislead a reader, and this covers all three:
///
/// - **invisible or reordering** — `char::is_control` (`Cc`) plus
///   `detoxrs_core::invisible::is_invisible` (`Cf` bidi controls/marks,
///   zero-width characters, and Tags: exactly stage 4's named set, including
///   the CVE-2021-42574 bidi overrides). Reusing that predicate instead of a
///   second hand-rolled list is deliberate: it is the one place in the codebase
///   that already enumerates "invisible", and duplicating the enumeration would
///   let the two drift.
/// - **row-forging** — `Zl`/`Zp` (`U+2028`/`U+2029`), which split one report
///   line into what a downstream line-splitter reads as several (§4.1's
///   "report-row forgery").
/// - **mistaken for formatting** — `Zs` other than plain `U+0020`: a character
///   that renders as blank space but is not the space the padding/columns in
///   `line()` are computed from.
///
/// Explicitly **excluded**: `Co` (private use). A private-use character is not
/// invisible, does not reorder anything, and does not impersonate a space or a
/// column — it renders as an unknown-glyph box in most fonts, the same harmless
/// failure mode as any other character the local font lacks. `Cs` cannot appear
/// here at all: a `char` cannot hold a surrogate, and a byte sequence encoding
/// one is not valid UTF-8, so `escape_bytes` already turns it into `<hh>` before
/// this function ever sees it.
///
/// This does **not** change what stage 4 or stage 7 do to a name (`invisible.rs`,
/// `classes.rs`): `Zl`/`Zp`/`Zs` passing through the *transform* untouched is a
/// documented M4 deferral, a deliberate roadmap decision. This function only
/// decides what the *display* layer prints for whatever the transform left in
/// place.
fn needs_escape(c: char) -> bool {
    c.is_control()
        || c == '<'
        || detoxrs_core::invisible::is_invisible(c)
        || matches!(c, '\u{2028}' | '\u{2029}')
        || is_unusual_space(c)
}

/// Unicode `Zs` (space separator), minus plain `U+0020`. Named ranges, same
/// style as `invisible.rs`: the full `Zs` set is small and stable enough that a
/// generated UCD table would be overkill here.
const fn is_unusual_space(c: char) -> bool {
    matches!(
        c,
        '\u{00a0}' // NO-BREAK SPACE
        | '\u{1680}' // OGHAM SPACE MARK
        | '\u{2000}'
            ..='\u{200a}' // EN QUAD .. HAIR SPACE
        | '\u{202f}' // NARROW NO-BREAK SPACE
        | '\u{205f}' // MEDIUM MATHEMATICAL SPACE
        | '\u{3000}' // IDEOGRAPHIC SPACE
    )
}

#[cfg(test)]
mod tests {
    use super::escape;
    use std::ffi::OsStr;

    #[test]
    fn valid_text_is_unchanged() {
        assert_eq!(escape(OsStr::new("Björk_-_Volta.mp3")), "Björk_-_Volta.mp3");
    }

    #[test]
    fn control_characters_are_escaped() {
        assert_eq!(escape(OsStr::new("Icon\r")), "Icon<0d>");
    }

    #[cfg(unix)]
    #[test]
    fn invalid_bytes_become_hex_escapes() {
        use std::os::unix::ffi::OsStrExt as _;
        let raw = OsStr::from_bytes(b"Bj\xf6rk - Vespertine.mp3");
        assert_eq!(escape(raw), "Bj<f6>rk - Vespertine.mp3");
    }

    #[cfg(unix)]
    #[test]
    fn a_run_of_invalid_bytes_escapes_each_one() {
        use std::os::unix::ffi::OsStrExt as _;
        assert_eq!(escape(OsStr::from_bytes(b"\xff\xfe")), "<ff><fe>");
    }

    /// C7: every character the consolidator found reaching the terminal raw
    /// must now come out as an escape token, not as the character itself. This
    /// is the exhaustive table from the consolidated review, minus the two
    /// (`Cc`) that were already covered.
    #[test]
    fn c7_invisible_reordering_and_space_lookalike_characters_are_escaped() {
        let cases: &[(char, &str)] = &[
            ('\u{202e}', "<u+202e>"),   // RLO (bidi override, CVE-2021-42574)
            ('\u{202d}', "<u+202d>"),   // LRO
            ('\u{200b}', "<u+200b>"),   // ZWSP
            ('\u{200d}', "<u+200d>"),   // ZWJ
            ('\u{061c}', "<u+061c>"),   // ALM
            ('\u{e0041}', "<u+e0041>"), // Tag
            ('\u{2028}', "<u+2028>"),   // LINE SEPARATOR
            ('\u{2029}', "<u+2029>"),   // PARAGRAPH SEPARATOR
            ('\u{00a0}', "<u+00a0>"),   // NO-BREAK SPACE
            ('\u{3000}', "<u+3000>"),   // IDEOGRAPHIC SPACE
        ];
        for (c, token) in cases {
            let name = format!("a{c}b.txt");
            assert_eq!(
                escape(OsStr::new(&name)),
                format!("a{token}b.txt"),
                "char U+{:04x}",
                *c as u32
            );
        }
    }

    /// A plain space is not escaped: only the *other* `Zs` characters are.
    #[test]
    fn plain_ascii_space_is_left_alone() {
        assert_eq!(escape(OsStr::new("a b.txt")), "a b.txt");
    }

    /// Private use (`Co`) is deliberately not escaped: it is not invisible,
    /// does not reorder, and does not impersonate a space — see `needs_escape`.
    #[test]
    fn private_use_characters_are_not_escaped() {
        assert_eq!(escape(OsStr::new("a\u{e000}b.txt")), "a\u{e000}b.txt");
    }

    /// C7 + C10 together: widening which characters get escaped must not
    /// collapse two distinct names onto the same rendering. Two different
    /// "misleading" characters, and a name using one of the new escape tokens
    /// literally, must all render distinctly from each other.
    #[test]
    fn widened_escaping_stays_injective() {
        let rlo = OsStr::new("a\u{202e}b.txt");
        let nbsp = OsStr::new("a\u{00a0}b.txt");
        let literal_token = OsStr::new("a<u+202e>b.txt");
        let rendered_rlo = escape(rlo);
        let rendered_nbsp = escape(nbsp);
        let rendered_literal = escape(literal_token);
        assert_ne!(rendered_rlo, rendered_nbsp);
        assert_ne!(rendered_rlo, rendered_literal);
        assert_eq!(rendered_rlo, "a<u+202e>b.txt");
        // The literal text of the escape token has its own `<` escaped, so it
        // is NOT the same string as the real character's rendering above.
        assert_eq!(rendered_literal, "a<3c>u+202e>b.txt");
    }

    /// C10: a real control character and the literal text of its own escape
    /// sequence must not render the same way. Before the fix both
    /// `escape(OsStr::new("a\nb.txt"))` and `escape(OsStr::new("a<0a>b.txt"))`
    /// produced `"a<0a>b.txt"` — two genuinely distinct files, one rendering.
    #[test]
    fn distinct_inputs_render_distinctly() {
        let real_control_char = OsStr::new("a\nb.txt");
        let literal_escape_text = OsStr::new("a<0a>b.txt");
        assert_ne!(escape(real_control_char), escape(literal_escape_text));
    }
}
