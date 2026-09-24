//! The subcommand shape: one positional gate, per-branch flags and positionals,
//! global flags that apply regardless of branch.
//!
//! Models the universal pattern real tools settled on:
//!
//!   program [global-flags] COMMAND [command-flags] [command-positional]
//!
//! Specifically, a trimmed version of Rhizoid's own surface:
//!
//!   rhizoid [--json] add <owner/repo> [--org ORG] [--tracked-ref REF]
//!   rhizoid [--json] status
//!
//! This is a *shallow* implementation: every input has only the fields that
//! can't be omitted (key, ty, one binding). The rest is left to be filled in
//! by whoever operates or inherits this program. Fields not declared here
//! default to `..Input::EMPTY`; they can be added incrementally without
//! breaking anything.
//!
//! ```text
//! cargo run -p argenv --example subcommand -- add owner/repo --org my-org
//! cargo run -p argenv --example subcommand -- status --json
//! cargo run -p argenv --example subcommand -- add         # missing positional
//! cargo run -p argenv --example subcommand -- status --bogus  # unknown flag
//! APP_ADD_ORG=env-org cargo run -p argenv --example subcommand -- add owner/repo
//! ```
use argenv::*;

// ── gate ─────────────────────────────────────────────────────────────────────

/// The single positional that selects the active branch.
/// One of these, or none — never two.
pub const COMMAND: Input<String> = Input {
    key: "command",
    ty: Type::Enum,
    allowed: &["add", "status"],
    arg: Some(Arg::positional(0)),
    summary: "Which command to run.",
    ..Input::EMPTY
};

// ── add branch ───────────────────────────────────────────────────────────────

/// `add`'s required value: `rhizoid add <owner/repo>`.
/// Position 0 here means the first positional *after* the gate token.
pub const SOURCE: Input<String> = Input {
    key: "source",
    ty: Type::String,
    gated_by: Some(GatedBy {
        value: "add",
        env_prefix: "APP_ADD_",
    }),
    arg: Some(Arg {
        value_name: "REPO",
        ..Arg::positional(0)
    }),
    env: Some(Env::new("APP_ADD_SOURCE")),
    summary: "Upstream repository to fork, owner/repo.",
    ..Input::EMPTY
};

/// Optional named flag, only meaningful when command = add.
/// Its env name carries the branch prefix so it can't collide with a
/// same-named flag on another branch that happens to share a key.
pub const ORG: Input<String> = Input {
    key: "org",
    ty: Type::String,
    gated_by: Some(GatedBy {
        value: "add",
        env_prefix: "APP_ADD_",
    }),
    arg: Some(Arg {
        value_name: "ORG",
        ..Arg::long("org")
    }),
    env: Some(Env::new("APP_ADD_ORG")),
    summary: "GitHub org to fork into (overrides the manifest default).",
    ..Input::EMPTY
};

pub const TRACKED_REF: Input<String> = Input {
    key: "tracked_ref",
    ty: Type::String,
    gated_by: Some(GatedBy {
        value: "add",
        env_prefix: "APP_ADD_",
    }),
    arg: Some(Arg {
        value_name: "REF",
        ..Arg::long("tracked-ref")
    }),
    env: Some(Env::new("APP_ADD_TRACKED_REF")),
    summary: "Branch or tag to track (defaults to upstream's own default).",
    ..Input::EMPTY
};

// ── global flags ─────────────────────────────────────────────────────────────

/// Always active regardless of which branch was selected.
/// `MANGOHUD=1 program add ...` still reads MANGOHUD even though `add`
/// gates its own inputs — global env vars are unconditionally resolved.
pub const JSON: Input<bool> = Input {
    key: "json",
    ty: Type::Bool,
    default: Some(false),
    arg: Some(Arg::long("json")),
    env: Some(Env::new("APP_JSON")),
    summary: "Machine-readable output.",
    ..Input::EMPTY
};

// ── model ─────────────────────────────────────────────────────────────────────

fn model() -> Vec<Record> {
    vec![
        COMMAND.to_record(),
        SOURCE.to_record(),
        ORG.to_record(),
        TRACKED_REF.to_record(),
        JSON.to_record(),
    ]
}

