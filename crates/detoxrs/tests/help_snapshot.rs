//! The `--help` and `--version` surface, pinned with `insta` (plan §7.1, `WP5a`:
//! "`--help` snapshot pinned first").
//!
//! This snapshot is the CLI's contract. A flag added, renamed, reordered or
//! re-worded shows up here as a reviewable diff instead of silently changing
//! what users read.

use assert_cmd::Command;

fn detoxrs() -> Command {
    Command::cargo_bin("detoxrs").expect("binary builds")
}

#[test]
fn help_is_stable() {
    let out = detoxrs().arg("--help").output().expect("runs");
    assert!(out.status.success());
    insta::assert_snapshot!(String::from_utf8(out.stdout).expect("help is UTF-8"));
}

/// `-h` is clap's short form, deliberately terser than `--help`. Both must exist
/// and both must show the whole flag set.
#[test]
fn short_help_lists_every_flag() {
    let out = detoxrs().arg("-h").output().expect("runs");
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).expect("help is UTF-8");
    for flag in [
        "--exec",
        "--dry-run",
        "--recursive",
        "--on-collision",
        "--verbose",
        "--quiet",
        "--json",
        "--help",
        "--version",
    ] {
        assert!(text.contains(flag), "-h omits {flag}:\n{text}");
    }
}

#[test]
fn version_prints_the_crate_version() {
    detoxrs()
        .arg("-V")
        .assert()
        .success()
        .stdout(format!("detoxrs {}\n", env!("CARGO_PKG_VERSION")));
}

/// `-n` is the explicit form of the default and `-x` is the write path; asking
/// for both at once is a usage error, not a silent precedence rule.
#[test]
fn dry_run_and_exec_conflict() {
    let out = detoxrs().args(["-n", "-x", "."]).output().expect("runs");
    assert_eq!(out.status.code(), Some(2), "clap usage errors exit 2");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("cannot be used with"), "stderr was: {err}");
}

/// No paths is a usage error: there is no implicit `.`, because the implicit
/// argument for a tool that rewrites names should never be "wherever I am".
#[test]
fn no_paths_is_a_usage_error() {
    let out = detoxrs().output().expect("runs");
    assert_eq!(out.status.code(), Some(2));
}
