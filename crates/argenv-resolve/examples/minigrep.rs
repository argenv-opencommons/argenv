//! The Rust Book's `minigrep` (chapter 12), with its arguments and its environment declared
//! once instead of read by hand.
//!
//! Chapter 12 of *The Rust Programming Language* builds a tiny `grep`: two positional arguments
//! (a query and a file) and a case-insensitive mode switched on by the `IGNORE_CASE` variable.
//! The book reads each one by hand (`args[1]`, `env::var("IGNORE_CASE").is_ok()`), which is the
//! right way to *learn* the standard library. This example is the same program with the
//! invocation written as data. What that buys, for the price of one declaration:
//!
//! - `IGNORE_CASE=1`, `--ignore-case` and `-i` are one setting with two doors, and the flag wins;
//! - a misspelled flag stops the program with a suggestion instead of being ignored;
//! - the usage text is generated from the declaration, so it cannot disagree with what is parsed;
//! - the same declaration can be published (`Contract::records()`) for tools to read.
//!
//! One behaviour differs from the book, on purpose: the book treats *any* value of `IGNORE_CASE`
//! as "on" (it only asks whether the variable is set), so `IGNORE_CASE=0` still ignores case.
//! Here the variable is a boolean, so `0`, `false`, `off` and `no` mean off.
//!
//! The `search` functions and their tests are the book's, under the same MIT OR Apache-2.0 terms
//! as this crate.
//!
//! ```text
//! cargo run -p argenv-resolve --example minigrep -- duct poem.txt
//! cargo run -p argenv-resolve --example minigrep -- --ignore-case rUsT poem.txt
//! IGNORE_CASE=1 cargo run -p argenv-resolve --example minigrep -- rUsT poem.txt
//! cargo run -p argenv-resolve --example minigrep -- --ignore-cas rust poem.txt   # a suggestion
//! cargo run -p argenv-resolve --example minigrep -- --help
//! ```
use argenv_resolve::*;
use std::path::PathBuf;
use std::process::ExitCode;

contract! {
    HELP: bool = Input {
        key: "help",
        ty: Type::Bool,
        default: Some(false),
        arg: Some(Arg::pair("help", 'h')),
        summary: "Show this help and exit.",
        ..Input::EMPTY
    };

    // Positional arguments are numbered from zero, in the order they appear.
    QUERY: String = Input {
        key: "query",
        ty: Type::String,
        arg: Some(Arg { value_name: "QUERY", ..Arg::positional(0) }),
        summary: "The text to search for.",
        ..Input::EMPTY
    };

    FILE_PATH: PathBuf = Input {
        key: "file_path",
        ty: Type::Path,
        arg: Some(Arg { value_name: "FILE", ..Arg::positional(1) }),
        summary: "The file to search in.",
        ..Input::EMPTY
    };

    // The book's `IGNORE_CASE`: one setting, two doors, declared next to each other.
    IGNORE_CASE: bool = Input {
        key: "ignore_case",
        ty: Type::Bool,
        default: Some(false),
        env: Some(Env::new("IGNORE_CASE")),
        arg: Some(Arg::pair("ignore-case", 'i')),
        summary: "Ignore the case of the query and of the text.",
        ..Input::EMPTY
    };
}

/// The book's `search` (listing 12-19).
pub fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

/// The book's `search_case_insensitive` (listing 12-21).
pub fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    let query = query.to_lowercase();
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query))
        .collect()
}

/// The lines that match, case-sensitively or not.
pub fn grep<'a>(query: &str, contents: &'a str, ignore_case: bool) -> Vec<&'a str> {
    if ignore_case {
        search_case_insensitive(query, contents)
    } else {
        search(query, contents)
    }
}

/// Usage text, generated from the declaration.
pub fn usage() -> String {
    let mut text = String::from("usage: minigrep [OPTIONS] <QUERY> <FILE>\n\n");
    for record in Contract::records() {
        text.push_str(&format!(
            "  {:<20} {}\n",
            record.usage(),
            record.summary.as_deref().unwrap_or("")
        ));
        if !record.env_names().is_empty() {
            text.push_str(&format!(
                "  {:<20} (or set {})\n",
                "",
                record.env_names().join(", ")
            ));
        }
    }
    text
}

