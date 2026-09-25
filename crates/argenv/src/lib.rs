//! A declared, typed **contract** for a program's invocation surface — argv
//! and envp — as data.
//!
//! A program is handed two things when it starts: an argument vector and an
//! environment. Both are interfaces. Both are depended upon by scripts,
//! images, pipelines, and users. And almost no program declares either one
//! in a form anything can check — the names, the types, the legal values
//! and the defaults live in prose, or in nothing at all.
//!
//! This crate makes that surface an artifact: declared, typed, checkable,
//! and emittable as a language-neutral JSON document.
//!
//! This crate is data only — it declares what a program accepts and can
//! check that declaration and project it to a portable document, but it
//! does not read argv or envp. For that, see
//! [`argenv-resolve`](https://docs.rs/argenv-resolve), which depends on
//! this crate and adds nothing else: a program's contract never needs to
//! know how it gets resolved, and a resolver never needs to know anything
//! this crate doesn't already say.
//!
//! # The one idea
//!
//! **The declaration is the accessor's contract.** An [`Input`] both
//! describes an input and — via `argenv-resolve`'s extension trait — reads
//! it, so the published contract cannot drift from the code that consumes
//! the value. And because identity ([`Input::key`]) is separate from
//! addressing ([`Env`], [`Arg`]), `--log-level` and `APP_LOG_LEVEL` are not
//! two settings — they are one setting with two doors.
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
//! assert!(LOG_LEVEL.check().is_empty());
//! assert_eq!(LOG_LEVEL.to_record().usage(), "-l, --log-level <LEVEL>");
//! ```
//!
//! A misspelled key is a compile error. A wrongly typed read is a compile
//! error — resolving and reading is `argenv-resolve`'s job, but the
//! `Input<T>` you declare here is the same value that crate's `get_from`
//! is typed against, so a mismatch is still caught here, at declaration.
//!
//! # Reading this crate
//!
//! * [`Input`] — the declaration. Start here; it is the surface you author.
//! * [`Arg`] and [`Env`] — the two bindings. Arity is derived from the type.
//! * [`Version`], [`ReviewDate`], [`Type`], [`Stability`], [`Since`],
//!   [`ConfigKeyRef`], [`Deprecation`] — the vocabulary. Skim once.
//! * [`FromRaw`] — how a type parses itself, so declarations carry no parser.
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
//! * **This crate does not resolve anything.** No argv, no envp, no
//!   `Invocation`. That separation is deliberate — see `argenv-resolve`.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod binding;
mod date;
mod from_raw;
mod input;
mod record;
mod version;
mod vocabulary;

#[cfg(feature = "contract")]
pub mod contract;

pub use binding::{Arg, Env};
pub use date::ReviewDate;
pub use from_raw::{FromRaw, LogLevel, Tristate};
pub use input::{Input, NoCommand};
pub use record::{
    check_unique, document, ArgBinding, EnvBinding, Record, CONTRACT_VERSION, PRECEDENCE,
};
pub use version::{Version, THIS_VERSION};
pub use vocabulary::{ConfigKeyRef, Deprecation, Since, Stability, Type};
