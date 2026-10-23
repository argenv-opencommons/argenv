//! What checking an invocation catches — and what it deliberately ignores.
mod common;
use argenv_resolve::*;
use common::*;

fn check(argv: &[&str], vars: &[(&str, &str)]) -> Vec<Finding> {
    let a = args(argv);
    let e = env(vars);
    let resolution = Invocation { args: &a, env: &e }.resolve(&model());
    lint(&model(), &resolution)
}

fn errors(f: &[Finding]) -> Vec<&Finding> {
    f.iter()
        .filter(|f| f.severity() == Severity::Error)
        .collect()
}

#[test]
fn a_satisfied_invocation_reports_nothing() {
    let f = check(&["--log-level", "warn"], &[("APP_KEY", "abc")]);
    assert!(errors(&f).is_empty(), "{f:#?}");
}

// ---- a flag given to a subcommand it does not belong to ----------------------------------

/// `--dry-run` belongs to `create`, `--force` to `delete`, `--json` to every command, and
/// `--jobs` (a number) to `create`.
fn scoped_flags() -> Vec<Record> {
    let dry_run: Input<bool, &'static str> = Input {
        key: "dry_run",
        ty: Type::Bool,
        default: Some(false),
        subcommands: &["create"],
        arg: Some(Arg::long("dry-run")),
        ..Input::EMPTY
    };
    let force: Input<bool, &'static str> = Input {
        key: "force",
        ty: Type::Bool,
        default: Some(false),
        subcommands: &["delete"],
        arg: Some(Arg::pair("force", 'f')),
        ..Input::EMPTY
    };
    let json: Input<bool, &'static str> = Input {
        key: "json",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::long("json")),
        ..Input::EMPTY
    };
    let jobs: Input<u64, &'static str> = Input {
        key: "jobs",
        ty: Type::Uint,
        subcommands: &["create"],
        arg: Some(Arg::long("jobs")),
        ..Input::EMPTY
    };
    vec![
        dry_run.to_record(),
        force.to_record(),
        json.to_record(),
        jobs.to_record(),
    ]
}

fn check_flags(argv: &[&str]) -> Vec<Finding> {
    let a = args(argv);
    let e = env(&[]);
    let model = scoped_flags();
    let resolution = Invocation { args: &a, env: &e }.resolve(&model);
    lint(&model, &resolution)
}

fn out_of_scope(f: &[Finding]) -> Vec<&Finding> {
    f.iter()
        .filter(|f| matches!(f, Finding::OutOfScope { .. }))
        .collect()
}

#[test]
fn a_flag_of_another_subcommand_is_reported_with_where_it_belongs() {
    let f = check_flags(&["delete", "--dry-run"]);
    assert_eq!(
        out_of_scope(&f),
        [&Finding::OutOfScope {
            arg: "--dry-run".into(),
            command: Some("delete".into()),
            applies_to: vec!["create".into()],
        }]
    );
    assert_eq!(
        f[0].to_string(),
        "--dry-run does not apply to `delete` (it applies to: create)"
    );
}

#[test]
fn a_flag_is_fine_for_its_own_subcommand_and_a_shared_flag_is_fine_anywhere() {
    assert!(out_of_scope(&check_flags(&["create", "--dry-run"])).is_empty());
    assert!(out_of_scope(&check_flags(&["delete", "-f"])).is_empty());
    for command in ["create", "delete", "list"] {
        assert!(
            out_of_scope(&check_flags(&[command, "--json"])).is_empty(),
            "--json is declared for every command, so it fits `{command}`"
        );
    }
}

#[test]
fn a_flag_that_belongs_to_a_subcommand_is_out_of_place_when_none_runs() {
    let f = check_flags(&["--dry-run"]);
    let findings = out_of_scope(&f);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].to_string(), "--dry-run only applies to: create");
}

