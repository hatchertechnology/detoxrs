//! The preview (proposal §2.2) and `--json` (plan §7.3).
//!
//! Data on stdout, diagnostics on stderr. `--json` is the only stable contract;
//! the human layout is fixed two columns with no `terminal_size` dependency
//! (decision recorded at M2).
//!
//! The rule this module exists to enforce: **an undecodable name is never
//! printed raw.** Invalid bytes become `<hh>`, and control characters do too, so
//! no filename can drive the terminal it is displayed on (§6.1). That applies to
//! the JSON output as well — a JSON string is not a safe place to put a raw
//! control byte either.

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
/// making them go and look for it is how `undo` ends up unused.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn applied(
    w: &mut impl Write,
    plan: &Plan,
    s: &Summary,
    batch: &str,
    journal_path: &str,
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
    writeln!(w, "Undo with: detoxrs undo {batch}")?;
    writeln!(w, "Journal: {journal_path}")?;
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
    }
}

/// The machine-readable form. Data on stdout; nothing else goes there.
///
/// `outcomes` is `None` for a preview and `Some` after an `-x` run, one entry per
/// plan item in the same order. The `applied` field is derived from which it is,
/// so a consumer can never read a preview as a completed run.
///
/// # Errors
///
/// Propagates any write error from `w`.
pub fn json(w: &mut impl Write, plan: &Plan, outcomes: Option<&[ItemResult]>) -> io::Result<()> {
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
/// not, and `<hh>`/`<u+XXXX>` for control characters either way.
///
/// This is the *only* place a name becomes a printable string, and it is display
/// code — the one place `to_string_lossy`-shaped conversion is permitted, and
/// even here nothing is lost, because the escapes are reversible.
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

/// Escape the control characters in text that is already valid.
fn escape_text(s: &str) -> String {
    if !s.chars().any(char::is_control) {
        return s.to_owned();
    }
    s.chars()
        .map(|c| {
            if !c.is_control() {
                c.to_string()
            } else if c.is_ascii() {
                format!("<{:02x}>", c as u32)
            } else {
                format!("<u+{:04x}>", c as u32)
            }
        })
        .collect()
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
}
