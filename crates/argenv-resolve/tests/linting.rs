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