#[test]
fn the_value_of_an_out_of_place_flag_is_not_judged_too() {
    // One problem, reported once: the flag is in the wrong place. Not also "jobs=many is not a
    // number", which would be noise about a flag that was never going to be read.
    let f = check_flags(&["delete", "--jobs", "many"]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(matches!(&f[0], Finding::OutOfScope { arg, .. } if arg == "--jobs"));
}

#[test]
fn the_same_flag_in_scope_is_judged_normally() {
    let f = check_flags(&["create", "--jobs", "many"]);
    assert!(out_of_scope(&f).is_empty());
    assert!(f
        .iter()
        .any(|f| matches!(f, Finding::InvalidValue { key, .. } if key == "jobs")));
}

#[test]
fn an_out_of_place_flag_is_an_error_by_default_and_the_project_decides() {
    let f = check_flags(&["delete", "--dry-run"]);
    let finding = &f[0];
    assert_eq!(finding.severity(), Severity::Error);
    assert_eq!(LintPolicy::default().severity(finding), Severity::Error);
    assert_eq!(LintPolicy::lenient().severity(finding), Severity::Warning);
    let only_this = LintPolicy::default().out_of_scope_flags(Severity::Warning);
    assert_eq!(only_this.severity(finding), Severity::Warning);
    // ...and relaxing it leaves the other kinds of typo strict.
    let typo = check(&["--log-levl", "warn"], &[]);
    let flag_typo = typo
        .iter()
        .find(|f| matches!(f, Finding::UnknownArg { .. }))
        .unwrap();
    assert_eq!(only_this.severity(flag_typo), Severity::Error);
}

#[test]
fn findings_are_non_exhaustive_so_a_wildcard_arm_is_required_of_callers() {
    // This match is written the way every caller must write it. If `Finding` stops being
    // `#[non_exhaustive]` the arm below is unreachable and the compiler says so.
    let f = check_flags(&["delete", "--dry-run"]);
    let label = match &f[0] {
        Finding::OutOfScope { .. } => "scope",
        _ => "something else",
    };
    assert_eq!(label, "scope");
}

// ---- how seriously each finding is taken: the policy ------------------------------------

fn typo_findings() -> Vec<Finding> {
    check(&["--log-levl", "warn"], &[("APP_LOG_LEVL", "warn")])
}

fn severity_of(policy: &LintPolicy, f: &[Finding], which: fn(&Finding) -> bool) -> Severity {
    policy.severity(f.iter().find(|f| which(f)).expect("the finding is there"))
}

fn is_flag_typo(f: &Finding) -> bool {
    matches!(f, Finding::UnknownArg { .. })
}

fn is_variable_typo(f: &Finding) -> bool {
    matches!(f, Finding::UnknownEnv { .. })
}

#[test]
fn a_misspelled_flag_is_an_error_by_default() {
    let f = typo_findings();
    assert_eq!(
        f.iter().find(|f| is_flag_typo(f)).unwrap().severity(),
        Severity::Error
    );
    assert_eq!(
        severity_of(&LintPolicy::default(), &f, is_flag_typo),
        Severity::Error
    );
}

#[test]
fn a_misspelled_variable_is_an_error_by_default() {
    let f = typo_findings();
    assert_eq!(
        f.iter().find(|f| is_variable_typo(f)).unwrap().severity(),
        Severity::Error
    );
}

#[test]
fn the_lenient_policy_takes_both_kinds_of_typo_as_warnings() {
    let f = typo_findings();
    let lenient = LintPolicy::lenient();
    assert_eq!(severity_of(&lenient, &f, is_flag_typo), Severity::Warning);
    assert_eq!(
        severity_of(&lenient, &f, is_variable_typo),
        Severity::Warning
    );
}

#[test]
fn each_kind_can_be_relaxed_on_its_own() {
    let f = typo_findings();
    let flags_only = LintPolicy::default().unknown_flags(Severity::Warning);
    assert_eq!(
        severity_of(&flags_only, &f, is_flag_typo),
        Severity::Warning
    );
    assert_eq!(
        severity_of(&flags_only, &f, is_variable_typo),
        Severity::Error
    );

    let variables_only = LintPolicy::default().unknown_variables(Severity::Warning);
    assert_eq!(
        severity_of(&variables_only, &f, is_flag_typo),
        Severity::Error
    );
    assert_eq!(
        severity_of(&variables_only, &f, is_variable_typo),
        Severity::Warning
    );
}

#[test]
fn a_policy_can_also_be_stricter_about_deprecation() {
    let f = check(&["--profile", "x"], &[]);
    let dep = f
        .iter()
        .find(|f| matches!(f, Finding::Deprecated { .. }))
        .expect("deprecated");
    assert_eq!(LintPolicy::default().severity(dep), Severity::Warning);
    assert_eq!(
        LintPolicy::default()
            .deprecated(Severity::Error)
            .severity(dep),
        Severity::Error
    );
}

#[test]
fn a_bad_value_is_an_error_whatever_the_policy() {
    // There is no usable reading of a value outside its domain, so no policy can excuse it. Both
    // ways of getting it wrong: a scalar outside its set, and a member of a list outside its set.
    let scalar = check(&["--log-level", "loud"], &[]);
    let member = check(&["--hud", "fps,bogus"], &[("APP_KEY", "k")]);
    let value_problems: Vec<&Finding> = scalar
        .iter()
        .chain(member.iter())
        .filter(|f| {
            matches!(
                f,
                Finding::InvalidValue { .. } | Finding::UnknownToken { .. }
            )
        })
        .collect();
    assert!(
        value_problems.len() >= 2,
        "both kinds of bad value are found: {scalar:?} {member:?}"
    );
    for finding in value_problems {
        for policy in [
            LintPolicy::default(),
            LintPolicy::lenient(),
            LintPolicy::lenient().unknown_flags(Severity::Warning),
        ] {
            assert_eq!(policy.severity(finding), Severity::Error, "{finding}");
        }
    }
}

#[test]
fn the_default_policy_is_strict_and_a_findings_own_severity_is_its_answer() {
    assert_eq!(LintPolicy::default(), LintPolicy::strict());
    for f in typo_findings() {
        assert_eq!(f.severity(), LintPolicy::strict().severity(&f));
    }
}

#[test]
fn a_misspelled_flag_is_caught_with_a_suggestion() {
    let f = check(&["--log-levl", "warn"], &[("APP_KEY", "k")]);
    assert!(
        f.iter().any(|f| matches!(
            f, Finding::UnknownArg { did_you_mean: Some(s), .. } if s == "--log-level"
        )),
        "{f:#?}"
    );
}

#[test]
fn a_misspelled_variable_is_caught_with_a_suggestion() {
    let f = check(&[], &[("APP_LOG_LEVL", "warn"), ("APP_KEY", "k")]);
    assert!(
        f.iter().any(|f| matches!(
            f, Finding::UnknownEnv { did_you_mean, .. } if did_you_mean == "APP_LOG_LEVEL"
        )),
        "{f:#?}"
    );
}

#[test]
fn the_rest_of_the_environment_is_never_mentioned() {
    let f = check(
        &[],
        &[("PATH", "/usr/bin"), ("HOME", "/root"), ("APP_KEY", "k")],
    );
    assert!(
        !f.iter().any(|f| matches!(f, Finding::UnknownEnv { .. })),
        "a checker that shouts about PATH is one nobody runs: {f:#?}"
    );
}

#[test]
fn a_flag_that_needs_a_value_and_gets_none_is_an_error() {
    let f = check(&["--log-level"], &[("APP_KEY", "k")]);
    assert!(f.iter().any(|f| matches!(f, Finding::MissingValue { .. })));
}

#[test]
fn a_flag_that_takes_no_value_and_gets_one_is_an_error() {
    let f = check(&["--hdr=yes"], &[("APP_KEY", "k")]);
    assert!(f
        .iter()
        .any(|f| matches!(f, Finding::UnexpectedValue { .. })));
}

#[test]
fn a_value_outside_its_domain_is_an_error_that_names_the_door() {
    let f = check(&["--log-level", "loud"], &[("APP_KEY", "k")]);
    let bad = f
        .iter()
        .find(|f| matches!(f, Finding::InvalidValue { .. }))
        .expect("expected InvalidValue");
    assert_eq!(bad.severity(), Severity::Error);
    assert!(bad.to_string().contains("argument"), "{bad}");

    let from_env = check(&[], &[("APP_HDR", "perhaps"), ("APP_KEY", "k")]);
    let bad = from_env
        .iter()
        .find(|f| matches!(f, Finding::InvalidValue { .. }))
        .expect("expected InvalidValue");
    assert!(bad.to_string().contains("environment"), "{bad}");
}

#[test]
fn a_declared_default_is_never_judged() {
    let f = check(&[], &[("APP_KEY", "k")]);
    assert!(
        !f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })),
        "the program's own default is its choice: {f:#?}"
    );
}

