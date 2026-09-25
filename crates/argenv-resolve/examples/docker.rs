//! A slice of `docker`'s real invocation surface:
//!
//!   docker [-H HOST] run [--rm] [-d] [-p HOST:CONTAINER] [--name NAME] IMAGE
//!   docker [-H HOST] ps  [-a]
//!
//! `run` carries four of docker's actual, everyday flags — `--rm`,
//! `-d/--detach`, `-p/--publish`, `--name` — the same shape as `git.rs`:
//! at least three real options on one real subcommand.
//!
//! ```text
//! cargo run -p argenv-resolve --example docker -- run --rm -d nginx
//! cargo run -p argenv-resolve --example docker -- run -p 8080:80 --name web nginx
//! cargo run -p argenv-resolve --example docker -- ps -a
//! cargo run -p argenv-resolve --example docker -- -H tcp://remote:2375 ps
//! ```
use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Run,
    Ps,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["run", "ps"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "run" => Command::Run,
            "ps" => Command::Ps,
            _ => return None,
        })
    }
}

contract! {
    // ── global: connect to a different daemon, before any subcommand ───────

    HOST: String = Input {
        key:     "host",
        ty:      Type::String,
        arg:     Some(Arg { value_name: "HOST", ..Arg::short('H') }),
        summary: "Daemon socket to connect to.",
        ..Input::EMPTY
    };

    // ── dispatch ─────────────────────────────────────────────────────────────

    COMMAND: Command = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: Command::TOKENS,
        arg:     Some(Arg::positional(0)),
        summary: "Which docker operation to run.",
        ..Input::EMPTY
    };

    // ── run: the image, plus four of docker's real, everyday flags ─────────

    IMAGE: String, Command = Input {
        key:         "image",
        ty:          Type::String,
        subcommands: &[Command::Run],
        arg:         Some(Arg { value_name: "IMAGE", ..Arg::positional(0) }),
        summary:     "Image to run.",
        ..Input::EMPTY
    };

    RM: bool, Command = Input {
        key:         "rm",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run],
        arg:         Some(Arg::long("rm")),
        summary:     "Automatically remove the container when it exits.",
        ..Input::EMPTY
    };

    DETACH: bool, Command = Input {
        key:         "detach",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run],
        arg:         Some(Arg::pair("detach", 'd')),
        summary:     "Run the container in the background.",
        ..Input::EMPTY
    };

    PUBLISH: String, Command = Input {
        key:         "publish",
        ty:          Type::String,
        subcommands: &[Command::Run],
        arg:         Some(Arg { value_name: "HOST:CONTAINER", ..Arg::pair("publish", 'p') }),
        summary:     "Publish a container port to the host.",
        ..Input::EMPTY
    };

    NAME: String, Command = Input {
        key:         "name",
        ty:          Type::String,
        subcommands: &[Command::Run],
        arg:         Some(Arg { value_name: "NAME", ..Arg::long("name") }),
        summary:     "Assign a name to the container.",
        ..Input::EMPTY
    };

    // ── ps: its own, unrelated flag ─────────────────────────────────────────

    ALL: bool, Command = Input {
        key:         "all",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Ps],
        arg:         Some(Arg::pair("all", 'a')),
        summary:     "Show all containers, not just running ones.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);
    if let Some(host) = Contract::HOST.get_from(&r) {
        println!("(connecting to {host})");
    }

    match Contract::COMMAND.get_from(&r) {
        Some(Command::Run) => {
            let image = Contract::IMAGE.get_from(&r);
            let rm = Contract::RM.get_from_or_default(&r).unwrap_or(false);
            let detach = Contract::DETACH.get_from_or_default(&r).unwrap_or(false);
            let publish = Contract::PUBLISH.get_from(&r);
            let name = Contract::NAME.get_from(&r);
            println!(
                "run image={image:?} rm={rm} detach={detach} publish={publish:?} name={name:?}"
            );
        }
        Some(Command::Ps) => {
            let all = Contract::ALL.get_from_or_default(&r).unwrap_or(false);
            println!("ps all={all}");
        }
        None => println!("usage: docker [-H HOST] run|ps ..."),
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
    fn run_resolves_image_and_all_four_real_flags_together() {
        let r = resolve(&[
            "run", "--rm", "-d", "-p", "8080:80", "--name", "web", "nginx",
        ]);
        assert_eq!(Contract::IMAGE.get_from(&r).as_deref(), Some("nginx"));
        assert_eq!(Contract::RM.get_from(&r), Some(true));
        assert_eq!(Contract::DETACH.get_from(&r), Some(true));
        assert_eq!(Contract::PUBLISH.get_from(&r).as_deref(), Some("8080:80"));
        assert_eq!(Contract::NAME.get_from(&r).as_deref(), Some("web"));
        // ps-only input is absent
        assert_eq!(Contract::ALL.get_from(&r), None);
    }

    #[test]
    fn ps_resolves_its_own_flag_and_not_runs_flags() {
        let r = resolve(&["ps", "-a"]);
        assert_eq!(Contract::ALL.get_from(&r), Some(true));
        assert_eq!(Contract::IMAGE.get_from(&r), None);
        assert_eq!(Contract::RM.get_from(&r), None);
    }

    #[test]
    fn global_host_flag_works_before_either_subcommand() {
        let r = resolve(&["-H", "tcp://remote:2375", "ps"]);
        assert_eq!(
            Contract::HOST.get_from(&r).as_deref(),
            Some("tcp://remote:2375")
        );
        assert_eq!(Contract::COMMAND.get_from(&r), Some(Command::Ps));
    }
}
