//! Tests for positional arguments and gated inputs.
//!
//! The shape validated here is the universal one real tools settled on:
//!
//!   program [global-flags] COMMAND [command-flags] [command-positionals]
//!
//! - One positional at slot 0 with no `gated_by`: the selector (gate).
//! - Per-branch named flags: declared with `gated_by`, active only when the
//!   gate resolved to their value. Env vars must carry the branch prefix.
//! - Per-branch positional values: also `gated_by`, also at relative slot 0.
//! - Global named flags: no `gated_by`, always active regardless of branch.

use argenv::*;
use std::collections::BTreeMap;

// ── shared model fixtures ────────────────────────────────────────────────────

const COMMAND: Input<String> = Input {
    key: "command",
    ty: Type::Enum,
    allowed: &["add", "status", "remove"],
    arg: Some(Arg::positional(0)),
    summary: "Which command to run.",
    ..Input::EMPTY
};

// per-branch positional: the first token after the selector when command=add
const SOURCE: Input<String> = Input {
    key: "source",
    ty: Type::String,
    gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD_" }),
    arg: Some(Arg::positional(0)),
    env: Some(Env::new("APP_ADD_SOURCE")),
    summary: "Repository to add, owner/repo.",
    ..Input::EMPTY
};

// per-branch positional for a different branch
const MODULE: Input<String> = Input {
    key: "module",
    ty: Type::String,
    gated_by: Some(GatedBy { value: "remove", env_prefix: "APP_REMOVE_" }),
    arg: Some(Arg::positional(0)),
    env: Some(Env::new("APP_REMOVE_MODULE")),
    summary: "Module name to remove.",
    ..Input::EMPTY
};

// per-branch named flag: only meaningful when command=add
const ORG: Input<String> = Input {
    key: "org",
    ty: Type::String,
    gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD_" }),
    arg: Some(Arg { value_name: "ORG", ..Arg::long("org") }),
    env: Some(Env::new("APP_ADD_ORG")),
    summary: "GitHub org to fork into.",
    ..Input::EMPTY
};

// global named flag: always active, no branch gating
const JSON: Input<bool> = Input {
    key: "json",
    ty: Type::Bool,
    default: Some(false),
    arg: Some(Arg::long("json")),
    env: Some(Env::new("APP_JSON")),
    summary: "Machine-readable output.",
    ..Input::EMPTY
};

fn model() -> Vec<Record> {
    vec![
        COMMAND.to_record(),
        SOURCE.to_record(),
        MODULE.to_record(),
        ORG.to_record(),
        JSON.to_record(),
    ]
}

fn resolve(args: &[&str], env: &[(&str, &str)]) -> Resolution {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let env: BTreeMap<String, String> =
        env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    Invocation { args: &args, env: &env }.resolve(&model())
}

// ── declaration checks ───────────────────────────────────────────────────────

#[test]
fn every_fixture_input_satisfies_argenvs_own_rules() {
    for input_problems in [
        COMMAND.check(),
        SOURCE.check(),
        MODULE.check(),
        ORG.check(),
        JSON.check(),
    ] {
        assert!(input_problems.is_empty(), "{input_problems:?}");
    }
}

#[test]
fn check_gates_passes_on_a_valid_model() {
    assert!(check_gates(&model()).is_empty());
    assert!(check_unique(&model()).is_empty());
}

#[test]
fn check_gates_catches_a_gated_value_not_in_gate_allowed() {
    let command = Input::<String> {
        allowed: &["status"], // "add" omitted
        ..COMMAND
    };
    let bad = vec![command.to_record(), SOURCE.to_record()];
    let problems = check_gates(&bad);
    assert!(
        problems.iter().any(|p| p.contains("add") && p.contains("allowed")),
        "expected an allowed-token error, got: {problems:?}"
    );
}

#[test]
fn check_gates_catches_gated_inputs_with_no_declared_gate() {
    // drop COMMAND entirely — SOURCE and ORG still reference it
    let no_gate = vec![SOURCE.to_record(), ORG.to_record()];
    let problems = check_gates(&no_gate);
    assert!(!problems.is_empty(), "should catch missing gate");
    assert!(problems.iter().any(|p| p.contains("gated_by")));
}

#[test]
fn check_gates_catches_multiple_gates() {
    let second_gate: Input<String> = Input {
        key: "mode",
        ty: Type::Enum,
        allowed: &["fast", "slow"],
        arg: Some(Arg::positional(0)),
        summary: "A second gate — not allowed.",
        ..Input::EMPTY
    };
    let two_gates = vec![COMMAND.to_record(), second_gate.to_record()];
    let problems = check_gates(&two_gates);
    assert!(
        problems.iter().any(|p| p.contains("more than one")),
        "{problems:?}"
    );
}

#[test]
fn env_prefix_mismatch_is_caught_by_input_check() {
    let bad_org: Input<String> = Input {
        env: Some(Env::new("APP_ORG")), // missing APP_ADD_ prefix
        ..ORG
    };
    let problems = bad_org.check();
    assert!(
        problems.iter().any(|p| p.contains("start with")),
        "expected prefix error, got: {problems:?}"
    );
}