#[test]
fn an_unknown_token_names_the_offender_and_spares_the_rest() {
    let f = check(&["--hud", "fps,bogus,gpuload"], &[("APP_KEY", "k")]);
    assert!(f.iter().any(|f| matches!(
        f, Finding::UnknownToken { token, .. } if token == "bogus"
    )));
    assert!(f.iter().all(|f| !matches!(
        f, Finding::UnknownToken { token, .. } if token == "fps" || token == "gpuload"
    )));
}

#[test]
fn a_missing_required_input_says_how_it_could_have_been_supplied() {
    let f = check(&[], &[]);
    let missing = f
        .iter()
        .find(|f| matches!(f, Finding::MissingRequired { key, .. } if key == "licence_key"))
        .expect("expected MissingRequired");
    let text = missing.to_string();
    assert!(
        text.contains("--licence-key") && text.contains("APP_KEY"),
        "{text}"
    );
}

/// A program with two subcommands, where only `delete` needs its argument.
fn scoped_model() -> Vec<Record> {
    let id: Input<String, &'static str> = Input {
        key: "id",
        ty: Type::String,
        required: true,
        subcommands: &["delete"],
        arg: Some(Arg {
            value_name: "ID",
            ..Arg::positional(0)
        }),
        ..Input::EMPTY
    };
    let json: Input<bool, &'static str> = Input {
        key: "json",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::long("json")),
        ..Input::EMPTY
    };
    vec![id.to_record(), json.to_record()]
}

