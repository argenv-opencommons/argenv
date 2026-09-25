//! Problem-handling policy: what to do when `lint` finds something.
//!
//! Rather than hand-rolling a loop every project, declare your policy once and
//! pass it to [`handle_problems`] or [`Model::parse_and_lint`]:
//!
//! ```no_run
//! # use argenv::*;
//! # let model: Vec<Record> = vec![];
//! # let resolved = argenv::parse(&model);
//! let findings = lint(&model, &resolved);
//! handle_problems(&findings, OnProblems::FailOnError);
//! ```

use crate::{Finding, Severity};

/// What to do when [`crate::lint`] returns findings.
///
/// Pass to [`handle_problems`] or, more commonly, let [`crate::contract!`]'s
/// generated `Model::parse_and_lint` handle both steps at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnProblems {
    /// Print errors and warnings to stderr; exit with status 1 on any error.
    /// Warnings alone are printed but do not stop the program.
    ///
    /// This is the right default for most programs: a misspelled flag is
    /// an error (the intent is clear but wrong), an unknown env var is a
    /// warning (it might just be a coincidence).
    FailOnError,

    /// Print all findings to stderr; exit with status 1 if there is anything
    /// at all — errors or warnings.
    ///
    /// Use this when warnings are unacceptable: a strict tool, a CI step,
    /// or a program where an unrecognised env var is never a coincidence.
    FailOnAny,

    /// Print all findings to stderr; never exit because of them.
    ///
    /// Use this for programs that want to surface problems without stopping,
    /// or for a startup phase where the caller handles the exit itself.
    WarnOnly,

    /// Discard all findings silently.
    ///
    /// Use this only when you have already handled findings yourself, or
    /// when the program genuinely cannot do anything useful with them.
    Ignore,
}

/// Apply a problem-handling policy to a set of findings.
///
/// Prints findings to stderr and exits the process if the policy requires it.
/// Returns normally when the policy allows the program to continue.
///
/// ```no_run
/// # use argenv::*;
/// # let model: Vec<Record> = vec![];
/// # let resolved = argenv::parse(&model);
/// handle_problems(&lint(&model, &resolved), OnProblems::FailOnError);
/// ```
pub fn handle_problems(findings: &[Finding], policy: OnProblems) {
    if findings.is_empty() || policy == OnProblems::Ignore {
        return;
    }

    for f in findings {
        let label = match f.severity() {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        eprintln!("{label}: {f}");
    }

    let should_exit = match policy {
        OnProblems::FailOnError => findings.iter().any(|f| f.severity() == Severity::Error),
        OnProblems::FailOnAny => !findings.is_empty(),
        OnProblems::WarnOnly | OnProblems::Ignore => false,
    };

    if should_exit {
        std::process::exit(1);
    }
}
