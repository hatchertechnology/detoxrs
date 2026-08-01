//! detoxrs: make filenames sane. See
//! `docs/research/00-proposal-rust-detox-successor.md`.
//!
//! **This build previews only (plan §7.1, `WP5a`).** It walks a tree, computes a
//! plan and prints it. It cannot rename, create, delete or modify anything — not
//! because a flag is unset, but because no code that writes to a filesystem has
//! been written yet. `fsops`, `apply`, `journal` and `undo` are `WP5b`. `-x` parses
//! so the CLI surface is stable from the first release, and is refused with a
//! message and a non-zero exit rather than being a silent no-op: a user who
//! types `-x` and sees success must never have to wonder whether files moved.
//!
//! Exit codes in this build: `0` for a preview produced with no errors, `2` for a
//! usage, walk, or plan error. `1` is reserved for per-item failures, which
//! cannot happen here because there are no per-item operations to fail; it starts
//! being reachable in `WP5b`.
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

mod cli;
mod report;
mod walk;

use clap::Parser as _;
use detoxrs_core::plan::{PlanError, plan};
use detoxrs_core::policy::Policy;
use std::io::{self, Write as _};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = cli::Cli::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("detoxrs: {msg}");
            ExitCode::from(2)
        }
    }
}

/// The whole program. Every failure is exit 2 in this build; see the module docs.
fn run(args: &cli::Cli) -> Result<(), String> {
    if args.exec {
        // Checked before the walk, so `-x` cannot even read the tree it was
        // going to rename. This is the one and only gate, and there is nothing
        // behind it yet.
        return Err(
            "-x is not implemented in this build: it previews only, and no rename code exists \
             in it. Nothing was changed. Re-run without -x to see the plan."
                .to_owned(),
        );
    }

    let entries = walk::snapshot(&args.paths, args.recursive).map_err(|e| e.to_string())?;
    let case = walk::volume_case(&entries);
    let policy = Policy::default(); // M1 has no transform flags; M3's config file fills this in.

    let stdout = io::stdout();
    let mut out = stdout.lock();
    match plan(&entries, &policy, args.on_collision.into(), case) {
        Ok(p) => {
            if args.json {
                report::json(&mut out, &p)
            } else if args.quiet {
                Ok(()) // --quiet is errors only, and a preview is not an error.
            } else {
                report::preview(&mut out, &p, args.verbose > 0)
            }
            .and_then(|()| out.flush())
            .map_err(|e| format!("cannot write output: {e}"))
        }
        Err(PlanError::BatchRefused(conflicting)) => {
            let mut err = io::stderr().lock();
            drop(report::items(&mut err, &conflicting, false));
            Err(
                "batch refused by --on-collision fail: the conflicts above were found, and \
                 nothing was planned. Re-run with --on-collision number or skip."
                    .to_owned(),
            )
        }
        Err(PlanError::InternalInconsistency(what)) => Err(format!(
            "internal inconsistency, please report this: {what}"
        )),
    }
}
