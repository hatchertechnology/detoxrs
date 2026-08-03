//! detoxrs: make filenames sane. See
//! `docs/research/00-proposal-rust-detox-successor.md`.
//!
//! Two phases, never interleaved (§5.1). `walk::snapshot` freezes the entry list,
//! `plan::plan` decides every destination with no I/O at all, and only then does
//! anything move. Without `-x` the run stops after printing that plan; with `-x`
//! it opens a journal and hands the plan to `apply::run`.
//!
//! **The `if !exec` branch below is the only gate on the only call site of
//! `rename_noreplace`.** There is no second path to a rename in this crate:
//! `fsops` owns the syscall, `apply` is its only caller, and `main` is `apply`'s.
//!
//! Exit codes: `0` everything requested was done (a preview that merely
//! *reports* a conflict is still `0` — it changed nothing and was not asked
//! to); `1` an `-x` run (or `undo`) that could not do everything it was
//! asked: an item failed, the batch aborted part-way, a conflict was left
//! unresolved, a subtree the walk needed to see could not be read, or a
//! rename broke a relative symlink elsewhere in the tree (C-8, C-9); `2`
//! usage, walk, or plan error — a failure where nothing was attempted at
//! all.
//!
//! Unsafe-code policy: `forbid`, same as `detoxrs-core`. An earlier version of
//! this crate used `deny` to leave room for a hand-written macOS `libc` FFI
//! shim, on the premise that neither `rustix` nor `nix` exposed
//! `renamex_np`/`RENAME_EXCL`. That premise was false and is withdrawn:
//! `rustix::fs::renameat_with` with `RenameFlags::NOREPLACE`/`EXCHANGE` wraps
//! `renameatx_np` under `#[cfg(apple)]` and `renameat2` under
//! `#[cfg(linux_kernel)]`, so both no-clobber rename paths are reachable from
//! safe code (proposal §5.4, §7.2). docs.rs hid the Apple items because its
//! default render target is Linux. With no FFI shim planned, there is no
//! future `#[allow(unsafe_code)]` to leave room for, and `forbid` -- which
//! cannot be downgraded by a later `allow` -- is the honest attribute.
#![forbid(unsafe_code)]

mod apply;
mod cli;
mod fsops;
mod journal;
mod report;
mod walk;

use clap::Parser as _;
use detoxrs_core::plan::{Plan, PlanError, Resolution, plan};
use detoxrs_core::policy::Policy;
use fsops::PlatformRenameOps;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(msg) => {
            eprintln!("detoxrs: {msg}");
            // C12: every non-exit-2 path emits a JSON document under --json
            // (`report::json`), but this one used to short-circuit before any
            // JSON was written at all, leaving a machine consumer unable to
            // tell a refusal from a crash even though `--help` promises "JSON
            // on stdout, diagnostics on stderr" with no carve-out. `undo`'s
            // subcommand cannot reach here with `args.json` set --
            // `args_conflicts_with_subcommands` in `cli.rs` forbids combining
            // them -- so this is only the forward-run refusal paths (a walk,
            // plan, or output error). `writeln!` rather than `println!`:
            // the latter panics on a write failure, and a refusal that
            // itself hits a broken pipe must not turn into a panic.
            if args.json {
                let doc = serde_json::json!({ "schema": 1, "error": msg });
                drop(writeln!(io::stdout(), "{doc}"));
            }
            ExitCode::from(2)
        }
    }
}

/// The whole program. `Err` is exit 2: a failure before anything was attempted.
/// `Ok` carries 0 or 1, which is [`apply::Summary::exit_code`].
fn run(args: &cli::Cli) -> Result<u8, String> {
    if let Some(cli::Command::Undo(u)) = &args.command {
        return undo(u);
    }

    let snap = walk::snapshot(&args.paths, args.recursive).map_err(|e| e.to_string())?;
    let case = walk::volume_case(&snap.entries);
    let policy = Policy::default(); // M1 has no transform flags; M3's config file fills this in.

    let p = plan(&snap.entries, &policy, args.on_collision.into(), case).map_err(describe)?;

    if args.exec {
        exec(&p, &policy, args.json, args.quiet, &snap.unreadable)
    } else {
        preview(&p, args, &snap.unreadable)
    }
}

