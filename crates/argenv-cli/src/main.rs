//! `argenv` — emit and check ArgEnv contract documents.
//!
//! ```text
//! argenv schema [-o PATH]        emit the JSON Schema contract
//! argenv check PATH...           validate contract documents
//! argenv lint [--lenient] PATH [-- ARGS...]  check this invocation against a contract
//! argenv usage PATH              render the help text a contract implies
//! argenv completions SHELL PATH  shell completions a contract implies (bash zsh fish nushell)
//! argenv man PATH                the man page a contract implies
//! ```
use argenv::{contract, Record};
use argenv_resolve::generate::{self, Command, Program, Shell};
use argenv_resolve::{lint, Invocation, LintPolicy, ProcessEnv, Severity};
use serde_json::Value;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("schema") => cmd_schema(&args[1..]),
        Some("check") => cmd_check(&args[1..]),
        Some("lint") => cmd_lint(&args[1..]),
        Some("usage") => cmd_usage(&args[1..]),
        Some("completions") => cmd_completions(&args[1..]),
        Some("man") => cmd_man(&args[1..]),
        Some("--version") | Some("-V") => {
            println!("argenv {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("--help") | Some("-h") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("argenv: unknown command `{other}`");
            print_help();
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!(
        "argenv {}\n\
         \n\
         USAGE\n    \
             argenv schema [-o PATH]         emit the JSON Schema contract\n    \
             argenv check PATH...            validate contract documents\n    \
             argenv lint [--lenient] PATH [-- ARGS...]   check this invocation against a contract\n    \
             argenv usage PATH               render the help text a contract implies\n    \
             argenv completions SHELL PATH [--program NAME]   shell completions (bash zsh fish nushell)\n    \
             argenv man PATH [--program NAME] [--about TEXT] [--date YYYY-MM-DD]   the man page\n    \
             argenv --version | --help\n",
        env!("CARGO_PKG_VERSION")
    );
}

fn cmd_schema(args: &[String]) -> ExitCode {
    let text = serde_json::to_string_pretty(&contract::json_schema()).expect("schema serialises");
    match args {
        [] => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        [flag, path] if flag == "-o" || flag == "--output" => {
            if let Some(dir) = std::path::Path::new(path).parent() {
                if !dir.as_os_str().is_empty() {
                    let _ = std::fs::create_dir_all(dir);
                }
            }
            match std::fs::write(path, format!("{text}\n")) {
                Ok(()) => {
                    eprintln!("wrote {path}");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("argenv: cannot write {path}: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("argenv: usage: schema [-o PATH]");
            ExitCode::from(2)
        }
    }
}

/// Read a contract document and return its records, stamped with provenance.
fn read_document(path: &str) -> Result<Vec<Record>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: cannot read: {e}"))?;
    let doc: Value =
        serde_json::from_str(&text).map_err(|e| format!("{path}: invalid JSON: {e}"))?;
    let source = doc
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or(path)
        .to_string();
    let inputs = doc
        .get("inputs")
        .cloned()
        .ok_or_else(|| format!("{path}: missing `inputs`"))?;
    let records: Vec<Record> =
        serde_json::from_value(inputs).map_err(|e| format!("{path}: cannot read inputs: {e}"))?;
    Ok(records
        .into_iter()
        .map(|mut r| {
            r.source.get_or_insert_with(|| source.clone());
            r
        })
        .collect())
}

fn cmd_check(paths: &[String]) -> ExitCode {
    if paths.is_empty() {
        eprintln!("argenv: usage: check PATH...");
        return ExitCode::from(2);
    }
    let mut failed = false;
    for path in paths {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{path}: cannot read: {e}");
                failed = true;
                continue;
            }
        };
        let doc: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{path}: invalid JSON: {e}");
                failed = true;
                continue;
            }
        };
        let problems = validate_document(&doc);
        if problems.is_empty() {
            let n = doc
                .get("inputs")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            println!("{path}: ok ({n} inputs)");
        } else {
            for p in &problems {
                eprintln!("{path}: {p}");
            }
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Check the current invocation against a published contract.
fn cmd_lint(args: &[String]) -> ExitCode {
    let (paths, argv) = match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], args[i + 1..].to_vec()),
        None => (args, Vec::new()),
    };
    // Typos in flags and variables are errors unless the caller says otherwise.
    let policy = if paths.iter().any(|p| p == "--lenient") {
        LintPolicy::lenient()
    } else {
        LintPolicy::default()
    };
    let Some(path) = paths.iter().find(|p| p.as_str() != "--lenient") else {
        eprintln!("argenv: usage: lint [--lenient] PATH [-- ARGS...]");
        return ExitCode::from(2);
    };
    let records = match read_document(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let env = ProcessEnv;
    let resolution = Invocation {
        args: &argv,
        env: &env,
    }
    .resolve(&records);
    let findings = lint(&records, &resolution);
    if findings.is_empty() {
        println!("invocation satisfies {} declared inputs", records.len());
        return ExitCode::SUCCESS;
    }
    let mut errors = 0usize;
    for f in &findings {
        match policy.severity(f) {
            Severity::Error => {
                errors += 1;
                eprintln!("error: {f}");
            }
            Severity::Warning => eprintln!("warning: {f}"),
        }
    }
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// What `completions` and `man` read from a contract document besides its inputs.
struct Generation {
    records: Vec<Record>,
    name: String,
    version: String,
    about: String,
    date: String,
    commands: Vec<String>,
}

/// Parse `PATH [--program NAME] [--about TEXT] [--date DATE]` and read the document.
fn generation(args: &[String], usage: &str) -> Result<Generation, ExitCode> {
    let mut path = None;
    let (mut program, mut about, mut date) = (None, None, String::new());
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let slot = match arg.as_str() {
            "--program" => &mut program,
            "--about" => &mut about,
            "--date" => {
                match it.next() {
                    Some(value) => date = value.clone(),
                    None => {
                        eprintln!("argenv: --date needs a value\nusage: {usage}");
                        return Err(ExitCode::from(2));
                    }
                }
                continue;
            }
            flag if flag.starts_with("--") => {
                eprintln!("argenv: unknown option `{flag}`\nusage: {usage}");
                return Err(ExitCode::from(2));
            }
            _ => {
                if path.replace(arg.clone()).is_some() {
                    eprintln!("argenv: usage: {usage}");
                    return Err(ExitCode::from(2));
                }
                continue;
            }
        };
        match it.next() {
            Some(value) => *slot = Some(value.clone()),
            None => {
                eprintln!("argenv: {arg} needs a value\nusage: {usage}");
                return Err(ExitCode::from(2));
            }
        }
    }
    let Some(path) = path else {
        eprintln!("argenv: usage: {usage}");
        return Err(ExitCode::from(2));
    };
    let records = read_document(&path).map_err(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })?;
    // `source` is "name@version" by convention; flags override it.
    let source = records
        .first()
        .and_then(|r| r.source.clone())
        .unwrap_or_default();
    let (source_name, source_version) = source.split_once('@').unwrap_or((source.as_str(), ""));
    let name = program.unwrap_or_else(|| source_name.to_string());
    if name.is_empty() {
        eprintln!("argenv: the document names no program (`source`); pass --program NAME");
        return Err(ExitCode::from(2));
    }
    // The subcommands a contract mentions, in the order it first mentions them.
    let mut commands: Vec<String> = Vec::new();
    for record in &records {
        for command in &record.subcommands {
            if !commands.contains(command) {
                commands.push(command.clone());
            }
        }
    }
    Ok(Generation {
        records,
        version: source_version.to_string(),
        about: about.unwrap_or_else(|| format!("{name} command line")),
        name,
        date,
        commands,
    })
}

