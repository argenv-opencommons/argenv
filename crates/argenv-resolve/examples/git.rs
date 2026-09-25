//! A slice of `git`'s real invocation surface:
//!
//!   git [--no-pager] [-C PATH] commit [-m MSG] [--amend] [-a] [--no-edit]
//!   git [--no-pager] [-C PATH] push   [remote] [branch] [--force] [-u]
//!
//! `commit` carries four of git's actual, everyday flags — `-m/--message`,
//! `--amend`, `-a/--all`, `--no-edit` — the case the task asked for: at
//! least three real options on one real subcommand, not invented ones.
//!
//! ```text
//! cargo run -p argenv-resolve --example git -- commit -m "fix typo" -a
//! cargo run -p argenv-resolve --example git -- commit --amend --no-edit
//! cargo run -p argenv-resolve --example git -- push origin main --force
//! cargo run -p argenv-resolve --example git -- --no-pager -C /repo push -u
//! ```
use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Commit,
    Push,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["commit", "push"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "commit" => Command::Commit,
            "push" => Command::Push,
            _ => return None,
        })
    }
}

contract! {
    // ── global: real git flags that work before any subcommand ─────────────

    NO_PAGER: bool = Input {
        key:     "no_pager",
        ty:      Type::Bool,
        default: Some(false),
        arg:     Some(Arg::long("no-pager")),
        summary: "Do not pipe output through a pager.",
        ..Input::EMPTY
    };

    WORK_DIR: String = Input {
        key:     "work_dir",
        ty:      Type::String,
        arg:     Some(Arg { value_name: "PATH", ..Arg::short('C') }),
        summary: "Run as if git was started in PATH.",
        ..Input::EMPTY
    };

    // ── dispatch ─────────────────────────────────────────────────────────────

    COMMAND: Command = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: Command::TOKENS,
        arg:     Some(Arg::positional(0)),
        summary: "Which git operation to run.",
        ..Input::EMPTY
    };

    // ── commit: four of git's real, everyday flags ──────────────────────────

    MESSAGE: String, Command = Input {
        key:         "message",
        ty:          Type::String,
        subcommands: &[Command::Commit],
        arg:         Some(Arg { value_name: "MSG", ..Arg::pair("message", 'm') }),
        summary:     "Commit message.",
        ..Input::EMPTY
    };

    AMEND: bool, Command = Input {
        key:         "amend",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Commit],
        arg:         Some(Arg::long("amend")),
        summary:     "Amend the previous commit instead of creating a new one.",
        ..Input::EMPTY
    };

    ALL: bool, Command = Input {
        key:         "all",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Commit],
        arg:         Some(Arg::pair("all", 'a')),
        summary:     "Stage all tracked, modified files before committing.",
        ..Input::EMPTY
    };

    NO_EDIT: bool, Command = Input {
        key:         "no_edit",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Commit],
        arg:         Some(Arg::long("no-edit")),
        summary:     "Use the existing commit message without launching an editor.",
        ..Input::EMPTY
    };

    // ── push: two real positionals plus two real flags ──────────────────────

    REMOTE: String, Command = Input {
        key:         "remote",
        ty:          Type::String,
        subcommands: &[Command::Push],
        arg:         Some(Arg { value_name: "REMOTE", ..Arg::positional(0) }),
        summary:     "Remote to push to (defaults to origin).",
        ..Input::EMPTY
    };

    BRANCH: String, Command = Input {
        key:         "branch",
        ty:          Type::String,
        subcommands: &[Command::Push],
        arg:         Some(Arg { value_name: "BRANCH", ..Arg::positional(1) }),
        summary:     "Branch to push (defaults to the current branch).",
        ..Input::EMPTY
    };

    FORCE: bool, Command = Input {
        key:         "force",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Push],
        arg:         Some(Arg::long("force")),
        summary:     "Force push, overwriting the remote's history.",
        ..Input::EMPTY
    };

    SET_UPSTREAM: bool, Command = Input {
        key:         "set_upstream",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Push],
        arg:         Some(Arg::pair("set-upstream", 'u')),
        summary:     "Set the pushed branch as the tracking branch.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);
    if let Some(dir) = Contract::WORK_DIR.get_from(&r) {
        println!("(running in {dir})");
    }

    match Contract::COMMAND.get_from(&r) {
        Some(Command::Commit) => {
            let message = Contract::MESSAGE.get_from(&r);
            let amend = Contract::AMEND.get_from_or_default(&r).unwrap_or(false);
            let all = Contract::ALL.get_from_or_default(&r).unwrap_or(false);
            let no_edit = Contract::NO_EDIT.get_from_or_default(&r).unwrap_or(false);
            println!("commit message={message:?} amend={amend} all={all} no-edit={no_edit}");
        }
        Some(Command::Push) => {
            let remote = Contract::REMOTE.get_from(&r);
            let branch = Contract::BRANCH.get_from(&r);
            let force = Contract::FORCE.get_from_or_default(&r).unwrap_or(false);
            let set_upstream = Contract::SET_UPSTREAM
                .get_from_or_default(&r)
                .unwrap_or(false);
            println!("push remote={remote:?} branch={branch:?} force={force} set-upstream={set_upstream}");
        }
        None => println!("usage: git [--no-pager] [-C PATH] commit|push ..."),
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
    fn commit_resolves_all_four_real_flags_together() {
        let r = resolve(&["commit", "-m", "fix typo", "-a", "--no-edit"]);
        assert_eq!(Contract::MESSAGE.get_from(&r).as_deref(), Some("fix typo"));
        assert_eq!(Contract::ALL.get_from(&r), Some(true));
        assert_eq!(Contract::NO_EDIT.get_from(&r), Some(true));
        assert_eq!(Contract::AMEND.get_from(&r), Some(false));
        // push-only inputs are absent
        assert_eq!(Contract::REMOTE.get_from(&r), None);
        assert_eq!(Contract::FORCE.get_from(&r), None);
    }

    #[test]
    fn push_resolves_two_positionals_and_two_flags() {
        let r = resolve(&["push", "origin", "main", "--force", "-u"]);
        assert_eq!(Contract::REMOTE.get_from(&r).as_deref(), Some("origin"));
        assert_eq!(Contract::BRANCH.get_from(&r).as_deref(), Some("main"));
        assert_eq!(Contract::FORCE.get_from(&r), Some(true));
        assert_eq!(Contract::SET_UPSTREAM.get_from(&r), Some(true));
        // commit-only inputs are absent
        assert_eq!(Contract::MESSAGE.get_from(&r), None);
    }

    #[test]
    fn global_flags_work_before_the_subcommand() {
        let r = resolve(&["--no-pager", "-C", "/repo", "push", "-u"]);
        assert_eq!(Contract::NO_PAGER.get_from(&r), Some(true));
        assert_eq!(Contract::WORK_DIR.get_from(&r).as_deref(), Some("/repo"));
        assert_eq!(Contract::SET_UPSTREAM.get_from(&r), Some(true));
    }
}
