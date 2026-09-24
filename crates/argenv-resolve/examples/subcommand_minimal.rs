//! The floor for subcommands: two branches, nothing shared, no per-branch
//! options at all. Compare against `names_only.rs` (the floor with no
//! subcommands) to see exactly what declaring a subcommand costs on top.
//!
//! ```text
//! cargo run -p argenv --example subcommand_minimal -- start
//! cargo run -p argenv --example subcommand_minimal -- stop
//! cargo run -p argenv --example subcommand_minimal -- restart
//! ```
use argenv_resolve::*;

/// The program's own subcommand enum — a real, exhaustively-matchable type,
/// not a string. `derive(Serialize)` with `rename_all = "snake_case"` is
/// what lets `subcommands: &[Command::Start]` (on a *scoped* input, not
/// used here) turn into the wire token `"start"` — see `subcommand_rich.rs`
/// for that in use.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Start,
    Stop,
    Restart,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["start", "stop", "restart"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "start" => Command::Start,
            "stop" => Command::Stop,
            "restart" => Command::Restart,
            _ => return None,
        })
    }
}

contract! {
    // The dispatch input: an ordinary Type::Enum input, positional instead
    // of a flag. Nothing about it is special-cased — it carries a summary,
    // an example, could carry stability/since/deprecation, exactly like
    // any other input.
    COMMAND: Command = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: Command::TOKENS,
        arg:     Some(Arg::positional(0)),
        summary: "Which action to take.",
        example: "start",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);

    match Contract::COMMAND.get_from(&r) {
        Some(Command::Start) => println!("starting"),
        Some(Command::Stop) => println!("stopping"),
        Some(Command::Restart) => println!("restarting"),
        None => println!("usage: subcommand_minimal start|stop|restart"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn resolve(args: &[&str]) -> Resolution {
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let env: BTreeMap<String, String> = BTreeMap::new();
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
    fn each_token_resolves_to_its_own_exhaustively_matched_variant() {
        assert_eq!(
            Contract::COMMAND.get_from(&resolve(&["start"])),
            Some(Command::Start)
        );
        assert_eq!(
            Contract::COMMAND.get_from(&resolve(&["stop"])),
            Some(Command::Stop)
        );
        assert_eq!(
            Contract::COMMAND.get_from(&resolve(&["restart"])),
            Some(Command::Restart)
        );
    }

    #[test]
    fn no_command_given_resolves_to_none() {
        assert_eq!(Contract::COMMAND.get_from(&resolve(&[])), None);
    }

    #[test]
    fn an_unknown_token_does_not_resolve_and_is_linted() {
        let r = resolve(&["frobnicate"]);
        assert_eq!(Contract::COMMAND.get_from(&r), None);
        let findings = lint(&Contract::records(), &r);
        assert!(
            !findings.is_empty(),
            "an out-of-domain token should be flagged"
        );
    }
}