/// Print the plan and change nothing.
///
/// C-8: a plain preview reporting the conflicts it found is not an error --
/// that is the whole point of a preview -- so `report::Tally`'s conflict
/// count never affects the exit code here. A subtree the walk could not even
/// see is different: the plan printed is not the plan for the whole tree the
/// user named, so this exits `1` when `unreadable` is non-empty, the one way
/// a preview can fail to do what it was asked without an `-x` in sight.
fn preview(p: &Plan, args: &cli::Cli, unreadable: &[PathBuf]) -> Result<u8, String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if args.json {
        report::json(&mut out, p, None, None, unreadable, &[])
    } else if args.quiet {
        Ok(()) // --quiet is errors only, and a preview is not an error.
    } else {
        report::preview(&mut out, p, args.verbose > 0, unreadable)
    }
    .and_then(|()| out.flush())
    .map_err(|e| format!("cannot write output: {e}"))?;
    Ok(u8::from(!unreadable.is_empty()))
}

/// Open the journal, apply, and report.
///
/// The journal is opened **before** the first rename and its failure is exit 2
/// with nothing attempted, which is §5.8's rule stated as control flow: renaming
/// without a journal is the one outcome `undo` cannot fix.
fn exec(
    p: &Plan,
    policy: &Policy,
    as_json: bool,
    quiet: bool,
    unreadable: &[PathBuf],
) -> Result<u8, String> {
    // Nothing to rename means no journal, and that is not tidiness: an empty batch
    // would be the newest one, so `undo --last` would stop meaning "undo what I
    // just did" after any no-op `-x` run.
    if !p.items.iter().any(|i| i.resolution == Resolution::Rename) {
        return report_nothing(p, as_json, quiet, unreadable);
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut j = journal::Journal::create(policy, &cwd).map_err(|e| {
        format!("cannot open an undo journal ({e}); nothing was renamed. See detoxrs undo --list.")
    })?;
    let batch = j.id().to_owned();
    let where_ = j.path().display().to_string();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    // Progress lines go nowhere under --json (a JSON document and per-item lines
    // cannot share stdout) and nowhere under -q, which means errors only on the
    // write path exactly as it does on the read path. The renames still happen and
    // the failures still reach stderr; it is the reporting that is silenced.
    let mut sink: Vec<u8> = Vec::new();
    let s = if as_json || quiet {
        apply::run(&p.items, &PlatformRenameOps, &mut j, &mut sink)
    } else {
        apply::run(&p.items, &PlatformRenameOps, &mut j, &mut out)
    };

    // Close the journal before reporting: the terminal record is what tells a
    // later `undo` that this batch is not still running.
    if let Err(e) = j.finish() {
        eprintln!(
            "detoxrs: warning: could not close the undo journal ({e}); `undo` will treat this \
             batch as unfinished."
        );
    }

    let report_result = if as_json {
        report::json(
            &mut out,
            p,
            Some(&s.outcomes),
            Some((&batch, &where_)),
            unreadable,
            &s.broken_symlinks,
        )
    } else if quiet {
        Ok(())
    } else {
        report::applied(&mut out, p, &s, Some((&batch, &where_)), unreadable)
    }
    .and_then(|()| out.flush());

    // C6: this used to be `?`, so a broken stdout on the *closing* report
    // (`detoxrs -x -r . | head -1`) discarded `s.exit_code()` and returned
    // exit 2 -- the code this file and `--help` both document as "nothing was
    // attempted at all" -- after real renames had already happened and been
    // journalled. By the time this write is attempted the batch is over: the
    // renames it did are done, `apply::run`'s own progress writes are already
    // fault-tolerant (see the comment in `apply::attempt`'s `Ok` arm), so the
    // only thing this write failing can mean is that the closing summary
    // itself did not reach the user. `.max(1)`: a run that otherwise fully
    // succeeded (`exit_code() == 0`) still has to surface as "something is
    // wrong" when its own report never arrived, without claiming nothing was
    // attempted.
    //
    // C-8: `unreadable` folds in the same way -- a subtree the walk could not
    // see is exactly as much "something is wrong" as a report write that
    // failed, and neither one is allowed to erase a real `failed`/`aborted`
    // count by taking `max` in the other direction.
    let base = s.exit_code().max(u8::from(!unreadable.is_empty()));
    if let Err(e) = report_result {
        eprintln!("detoxrs: cannot write output: {e}");
        return Ok(base.max(1));
    }

    Ok(base)
}

/// An `-x` run that found nothing to rename: the same closing report, minus the
/// undo line, and no journal was opened.
///
/// C-8: "nothing to rename" is not the same claim as "nothing was wrong" -- a
/// batch that is entirely `--on-collision skip` conflicts never reaches
/// `apply::run` at all (the guard above short-circuits before it), so the
/// conflict count has to be worked out here, the same way `apply::run` works
/// it out for a batch that does have renames.
fn report_nothing(
    p: &Plan,
    as_json: bool,
    quiet: bool,
    unreadable: &[PathBuf],
) -> Result<u8, String> {
    let empty = apply::Summary {
        outcomes: vec![apply::ItemResult::NotAttempted; p.items.len()],
        conflicts: p
            .items
            .iter()
            .filter(|i| matches!(i.resolution, Resolution::Conflict(_)))
            .count(),
        ..apply::Summary::default()
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if as_json {
        report::json(
            &mut out,
            p,
            Some(&empty.outcomes),
            None,
            unreadable,
            &empty.broken_symlinks,
        )
    } else if quiet {
        Ok(())
    } else {
        report::applied(&mut out, p, &empty, None, unreadable)
    }
    .and_then(|()| out.flush())
    .map_err(|e| format!("cannot write output: {e}"))?;
    Ok(empty.exit_code().max(u8::from(!unreadable.is_empty())))
}

/// `detoxrs undo`.
fn undo(u: &cli::Undo) -> Result<u8, String> {
    let batches = journal::list().map_err(|e| format!("cannot read the journal directory: {e}"))?;

    if u.list {
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for b in &batches {
            let id = b.file_stem().unwrap_or(b.as_os_str());
            drop(writeln!(out, "{}", report::escape(id)));
        }
        if batches.is_empty() {
            drop(writeln!(out, "no recorded batches"));
        }
        return out.flush().map(|()| 0).map_err(|e| e.to_string());
    }

    let path = if u.last {
        resolve_last(&batches)?
    } else if let Some(id) = &u.batch_id {
        journal::path_of(id).map_err(|e| e.to_string())?
    } else {
        return Err(
            "undo needs a BATCH-ID or --last. `detoxrs undo --list` shows them.".to_owned(),
        );
    };

    let replay =
        journal::replay(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;

    // A journal that does not add up is reported in full and never silently worked
    // around: it is the only record of what happened.
    for a in &replay.anomalies {
        eprintln!("detoxrs: journal problem: {a}");
    }
    if let Some(item) = &replay.interrupted {
        // The crash protocol's whole promise, discharged in one line.
        eprintln!(
            "detoxrs: note: batch was interrupted while renaming {} -> {}; its outcome was never \
             recorded, so it is left alone. Check that name by hand.",
            report::escape(&item.original),
            report::escape(&item.current)
        );
    }
    if !replay.complete {
        eprintln!(
            "detoxrs: warning: batch {} has no completion record, so it either crashed or is \
             still running. If a detoxrs run is still in progress, its remaining items will not \
             be reverted and the tree will be left half-cleaned.",
            path.file_stem().unwrap_or_default().to_string_lossy()
        );
    }
    // Anything unexplained about the journal makes this a non-zero exit even when
    // every rename it did describe was reverted cleanly.
    let suspect = !replay.complete || replay.interrupted.is_some() || !replay.anomalies.is_empty();

    if replay.items.is_empty() {
        // C-11: a batch can have renamed items yet still resolve to zero
        // `UndoItem`s, when every one of them was dropped before this loop --
        // see `Replay::lost`. "Nothing to undo" would be a lie in that case.
        if replay.lost > 0 {
            println!(
                "nothing to undo in that batch: {} renamed item{} could not be undone at all.",
                replay.lost,
                if replay.lost == 1 { "" } else { "s" }
            );
        } else {
            println!("nothing to undo in that batch.");
        }
        return Ok(u8::from(suspect));
    }

    // An undo is itself a batch of renames, so it gets its own journal and can
    // itself be undone.
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut j = journal::Journal::create(&Policy::default(), &cwd)
        .map_err(|e| format!("cannot open an undo journal ({e}); nothing was reverted."))?;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let s = apply::undo(&replay.items, &PlatformRenameOps, &mut j, &mut out);
    if let Err(e) = j.finish() {
        eprintln!("detoxrs: warning: could not close the undo journal ({e}).");
    }
    // C-11: `s.renamed + s.failed` only covers items that reached this loop.
    // `replay.lost` is the rest of the batch -- renamed items whose own intent
    // record was dropped before the loop ever saw them -- and the tally must
    // say so instead of quietly adding up to less than the batch it describes.
    let lost_note = if replay.lost > 0 {
        format!(", {} could not be undone at all", replay.lost)
    } else {
        String::new()
    };
    drop(writeln!(
        out,
        "\n{} reverted, {} refused{lost_note}. This undo is itself batch {}.",
        s.renamed,
        s.failed,
        j.id()
    ));
    drop(out.flush());
    Ok(if suspect { 1 } else { s.exit_code() })
}

/// What `--last` means: the newest journal that either completed a rename or
/// has one whose outcome is still unknown, not merely the newest file by
/// name.
///
/// C7 (three routes, one root cause). `batches.last()` used to be the whole
/// answer, but a journal can exist and sort newest while describing nothing
/// that needs attention: an all-refused `undo` writes its own journal with
/// nothing in it; a forward `-x` run in which every item failed writes one
/// too (its `intent`/`failed` records exist, but nothing was ever `done`);
/// and a concurrent run's journal has no `done` record yet the instant after
/// `Journal::create` returns. Any of the three sorting newest silently
/// shadows the real batch underneath it -- the forward path already carries
/// this exact reasoning for the empty-plan case (`exec`, above: "an empty
/// batch would be the newest one, so `undo --last` would stop meaning 'undo
/// what I just did'"), and this is that same guard, generalized to the place
/// all three routes actually share: eligibility for `--last`, not any one of
/// the call sites that can produce an empty journal.
///
/// **`items.is_empty()` alone is the wrong test (a real regression a
/// reviewer caught).** A batch that crashed *before* its first `done` has an
/// `intent` with no outcome -- `replay.items` is empty exactly like the
/// all-failed and all-refused cases, but `replay.interrupted` is `Some(..)`:
/// a real rename whose fate on disk is unknown. Skipping that batch the same
/// way falls through to an older, already-clean batch, silently reverts
/// *that* instead, and never mentions the crash at all -- worse than the
/// pre-fix behaviour, which at least landed on the crashed batch and exited
/// 1 with a warning. "Recorded work whose outcome is unknown" and "recorded
/// no work at all" are different states; only the second is skipped here.
/// A batch is eligible when it has a completed rename *or* an unresolved
/// one; only all-failed, all-refused and no-op journals -- which have
/// neither -- are skipped.
///
/// A journal that is skipped here is not deleted or hidden -- explicit
/// `detoxrs undo <BATCH-ID>` still reaches it and reports "nothing to undo in
/// that batch" exactly as before -- it is only skipped when resolving
/// *`--last`*, so an older real batch underneath it stays reachable instead
/// of being permanently shadowed by an empty file that happens to sort after
/// it.
///
/// This also covers the crash-mid-batch case
/// (`undoing_an_unfinished_batch_warns_and_does_not_report_success`) whether
/// or not it has a completed rename yet: `undo`'s existing interrupted-item
/// and no-completion-record warnings fire unconditionally once a batch is
/// selected, so a still-being-written journal that truly has nothing in it
/// yet (R6's race) is the only case still skipped -- "not picked at all"
/// rather than "picked, then warned about" -- while one with a genuine
/// unresolved item is surfaced, not silently stepped over.
fn resolve_last(batches: &[PathBuf]) -> Result<PathBuf, String> {
    for path in batches.iter().rev() {
        let replay =
            journal::replay(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if !replay.items.is_empty() || replay.interrupted.is_some() {
            return Ok(path.clone());
        }
    }
    // Nothing among them ever completed or interrupted a rename: fall back
    // to the newest file so the existing empty-batch reporting below still
    // applies, rather than inventing a new "no batches" message for a
    // directory that is not actually empty.
    batches
        .last()
        .cloned()
        .ok_or_else(|| "no recorded batches to undo".to_owned())
}

/// A plan error as the user should read it.
fn describe(e: PlanError) -> String {
    match e {
        PlanError::BatchRefused(conflicting) => {
            let mut err = io::stderr().lock();
            drop(report::items(&mut err, &conflicting, false));
            "batch refused by --on-collision fail: the conflicts above were found, and nothing \
             was planned. Re-run with --on-collision number or skip."
                .to_owned()
        }
        PlanError::InternalInconsistency(what) => {
            format!("internal inconsistency, please report this: {what}")
        }
    }
}