fn with_program<T>(g: &Generation, f: impl FnOnce(&Program<'_>, &[Record]) -> T) -> T {
    let commands: Vec<Command<'_>> = g
        .commands
        .iter()
        .map(|name| Command { name, summary: "" })
        .collect();
    let program = Program {
        name: &g.name,
        version: &g.version,
        about: &g.about,
        commands: &commands,
        exit_status: &[],
        see_also: &[],
        date: &g.date,
    };
    f(&program, &g.records)
}

fn print_generated(result: Result<String, generate::GenerateError>) -> ExitCode {
    match result {
        Ok(text) => {
            print!("{text}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("argenv: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Shell completions a contract implies.
fn cmd_completions(args: &[String]) -> ExitCode {
    let usage = "completions SHELL PATH [--program NAME]  (SHELL: bash zsh fish nushell)";
    let Some((shell, rest)) = args.split_first() else {
        eprintln!("argenv: usage: {usage}");
        return ExitCode::from(2);
    };
    let Some(shell) = Shell::parse(shell) else {
        eprintln!("argenv: unknown shell `{shell}` (bash zsh fish nushell)");
        return ExitCode::from(2);
    };
    match generation(rest, usage) {
        Ok(g) => print_generated(with_program(&g, |p, r| generate::completions(shell, p, r))),
        Err(code) => code,
    }
}

/// The man page a contract implies.
fn cmd_man(args: &[String]) -> ExitCode {
    let usage = "man PATH [--program NAME] [--about TEXT] [--date YYYY-MM-DD]";
    match generation(args, usage) {
        Ok(g) => print_generated(with_program(&g, generate::man_page)),
        Err(code) => code,
    }
}

/// Render the help text a contract implies, so documentation has one source.
fn cmd_usage(args: &[String]) -> ExitCode {
    let Some(path) = args.first() else {
        eprintln!("argenv: usage: usage PATH");
        return ExitCode::from(2);
    };
    let records = match read_document(path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let mut groups: std::collections::BTreeMap<String, Vec<&Record>> = Default::default();
    for r in &records {
        groups
            .entry(r.group.clone().unwrap_or_else(|| "options".into()))
            .or_default()
            .push(r);
    }
    for (group, records) in groups {
        println!("\n{}", group.to_uppercase());
        for r in records {
            let usage = r.usage();
            let summary = r.summary.clone().unwrap_or_default();
            if usage.is_empty() {
                let names = r.env_names().join(", ");
                println!("    {names:<30} {summary}");
            } else {
                println!("    {usage:<30} {summary}");
            }
        }
    }
    ExitCode::SUCCESS
}

/// Fields this version of the contract knows. Anything else is reported so an
/// authoring typo is visible, without making the document invalid: later
/// versions legitimately add fields.
/// The fields a record may carry, read from the model's own schema.
///
/// This used to be a hand-kept list, and it drifted: `subcommands` was added to the model
/// and not to the list, so `argenv check` rejected the very document the model emits. Asking
/// the schema, which is generated from the same type, makes that impossible.
fn known_fields() -> std::collections::BTreeSet<String> {
    contract::json_schema()["properties"]["inputs"]["items"]["properties"]
        .as_object()
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

fn validate_document(doc: &Value) -> Vec<String> {
    let mut errs = Vec::new();

    match doc.get("contract_version").and_then(Value::as_u64) {
        Some(v) if v as u32 == argenv::CONTRACT_VERSION => {}
        Some(v) => errs.push(format!(
            "contract_version {v} is not supported (this tool speaks {})",
            argenv::CONTRACT_VERSION
        )),
        None => errs.push("missing contract_version".into()),
    }

    let Some(inputs) = doc.get("inputs").and_then(Value::as_array) else {
        errs.push("missing inputs array".into());
        return errs;
    };

    let known = known_fields();
    let mut keys = std::collections::BTreeSet::new();
    let mut flags = std::collections::BTreeSet::new();
    let mut names = std::collections::BTreeSet::new();

    for (i, v) in inputs.iter().enumerate() {
        let key = v.get("key").and_then(Value::as_str).unwrap_or("");
        let at = if key.is_empty() {
            format!("inputs[{i}]")
        } else {
            key.to_string()
        };

        if key.is_empty() {
            errs.push(format!("{at}: missing or empty key"));
        } else if !keys.insert(key.to_string()) {
            errs.push(format!("{at}: duplicate key"));
        }

        if let Some(obj) = v.as_object() {
            for k in obj.keys() {
                if !known.contains(k.as_str()) {
                    errs.push(format!(
                        "{at}: unrecognised field `{k}` (a typo, or a field from a later contract version)"
                    ));
                }
            }
        }

        let ty = v.get("type").and_then(Value::as_str).unwrap_or("");
        match ty {
            "" => errs.push(format!("{at}: missing type")),
            t if argenv::Type::ALL.contains(&t) => {}
            t => errs.push(format!("{at}: unknown type `{t}`")),
        }

        match v.get("stability").and_then(Value::as_str) {
            None => {}
            Some(s) if argenv::Stability::ALL.contains(&s) => {
                if s == "deprecated" && v.get("deprecation").is_none() {
                    errs.push(format!("{at}: deprecated but no deprecation details"));
                }
                if s != "deprecated" && v.get("deprecation").is_some() {
                    errs.push(format!(
                        "{at}: has deprecation details but stability is `{s}`"
                    ));
                }
            }
            Some(s) => errs.push(format!("{at}: unknown stability `{s}`")),
        }

        if let Some(s) = v.get("since").and_then(Value::as_str) {
            if argenv::Version::try_parse(s).is_none() {
                errs.push(format!("{at}: malformed since `{s}`"));
            }
        }
        for field in ["reviewed", "modified"] {
            if let Some(s) = v.get(field).and_then(Value::as_str) {
                if argenv::ReviewDate::try_parse(s).is_none() {
                    errs.push(format!("{at}: malformed {field} date `{s}`"));
                }
            }
        }

        let allowed = v
            .get("allowed")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        if matches!(ty, "enum" | "list") && allowed == 0 {
            errs.push(format!("{at}: type is `{ty}` but allowed is empty"));
        }
        if ty == "list"
            && v.get("separators")
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
                == 0
        {
            errs.push(format!(
                "{at}: type is `list` but no separators are declared"
            ));
        }

        let has_env = v.get("env").is_some();
        let has_arg = v.get("arg").is_some();
        if !has_env && !has_arg {
            errs.push(format!("{at}: declares no binding, so nothing can set it"));
        }
        if let Some(e) = v
            .get("env")
            .and_then(|e| e.get("name"))
            .and_then(Value::as_str)
        {
            if !names.insert(e.to_string()) {
                errs.push(format!("{at}: environment name `{e}` is claimed twice"));
            }
        }
        if let Some(a) = v.get("arg") {
            if let Some(l) = a.get("long").and_then(Value::as_str) {
                if !flags.insert(format!("--{l}")) {
                    errs.push(format!("{at}: flag `--{l}` is claimed twice"));
                }
            }
            if let Some(s) = a.get("short").and_then(Value::as_str) {
                if !flags.insert(format!("-{s}")) {
                    errs.push(format!("{at}: flag `-{s}` is claimed twice"));
                }
            }
            match a.get("arity").and_then(Value::as_u64) {
                Some(0) | Some(1) => {}
                Some(n) => errs.push(format!("{at}: arity {n} is not 0 or 1")),
                None => errs.push(format!("{at}: arg binding is missing arity")),
            }
        }
    }
    errs
}
