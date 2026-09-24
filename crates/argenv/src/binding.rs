//! The two doors a value can arrive through, [`Arg`] and [`Env`], and
//! [`GatedBy`], which restricts an input to one branch of the model's own
//! positional dispatch rather than addressing it.
//!
//! An input's *identity* (its `key`) is separate from how it is *addressed*.
//! That separation is the whole reason argv and envp share one model instead of
//! needing two: everything except addressing is common to both.

/// The environment binding: the variable name a value may arrive under.
#[derive(Clone, Copy, Debug)]
pub struct Env {
    /// The variable name, e.g. `"APP_LOG_LEVEL"`.
    pub name: &'static str,
    /// Other names still honoured, so a legacy name is recognised rather than
    /// reported as a mistake.
    pub aliases: &'static [&'static str],
}

impl Env {
    /// A binding with no aliases: `Env { name: "APP_X", ..Env::EMPTY }`.
    pub const EMPTY: Env = Env {
        name: "",
        aliases: &[],
    };

    /// Construct a plain binding.
    pub const fn new(name: &'static str) -> Env {
        Env { name, aliases: &[] }
    }
}

/// The argument-vector binding: the flag a value may arrive under.
///
/// How many values the flag takes is **not** declared here — it follows from the
/// input's [`crate::Type`]. A boolean flag takes none and means *true* by its
/// presence; anything else takes exactly one. Deriving it removes a field that
/// could contradict the type, and the derived arity is still written out
/// explicitly in the published contract so no other language has to re-derive it.
#[derive(Clone, Copy, Debug)]
pub struct Arg {
    /// The long form without dashes, e.g. `"log-level"` for `--log-level`.
    /// Mutually exclusive with `position` - see [`Arg::positional`].
    pub long: Option<&'static str>,
    /// The short form without its dash, e.g. `'l'` for `-l`.
    /// Mutually exclusive with `position` - see [`Arg::positional`].
    pub short: Option<char>,
    /// Whether `--no-<long>` is accepted to force the value off. Booleans only.
    pub negatable: bool,
    /// Whether the flag may be repeated, accumulating values. Lists only.
    pub repeatable: bool,
    /// The placeholder shown in help for the value, e.g. `"LEVEL"` in
    /// `--log-level <LEVEL>`. Ignored for booleans, which take no value.
    pub value_name: &'static str,
    /// A bare positional slot instead of a `--flag`, addressed by position
    /// rather than by name - see [`Arg::positional`]. `Some(0)` for an input
    /// gated by nothing is the model's one dispatch gate (at most one such
    /// input may exist); `Some(n)` on an input with [`crate::Input::gated_by`]
    /// set means "the n-th positional after the gate's own token", so two
    /// different branches may each use `Some(0)` for their own first
    /// positional without conflict - they are mutually exclusive by
    /// construction, never both active in the same invocation.
    pub position: Option<u16>,
}

impl Arg {
    /// A binding with nothing set: `Arg { long: Some("x"), ..Arg::EMPTY }`.
    pub const EMPTY: Arg = Arg {
        long: None,
        short: None,
        negatable: false,
        repeatable: false,
        value_name: "",
        position: None,
    };

    /// A long flag, e.g. `Arg::long("log-level")` for `--log-level`.
    pub const fn long(long: &'static str) -> Arg {
        Arg {
            long: Some(long),
            ..Arg::EMPTY
        }
    }

    /// A short flag, e.g. `Arg::short('l')` for `-l`.
    pub const fn short(short: char) -> Arg {
        Arg {
            short: Some(short),
            ..Arg::EMPTY
        }
    }

    /// Both forms, e.g. `Arg::pair("log-level", 'l')`.
    pub const fn pair(long: &'static str, short: char) -> Arg {
        Arg {
            long: Some(long),
            short: Some(short),
            ..Arg::EMPTY
        }
    }

    /// A bare positional, e.g. `Arg::positional(0)` for the first one.
    ///
    /// Not a `--flag`: consumed by position in argv, same slot [`crate::Resolution::positionals`]
    /// already collects - this just lets a specific one be declared, typed,
    /// and documented like any other input, instead of read out of that raw
    /// list by hand.
    pub const fn positional(position: u16) -> Arg {
        Arg {
            position: Some(position),
            ..Arg::EMPTY
        }
    }
}


/// Restricts an input to one branch of the model's one positional gate.
///
/// A gate is an ordinary [`Input`](crate::Input) whose `arg` is
/// [`Arg::positional(0)`](Arg::positional) and whose own `gated_by` is `None`
/// — typically [`crate::Type::Enum`], with `allowed` naming the branches. Any
/// other input may then set `gated_by` to say "only when the gate resolved
/// to this value". [`crate::check_gates`] verifies the whole model agrees
/// with itself: at most one gate; every branch named actually exists; nothing
/// gated is itself a gate for something else. That last rule is what keeps
/// dispatch to one level rather than a subcommand tree.
///
/// ```
/// use argenv::*;
/// pub const COMMAND: Input<String> = Input {
///     key: "command",
///     ty: Type::Enum,
///     allowed: &["add", "status"],
///     arg: Some(Arg::positional(0)),
///     summary: "Which command to run",
///     ..Input::EMPTY
/// };
/// pub const SOURCE: Input<String> = Input {
///     key: "source",
///     arg: Some(Arg::positional(0)), // position 0 *after* the gate's own token
///     gated_by: Some(GatedBy { value: "add", env_prefix: "APP_ADD" }),
///     summary: "The repository to add, owner/repo",
///     ..Input::EMPTY
/// };
/// assert!(COMMAND.check().is_empty());
/// assert!(SOURCE.check().is_empty());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct GatedBy {
    /// The gate's resolved value this input requires, e.g. `"add"`.
    pub value: &'static str,
    /// Every env name on this input (its `env.name` and any `env.aliases`)
    /// must start with this - checked by [`crate::Input::check`], never
    /// generated for you, so the name a person actually reads in their shell
    /// is never hidden behind an inferred prefix. Env vars are process-global
    /// while argv position is not, so without this, two branches declaring
    /// the same flag `key` (fine - they are mutually exclusive) would
    /// collide the moment either one picked an env var.
    pub env_prefix: &'static str,
}