#[test]
fn a_named_flag_and_a_positional_on_the_same_arg_binding_is_rejected() {
    let bad: Input<String> = Input {
        key: "bad",
        ty: Type::String,
        arg: Some(Arg { long: Some("bad"), position: Some(0), ..Arg::EMPTY }),
        summary: "Invalid.",
        ..Input::EMPTY
    };
    let problems = bad.check();
    assert!(
        problems.iter().any(|p| p.contains("both a long/short form and a position")),
        "{problems:?}"
    );
}

// ── resolution ───────────────────────────────────────────────────────────────

#[test]
fn gate_resolves_from_its_positional_slot() {
    let r = resolve(&["add", "owner/repo"], &[]);
    assert_eq!(COMMAND.get_from(&r), Some("add".to_string()));
    assert_eq!(r.source("command"), Some(Source::Arg));
}

#[test]
fn per_branch_positional_resolves_when_its_branch_is_selected() {
    let r = resolve(&["add", "owner/repo"], &[]);
    assert_eq!(SOURCE.get_from(&r), Some("owner/repo".to_string()));
}

#[test]
fn per_branch_positional_is_absent_when_a_different_branch_is_selected() {
    let r = resolve(&["status"], &[]);
    assert_eq!(SOURCE.get_from(&r), None);
    assert_eq!(MODULE.get_from(&r), None);
}

#[test]
fn per_branch_named_flag_resolves_when_its_branch_is_selected() {
    let r = resolve(&["add", "owner/repo", "--org", "my-org"], &[]);
    assert_eq!(ORG.get_from(&r), Some("my-org".to_string()));
}

#[test]
fn per_branch_named_flag_is_absent_when_a_different_branch_is_selected() {
    // --org is declared for "add" only; passing it under "status" still
    // resolves (the parser doesn't know about branches, it sees a known flag),
    // but gated_by means a consumer reads it correctly as None.
    // More importantly: it does NOT resolve from env when the branch is wrong.
    let r = resolve(&["status"], &[("APP_ADD_ORG", "leaked-org")]);
    // env is gated too — APP_ADD_ORG must not bleed into the status branch
    assert_eq!(ORG.get_from(&r), None);
}

#[test]
fn global_flag_resolves_regardless_of_branch() {
    let add = resolve(&["add", "owner/repo", "--json"], &[]);
    let status = resolve(&["status", "--json"], &[]);
    let no_command = resolve(&["--json"], &[]);
    assert_eq!(JSON.get_from(&add), Some(true));
    assert_eq!(JSON.get_from(&status), Some(true));
    assert_eq!(JSON.get_from(&no_command), Some(true));
}

#[test]
fn global_env_resolves_regardless_of_branch() {
    let r = resolve(&["status"], &[("APP_JSON", "true")]);
    assert_eq!(JSON.get_from(&r), Some(true));
}

#[test]
fn per_branch_env_resolves_when_branch_is_selected() {
    // no positional token for source — env provides it instead
    let r = resolve(&["add"], &[("APP_ADD_SOURCE", "env-owner/repo")]);
    assert_eq!(SOURCE.get_from(&r), Some("env-owner/repo".to_string()));
}

#[test]
fn per_branch_env_does_not_resolve_when_wrong_branch_selected() {
    let r = resolve(&["status"], &[("APP_ADD_SOURCE", "env-owner/repo")]);
    assert_eq!(SOURCE.get_from(&r), None);
}

#[test]
fn two_branches_each_get_their_own_positional() {
    let add_r = resolve(&["add", "owner/repo"], &[]);
    let remove_r = resolve(&["remove", "my-module"], &[]);
    assert_eq!(SOURCE.get_from(&add_r), Some("owner/repo".to_string()));
    assert_eq!(MODULE.get_from(&add_r), None);
    assert_eq!(MODULE.get_from(&remove_r), Some("my-module".to_string()));
    assert_eq!(SOURCE.get_from(&remove_r), None);
}

#[test]
fn global_flags_may_appear_before_the_gate_token() {
    // program --json add owner/repo  — json is global, so order doesn't matter
    let r = resolve(&["--json", "add", "owner/repo"], &[]);
    assert_eq!(JSON.get_from(&r), Some(true));
    assert_eq!(COMMAND.get_from(&r), Some("add".to_string()));
    assert_eq!(SOURCE.get_from(&r), Some("owner/repo".to_string()));
}

// ── usage() rendering ────────────────────────────────────────────────────────

#[test]
fn positional_usage_renders_as_angle_bracket_name() {
    let cmd = COMMAND.to_record();
    assert_eq!(cmd.usage(), "<COMMAND>");
}

#[test]
fn named_flag_usage_still_renders_with_dashes() {
    let org = ORG.to_record();
    assert_eq!(org.usage(), "--org <ORG>");
}
