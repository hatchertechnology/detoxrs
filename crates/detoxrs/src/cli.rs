//! The command-line surface (plan §7.3, proposal §2.4).
//!
//! M1's flag set only, and deliberately no more: `--case`, `--separator`,
//! `--target`, `--exclude`, `--config` and friends arrive with the milestones
//! that implement them, because a flag that parses and does nothing is worse
//! documentation than a flag that is absent.
//!
//! `-x` is the exception. It parses here even though this build has no write
//! path, so the CLI surface is complete and stable from the first release; the
//! refusal lives in `main::run`, which is also the only place that would ever
//! call an apply path.

use clap::{Parser, ValueEnum};
use detoxrs_core::plan::OnCollision;
use std::path::PathBuf;

/// Make filenames sane: unix-safe, portable, readable. Preview by default.
#[derive(Debug, Parser)]
#[command(
    name = "detoxrs",
    version,
    about = "Make filenames sane: unix-safe, portable, readable. Preview by default.",
    after_help = "Nothing is renamed unless you pass -x. This build previews only: -x is \
                  parsed but refused, because no write path exists in it yet.\n\n\
                  Without -r, a directory argument has only its own name cleaned and nothing \
                  inside it is touched (detox differs).\n\n\
                  Exit codes:\n  \
                  0  preview produced with no errors\n  \
                  2  usage, walk, or plan error"
)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "this is a flag surface: each bool is one command-line switch, and \
              collapsing them into an enum would misrepresent flags that are \
              independently settable"
)]
pub struct Cli {
    /// Files and/or directories to clean
    #[arg(required = true, value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Perform the renames (not implemented in this build)
    #[arg(short = 'x', long)]
    pub exec: bool,

    /// Preview only; explicit form of the default
    #[arg(short = 'n', long, conflicts_with = "exec")]
    pub dry_run: bool,

    /// Descend into directories; without it, a directory argument has only its own name cleaned
    #[arg(short, long)]
    pub recursive: bool,

    /// What to do when two names want the same destination
    #[arg(long, value_name = "POLICY", value_enum, default_value_t = CollisionArg::Number)]
    pub on_collision: CollisionArg,

    /// List unchanged entries too
    #[arg(short, long, action = clap::ArgAction::Count, conflicts_with = "quiet")]
    pub verbose: u8,

    /// Errors only
    #[arg(short, long)]
    pub quiet: bool,

    /// JSON on stdout, diagnostics on stderr
    #[arg(long)]
    pub json: bool,
}

/// `--on-collision`, as spelled on the command line.
///
/// A separate enum from [`OnCollision`] so `detoxrs-core` never grows a `clap`
/// dependency; the mapping below is the whole cost of that separation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CollisionArg {
    /// Insert `-N` before the extension, smallest free `N >= 2`.
    Number,
    /// Leave every colliding entry alone and report it.
    Skip,
    /// Refuse the entire batch.
    Fail,
}

impl From<CollisionArg> for OnCollision {
    fn from(a: CollisionArg) -> Self {
        match a {
            CollisionArg::Number => Self::Number,
            CollisionArg::Skip => Self::Skip,
            CollisionArg::Fail => Self::Fail,
        }
    }
}
