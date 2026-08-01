//! The command-line surface (plan §7.3, proposal §2.4).
//!
//! M1's flag set only, and deliberately no more: `--case`, `--separator`,
//! `--target`, `--exclude`, `--config` and friends arrive with the milestones
//! that implement them, because a flag that parses and does nothing is worse
//! documentation than a flag that is absent.
//!
//! `--no-journal` is the one flag §5.5 describes that is absent here on purpose:
//! it exists to trade `undo` away for speed on huge trees, and until someone has
//! measured the journal costing them something there is nothing to trade.

use clap::{Args, Parser, Subcommand, ValueEnum};
use detoxrs_core::plan::OnCollision;
use std::path::PathBuf;

/// Make filenames sane: unix-safe, portable, readable. Preview by default.
#[derive(Debug, Parser)]
#[command(
    name = "detoxrs",
    version,
    about = "Make filenames sane: unix-safe, portable, readable. Preview by default.",
    // Both are needed for `detoxrs undo` to coexist with a required positional:
    // one stops clap demanding PATH when a subcommand is used, the other stops it
    // accepting flags meant for a forward run alongside it.
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true,
    after_help = "Nothing is renamed unless you pass -x. Every -x run writes an undo journal \
                  to $XDG_STATE_HOME/detoxrs/journal; `detoxrs undo --last` reverts the most \
                  recent one.\n\n\
                  Without -r, a directory argument has only its own name cleaned and nothing \
                  inside it is touched (detox differs).\n\n\
                  Exit codes:\n  \
                  0  no errors\n  \
                  1  one or more items could not be renamed\n  \
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

    /// Perform the renames, recording an undo journal
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

    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Everything that is not a forward run.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Put a recorded batch of renames back
    Undo(Undo),
}

/// `detoxrs undo` (proposal §5.5).
#[derive(Debug, Args)]
pub struct Undo {
    /// The batch to revert, as printed by --list
    #[arg(value_name = "BATCH-ID")]
    pub batch_id: Option<String>,

    /// Revert the most recent batch
    #[arg(long, conflicts_with = "batch_id")]
    pub last: bool,

    /// Show the recorded batches and exit
    #[arg(long, conflicts_with_all = ["last", "batch_id"])]
    pub list: bool,
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
