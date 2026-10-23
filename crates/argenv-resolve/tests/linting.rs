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