fn main() -> ExitCode {
    // A misspelled flag or variable is an error by default, so `--ignore-cas` stops the program
    // here instead of running the search case-sensitively. A program that would rather carry on
    // says so with its own policy: `Contract::parse_and_lint_with(OnProblems::FailOnError,
    // &LintPolicy::lenient())`.
    let resolution = Contract::parse_and_lint(OnProblems::FailOnError);

    if Contract::HELP.get_from_or_default(&resolution) == Some(true) {
        print!("{}", usage());
        return ExitCode::SUCCESS;
    }
    let (Some(query), Some(path)) = (
        Contract::QUERY.get_from(&resolution),
        Contract::FILE_PATH.get_from(&resolution),
    ) else {
        eprint!("{}", usage());
        return ExitCode::from(2);
    };
    let ignore_case = Contract::IGNORE_CASE
        .get_from_or_default(&resolution)
        .unwrap_or(false);

    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("minigrep: cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    for line in grep(&query, &contents, ignore_case) {
        println!("{line}");
    }
    ExitCode::SUCCESS
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

    fn ignore_case(args: &[&str], env: &[(&str, &str)]) -> bool {
        let r = resolve(args, env);
        Contract::IGNORE_CASE
            .get_from_or_default(&r)
            .unwrap_or(false)
    }

    // ---- the declaration -------------------------------------------------------------

    #[test]
    fn the_declaration_is_valid() {
        assert!(
            Contract::problems().is_empty(),
            "{:#?}",
            Contract::problems()
        );
    }

    #[test]
    fn the_two_positionals_are_the_query_and_the_file() {
        let r = resolve(&["duct", "poem.txt"], &[]);
        assert_eq!(Contract::QUERY.get_from(&r).as_deref(), Some("duct"));
        assert_eq!(
            Contract::FILE_PATH.get_from(&r),
            Some(PathBuf::from("poem.txt"))
        );
    }

    // ---- ignore case: one setting, two doors -----------------------------------------

    #[test]
    fn case_is_respected_unless_asked_otherwise() {
        assert!(!ignore_case(&["q", "f"], &[]));
    }

    #[test]
    fn the_flag_turns_it_on_in_both_spellings() {
        assert!(ignore_case(&["--ignore-case", "q", "f"], &[]));
        assert!(ignore_case(&["-i", "q", "f"], &[]));
    }

    #[test]
    fn the_environment_variable_turns_it_on_like_in_the_book() {
        assert!(ignore_case(&["q", "f"], &[("IGNORE_CASE", "1")]));
    }

    #[test]
    fn a_false_value_means_off_which_is_where_this_differs_from_the_book() {
        // The book only asks whether IGNORE_CASE is set, so IGNORE_CASE=0 still ignores case.
        assert!(!ignore_case(&["q", "f"], &[("IGNORE_CASE", "0")]));
    }

    #[test]
    fn the_flag_wins_over_the_environment() {
        assert!(ignore_case(
            &["--ignore-case", "q", "f"],
            &[("IGNORE_CASE", "0")]
        ));
    }

    // ---- what declaring it catches ---------------------------------------------------

    #[test]
    fn a_misspelled_flag_is_reported_with_the_right_suggestion() {
        let r = resolve(&["--ignore-cas", "q", "f"], &[]);
        let findings = lint(&Contract::records(), &r);
        assert!(
            findings.iter().any(|f| matches!(
                f,
                Finding::UnknownArg { did_you_mean: Some(s), .. } if s == "--ignore-case"
            )),
            "{findings:#?}"
        );
    }

    #[test]
    fn an_invalid_value_for_the_variable_is_an_error_naming_where_it_came_from() {
        let r = resolve(&["q", "f"], &[("IGNORE_CASE", "maybe")]);
        let findings = lint(&Contract::records(), &r);
        let error = findings
            .iter()
            .find(|f| f.severity() == Severity::Error)
            .expect("an error finding");
        assert!(error.to_string().contains("environment"), "{error}");
    }

    #[test]
    fn the_usage_text_mentions_every_input_and_the_variable() {
        let text = usage();
        for needle in [
            "<QUERY>",
            "<FILE>",
            "--ignore-case",
            "-i",
            "IGNORE_CASE",
            "--help",
        ] {
            assert!(text.contains(needle), "usage lacks {needle}:\n{text}");
        }
    }

    // ---- the book's own search tests (listings 12-20 and 12-21) ----------------------

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search_case_insensitive(query, contents)
        );
    }

    #[test]
    fn grep_picks_the_search_by_the_setting() {
        // "Trust me." contains a lowercase "rust"; "Rust:" does not.
        let contents = "Rust:\nTrust me.";
        assert_eq!(grep("rust", contents, false), vec!["Trust me."]);
        assert_eq!(grep("rust", contents, true), vec!["Rust:", "Trust me."]);
    }
}
