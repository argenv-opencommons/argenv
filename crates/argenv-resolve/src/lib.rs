//! Resolve an [`argenv`] contract against a real invocation — argv and
//! envp — with precedence, defaults, and linting.
//!
//! `argenv` declares what a program accepts, as data, and does not read
//! argv or envp at all. This crate is the other half: it depends on
//! `argenv` for [`Input`]/[`Record`] and adds exactly the runtime concern
//! `argenv` deliberately leaves out. A program's contract never needs to
//! know how it gets resolved; this crate never needs to know anything
//! `argenv` doesn't already say.
//!
//! Re-exports everything from `argenv`, so `use argenv_resolve::*;` is the
//! only import most consumers ever need.
//!
//! # The common case
//!
//! ```
//! use argenv_resolve::*;
//!
//! contract! {
//!     LOG_LEVEL: LogLevel = Input {
//!         key:     "log_level",
//!         ty:      Type::Enum,
//!         default: Some(LogLevel::Info),
//!         allowed: LogLevel::TOKENS,
//!         env:     Some(Env::new("APP_LOG_LEVEL")),
//!         arg:     Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
//!         summary: "Log verbosity.",
//!         ..Input::EMPTY
//!     };
//! }
//!
//! // In main(): let resolved = Contract::parse_and_lint(OnProblems::FailOnError);
//! let args = vec!["--log-level".to_string(), "warn".to_string()];
//! let env = std::collections::BTreeMap::new();
//! let resolved = Invocation { args: &args, env: &env }.resolve(&Contract::records());
//!
//! assert_eq!(Contract::LOG_LEVEL.get_from(&resolved), Some(LogLevel::Warn));
//! assert_eq!(resolved.source("log_level"), Some(Source::Arg));
//! ```
//!
//! # Reading this crate
//!
//! * [`contract!`] — declare a contract and get `records()`, `problems()`,
//!   and `parse_and_lint()` generated for you. Start here.
//! * [`parse()`] — the one-liner: read process argv+env, resolve, done.
//! * [`Invocation`], [`Resolution`], [`Source`] — resolving argv and envp
//!   directly, with provenance for every value. Use this in tests, to
//!   inject specific args/env rather than mutating global process state.
//! * [`lint()`] — what an invocation gets wrong.
//! * [`OnProblems`], [`handle_problems()`] — named policies for what to do
//!   about it, instead of a hand-rolled loop per project.
//! * [`InputResolveExt`] — the trait giving [`Input`] its `get`/`get_from`/
//!   etc. methods. Implemented for every `Input<T, C>`; you will not
//!   usually name this trait directly, just `use argenv_resolve::*;`.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Declare a program's invocation surface as a typed contract.
///
/// Generates a `Contract` struct with one `pub const` per input, plus
/// `records()`, `problems()`, and `parse_and_lint()` — the three things every
/// consumer needs and would otherwise write by hand for each project.
///
/// # Usage
///
/// ```
/// use argenv_resolve::*;
///
/// contract! {
///     /// Verbosity level — both a flag and an env var.
///     LOG_LEVEL: LogLevel = Input {
///         key:     "log_level",
///         ty:      Type::Enum,
///         default: Some(LogLevel::Info),
///         allowed: LogLevel::TOKENS,
///         env:     Some(Env::new("APP_LOG_LEVEL")),
///         arg:     Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
///         summary: "Log verbosity.",
///         ..Input::EMPTY
///     };
///
///     /// HDR output switch.
///     HDR: bool = Input {
///         key:     "hdr",
///         ty:      Type::Bool,
///         default: Some(false),
///         env:     Some(Env::new("APP_HDR")),
///         arg:     Some(Arg { negatable: true, ..Arg::long("hdr") }),
///         summary: "Expose HDR output to the application.",
///         ..Input::EMPTY
///     };
/// }
///
/// let p = Contract::problems();
/// assert!(p.is_empty(), "{p:#?}");
///
/// // In main(), one line resolves and enforces the policy:
/// // let resolved = Contract::parse_and_lint(OnProblems::FailOnError);
/// // let level = Contract::LOG_LEVEL.get_from_or_default(&resolved);
/// ```
///
/// # What is generated
///
/// - `pub struct Contract;`
/// - `impl Contract { pub const $ID: Input<$T, ..> = ...; ... }` — one typed
///   constant per input, carrying both its declaration and (via
///   [`InputResolveExt`]) its accessor.
/// - `Contract::records() -> Vec<Record>` — every input as a portable contract record.
/// - `Contract::problems() -> Vec<String>` — every declaration error across
///   the contract (per-input `check()`, plus `check_unique`). Call this in a
///   test so a bad declaration fails the build rather than a user's launch.
/// - `Contract::parse_and_lint(policy: OnProblems) -> Resolution` — reads the
///   process argv and env, resolves, lints, and applies the given policy.
///   The one call that replaces manual `parse` + `lint` + error-loop wiring.
#[macro_export]
macro_rules! contract {
    ( $( $(#[$m:meta])* $id:ident : $t:ty $(, $c:ty)? = $body:expr ; )+ ) => {
        /// This program's invocation surface, declared as typed constants.
        pub struct Contract;

        impl Contract {
            // The optional `, $c:ty` is the input's subcommand-enum
            // parameter (see `Input::subcommands`) - omitted, it defaults to
            // `NoCommand` exactly as writing `Input<$t>` by hand would.
            $( $(#[$m])* pub const $id: $crate::Input<$t $(, $c)?> = $body; )+

            /// Every input projected to a portable [`$crate::Record`].
            ///
            /// Hand this to [`$crate::lint`], [`$crate::document`], or any
            /// tool that works from the cross-language contract.
            pub fn records() -> Vec<$crate::Record> {
                vec![ $( Contract::$id.to_record() ),+ ]
            }

            /// Every declaration error across the whole contract.
            ///
            /// Empty means valid. Call this once at startup (or in a test)
            /// so a bad declaration fails loudly rather than silently doing
            /// the wrong thing at runtime.
            pub fn problems() -> Vec<String> {
                let mut v = Vec::new();
                $( v.extend(Contract::$id.check()); )+
                v.extend($crate::check_unique(&Contract::records()));
                v
            }

            /// Parse the process argv and env, lint the result, and apply
            /// the given policy — all in one call.
            ///
            /// Panics on declaration errors (bugs in this program, not user
            /// input) before touching argv/env, so they never reach production.
            /// Applies `policy` to invocation problems (user mistakes).
            pub fn parse_and_lint(policy: $crate::OnProblems) -> $crate::Resolution {
                let p = Contract::problems();
                assert!(p.is_empty(), "declaration errors: {p:#?}");
                let resolved = $crate::parse(&Contract::records());
                $crate::handle_problems(&$crate::lint(&Contract::records(), &resolved), policy);
                resolved
            }
        }
    };
}

mod ext;
mod invocation;
mod lint;
mod problems;
mod source;

pub use argenv::*;
pub use ext::{InputResolveDefaultExt, InputResolveExt};
pub use invocation::{Invocation, Resolution, Resolved, Source};
pub use lint::{lint, Finding, Severity};
pub use problems::{handle_problems, OnProblems};
pub use source::{EnvSource, ProcessEnv};

/// Parse the current process's argument vector and environment against a
/// declared model, returning a fully resolved [`Resolution`].
///
/// This is the one-liner for the common case — no manual argv slicing,
/// no `Invocation` construction, no `ProcessEnv` import needed:
///
/// ```no_run
/// # use argenv_resolve::*;
/// # pub const HOST: Input<String> = Input {
/// #     key: "host", ty: Type::String,
/// #     env: Some(Env::new("APP_HOST")),
/// #     arg: Some(Arg { value_name: "HOST", ..Arg::long("host") }),
/// #     ..Input::EMPTY
/// # };
/// let model = vec![HOST.to_record()];
/// let resolution = argenv_resolve::parse(&model);
/// let host = HOST.get_from(&resolution);
/// ```
///
/// For tests, inject specific args and env via [`Invocation`] directly
/// rather than mutating global process state.
pub fn parse(model: &[Record]) -> Resolution {
    let args: Vec<String> = std::env::args().skip(1).collect();
    Invocation {
        args: &args,
        env: &ProcessEnv,
    }
    .resolve(model)
}
