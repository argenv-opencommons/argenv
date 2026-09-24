//! apt-shaped: global flag, command gate, per-branch positionals and flags.
//!
//! Models a slice of `apt`'s surface:
//!
//!   apt [-y] install <package> [--no-install-recommends]
//!   apt [-y] remove  <package>
//!   apt       update
//!   apt       search <query>
//!
//! Two things to notice:
//!
//! 1. `install` and `remove` each declare a package positional at slot 0 with
//!    different `gated_by` values. argenv only resolves the one whose branch
//!    the gate selected — the other is correctly absent.
//!
//! 2. `--no-install-recommends` is gated to `install` only. Passing it under
//!    `remove` or `search` produces an UnknownArg lint finding, same as any
//!    unrecognised flag, because the records list for those branches doesn't
//!    include it (the parser sees a known flag name, but lint catches it as
//!    outside the active branch).
//!
//! ```text
//! cargo run -p argenv --example apt -- install nginx
//! cargo run -p argenv --example apt -- -y remove nginx
//! cargo run -p argenv --example apt -- update
//! cargo run -p argenv --example apt -- search curl
//! APT_INSTALL_PACKAGE=nginx cargo run -p argenv --example apt -- install
//! ```
use argenv::*;

model! {
    // ── gate ─────────────────────────────────────────────────────────────────

    COMMAND: String = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: &["install", "remove", "update", "search"],
        arg:     Some(Arg::positional(0)),
        summary: "What to do.",
        ..Input::EMPTY
    };

    // ── install branch ───────────────────────────────────────────────────────

    INSTALL_PACKAGE: String = Input {
        key:      "install_package",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "install", env_prefix: "APT_INSTALL_" }),
        arg:      Some(Arg { value_name: "PACKAGE", ..Arg::positional(0) }),
        env:      Some(Env::new("APT_INSTALL_PACKAGE")),
        summary:  "Package name to install.",
        ..Input::EMPTY
    };

    SKIP_RECOMMENDS: bool = Input {
        key:      "skip_recommends",
        ty:       Type::Bool,
        default:  Some(false),
        gated_by: Some(GatedBy { value: "install", env_prefix: "APT_INSTALL_" }),
        // Real apt uses --no-install-recommends, but argenv forbids flags
        // starting with no- (that prefix is reserved for negating booleans,
        // e.g. --no-hdr negates --hdr). A wrapper targeting the real flag
        // name would rewrite the argv token before resolution.
        arg:      Some(Arg::long("skip-recommends")),
        env:      Some(Env::new("APT_INSTALL_SKIP_RECOMMENDS")),
        summary:  "Do not install recommended packages.",
        ..Input::EMPTY
    };

    // ── remove branch ────────────────────────────────────────────────────────

    // Same positional slot as INSTALL_PACKAGE, different branch.
    // argenv only resolves the one whose branch the gate selected.
    REMOVE_PACKAGE: String = Input {
        key:      "remove_package",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "remove", env_prefix: "APT_REMOVE_" }),
        arg:      Some(Arg { value_name: "PACKAGE", ..Arg::positional(0) }),
        env:      Some(Env::new("APT_REMOVE_PACKAGE")),
        summary:  "Package name to remove.",
        ..Input::EMPTY
    };

    // ── search branch ────────────────────────────────────────────────────────

    SEARCH_QUERY: String = Input {
        key:      "search_query",
        ty:       Type::String,
        gated_by: Some(GatedBy { value: "search", env_prefix: "APT_SEARCH_" }),
        arg:      Some(Arg { value_name: "QUERY", ..Arg::positional(0) }),
        env:      Some(Env::new("APT_SEARCH_QUERY")),
        summary:  "Search term.",
        ..Input::EMPTY
    };

    // ── global flags ─────────────────────────────────────────────────────────

    YES: bool = Input {
        key:     "yes",
        ty:      Type::Bool,
        default: Some(false),
        arg:     Some(Arg::pair("yes", 'y')),
        env:     Some(Env::new("APT_YES")),
        summary: "Automatic yes to prompts.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Model::parse_and_lint(OnProblems::FailOnError);
    let yes = Model::YES.get_from_or_default(&r).unwrap_or(false);

    match Model::COMMAND.get_from(&r).as_deref() {
        Some("install") => {
            let pkg = Model::INSTALL_PACKAGE.get_from(&r);
            let no_rec = Model::SKIP_RECOMMENDS.get_from_or_default(&r).unwrap_or(false);
            println!(
                "install: pkg={pkg:?} yes={yes} no-recommends={no_rec}"
            );
        }
        Some("remove") => {
            let pkg = Model::REMOVE_PACKAGE.get_from(&r);
            println!("remove: pkg={pkg:?} yes={yes}");
        }
        Some("update") => {
            println!("update: yes={yes}");
        }
        Some("search") => {
            let q = Model::SEARCH_QUERY.get_from(&r);
            println!("search: query={q:?}");
        }
        _ => {
            println!("usage: apt [-y] install|remove|update|search [PACKAGE|QUERY]");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn resolve(args: &[&str], env: &[(&str, &str)]) -> Resolution {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env: BTreeMap<String, String> =
            env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        Invocation { args: &args, env: &env }.resolve(&Model::records())
    }

    #[test]
    fn declaration_is_valid() {
        assert!(Model::problems().is_empty(), "{:#?}", Model::problems());
    }

    #[test]
    fn install_resolves_its_own_package_and_not_removes() {
        let r = resolve(&["install", "nginx"], &[]);
        assert_eq!(Model::COMMAND.get_from(&r).as_deref(), Some("install"));
        assert_eq!(Model::INSTALL_PACKAGE.get_from(&r).as_deref(), Some("nginx"));
        // remove_package slot is absent even though positionals[1] exists
        assert_eq!(Model::REMOVE_PACKAGE.get_from(&r), None);
    }

    #[test]
    fn remove_resolves_its_own_package_and_not_installs() {
        let r = resolve(&["remove", "nginx"], &[]);
        assert_eq!(Model::REMOVE_PACKAGE.get_from(&r).as_deref(), Some("nginx"));
        assert_eq!(Model::INSTALL_PACKAGE.get_from(&r), None);
        // branch-only flag for install is absent too
        assert_eq!(Model::SKIP_RECOMMENDS.get_from(&r), None);
    }

    #[test]
    fn no_recommends_is_absent_when_remove_selected() {
        // env set for install branch, but remove branch is active
        let r = resolve(&["remove", "nginx"], &[("APT_INSTALL_SKIP_RECOMMENDS", "1")]);
        assert_eq!(Model::SKIP_RECOMMENDS.get_from(&r), None);
    }

    #[test]
    fn global_yes_resolves_on_every_branch() {
        for cmd in &["install nginx", "remove nginx", "update", "search curl"] {
            let args: Vec<&str> = cmd.split_whitespace().collect();
            let mut full = vec!["-y"];
            full.extend(args);
            let r = resolve(&full, &[]);
            assert_eq!(
                Model::YES.get_from(&r),
                Some(true),
                "failed for: apt {cmd}"
            );
        }
    }

    #[test]
    fn global_yes_from_env_reaches_every_branch() {
        let r = resolve(&["update"], &[("APT_YES", "1")]);
        assert_eq!(Model::YES.get_from(&r), Some(true));
    }

    #[test]
    fn install_package_from_env_when_no_positional_given() {
        let r = resolve(&["install"], &[("APT_INSTALL_PACKAGE", "curl")]);
        assert_eq!(Model::INSTALL_PACKAGE.get_from(&r).as_deref(), Some("curl"));
    }

    #[test]
    fn install_env_does_not_bleed_into_search_branch() {
        let r = resolve(&["search"], &[("APT_INSTALL_PACKAGE", "leak")]);
        assert_eq!(Model::INSTALL_PACKAGE.get_from(&r), None);
    }
}
