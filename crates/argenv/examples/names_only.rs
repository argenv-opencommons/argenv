//! The floor: declare what your program takes, let argenv do the rest.
//!
//! Shallow implementation — only `key`, `ty`, and one binding per input.
//! Everything else (`stability`, `reviewed`, `example`, ...) is optional;
//! add it later without changing anything that already works.
//!
//! ```text
//! cargo run -p argenv --example names_only -- --host db.example.com --port 5432
//! cargo run -p argenv --example names_only -- --bogus-flag
//! ```
use argenv::*;

contract! {
    HOST: String = Input {
        key: "host",
        ty: Type::String,
        env: Some(Env::new("APP_HOST")),
        arg: Some(Arg { value_name: "HOST", ..Arg::long("host") }),
        ..Input::EMPTY
    };

    PORT: u16 = Input {
        key: "port",
        ty: Type::Uint,
        default: Some(5432),
        env: Some(Env::new("APP_PORT")),
        arg: Some(Arg { value_name: "PORT", ..Arg::pair("port", 'p') }),
        ..Input::EMPTY
    };

    VERBOSE: bool = Input {
        key: "verbose",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::pair("verbose", 'v')),
        ..Input::EMPTY
    };
}

fn main() {
    let resolved = Contract::parse_and_lint(OnProblems::FailOnError);

    let host = Contract::HOST.get_from(&resolved);
    let port = Contract::PORT.get_from_or_default(&resolved);
    let verbose = Contract::VERBOSE.get_from_or_default(&resolved);

    println!("host={host:?}  port={port:?}  verbose={verbose:?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_is_valid() {
        assert!(Contract::problems().is_empty());
    }
}
