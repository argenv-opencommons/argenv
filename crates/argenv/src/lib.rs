//! A declared, typed contract for a program's **invocation surface** — argv and
//! envp.
//!
//! A program is handed two things when it starts: an argument vector and an
//! environment. Both are interfaces. Both are depended upon by scripts, images,
//! pipelines, and users. And almost no program declares either one in a form
//! anything can check — the names, the types, the legal values and the defaults
//! live in prose, or in nothing at all.
//!
//! This crate makes that surface an artifact: declared, typed, checkable, and
//! emittable as a language-neutral JSON document.
//!
//! # The one idea
//!
//! **The declaration is the accessor.** An [`Input`] both describes an input and
//! reads it, so the published contract cannot drift from the code that consumes
//! the value. And because identity ([`Input::key`]) is separate from addressing
//! ([`Env`], [`Arg`]), `--log-level` and `APP_LOG_LEVEL` are not two settings —
//! they are one setting with two doors.
//!
//! ```
//! use argenv::*;
//!
//! pub const LOG_LEVEL: Input<LogLevel> = Input {
//!     key:     "log_level",
//!     ty:      Type::Enum,
//!     default: Some(LogLevel::Info),
//!     allowed: LogLevel::TOKENS,            // one source for parser *and* contract
//!     env:     Some(Env::new("APP_LOG_LEVEL")),
//!     arg:     Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
//!     summary: "Log verbosity",
//!     ..Input::EMPTY
//! };
//!
//! let model = vec![LOG_LEVEL.to_record()];
//! let args = vec!["--log-level".to_string(), "warn".to_string()];
//! let env = std::collections::BTreeMap::new();
//! let resolved = Invocation { args: &args, env: &env }.resolve(&model);
//!
//! assert_eq!(LOG_LEVEL.get_from(&resolved), Some(LogLevel::Warn));
//! assert_eq!(resolved.source("log_level"), Some(Source::Arg));
//! ```
//!
//! A misspelled key is a compile error. A wrongly typed read is a compile error.
//! A misspelled *flag or variable* is a [`Finding`] instead of silence.
//!
//! # Reading this crate
//!
//! * [`Input`] — the declaration. Start here; it is the surface you author.
//! * [`Arg`] and [`Env`] — the two bindings. Arity is derived from the type.
//! * [`Version`], [`ReviewDate`], [`Type`], [`Stability`], [`Since`],
//!   [`ConfigKeyRef`], [`Deprecation`] — the vocabulary. Skim once.
//! * [`FromRaw`] — how a type parses itself, so declarations carry no parser.
//! * [`Invocation`], [`Resolution`], [`Source`] — resolving argv and envp, with
//!   provenance for every value.
//! * [`lint()`] — what an invocation gets wrong.
//! * [`Record`], [`document`] — the portable projection and its envelope.
//! * `contract::json_schema` (feature `contract`) — the cross-language schema.
//!
//! # Design rules
//!
//! * **No naked primitive stands for a domain value.** Versions and dates parse
//!   and compare; closed sets are enums; constrained numbers are newtypes.
//!   Plain `String` survives only for genuine prose.
//! * **Anything a human would forget is derived**: arity from the type, `modified`
//!   and `generated` from tooling. The one exception is [`Input::reviewed`],
//!   which asserts human judgement nothing can compute.
//! * **Partial entries are first-class.** A key, a type and one binding are all
//!   that is required, so a harvested name is a valid entry and enrichment is a
//!   one-line pull request.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod binding;
mod date;
mod from_raw;
mod input;
mod invocation;
mod lint;
mod record;
mod source;
mod version;
mod vocabulary;

#[cfg(feature = "contract")]
pub mod contract;

/// Parse the current process's argument vector and environment against a
/// declared model, returning a fully resolved [`Resolution`].
///
/// This is the one-liner for the common case — no manual argv slicing,
/// no `Invocation` construction, no `ProcessEnv` import needed:
///
/// ```no_run
/// # use argenv::*;
/// # pub const HOST: Input<String> = Input {
/// #     key: "host", ty: Type::String,
/// #     env: Some(Env::new("APP_HOST")),
/// #     arg: Some(Arg { value_name: "HOST", ..Arg::long("host") }),
/// #     ..Input::EMPTY
/// # };
/// let model = vec![HOST.to_record()];
/// let resolution = argenv::parse(&model);
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

pub use binding::{Arg, Env, GatedBy};
pub use date::ReviewDate;
pub use from_raw::{FromRaw, LogLevel, Tristate};
pub use input::Input;
pub use invocation::{Invocation, Resolution, Resolved, Source};
pub use lint::{lint, Finding, Severity};
pub use record::{
    check_gates, check_unique, document, ArgBinding, EnvBinding, GatedByRecord, Record,
    CONTRACT_VERSION, PRECEDENCE,
};
pub use source::{EnvSource, ProcessEnv};
pub use version::{Version, THIS_VERSION};
pub use vocabulary::{ConfigKeyRef, Deprecation, Since, Stability, Type};
