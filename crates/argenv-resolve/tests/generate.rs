//! The generators: what they derive from a contract, that they treat it as untrusted, and that
//! the output works in real shells.
//!
//! The shell tests run bash, fish, zsh and nushell when they are installed. Set
//! `ARGENV_REQUIRE_SHELLS=1` (CI does) to turn "not installed" into a failure, so a missing shell
//! cannot make a test silently vacuous.

use argenv_resolve::generate::{
    bash, completions, fish, man_page, nushell, zsh, Command, GenerateError, Program, Shell,
};
use argenv_resolve::*;
use std::path::{Path, PathBuf};
use std::process::Command as Process;

/// A summary built to break anything that quotes carelessly.
const HOSTILE: &str = "Names; it's `x` [y]: $(touch PWNED) \"q\" \\ \n done; touch PWNED2 #";

fn records() -> Vec<Record> {
    let help: Input<bool, &'static str> = Input {
        key: "help",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::pair("help", 'h')),
        group: "general",
        summary: "Show this help and exit.",
        ..Input::EMPTY
    };
    let log: Input<String, &'static str> = Input {
        key: "log",
        ty: Type::Enum,
        allowed: &["off", "text", "json"],
        default: Some("off".to_string()),
        env: Some(Env::new("APP_LOG")),
        arg: Some(Arg {
            value_name: "FORMAT",
            ..Arg::long("log")
        }),
        group: "logging",
        summary: "What to log.",
        ..Input::EMPTY
    };
    let log_file: Input<PathBuf, &'static str> = Input {
        key: "log_file",
        ty: Type::Path,
        env: Some(Env::new("APP_LOG_FILE")),
        arg: Some(Arg {
            value_name: "PATH",
            ..Arg::long("log-file")
        }),
        group: "logging",
        summary: "Where to log.",
        ..Input::EMPTY
    };
    let color: Input<bool, &'static str> = Input {
        key: "color",
        ty: Type::Bool,
        arg: Some(Arg {
            negatable: true,
            ..Arg::long("color")
        }),
        group: "general",
        summary: "Use colour.",
        ..Input::EMPTY
    };
    let only: Input<String, &'static str> = Input {
        key: "only",
        ty: Type::String,
        subcommands: &["create"],
        env: Some(Env::new("APP_ONLY")),
        arg: Some(Arg {
            value_name: "CONFIGS",
            ..Arg::long("only")
        }),
        group: "create",
        summary: HOSTILE,
        ..Input::EMPTY
    };
    let dry_run: Input<bool, &'static str> = Input {
        key: "dry_run",
        ty: Type::Bool,
        default: Some(false),
        subcommands: &["create"],
        arg: Some(Arg::pair("dry-run", 'n')),
        group: "create",
        summary: "Show what would happen.",
        ..Input::EMPTY
    };
    let mode: Input<String, &'static str> = Input {
        key: "mode",
        ty: Type::Enum,
        allowed: &["fast", "slow"],
        subcommands: &["create"],
        arg: Some(Arg {
            value_name: "MODE",
            ..Arg::long("mode")
        }),
        group: "create",
        summary: "How to snapshot.",
        ..Input::EMPTY
    };
    let description: Input<String, &'static str> = Input {
        key: "description",
        ty: Type::String,
        subcommands: &["create"],
        arg: Some(Arg {
            value_name: "DESCRIPTION",
            ..Arg::positional(0)
        }),
        group: "create",
        summary: "Text stored with each snapshot.",
        ..Input::EMPTY
    };
    let shell: Input<String, &'static str> = Input {
        key: "shell",
        ty: Type::Enum,
        allowed: &["bash", "zsh", "fish", "nushell"],
        required: true,
        subcommands: &["completions"],
        arg: Some(Arg {
            value_name: "SHELL",
            ..Arg::positional(0)
        }),
        group: "completions",
        summary: "Which shell.",
        ..Input::EMPTY
    };
    let target: Input<PathBuf, &'static str> = Input {
        key: "target",
        ty: Type::Path,
        subcommands: &["export"],
        arg: Some(Arg {
            value_name: "FILE",
            ..Arg::positional(0)
        }),
        group: "export",
        summary: "Where to write.",
        ..Input::EMPTY
    };
    vec![
        help.to_record(),
        log.to_record(),
        log_file.to_record(),
        color.to_record(),
        only.to_record(),
        dry_run.to_record(),
        mode.to_record(),
        description.to_record(),
        shell.to_record(),
        target.to_record(),
    ]
}