fn check_scoped(argv: &[&str]) -> Vec<Finding> {
    let a = args(argv);
    let e = env(&[]);
    let model = scoped_model();
    let resolution = Invocation { args: &a, env: &e }.resolve(&model);
    lint(&model, &resolution)
}

fn missing(f: &[Finding]) -> bool {
    f.iter()
        .any(|f| matches!(f, Finding::MissingRequired { key, .. } if key == "id"))
}

#[test]
fn a_required_input_is_missing_when_its_own_subcommand_runs_without_it() {
    assert!(missing(&check_scoped(&["delete"])));
}

#[test]
fn a_required_input_is_satisfied_when_its_own_subcommand_supplies_it() {
    assert!(!missing(&check_scoped(&["delete", "42"])));
}

#[test]
fn a_required_input_scoped_to_another_subcommand_is_not_missing() {
    assert!(!missing(&check_scoped(&["list"])));
    assert!(!missing(&check_scoped(&["list", "--json"])));
}

#[test]
fn a_scoped_required_input_is_not_demanded_when_no_subcommand_runs() {
    assert!(!missing(&check_scoped(&[])));
    assert!(!missing(&check_scoped(&["--json"])));
}

#[test]
fn an_unscoped_required_input_is_still_demanded_everywhere() {
    // The long-standing behaviour: no scope means every invocation needs it.
    let f = check(&[], &[]);
    assert!(f
        .iter()
        .any(|f| matches!(f, Finding::MissingRequired { key, .. } if key == "licence_key")));
}

#[test]
fn a_deprecated_input_in_use_is_a_warning_that_names_its_successor() {
    let f = check(&["--profile"], &[("APP_KEY", "k")]);
    let dep = f
        .iter()
        .find(|f| matches!(f, Finding::Deprecated { .. }))
        .expect("expected Deprecated");
    assert_eq!(dep.severity(), Severity::Warning);
    assert!(dep.to_string().contains("hud"), "{dep}");
}

#[test]
fn a_legacy_name_is_recognised_rather_than_reported_as_a_mistake() {
    let f = check(&[], &[("APP_LOGFILE", "/tmp/a.log"), ("APP_KEY", "k")]);
    assert!(
        !f.iter().any(|f| matches!(f, Finding::UnknownEnv { .. })),
        "{f:#?}"
    );
}

#[test]
fn checking_works_from_a_published_document_alone() {
    // A consumer that never saw the Rust types, in any language, can do this.
    let json = serde_json::to_string(&document("app@abc", &model())).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    let declared: Vec<Record> = serde_json::from_value(doc["inputs"].clone()).unwrap();

    let a = args(&["--log-level", "loud"]);
    let e = env(&[("APP_KEY", "k")]);
    let resolution = Invocation { args: &a, env: &e }.resolve(&declared);
    let f = lint(&declared, &resolution);
    assert!(f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })));
}

#[test]
fn an_environment_only_program_can_be_checked_without_arguments() {
    let e = env(&[("APP_HDR", "perhaps"), ("APP_KEY", "k")]);
    let resolution = Invocation { args: &[], env: &e }.resolve(&model());
    let f = lint(&model(), &resolution);
    assert!(f.iter().any(|f| matches!(f, Finding::InvalidValue { .. })));
}
