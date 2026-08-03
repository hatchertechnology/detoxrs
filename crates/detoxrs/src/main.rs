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
//! Exit codes: `0` no errors, `1` one or more items could not be renamed (or the
//! batch aborted part-way), `2` usage, walk, or plan error — which are the
//! failures where nothing was attempted at all.
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
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(msg) => {
            eprintln!("detoxrs: {msg}");
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

    let entries = walk::snapshot(&args.paths, args.recursive).map_err(|e| e.to_string())?;
    let case = walk::volume_case(&entries);
    let policy = Policy::default(); // M1 has no transform flags; M3's config file fills this in.

    let p = plan(&entries, &policy, args.on_collision.into(), case).map_err(describe)?;

    if args.exec {
        exec(&p, &policy, args.json, args.quiet)
    } else {
        preview(&p, args).map(|()| 0)
    }
}

/// Print the plan and change nothing.
fn preview(p: &Plan, args: &cli::Cli) -> Result<(), String> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if args.json {
        report::json(&mut out, p, None)
    } else if args.quiet {
        Ok(()) // --quiet is errors only, and a preview is not an error.
    } else {
        report::preview(&mut out, p, args.verbose > 0)
    }
    .and_then(|()| out.flush())
    .map_err(|e| format!("cannot write output: {e}"))
}

/// Open the journal, apply, and report.
///
/// The journal is opened **before** the first rename and its failure is exit 2
/// with nothing attempted, which is §5.8's rule stated as control flow: renaming
/// without a journal is the one outcome `undo` cannot fix.
fn exec(p: &Plan, policy: &Policy, as_json: bool, quiet: bool) -> Result<u8, String> {
    // Nothing to rename means no journal, and that is not tidiness: an empty batch
    // would be the newest one, so `undo --last` would stop meaning "undo what I
    // just did" after any no-op `-x` run.
    if !p.items.iter().any(|i| i.resolution == Resolution::Rename) {
        return report_nothing(p, as_json, quiet).map(|()| 0);
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

    if as_json {
        report::json(&mut out, p, Some(&s.outcomes))
    } else if quiet {
        Ok(())
    } else {
        report::applied(&mut out, p, &s, Some((&batch, &where_)))
    }
    .and_then(|()| out.flush())
    .map_err(|e| format!("cannot write output: {e}"))?;

    Ok(s.exit_code())
}

/// An `-x` run that found nothing to rename: the same closing report, minus the
/// undo line, and no journal was opened.
fn report_nothing(p: &Plan, as_json: bool, quiet: bool) -> Result<(), String> {
    let empty = apply::Summary {
        outcomes: vec![apply::ItemResult::NotAttempted; p.items.len()],
        ..apply::Summary::default()
    };
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if as_json {
        report::json(&mut out, p, Some(&empty.outcomes))
    } else if quiet {
        Ok(())
    } else {
        report::applied(&mut out, p, &empty, None)
    }
    .and_then(|()| out.flush())
    .map_err(|e| format!("cannot write output: {e}"))
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
        batches
            .last()
            .cloned()
            .ok_or_else(|| "no recorded batches to undo".to_owned())?
    } else if let Some(id) = &u.batch_id {
        journal::path_of(id).map_err(|e| e.to_string())?
    } else {
        return Err(
            "undo needs a BATCH-ID or --last. `detoxrs undo --list` shows them.".to_owned(),
        );
    };

    let replay =
        journal::replay(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if let Some(item) = &replay.interrupted {
        // The crash protocol's whole promise, discharged in one line.
        eprintln!(
            "detoxrs: note: batch was interrupted while renaming {} -> {}; its outcome was never \
             recorded, so it is left alone. Check that name by hand.",
            report::escape(&item.original),
            report::escape(&item.current)
        );
    }
    if replay.items.is_empty() {
        println!("nothing to undo in that batch.");
        return Ok(u8::from(replay.interrupted.is_some()));
    }

    // An undo is itself a batch of renames, so it gets its own journal and can
    // itself be undone.
    let cwd = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
    let mut j = journal::Journal::create(&Policy::default(), &cwd)
        .map_err(|e| format!("cannot open an undo journal ({e}); nothing was reverted."))?;

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let s = apply::undo(&replay.items, &PlatformRenameOps, &mut j, &mut out);
    drop(writeln!(
        out,
        "\n{} reverted, {} refused. This undo is itself batch {}.",
        s.renamed,
        s.failed,
        j.id()
    ));
    drop(out.flush());
    Ok(if replay.interrupted.is_some() {
        1
    } else {
        s.exit_code()
    })
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
