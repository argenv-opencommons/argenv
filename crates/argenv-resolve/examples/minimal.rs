//! Every feature this crate has, with no metadata beyond a `key` and a `ty` —
//! no `summary`, no `example`, no `stability`, nothing that documents rather
//! than functions. Compare against `consumer.rs` (the fully-documented
//! reference) to see exactly what's optional.
//!
//! ```text
//! cargo run -p argenv-resolve --example minimal -- start db --force
//! cargo run -p argenv-resolve --example minimal -- stop db --timeout 30
//! cargo run -p argenv-resolve --example minimal -- --verbose status
//! APP_TIMEOUT=10 cargo run -p argenv-resolve --example minimal -- start db
//! ```
use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Start,
    Stop,
    Status,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["start", "stop", "status"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "start" => Command::Start,
            "stop" => Command::Stop,
            "status" => Command::Status,
            _ => return None,
        })
    }
}

contract! {
    // subcommand dispatch: a positional, Type::Enum
    COMMAND: Command = Input {
        key: "command",
        ty: Type::Enum,
        allowed: Command::TOKENS,
        arg: Some(Arg::positional(0)),
        ..Input::EMPTY
    };

    // per-branch positional: only meaningful for start/stop, not status
    SERVICE: String, Command = Input {
        key: "service",
        ty: Type::String,
        subcommands: &[Command::Start, Command::Stop],
        arg: Some(Arg::positional(0)),
        ..Input::EMPTY
    };

    // shared across two subcommands, not the third
    TIMEOUT: u16, Command = Input {
        key: "timeout",
        ty: Type::Uint,
        default: Some(5),
        subcommands: &[Command::Start, Command::Stop],
        arg: Some(Arg::long("timeout")),
        env: Some(Env::new("APP_TIMEOUT")),
        ..Input::EMPTY
    };

    // unique to one subcommand
    FORCE: bool, Command = Input {
        key: "force",
        ty: Type::Bool,
        default: Some(false),
        subcommands: &[Command::Start],
        arg: Some(Arg::long("force")),
        ..Input::EMPTY
    };

    // global: no subcommands list, active regardless of dispatch
    VERBOSE: bool = Input {
        key: "verbose",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::pair("verbose", 'v')),
        env: Some(Env::new("APP_VERBOSE")),
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);

    let verbose = Contract::VERBOSE.get_from_or_default(&r).unwrap_or(false);
    match Contract::COMMAND.get_from(&r) {
        Some(Command::Start) => {
            let service = Contract::SERVICE.get_from(&r);
            let timeout = Contract::TIMEOUT.get_from_or_default(&r);
            let force = Contract::FORCE.get_from_or_default(&r).unwrap_or(false);
            println!(
                "start service={service:?} timeout={timeout:?} force={force} verbose={verbose}"
            );
        }
        Some(Command::Stop) => {
            let service = Contract::SERVICE.get_from(&r);
            let timeout = Contract::TIMEOUT.get_from_or_default(&r);
            println!("stop service={service:?} timeout={timeout:?} verbose={verbose}");
        }
        Some(Command::Status) => {
            println!("status verbose={verbose}");
        }
        None => println!("usage: minimal [--verbose] start|stop <service>|status"),
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
    fn every_feature_resolves_correctly_together() {
        let r = resolve(
            &["--verbose", "start", "db", "--force"],
            &[("APP_TIMEOUT", "30")],
        );
        assert_eq!(Contract::COMMAND.get_from(&r), Some(Command::Start));
        assert_eq!(Contract::SERVICE.get_from(&r).as_deref(), Some("db"));
        assert_eq!(Contract::FORCE.get_from(&r), Some(true));
        assert_eq!(Contract::TIMEOUT.get_from(&r), Some(30));
        assert_eq!(Contract::VERBOSE.get_from(&r), Some(true));
    }

    #[test]
    fn status_takes_neither_service_nor_timeout_nor_force() {
        let r = resolve(&["status"], &[("APP_TIMEOUT", "30")]);
        assert_eq!(Contract::SERVICE.get_from(&r), None);
        assert_eq!(Contract::TIMEOUT.get_from(&r), None);
        assert_eq!(Contract::FORCE.get_from(&r), None);
    }
}
