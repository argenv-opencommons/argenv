//! The floor: declare what your program takes, let argenv do the rest.
//!
//! This is a shallow implementation — only `key`, `ty`, and one binding per
//! input. No doc strings, no defaults, no stability markers, no reviewed dates.
//! Every field left at `..Input::EMPTY` is optional by design. Add them when
//! they matter; they will never break existing callers.
//!
//! ```text
//! cargo run -p argenv --example names_only -- --host db.example.com --port 5432
//! cargo run -p argenv --example names_only -- --help          # usage lines from the model
//! cargo run -p argenv --example names_only -- --bogus-flag    # lint catches the typo
//! ```
use argenv::*;

// One input per constant — transport-free key, wire type, one or both doors.
// That is all argenv needs to resolve, lint, and emit a contract document.

pub const HOST: Input<String> = Input {
    key: "host",
    ty: Type::String,
    env: Some(Env::new("APP_HOST")),
    arg: Some(Arg { value_name: "HOST", ..Arg::long("host") }),
    ..Input::EMPTY
};

pub const PORT: Input<u16> = Input {
    key: "port",
    ty: Type::Uint,
    default: Some(5432),
    env: Some(Env::new("APP_PORT")),
    arg: Some(Arg { value_name: "PORT", ..Arg::pair("port", 'p') }),
    ..Input::EMPTY
};

pub const VERBOSE: Input<bool> = Input {
    key: "verbose",
    ty: Type::Bool,
    default: Some(false),
    arg: Some(Arg::pair("verbose", 'v')),
    ..Input::EMPTY
};

fn model() -> Vec<Record> {
    vec![HOST.to_record(), PORT.to_record(), VERBOSE.to_record()]
}

fn problems() -> Vec<String> {
    let mut v = Vec::new();
    v.extend(HOST.check());
    v.extend(PORT.check());
    v.extend(VERBOSE.check());
    v.extend(check_unique(&model()));
    v
}

fn main() {
    // Fail hard on declaration mistakes — catches typos in Input fields,
    // colliding flag names, and so on, before any real work begins.
    let p = problems();
    assert!(p.is_empty(), "declaration errors: {p:#?}");

    let args: Vec<String> = std::env::args().skip(1).collect();
    let resolved = Invocation { args: &args, env: &ProcessEnv }.resolve(&model());

    // Lint the invocation: unknown flags, missing required values, etc.
    for finding in lint(&model(), &Invocation { args: &args, env: &ProcessEnv }) {
        eprintln!("{:?}: {finding}", finding.severity());
    }

    // Read typed values — None when absent and no default was declared.
    let host = HOST.get_from(&resolved);
    let port = PORT.get_from_or_default(&resolved);
    let verbose = VERBOSE.get_from_or_default(&resolved);

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("USAGE");
        for r in &model() {
            if !r.usage().is_empty() {
                println!("    {}", r.usage());
            }
        }
        return;
    }

    println!("host={host:?}  port={port:?}  verbose={verbose:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_is_valid() {
        assert!(problems().is_empty());
    }
}
