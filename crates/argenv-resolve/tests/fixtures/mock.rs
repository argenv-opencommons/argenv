//! Not a user-facing example — a real, compiled program that exists only so
//! `tests/integration.rs` can spawn it as a subprocess and check its actual
//! behaviour with real argv and real env vars, not mocked resolution calls.
//!
//! Deliberately exercises every feature in one surface, plus the specific
//! anti-patterns that must be rejected:
//!
//!   mock [--verbose] [--config PATH] build `<target>` [--release] [--jobs N]
//!   mock [--verbose] [--config PATH] run   `<target>` [--jobs N] [--watch]
//!   mock [--verbose] [--config PATH] clean
//!
//! - `--verbose`, `--config`: global, active on every subcommand and with none
//! - `--jobs`: shared between build and run, not clean (the case that
//!   motivated `subcommands` being a list)
//! - `--release`: unique to build
//! - `--watch`: unique to run
//! - `<target>`: a per-branch positional, present for build and run, absent
//!   for clean

use argenv_resolve::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    Build,
    Run,
    Clean,
}

impl Command {
    pub const TOKENS: &'static [&'static str] = &["build", "run", "clean"];
}

impl FromRaw for Command {
    fn from_raw(s: &str) -> Option<Command> {
        Some(match s.trim() {
            "build" => Command::Build,
            "run" => Command::Run,
            "clean" => Command::Clean,
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
        summary: "Which action to run.",
        ..Input::EMPTY
    };

    VERBOSE: bool = Input {
        key:     "verbose",
        ty:      Type::Bool,
        default: Some(false),
        arg:     Some(Arg::pair("verbose", 'v')),
        env:     Some(Env::new("MOCK_VERBOSE")),
        summary: "Print extra detail.",
        ..Input::EMPTY
    };

    CONFIG: String = Input {
        key:     "config",
        ty:      Type::String,
        arg:     Some(Arg { value_name: "PATH", ..Arg::long("config") }),
        env:     Some(Env::new("MOCK_CONFIG")),
        summary: "Path to a config file.",
        ..Input::EMPTY
    };

    BUILD_TARGET: String, Command = Input {
        key:         "build_target",
        ty:          Type::String,
        subcommands: &[Command::Build],
        arg:         Some(Arg { value_name: "TARGET", ..Arg::positional(0) }),
        summary:     "What to build.",
        ..Input::EMPTY
    };

    RUN_TARGET: String, Command = Input {
        key:         "run_target",
        ty:          Type::String,
        subcommands: &[Command::Run],
        arg:         Some(Arg { value_name: "TARGET", ..Arg::positional(0) }),
        summary:     "What to run.",
        ..Input::EMPTY
    };

    JOBS: u16, Command = Input {
        key:         "jobs",
        ty:          Type::Uint,
        subcommands: &[Command::Build, Command::Run],
        arg:         Some(Arg { value_name: "N", ..Arg::long("jobs") }),
        env:         Some(Env::new("MOCK_JOBS")),
        summary:     "Parallel job count.",
        ..Input::EMPTY
    };

    RELEASE: bool, Command = Input {
        key:         "release",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Build],
        arg:         Some(Arg::long("release")),
        summary:     "Build with optimizations.",
        ..Input::EMPTY
    };

    WATCH: bool, Command = Input {
        key:         "watch",
        ty:          Type::Bool,
        default:     Some(false),
        subcommands: &[Command::Run],
        arg:         Some(Arg::long("watch")),
        env:         Some(Env::new("MOCK_WATCH")),
        summary:     "Re-run on file change.",
        ..Input::EMPTY
    };
}

fn main() {
    let r = Contract::parse_and_lint(OnProblems::FailOnError);

    let verbose = Contract::VERBOSE.get_from_or_default(&r).unwrap_or(false);
    let config = Contract::CONFIG.get_from(&r);
    if verbose {
        eprintln!("config={config:?}");
    }

    match Contract::COMMAND.get_from(&r) {
        Some(Command::Build) => {
            let target = Contract::BUILD_TARGET.get_from(&r);
            let release = Contract::RELEASE.get_from_or_default(&r).unwrap_or(false);
            let jobs = Contract::JOBS.get_from(&r);
            println!("build target={target:?} release={release} jobs={jobs:?} verbose={verbose}");
        }
        Some(Command::Run) => {
            let target = Contract::RUN_TARGET.get_from(&r);
            let watch = Contract::WATCH.get_from_or_default(&r).unwrap_or(false);
            let jobs = Contract::JOBS.get_from(&r);
            println!("run target={target:?} watch={watch} jobs={jobs:?} verbose={verbose}");
        }
        Some(Command::Clean) => {
            println!("clean verbose={verbose}");
        }
        None => {
            eprintln!("usage: mock [--verbose] [--config PATH] build|run|clean ...");
            std::process::exit(2);
        }
    }
}
