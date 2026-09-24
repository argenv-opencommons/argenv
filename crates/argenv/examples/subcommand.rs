//! The subcommand shape: one positional gate, per-branch flags, global flags.
//!
//! Models the pattern all serious CLI tools use:
//!
//!   program [global-flags] COMMAND [command-flags] [command-positional]
//!
//! A shallow Rhizoid-shaped surface:
//!
//!   rhizoid [--json] add <owner/repo> [--org ORG] [--tracked-ref REF]
//!   rhizoid [--json] status
//!
//! ```text
//! cargo run -p argenv --example subcommand -- add owner/repo --org my-org
//! cargo run -p argenv --example subcommand -- status --json
//! APP_ADD_ORG=env-org cargo run -p argenv --example subcommand -- add owner/repo
//! ```
use argenv::*;

model! {
    // ── gate: selects the active branch ──────────────────────────────────────

    COMMAND: String = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: &["add", "status"],
        arg:     Some(Arg::positional(0)),
        summary: "Which command to run.",
        ..Input::EMPTY
    };

    // ── add branch ───────────────────────────────────────────────────────────

    /// `add`'s required value: `rhizoid add <owner/repo>`.
    SOURCE: String = Input {
        key:      "source",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD_" }),
        arg:      Some(Arg { value_name: "REPO", ..Arg::positional(0) }),
        env:      Some(Env::new("APP_ADD_SOURCE")),
        summary:  "Upstream repository to fork, owner/repo.",
        ..Input::EMPTY
    };

    ORG: String = Input {
        key:      "org",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD_" }),
        arg:      Some(Arg { value_name: "ORG", ..Arg::long("org") }),
        env:      Some(Env::new("APP_ADD_ORG")),
        summary:  "GitHub org to fork into.",
        ..Input::EMPTY
    };

    TRACKED_REF: String = Input {
        key:      "tracked_ref",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD_" }),
        arg:      Some(Arg { value_name: "REF", ..Arg::long("tracked-ref") }),
        env:      Some(Env::new("APP_ADD_TRACKED_REF")),
        summary:  "Branch or tag to track.",
        ..Input::EMPTY
    };

    // ── global flags — always active regardless of branch ────────────────────

    JSON: bool = Input {
        key:     "json",
        ty:      Type::Bool,
        default: Some(false),
        arg:     Some(Arg::long("json")),
        env:     Some(Env::new("APP_JSON")),
        summary: "Machine-readable output.",
        ..Input::EMPTY
    };
}

fn main() {
    // One call: parse, lint, fail on errors. COMMAND is a required positional;
    // lint will surface its absence as a MissingRequired error automatically.
    let resolved = Model::parse_and_lint(OnProblems::FailOnError);

    match Model::COMMAND.get_from(&resolved).as_deref() {
        Some("add") => {
            let source = Model::SOURCE.get_from(&resolved);
            let org = Model::ORG.get_from(&resolved);
            let tracked_ref = Model::TRACKED_REF.get_from(&resolved);
            let json = Model::JSON.get_from_or_default(&resolved).unwrap_or(false);

            if source.is_none() {
                eprintln!("error: missing <REPO>");
                std::process::exit(1);
            }

            if json {
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
            let json = Model::JSON.get_from_or_default(&resolved).unwrap_or(false);
            if json {
                println!("{}", serde_json::json!({"command": "status"}));
            } else {
                println!("status: (not implemented yet)");
            }
        }
        _ => {
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
        .resolve(&Model::records())
    }

    #[test]
    fn declaration_is_valid() {
        assert!(Model::problems().is_empty(), "{:#?}", Model::problems());
    }

    #[test]
    fn add_branch_resolves_source_and_flags() {
        let r = resolve(&["add", "owner/repo", "--org", "my-org"], &[]);
        assert_eq!(Model::COMMAND.get_from(&r).as_deref(), Some("add"));
        assert_eq!(Model::SOURCE.get_from(&r).as_deref(), Some("owner/repo"));
        assert_eq!(Model::ORG.get_from(&r).as_deref(), Some("my-org"));
    }

    #[test]
    fn status_branch_does_not_resolve_add_inputs() {
        let r = resolve(&["status"], &[("APP_ADD_ORG", "leaked")]);
        assert_eq!(Model::SOURCE.get_from(&r), None);
        assert_eq!(Model::ORG.get_from(&r), None);
    }

    #[test]
    fn global_env_resolves_on_any_branch() {
        // MANGOHUD=1 proton add owner/repo — global env reaches through the gate
        let r = resolve(&["add", "owner/repo"], &[("APP_JSON", "true")]);
        assert_eq!(Model::JSON.get_from(&r), Some(true));
    }
}