const COMMANDS: &[Command<'static>] = &[
    Command {
        name: "create",
        summary: "Take a snapshot",
    },
    Command {
        name: "list",
        summary: "List the configs",
    },
    Command {
        name: "completions",
        summary: "Print shell completions",
    },
    Command {
        name: "export",
        summary: "Write a file",
    },
];

fn program() -> Program<'static> {
    Program {
        name: "app",
        version: "1.2.3",
        about: "Snapshot every config",
        commands: COMMANDS,
        exit_status: &[("0", "done"), ("2", "the invocation is wrong")],
        see_also: &["snapper(8)"],
        date: "2026-10-23",
    }
}

// ---- finding shells ---------------------------------------------------------------------

fn tool(name: &str) -> Option<PathBuf> {
    let found = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file())
    });
    if found.is_none() && std::env::var_os("ARGENV_REQUIRE_SHELLS").is_some() {
        panic!("{name} is required (ARGENV_REQUIRE_SHELLS is set) but is not installed");
    }
    found
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argenv-generate-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write file");
    path
}

fn run(process: &mut Process) -> (bool, String, String) {
    let out = process.output().expect("run");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ---- what is derived --------------------------------------------------------------------

#[test]
fn every_shell_mentions_every_spelling_of_every_flag() {
    let all: Vec<(Shell, String)> = Shell::ALL
        .iter()
        .map(|s| (*s, completions(*s, &program(), &records()).unwrap()))
        .collect();
    for (shell, text) in &all {
        for needle in ["help", "log", "log-file", "color", "only", "dry-run"] {
            assert!(text.contains(needle), "{shell:?} lacks `{needle}`:\n{text}");
        }
    }
    // short spellings are completed too (bash lists them; the others declare them)
    let (_, bash_text) = &all[0];
    assert!(
        bash_text.contains("-h") && bash_text.contains("-n"),
        "{bash_text}"
    );
}

#[test]
fn a_negatable_flag_also_offers_its_negation() {
    for shell in Shell::ALL {
        let text = completions(shell, &program(), &records()).unwrap();
        assert!(text.contains("no-color"), "{shell:?}: {text}");
    }
}

#[test]
fn enumerated_values_and_commands_are_offered() {
    for shell in Shell::ALL {
        let text = completions(shell, &program(), &records()).unwrap();
        for value in ["off", "text", "json", "bash", "zsh", "fish", "nushell"] {
            assert!(text.contains(value), "{shell:?} lacks value `{value}`");
        }
        for command in ["create", "list", "completions", "export"] {
            assert!(
                text.contains(command),
                "{shell:?} lacks command `{command}`"
            );
        }
    }
}

#[test]
fn a_flag_scoped_to_a_command_is_only_offered_for_that_command() {
    // bash: the flag list for `list` must not contain --dry-run, the one for `create` must.
    let text = bash(&program(), &records()).unwrap();
    let line = |command: &str| {
        text.lines()
            .find(|l| l.trim_start().starts_with(&format!("{command}) flags=")))
            .unwrap_or_else(|| panic!("no flags line for {command}:\n{text}"))
            .to_string()
    };
    assert!(line("create").contains("--dry-run"));
    assert!(!line("list").contains("--dry-run") && !line("list").contains("--only"));
    assert!(
        line("list").contains("--log"),
        "unscoped flags are offered everywhere"
    );

    // fish: the dry-run line carries the condition.
    let text = fish(&program(), &records()).unwrap();
    let dry = text.lines().find(|l| l.contains("-l dry-run")).unwrap();
    assert!(dry.contains("__fish_seen_subcommand_from create"), "{dry}");
    let log = text.lines().find(|l| l.contains("-l log ")).unwrap();
    assert!(!log.contains("__fish_seen_subcommand_from"), "{log}");

    // zsh: --dry-run only appears inside the create branch.
    let text = zsh(&program(), &records()).unwrap();
    let create = text.find("        create)").unwrap();
    let after = &text[create..];
    let end = after.find(";;").unwrap();
    assert!(after[..end].contains("--dry-run"));
    let before_create = &text[..create];
    assert!(
        !before_create.contains("dry-run"),
        "not in the top-level flags:\n{before_create}"
    );

    // nushell: the `app list` extern has no --dry-run.
    let text = nushell(&program(), &records()).unwrap();
    let block = |name: &str| {
        let start = text.find(&format!("export extern \"{name}\"")).unwrap();
        let rest = &text[start..];
        rest[..rest.find("  ]").unwrap()].to_string()
    };
    assert!(block("app create").contains("--dry-run"));
    assert!(!block("app list").contains("--dry-run"));
    assert!(block("app list").contains("--log"));
}

#[test]
fn output_is_deterministic() {
    for shell in Shell::ALL {
        assert_eq!(
            completions(shell, &program(), &records()).unwrap(),
            completions(shell, &program(), &records()).unwrap()
        );
    }
    assert_eq!(
        man_page(&program(), &records()).unwrap(),
        man_page(&program(), &records()).unwrap()
    );
}

#[test]
fn a_program_without_subcommands_still_gets_completions() {
    let bare = Program {
        commands: &[],
        ..program()
    };
    let flags_only: Vec<Record> = records()
        .into_iter()
        .filter(|r| r.subcommands.is_empty())
        .collect();
    for shell in Shell::ALL {
        let text = completions(shell, &bare, &flags_only).unwrap();
        assert!(text.contains("log"), "{shell:?}");
    }
}

// ---- the contract is untrusted ----------------------------------------------------------

#[test]
fn a_program_or_command_name_that_is_not_a_plain_name_is_refused() {
    for bad in ["my app", "a;b", "$(x)", "-x", "", "a'b", "a\"b", "a`b"] {
        let p = Program {
            name: bad,
            ..program()
        };
        for shell in Shell::ALL {
            assert_eq!(
                completions(shell, &p, &records()),
                Err(GenerateError::InvalidName(bad.to_string())),
                "{shell:?} {bad:?}"
            );
        }
        assert!(man_page(&p, &records()).is_err());
    }
    let commands = [
        Command {
            name: "ok",
            summary: "",
        },
        Command {
            name: "b ad",
            summary: "",
        },
    ];
    let p = Program {
        commands: &commands,
        ..program()
    };
    assert!(bash(&p, &records()).is_err());
}

#[test]
fn flag_names_and_values_that_are_not_plain_tokens_are_left_out_not_emitted() {
    let evil: Input<String, &'static str> = Input {
        key: "evil",
        ty: Type::Enum,
        allowed: &["fine", "x; rm -rf PWNEDVALUE", "$(id)", "has space"],
        arg: Some(Arg {
            value_name: "V",
            ..Arg::long("evil")
        }),
        ..Input::EMPTY
    };
    let mut recs = records();
    recs.push(evil.to_record());
    let mut weird = records()[1].clone();
    weird.key = "weird".into();
    weird.arg.as_mut().unwrap().long = Some("x;touch PWNEDFLAG".into());
    weird.arg.as_mut().unwrap().short = None;
    recs.push(weird);
    for shell in Shell::ALL {
        let text = completions(shell, &program(), &recs).unwrap();
        assert!(text.contains("fine"), "{shell:?}: the safe value stays");
        for forbidden in ["PWNEDVALUE", "PWNEDFLAG", "$(id)", "has space"] {
            assert!(
                !text.contains(forbidden),
                "{shell:?} emitted `{forbidden}`:\n{text}"
            );
        }
    }
}

