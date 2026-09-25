//! A slice of Proton's real invocation surface. Proton is configured almost
//! entirely through environment variables, not flags — `PROTON_USE_WINED3D`,
//! `PROTON_NO_ESYNC`, and `PROTON_LOG` are genuine, currently-documented
//! Proton runtime options (see `ValveSoftware/Proton`'s own README and the
//! Proton-GE/CachyOS forks' identical documentation of them). This is the
//! case the crate-level docs describe as "environment only, because it is
//! set by a launcher, not a person" — here the launcher is Steam itself,
//! writing these into the child process's environment before exec.
//!
//! ```text
//! proton run game.exe
//! PROTON_USE_WINED3D=1 proton run game.exe
//! PROTON_NO_ESYNC=1 PROTON_LOG=1 proton run game.exe --windowed
//! proton waitforexitandrun game.exe
//! ```
//!
//! ```text
//! cargo run -p argenv-resolve --example proton -- run game.exe
//! PROTON_USE_WINED3D=1 cargo run -p argenv-resolve --example proton -- run game.exe
//! PROTON_NO_ESYNC=1 PROTON_LOG=1 cargo run -p argenv-resolve --example proton -- run game.exe
//! cargo run -p argenv-resolve --example proton -- waitforexitandrun game.exe
//! ```
use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Run,
    Waitforexitandrun,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["run", "waitforexitandrun"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "run" => Command::Run,
            "waitforexitandrun" => Command::Waitforexitandrun,
            _ => return None,
        })
    }
}

contract! {
    COMMAND: Command = Input {
        key:     "command",
        ty:      Type::Enum,
        allowed: Command::TOKENS,
        arg:     Some(Arg::positional(0)),
        summary: "run launches and detaches; waitforexitandrun blocks until the game exits.",
        ..Input::EMPTY
    };

    // The wrapped Windows executable, plus anything after it - Proton passes
    // the rest of argv straight through to the game, unexamined.
    EXE: String, Command = Input {
        key:         "exe",
        ty:          Type::String,
        subcommands: &[Command::Run, Command::Waitforexitandrun],
        arg:         Some(Arg { value_name: "EXE", ..Arg::positional(0) }),
        summary:     "The Windows executable to run under Proton.",
        ..Input::EMPTY
    };

    // ── real Proton runtime options: environment only, no flag form ─────────
    //
    // Scoped to both subcommands, not left global: these apply whenever a
    // game is actually being launched (which is what both subcommands do),
    // not to some hypothetical non-launch subcommand Proton doesn't have -
    // subcommands: &[..] with two entries is the accurate shape here, not
    // an empty (fully global) list.

    USE_WINED3D: bool, Command = Input {
        key:         "use_wined3d",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run, Command::Waitforexitandrun],
        env:         Some(Env::new("PROTON_USE_WINED3D")),
        summary:     "Use OpenGL-based wined3d instead of Vulkan-based DXVK.",
        ..Input::EMPTY
    };

    NO_ESYNC: bool, Command = Input {
        key:         "no_esync",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run, Command::Waitforexitandrun],
        env:         Some(Env::new("PROTON_NO_ESYNC")),
        summary:     "Do not use eventfd-based in-process synchronization primitives.",
        ..Input::EMPTY
    };

    LOG: bool, Command = Input {
        key:         "log",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run, Command::Waitforexitandrun],
        env:         Some(Env::new("PROTON_LOG")),
        summary:     "Dump a debug log to $HOME/steam-$APPID.log.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);

    let use_wined3d = Contract::USE_WINED3D
        .get_from_or_default(&r)
        .unwrap_or(false);
    let no_esync = Contract::NO_ESYNC.get_from_or_default(&r).unwrap_or(false);
    let log = Contract::LOG.get_from_or_default(&r).unwrap_or(false);

    match Contract::COMMAND.get_from(&r) {
        Some(cmd) => {
            let exe = Contract::EXE.get_from(&r);
            println!("{cmd:?} exe={exe:?} use_wined3d={use_wined3d} no_esync={no_esync} log={log}");
        }
        None => println!("usage: proton run|waitforexitandrun EXE"),
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
    fn run_resolves_the_wrapped_executable() {
        let r = resolve(&["run", "game.exe"], &[]);
        assert_eq!(Contract::COMMAND.get_from(&r), Some(Command::Run));
        assert_eq!(Contract::EXE.get_from(&r).as_deref(), Some("game.exe"));
    }

    #[test]
    fn real_proton_env_vars_resolve_with_no_flag_equivalent() {
        let r = resolve(
            &["run", "game.exe"],
            &[
                ("PROTON_USE_WINED3D", "1"),
                ("PROTON_NO_ESYNC", "1"),
                ("PROTON_LOG", "1"),
            ],
        );
        assert_eq!(Contract::USE_WINED3D.get_from(&r), Some(true));
        assert_eq!(Contract::NO_ESYNC.get_from(&r), Some(true));
        assert_eq!(Contract::LOG.get_from(&r), Some(true));
    }

    #[test]
    fn absent_env_vars_default_to_false_not_none() {
        let r = resolve(&["run", "game.exe"], &[]);
        assert_eq!(Contract::USE_WINED3D.get_from_or_default(&r), Some(false));
    }

    #[test]
    fn waitforexitandrun_also_takes_an_executable() {
        let r = resolve(&["waitforexitandrun", "game.exe"], &[]);
        assert_eq!(
            Contract::COMMAND.get_from(&r),
            Some(Command::Waitforexitandrun)
        );
        assert_eq!(Contract::EXE.get_from(&r).as_deref(), Some("game.exe"));
    }
}