fn problems() -> Vec<String> {
    let mut v = Vec::new();
    v.extend(COMMAND.check());
    v.extend(SOURCE.check());
    v.extend(ORG.check());
    v.extend(TRACKED_REF.check());
    v.extend(JSON.check());
    v.extend(check_unique(&model()));
    v.extend(check_gates(&model())); // model-wide gate invariants
    v
}

// ── dispatch ──────────────────────────────────────────────────────────────────

fn main() {
    let p = problems();
    assert!(p.is_empty(), "declaration errors: {p:#?}");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let resolved = Invocation {
        args: &args,
        env: &ProcessEnv,
    }
    .resolve(&model());

    for finding in lint(
        &model(),
        &Invocation {
            args: &args,
            env: &ProcessEnv,
        },
    ) {
        eprintln!("{:?}: {finding}", finding.severity());
    }

    // Dispatch on the gate — a plain match, same as git/cargo/docker.
    // argenv resolves values; what to *do* with them is the program's call.
    match COMMAND.get_from(&resolved).as_deref() {
        Some("add") => {
            let source = SOURCE.get_from(&resolved);
            let org = ORG.get_from(&resolved);
            let tracked_ref = TRACKED_REF.get_from(&resolved);
            let json = JSON.get_from_or_default(&resolved);

            if source.is_none() {
                eprintln!("rhizoid: missing <REPO>");
                std::process::exit(2);
            }

            if json.unwrap_or(false) {
                println!(
                    "{}",
                    serde_json::json!({
                        "command": "add",
                        "source": source,
                        "org": org,
                        "tracked_ref": tracked_ref,
                    })
                );
            } else {
                println!("add: source={source:?} org={org:?} tracked_ref={tracked_ref:?}");
            }
        }
        Some("status") => {
            let json = JSON.get_from_or_default(&resolved);
            if json.unwrap_or(false) {
                println!("{}", serde_json::json!({"command": "status"}));
            } else {
                println!("status: (not implemented yet)");
            }
        }
        Some(other) => {
            eprintln!("rhizoid: unknown command `{other}`");
            std::process::exit(2);
        }
        None => {
            println!("USAGE");
            println!("    rhizoid [--json] add <REPO> [--org ORG] [--tracked-ref REF]");
            println!("    rhizoid [--json] status");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn resolve(args: &[&str], env: &[(&str, &str)]) -> Resolution {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env: BTreeMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Invocation {
            args: &args,
            env: &env,
        }
        .resolve(&model())
    }

    #[test]
    fn declaration_is_valid() {
        assert!(problems().is_empty(), "{:#?}", problems());
    }

    #[test]
    fn add_branch_resolves_source_and_flags() {
        let r = resolve(&["add", "owner/repo", "--org", "my-org"], &[]);
        assert_eq!(COMMAND.get_from(&r).as_deref(), Some("add"));
        assert_eq!(SOURCE.get_from(&r).as_deref(), Some("owner/repo"));
        assert_eq!(ORG.get_from(&r).as_deref(), Some("my-org"));
    }

    #[test]
    fn status_branch_does_not_resolve_add_inputs() {
        let r = resolve(&["status"], &[("APP_ADD_ORG", "leaked")]);
        assert_eq!(COMMAND.get_from(&r).as_deref(), Some("status"));
        assert_eq!(SOURCE.get_from(&r), None);
        assert_eq!(ORG.get_from(&r), None); // env gated too
    }

    #[test]
    fn global_flag_resolves_on_any_branch() {
        assert_eq!(
            JSON.get_from(&resolve(&["status", "--json"], &[])),
            Some(true)
        );
        assert_eq!(
            JSON.get_from(&resolve(&["add", "owner/repo", "--json"], &[])),
            Some(true)
        );
    }

    #[test]
    fn global_env_resolves_even_for_add_branch() {
        // APP_JSON is global — it must reach the resolution regardless of branch.
        // This is the MANGOHUD=1 proton start --enable-hdr pattern.
        let r = resolve(&["add", "owner/repo"], &[("APP_JSON", "true")]);
        assert_eq!(JSON.get_from(&r), Some(true));
        assert_eq!(SOURCE.get_from(&r).as_deref(), Some("owner/repo"));
    }

    #[test]
    fn add_env_resolves_when_add_is_selected() {
        let r = resolve(&["add"], &[("APP_ADD_SOURCE", "env-owner/repo")]);
        assert_eq!(SOURCE.get_from(&r).as_deref(), Some("env-owner/repo"));
    }
}