#[test]
fn hostile_free_text_is_quoted_in_the_man_page_too() {
    let text = man_page(&program(), &records()).unwrap();
    assert!(!text.contains("\n done"), "newlines are flattened");
    for line in text.lines() {
        assert!(
            !(line.starts_with('.') && line.contains("touch PWNED")),
            "text must not become a roff request: {line}"
        );
    }
}

fn assert_nothing_ran(dir: &Path) {
    for marker in ["PWNED", "PWNED2"] {
        assert!(
            !dir.join(marker).exists(),
            "hostile text was executed ({marker})"
        );
    }
}

#[test]
fn bash_output_is_valid_and_inert() {
    let Some(bash_path) = tool("bash") else {
        return;
    };
    let dir = scratch("bash-inert");
    let script = write(&dir, "app.bash", &bash(&program(), &records()).unwrap());
    let (ok, _, stderr) = run(Process::new(&bash_path).arg("-n").arg(&script));
    assert!(ok, "syntax: {stderr}");
    let (ok, _, stderr) = run(Process::new(&bash_path)
        .current_dir(&dir)
        .arg("-c")
        .arg(format!("source {}; type _app >/dev/null", script.display())));
    assert!(ok, "{stderr}");
    assert_nothing_ran(&dir);
}

fn bash_complete(script: &Path, dir: &Path, words: &[&str]) -> Vec<String> {
    let quoted: Vec<String> = words.iter().map(|w| format!("'{w}'")).collect();
    let program = format!(
        "source {}; COMP_WORDS=({}); COMP_CWORD={}; COMPREPLY=(); _app; printf '%s\\n' \"${{COMPREPLY[@]}}\"",
        script.display(),
        quoted.join(" "),
        words.len() - 1
    );
    let (ok, stdout, stderr) = run(Process::new("bash").current_dir(dir).arg("-c").arg(program));
    assert!(ok, "{stderr}");
    stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn bash_completes_commands_flags_values_and_files_the_way_a_user_would_type_them() {
    if tool("bash").is_none() {
        return;
    }
    let dir = scratch("bash-complete");
    std::fs::write(dir.join("alpha.txt"), "").unwrap();
    let script = write(&dir, "app.bash", &bash(&program(), &records()).unwrap());
    let complete = |words: &[&str]| bash_complete(&script, &dir, words);

    let mut commands = complete(&["app", ""]);
    commands.sort();
    assert_eq!(commands, ["completions", "create", "export", "list"]);
    assert_eq!(complete(&["app", "cr"]), ["create"]);

    assert_eq!(complete(&["app", "create", "--d"]), ["--dry-run"]);
    assert!(
        complete(&["app", "list", "--d"]).is_empty(),
        "scoped to create"
    );
    assert!(complete(&["app", "create", "-"]).contains(&"-n".to_string()));
    assert!(
        complete(&["app", "list", "-"]).contains(&"--log".to_string()),
        "unscoped everywhere"
    );
    assert!(complete(&["app", "--"]).contains(&"--color".to_string()));
    assert!(
        !complete(&["app", "--"]).contains(&"--dry-run".to_string()),
        "no command yet"
    );

    let mut formats = complete(&["app", "--log", ""]);
    formats.sort();
    assert_eq!(formats, ["json", "off", "text"]);
    assert_eq!(complete(&["app", "--log", "t"]), ["text"]);
    assert_eq!(
        complete(&["app", "--log", "=", "j"]),
        ["json"],
        "the --flag=value form"
    );
    assert!(
        complete(&["app", "create", "--only", ""]).is_empty(),
        "a free-text value: nothing to suggest"
    );

    // The values of a flag that belongs to `create` are completed after `create`, and only there.
    let mut modes = complete(&["app", "create", "--mode", ""]);
    modes.sort();
    assert_eq!(modes, ["fast", "slow"]);
    let before_a_command = complete(&["app", "--mode", ""]);
    assert!(
        !before_a_command.iter().any(|w| w == "fast" || w == "slow"),
        "no command yet: --mode is not in play, so its values are not offered: {before_a_command:?}"
    );
    assert!(
        complete(&["app", "list", "--mode", ""]).is_empty(),
        "list has no --mode"
    );
    assert_eq!(
        complete(&["app", "create", "--mode", "=", "s"]),
        ["slow"],
        "the --flag=value form too"
    );

    let mut shells = complete(&["app", "completions", ""]);
    shells.sort();
    assert_eq!(shells, ["bash", "fish", "nushell", "zsh"]);
    assert_eq!(
        complete(&["app", "export", "al"]),
        ["alpha.txt"],
        "a path positional completes files"
    );
    assert_eq!(
        complete(&["app", "--log-file", "al"]),
        ["alpha.txt"],
        "a path flag completes files"
    );
}

#[test]
fn fish_output_is_valid_inert_and_completes() {
    let Some(fish_path) = tool("fish") else {
        return;
    };
    let dir = scratch("fish");
    let script = write(&dir, "app.fish", &fish(&program(), &records()).unwrap());
    let ask = |line: &str| {
        let (ok, stdout, stderr) = run(Process::new(&fish_path)
            .current_dir(&dir)
            .arg("--no-config")
            .arg("-c")
            .arg(format!("source {}; complete -C '{line}'", script.display())));
        assert!(ok, "{stderr}");
        stdout
    };
    let flags = ask("app create --d");
    assert!(flags.contains("--dry-run"), "{flags}");
    assert!(!ask("app list --d").contains("--dry-run"));
    let values = ask("app --log ");
    assert!(
        values.contains("off") && values.contains("text") && values.contains("json"),
        "{values}"
    );
    let commands = ask("app ");
    for command in ["create", "list", "completions", "export"] {
        assert!(commands.contains(command), "{commands}");
    }
    assert!(ask("app completions ").contains("nushell"));
    assert_nothing_ran(&dir);
}

#[test]
fn zsh_output_is_valid_and_inert() {
    let Some(zsh_path) = tool("zsh") else { return };
    let dir = scratch("zsh");
    let script = write(&dir, "_app", &zsh(&program(), &records()).unwrap());
    let (ok, _, stderr) = run(Process::new(&zsh_path).arg("-n").arg(&script));
    assert!(ok, "syntax: {stderr}");
    // Loading the function (not running the completer) must execute nothing.
    let (ok, _, stderr) = run(Process::new(&zsh_path).current_dir(&dir).arg("-fc").arg(format!("autoload -Uz compinit && compinit -u -d {}/zcomp && fpath=({} $fpath) && autoload -Uz _app && whence -w _app", dir.display(), dir.display())));
    assert!(ok, "{stderr}");
    assert_nothing_ran(&dir);
}

/// Drives an interactive zsh through a pseudo-terminal: types `typed`, presses Tab, returns what
/// the terminal showed. This is the only honest test of an `_arguments` spec: a wrong one does not
/// fail, it silently completes nothing.
const ZSH_DRIVER: &str = r#"
zmodload zsh/zpty || exit 3
dir=$1; typed=$2
zpty zt 'zsh -f -i' || exit 4
zpty -w zt "fpath=($dir \$fpath); autoload -Uz compinit; compinit -u -d $dir/.zcompdump; autoload -Uz _app; compdef _app app; PS1='READY> '; setopt no_beep; zstyle ':completion:*' list-prompt ''; zstyle ':completion:*' menu no"
sleep 1; zpty -r -t zt junk 2>/dev/null
zpty -w -n zt "$typed"
sleep 0.3
zpty -w -n zt $'\t'
sleep 2
out=""; while zpty -r -t zt chunk 2>/dev/null; do out+=$chunk; done
print -r -- "$out"
zpty -d zt
"#;

#[test]
fn zsh_completes_for_real_in_a_terminal() {
    let Some(zsh_path) = tool("zsh") else { return };
    let dir = scratch("zsh-real");
    write(&dir, "_app", &zsh(&program(), &records()).unwrap());
    let driver = write(&dir, "driver.zsh", ZSH_DRIVER);
    let tab = |typed: &str| -> String {
        let (ok, stdout, stderr) = run(Process::new("timeout")
            .arg("40")
            .arg(&zsh_path)
            .current_dir(&dir)
            .arg("-f")
            .arg(&driver)
            .arg(&dir)
            .arg(typed));
        assert!(ok, "zsh driver failed: {stderr}\n{stdout}");
        stdout.replace('\r', "")
    };

    let completed = tab("app create --d");
    assert!(
        completed.contains("--dry-run") && completed.contains("Show what would happen."),
        "the flag and its description are offered:\n{completed}"
    );
    let listed = tab("app create -");
    for flag in [
        "--color",
        "--dry-run",
        "--log",
        "--log-file",
        "--no-color",
        "--only",
        "--help",
    ] {
        assert!(listed.contains(flag), "{flag} missing:\n{listed}");
    }
    let list_flags = tab("app list --d");
    assert!(
        !list_flags.contains("dry-run"),
        "--dry-run is scoped to create:\n{list_flags}"
    );
    let values = tab("app --log ");
    assert!(
        values.contains("off") && values.contains("text") && values.contains("json"),
        "{values}"
    );
    assert!(tab("app completions ").contains("nushell"));
    assert!(
        tab("app cr").contains("app create"),
        "a command completes by prefix"
    );
    assert_nothing_ran(&dir);
}

#[test]
fn nushell_output_loads_and_defines_every_command() {
    let Some(nu) = tool("nu") else { return };
    let dir = scratch("nu");
    let script = write(&dir, "app.nu", &nushell(&program(), &records()).unwrap());
    let (ok, stdout, stderr) = run(Process::new(&nu)
        .current_dir(&dir)
        .arg("--no-config-file")
        .arg("-c")
        .arg(format!(
            "source {}; scope externs | where name =~ '^app' | get name | to json",
            script.display()
        )));
    assert!(ok, "{stderr}\n{stdout}");
    let names: Vec<String> = serde_json::from_str(&stdout).expect("nu prints a JSON list");
    for expected in [
        "app",
        "app create",
        "app list",
        "app completions",
        "app export",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "{expected} missing from {names:?}"
        );
    }
    assert_nothing_ran(&dir);
}

