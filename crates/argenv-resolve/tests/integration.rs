//! Real integration tests: spawn the compiled mock program
//! (`tests/fixtures/mock.rs`) as a subprocess with real argv and real
//! environment variables, and check its actual stdout, stderr, and exit
//! code — not a mocked `Invocation::resolve()` call. This is the level at
//! which "does a feature actually work end to end" and "is an anti-pattern
//! actually rejected" get answered for real.

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_argenv-resolve-mock")
}

fn run(args: &[&str], env: &[(&str, &str)]) -> (bool, String, String) {
    let mut cmd = Command::new(bin());
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("mock program runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

// ── features: the shared/unique/global scoping actually works over real argv ──

#[test]
fn build_resolves_its_own_target_and_unique_and_shared_options() {
    let (ok, stdout, _) = run(&["build", "app", "--release", "--jobs", "4"], &[]);
    assert!(ok);
    assert!(stdout.contains(r#"target=Some("app")"#));
    assert!(stdout.contains("release=true"));
    assert!(stdout.contains("jobs=Some(4)"));
}

#[test]
fn run_resolves_its_own_target_and_unique_and_shared_options() {
    let (ok, stdout, _) = run(&["run", "app", "--jobs", "2", "--watch"], &[]);
    assert!(ok);
    assert!(stdout.contains(r#"target=Some("app")"#));
    assert!(stdout.contains("watch=true"));
    assert!(stdout.contains("jobs=Some(2)"));
}

#[test]
fn clean_takes_no_target_and_no_shared_or_unique_options() {
    let (ok, stdout, _) = run(&["clean"], &[]);
    assert!(ok);
    assert_eq!(stdout.trim(), "clean verbose=false");
}

#[test]
fn global_verbose_flag_works_before_the_subcommand() {
    let (ok, _, stderr) = run(&["--verbose", "build", "app"], &[]);
    assert!(ok);
    assert!(stderr.contains("config=None"));
}

#[test]
fn global_config_resolves_via_env_on_every_subcommand() {
    let (ok, _, stderr) = run(
        &["clean", "--verbose"],
        &[("MOCK_CONFIG", "/etc/mock.toml")],
    );
    assert!(ok);
    assert!(stderr.contains(r#"config=Some("/etc/mock.toml")"#));
}

#[test]
fn shared_jobs_env_resolves_under_build() {
    let (ok, stdout, _) = run(&["build", "app"], &[("MOCK_JOBS", "8")]);
    assert!(ok);
    assert!(stdout.contains("jobs=Some(8)"));
}

#[test]
fn shared_jobs_env_resolves_under_run_too() {
    let (ok, stdout, _) = run(&["run", "app"], &[("MOCK_JOBS", "8")]);
    assert!(ok);
    assert!(stdout.contains("jobs=Some(8)"));
}

// ── anti-patterns: scoping is enforced, not just documented ───────────────────

#[test]
fn shared_jobs_env_does_not_leak_into_clean() {
    let (ok, stdout, _) = run(&["clean"], &[("MOCK_JOBS", "8")]);
    assert!(ok);
    assert_eq!(stdout.trim(), "clean verbose=false");
}

#[test]
fn build_only_release_flag_is_absent_under_run() {
    // Real argv: pass --release under `run`, where it isn't declared.
    // The parser doesn't know about scoping, so it still recognises the
    // flag by name; it just never gets consumed by anything relevant.
    let (_, stdout, _) = run(&["run", "app", "--release"], &[]);
    assert!(!stdout.contains("release=true"));
}

#[test]
fn watch_env_does_not_leak_into_build() {
    let (ok, stdout, _) = run(&["build", "app"], &[("MOCK_WATCH", "1")]);
    assert!(ok);
    assert!(!stdout.contains("watch=true"));
}

#[test]
fn missing_subcommand_exits_nonzero_with_usage() {
    let (ok, _, stderr) = run(&[], &[]);
    assert!(!ok);
    assert!(stderr.contains("usage:"));
}

#[test]
fn unknown_subcommand_is_rejected_with_a_clear_message() {
    let (ok, _, stderr) = run(&["frobnicate"], &[]);
    assert!(!ok);
    assert!(stderr.contains("frobnicate"));
    assert!(stderr.contains("invalid"));
}

#[test]
fn wrong_type_for_a_uint_option_is_rejected_not_silently_zero() {
    let (ok, stdout, stderr) = run(&["build", "app", "--jobs", "notanumber"], &[]);
    assert!(!ok);
    assert!(stderr.contains("jobs"));
    assert!(
        stdout.is_empty(),
        "must not print a result for a rejected invocation"
    );
}

#[test]
fn unknown_flag_under_a_valid_subcommand_is_a_warning_not_a_hard_failure() {
    // An unknown flag name is a Severity::Warning (surfaced, not silently
    // dropped) - unlike an unparseable value, which is a Severity::Error and
    // does stop the program (see wrong_type_for_a_uint_option above). Real,
    // deliberate distinction: a typo'd flag name might still be a usable
    // invocation; a value that can't be parsed as its declared type can't.
    let (ok, stdout, stderr) = run(&["build", "app", "--bogus-flag"], &[]);
    assert!(ok, "an unknown flag alone should not be fatal");
    assert!(
        stderr.contains("--bogus-flag"),
        "but it must still be surfaced: {stderr}"
    );
    assert!(stdout.contains(r#"target=Some("app")"#));
}
