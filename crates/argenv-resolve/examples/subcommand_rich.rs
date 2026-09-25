//! Rich subcommand usage: an option genuinely shared across two subcommands
//! (not duplicated, not wrongly made global), options unique to one branch
//! each, and a global flag active regardless of dispatch.
//!
//! A package-manager-shaped surface:
//!
//!   pkg [--json] install <name> [--force] [--dry-run]
//!   pkg [--json] remove  <name> [--purge]  [--dry-run]
//!   pkg [--json] list
//!
//! ```text
//! cargo run -p argenv --example subcommand_rich -- install curl --force
//! cargo run -p argenv --example subcommand_rich -- remove curl --purge --dry-run
//! cargo run -p argenv --example subcommand_rich -- --json list
//! PKG_DRY_RUN=1 cargo run -p argenv --example subcommand_rich -- install curl
//! ```
use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Install,
    Remove,
    List,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["install", "remove", "list"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "install" => Command::Install,
            "remove" => Command::Remove,
            "list" => Command::List,
            _ => return None,
        })
    }
}

contract! {
    // ── dispatch ─────────────────────────────────────────────────────────────

    COMMAND: Command = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: Command::TOKENS,
        arg:     Some(Arg::positional(0)),
        summary: "Which operation to run.",
        ..Input::EMPTY
    };

    // ── per-branch positional, one slot shared by two branches ──────────────

    // install and remove each take a package name right after the command
    // token. Two different Inputs, not one shared Input — the position (0,
    // relative to the command) is the same for both, but the *identity*
    // (key, and therefore the resolved value's meaning) is different, which
    // is correct: "the package being installed" and "the package being
    // removed" are different settings even though they occupy the same slot.
    INSTALL_NAME: String, Command = Input {
        key:         "install_name",
        ty:          Type::String,
        subcommands: &[Command::Install],
        arg:         Some(Arg { value_name: "NAME", ..Arg::positional(0) }),
        summary:     "Package to install.",
        ..Input::EMPTY
    };

    REMOVE_NAME: String, Command = Input {
        key:         "remove_name",
        ty:          Type::String,
        subcommands: &[Command::Remove],
        arg:         Some(Arg { value_name: "NAME", ..Arg::positional(0) }),
        summary:     "Package to remove.",
        ..Input::EMPTY
    };

    // ── one option, genuinely shared across two subcommands ─────────────────

    // The case that motivated subcommands being a *list*: install and
    // remove both want --dry-run, but list does not (nothing to dry-run
    // when nothing changes). One declaration, not two, and not wrongly
    // made global either.
    DRY_RUN: bool, Command = Input {
        key:         "dry_run",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Install, Command::Remove],
        arg:         Some(Arg::long("dry-run")),
        env:         Some(Env::new("PKG_DRY_RUN")),
        summary:     "Show what would happen without doing it.",
        ..Input::EMPTY
    };

    // ── options unique to one branch each ────────────────────────────────────

    FORCE: bool, Command = Input {
        key:         "force",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Install],
        arg:         Some(Arg::long("force")),
        summary:     "Install even if already present.",
        ..Input::EMPTY
    };

    PURGE: bool, Command = Input {
        key:         "purge",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Remove],
        arg:         Some(Arg::long("purge")),
        env:         Some(Env::new("PKG_REMOVE_PURGE")),
        summary:     "Also remove configuration files.",
        ..Input::EMPTY
    };

    // ── global — no subcommands list at all, always active ──────────────────

    JSON: bool = Input {
        key:     "json",
        ty:      Type::Bool,
        default: Some(false),
        arg:     Some(Arg::long("json")),
        env:     Some(Env::new("PKG_JSON")),
        summary: "Machine-readable output.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);
    let json = Contract::JSON.get_from_or_default(&r).unwrap_or(false);

    match Contract::COMMAND.get_from(&r) {
        Some(Command::Install) => {
            let name = Contract::INSTALL_NAME.get_from(&r);
            let force = Contract::FORCE.get_from_or_default(&r).unwrap_or(false);
            let dry_run = Contract::DRY_RUN.get_from_or_default(&r).unwrap_or(false);
            if json {
                println!(
                    "{}",
                    serde_json::json!({"command": "install", "name": name, "force": force, "dry_run": dry_run})
                );
            } else {
                println!("install: name={name:?} force={force} dry-run={dry_run}");
            }
        }
        Some(Command::Remove) => {
            let name = Contract::REMOVE_NAME.get_from(&r);
            let purge = Contract::PURGE.get_from_or_default(&r).unwrap_or(false);
            let dry_run = Contract::DRY_RUN.get_from_or_default(&r).unwrap_or(false);
            if json {
                println!(
                    "{}",
                    serde_json::json!({"command": "remove", "name": name, "purge": purge, "dry_run": dry_run})
                );
            } else {
                println!("remove: name={name:?} purge={purge} dry-run={dry_run}");
            }
        }
        Some(Command::List) => {
            println!("list: (nothing installed yet)");
        }
        None => {
            println!("usage: pkg [--json] install|remove|list ...");
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
        .resolve(&Contract::records())
    }

    #[test]
    fn declaration_is_valid() {
        assert!(
            Contract::problems().is_empty(),
            "{:#?}",
            Contract::problems()
        );
    }

    #[test]
    fn install_resolves_its_own_name_and_not_removes() {
        let r = resolve(&["install", "curl", "--force"], &[]);
        assert_eq!(Contract::COMMAND.get_from(&r), Some(Command::Install));
        assert_eq!(Contract::INSTALL_NAME.get_from(&r).as_deref(), Some("curl"));
        assert_eq!(Contract::FORCE.get_from(&r), Some(true));
        // remove-only inputs are absent even though "curl" also occupies
        // relative positional slot 0 under a different command
        assert_eq!(Contract::REMOVE_NAME.get_from(&r), None);
        assert_eq!(Contract::PURGE.get_from(&r), None);
    }

    #[test]
    fn remove_resolves_its_own_name_and_not_installs() {
        let r = resolve(&["remove", "curl", "--purge"], &[]);
        assert_eq!(Contract::REMOVE_NAME.get_from(&r).as_deref(), Some("curl"));
        assert_eq!(Contract::PURGE.get_from(&r), Some(true));
        assert_eq!(Contract::INSTALL_NAME.get_from(&r), None);
        assert_eq!(Contract::FORCE.get_from(&r), None);
    }

    #[test]
    fn dry_run_resolves_on_both_install_and_remove() {
        let install = resolve(&["install", "curl", "--dry-run"], &[]);
        let remove = resolve(&["remove", "curl", "--dry-run"], &[]);
        assert_eq!(Contract::DRY_RUN.get_from(&install), Some(true));
        assert_eq!(Contract::DRY_RUN.get_from(&remove), Some(true));
    }

    #[test]
    fn dry_run_is_absent_under_list_even_via_env() {
        // dry_run is scoped to install/remove only - env must not leak into list
        let r = resolve(&["list"], &[("PKG_DRY_RUN", "1")]);
        assert_eq!(Contract::DRY_RUN.get_from(&r), None);
    }

    #[test]
    fn purge_env_resolves_under_remove() {
        let r = resolve(&["remove", "curl"], &[("PKG_REMOVE_PURGE", "1")]);
        assert_eq!(Contract::PURGE.get_from(&r), Some(true));
    }

    #[test]
    fn purge_env_does_not_leak_into_install() {
        let r = resolve(&["install", "curl"], &[("PKG_REMOVE_PURGE", "1")]);
        assert_eq!(Contract::PURGE.get_from(&r), None);
    }

    #[test]
    fn global_json_flag_resolves_on_every_branch_including_none() {
        for args in [
            vec!["--json", "install", "curl"],
            vec!["--json", "remove", "curl"],
            vec!["--json", "list"],
            vec!["--json"],
        ] {
            let r = resolve(&args, &[]);
            assert_eq!(
                Contract::JSON.get_from(&r),
                Some(true),
                "failed for {args:?}"
            );
        }
    }

    #[test]
    fn global_json_env_reaches_every_branch() {
        let r = resolve(&["install", "curl"], &[("PKG_JSON", "1")]);
        assert_eq!(Contract::JSON.get_from(&r), Some(true));
    }

    #[test]
    fn list_takes_no_positional_and_none_resolve() {
        let r = resolve(&["list", "ignored-extra-token"], &[]);
        assert_eq!(Contract::COMMAND.get_from(&r), Some(Command::List));
        assert_eq!(Contract::INSTALL_NAME.get_from(&r), None);
        assert_eq!(Contract::REMOVE_NAME.get_from(&r), None);
    }
}