// ---- the man page -----------------------------------------------------------------------

#[test]
fn the_man_page_has_its_sections_and_escapes_hyphens() {
    let text = man_page(&program(), &records()).unwrap();
    assert!(
        text.starts_with(".TH APP 1 \"2026-10-23\" \"app 1.2.3\" \"User Commands\"\n"),
        "{text}"
    );
    for section in [
        "NAME",
        "SYNOPSIS",
        "COMMANDS",
        "OPTIONS",
        "ARGUMENTS",
        "ENVIRONMENT",
        "EXIT STATUS",
        "SEE ALSO",
    ] {
        assert!(
            text.contains(&format!(".SH {section}\n")),
            "{section}:\n{text}"
        );
    }
    assert!(
        text.contains("\\fB\\-\\-dry\\-run\\fR"),
        "hyphens are escaped:\n{text}"
    );
    assert!(text.contains("Applies to: \\fBcreate\\fR."));
    assert!(text.contains("One of: \\fBoff\\fR, \\fBtext\\fR, \\fBjson\\fR."));
    assert!(text.contains("Default: \\fBoff\\fR."));
    assert!(
        text.contains("\\fBAPP_LOG\\fR"),
        "environment variables are listed"
    );
    assert!(text.contains("snapper(8)"));
    assert!(text.contains("app create") || text.contains("\\fBcreate\\fR"));
}

#[test]
fn the_man_page_passes_a_roff_syntax_check_when_groff_is_installed() {
    let Some(groff) = tool("groff") else { return };
    let dir = scratch("man");
    let page = write(&dir, "app.1", &man_page(&program(), &records()).unwrap());
    let (ok, _, stderr) = run(Process::new(&groff)
        .arg("-man")
        .arg("-z")
        .arg("-ww")
        .arg(&page));
    assert!(ok && stderr.trim().is_empty(), "groff complained: {stderr}");
}

#[test]
fn shell_tokens_round_trip() {
    for shell in Shell::ALL {
        assert_eq!(Shell::parse(shell.token()), Some(shell));
    }
    assert_eq!(Shell::parse("nu"), Some(Shell::Nushell));
    assert_eq!(Shell::parse("powershell"), None);
}
