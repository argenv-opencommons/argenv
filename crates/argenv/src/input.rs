//! [`Input`] — the declaration. One input a program accepts, however it arrives.
use crate::{
    Arg, ArgBinding, ConfigKeyRef, Deprecation, Env, EnvBinding, Record, ReviewDate, Since,
    Stability, Type, THIS_VERSION,
};
use serde::Serialize;

/// The default for [`Input`]'s second type parameter — an input with no
/// `subcommands` scoping never needs to name a real subcommand enum. Zero
/// variants: nothing can ever construct one, which is the type-level way of
/// saying "this input is not scoped to anything."
#[derive(Clone, Copy, Debug, Serialize)]
pub enum NoCommand {}

/// One input a program accepts — its identity, its domain, and the doors it can
/// arrive through.
///
/// A declaration is simultaneously the **contract entry**, the **typed
/// accessor**, and a **compile-checked symbol**. Because there is only one
/// artifact, the published contract cannot drift from the code that reads the
/// value.
///
/// `key` is the identity and is transport-free; `env` and `arg` are bindings.
/// `--log-level` and `APP_LOG_LEVEL` are not two settings, they are one setting
/// with two doors, which is why they share a type, a domain, and a default.
///
/// ```
/// use argenv::*;
/// pub const LOG_LEVEL: Input<LogLevel> = Input {
///     key:     "log_level",
///     ty:      Type::Enum,
///     default: Some(LogLevel::Info),
///     allowed: LogLevel::TOKENS,
///     env:     Some(Env::new("APP_LOG_LEVEL")),
///     arg:     Some(Arg { value_name: "LEVEL", ..Arg::pair("log-level", 'l') }),
///     summary: "Log verbosity",
///     ..Input::EMPTY
/// };
/// assert!(LOG_LEVEL.check().is_empty());
/// assert_eq!(LOG_LEVEL.to_record().usage(), "-l, --log-level <LEVEL>");
/// ```
#[derive(Debug)]
pub struct Input<T: 'static, C: 'static = NoCommand> {
    /// **Required.** Transport-free identity, `snake_case`.
    ///
    /// This is what the contract, the resolver, and every generated binding join
    /// on. It is deliberately not a variable name or a flag: an input keeps its
    /// identity when a binding is renamed or a second one is added.
    pub key: &'static str,

    /// **Required.** The kind of value, which also fixes how many values the
    /// argument form takes: none for [`Type::Bool`], one for everything else.
    pub ty: Type,

    /// The value used when the input is absent — the real typed `T`, so an
    /// impossible default cannot be written down.
    pub default: Option<T>,

    /// Accepted tokens for [`Type::Enum`] and [`Type::List`].
    ///
    /// Point this at the domain type's own `TOKENS` const so the contract and
    /// the parser cannot disagree.
    pub allowed: &'static [&'static str],

    /// Characters that separate items in a [`Type::List`] value.
    pub separators: &'static [char],

    /// Whether the program requires this input.
    pub required: bool,

    /// How much a consumer may rely on this input.
    pub stability: Stability,

    /// The version in which this input was introduced.
    pub since: Since,

    /// Present if and only if `stability == Stability::Deprecated`.
    pub deprecation: Option<Deprecation>,

    /// A free-form grouping tag for documentation and help output.
    pub group: &'static str,

    /// A key in another configuration surface this input bridges to.
    pub maps_to: Option<ConfigKeyRef>,

    /// A copy-pasteable example, e.g. `"--log-level warn"`.
    pub example: &'static str,

    /// A field in the program's resolved-configuration output that this input
    /// visibly changes — the hook for confirming a declaration empirically.
    pub observe: Option<&'static str>,

    /// The date a **human** last verified this entry, `YYYY-MM-DD`.
    ///
    /// Hand-written on purpose: it asserts a person's judgement, which no tool
    /// can derive. `None` means nobody has vouched for the entry yet.
    pub reviewed: Option<ReviewDate>,

    /// What the input does, in prose.
    pub summary: &'static str,

    /// How the value may arrive from the environment.
    pub env: Option<Env>,

    /// How the value may arrive from the argument vector.
    pub arg: Option<Arg>,

    /// Restricts this input to one or more subcommands. Empty (the default)
    /// means always active, regardless of which subcommand — or none — was
    /// given.
    ///
    /// `C` is the program's own subcommand enum (the same type as the
    /// dispatch input's `T` — see the crate-level docs on subcommands), so
    /// `subcommands: &[Command::Add, Command::Import]` is checked by the
    /// compiler: a typo'd or removed variant fails to build, rather than
    /// silently doing nothing at runtime. One option genuinely shared by
    /// several subcommands (an `--input-dir` used by both `build` and `run`)
    /// is declared once, with both listed here, instead of duplicated per
    /// subcommand or wrongly made global.
    pub subcommands: &'static [C],
}

impl<T: 'static, C: 'static> Input<T, C> {
    /// The empty baseline: `Input { key: "...", ty: ..., ..Input::EMPTY }`.
    pub const EMPTY: Input<T, C> = Input {
        key: "",
        ty: Type::String,
        default: None,
        allowed: &[],
        separators: &[],
        required: false,
        stability: Stability::Unknown,
        since: Since::Unknown,
        deprecation: None,
        group: "",
        maps_to: None,
        example: "",
        observe: None,
        reviewed: None,
        summary: "",
        env: None,
        arg: None,
        subcommands: &[],
    };

    /// How many values the argument form consumes: `0` for a boolean flag whose
    /// presence is the value, `1` otherwise. Derived, never declared.
    pub const fn arity(&self) -> u8 {
        match self.ty {
            Type::Bool => 0,
            _ => 1,
        }
    }

    /// Validate this declaration against the contract's rules.
    ///
    /// Returns human-readable problems; empty means valid. Run it in a test so a
    /// bad declaration fails the build rather than a user's launch.
    pub fn check(&self) -> Vec<String> {
        let mut e = Vec::new();
        let at = if self.key.is_empty() {
            "<unnamed>"
        } else {
            self.key
        };

        if self.key.is_empty() {
            e.push("an input has an empty key".to_string());
        } else if let Err(why) = valid_key(self.key) {
            e.push(format!("{at}: key {why}"));
        }

        // An input with no binding cannot be supplied by anyone.
        if self.env.is_none() && self.arg.is_none() {
            e.push(format!(
                "{at}: declares neither an env nor an arg binding, so nothing can ever set it"
            ));
        }

        if let Some(env) = self.env {
            if let Err(why) = valid_env_name(env.name) {
                e.push(format!("{at}: env name `{}` {why}", env.name));
            }
            for a in env.aliases {
                if let Err(why) = valid_env_name(a) {
                    e.push(format!("{at}: env alias `{a}` {why}"));
                }
                if *a == env.name {
                    e.push(format!("{at}: env alias repeats the primary name"));
                }
            }
        }

        if let Some(arg) = self.arg {
            let named = arg.long.is_some() || arg.short.is_some();
            let positional = arg.position.is_some();
            if !named && !positional {
                e.push(format!(
                    "{at}: arg binding has neither a long/short form nor a position — it must be one or the other"
                ));
            }
            if named && positional {
                e.push(format!(
                    "{at}: arg binding has both a long/short form and a position — a flag is one or the other, never both"
                ));
            }
            if positional && self.ty == Type::Bool {
                e.push(format!(
                    "{at}: a positional cannot be Type::Bool — a positional's presence already                      means a value was given, so there is no separate on/off state for a bool to                      express; use a named flag for a switch instead"
                ));
            }
            if positional && arg.negatable {
                e.push(format!(
                    "{at}: `negatable` produces --no-… which only makes sense for a named flag"
                ));
            }
            if positional && arg.repeatable {
                e.push(format!("{at}: `repeatable` accumulates repeated occurrences of a named flag; a positional occurs at most once by construction"));
            }
            if let Some(l) = arg.long {
                if let Err(why) = valid_long(l) {
                    e.push(format!("{at}: long `--{l}` {why}"));
                }
            }
            if let Some(s) = arg.short {
                if !s.is_ascii_alphanumeric() {
                    e.push(format!(
                        "{at}: short `-{s}` must be an ASCII letter or digit"
                    ));
                }
            }
            if arg.negatable && self.ty != Type::Bool {
                e.push(format!(
                    "{at}: `negatable` produces --no-… and is meaningful only for bool, but type is {}",
                    self.ty
                ));
            }
            if arg.negatable && arg.long.is_none() {
                e.push(format!("{at}: `negatable` needs a long form to negate"));
            }
            if arg.repeatable && self.ty != Type::List {
                e.push(format!(
                    "{at}: `repeatable` accumulates values and is meaningful only for list, but type is {}",
                    self.ty
                ));
            }
            if self.ty == Type::Bool && !arg.value_name.is_empty() {
                e.push(format!(
                    "{at}: a bool flag takes no value, so `value_name` is meaningless"
                ));
            }
        }

        if let Some(v) = self.since.resolve() {
            if v > THIS_VERSION {
                e.push(format!(
                    "{at}: `since` {v} is newer than the current version {THIS_VERSION}"
                ));
            }
        }
        if let Some(r) = self.reviewed {
            // One day of slack: an author east of UTC writes their local date,
            // which is legitimately "tomorrow" by the build machine's clock.
            if r > ReviewDate::today().next_day() {
                e.push(format!("{at}: `reviewed` date {r} is in the future"));
            }
        }
        match (self.stability, self.deprecation.is_some()) {
            (Stability::Deprecated, false) => e.push(format!(
                "{at}: stability is Deprecated but no Deprecation details are given"
            )),
            (s, true) if s != Stability::Deprecated => e.push(format!(
                "{at}: has Deprecation details but stability is {s}"
            )),
            _ => {}
        }
        if let Some(d) = &self.deprecation {
            if d.since > THIS_VERSION {
                e.push(format!(
                    "{at}: deprecated-since {} is newer than the current version {THIS_VERSION}",
                    d.since
                ));
            }
        }

        let listy = matches!(self.ty, Type::Enum | Type::List);
        if listy && self.allowed.is_empty() {
            e.push(format!("{at}: type is {} but `allowed` is empty", self.ty));
        }
        if !self.allowed.is_empty() && !listy {
            e.push(format!(
                "{at}: `allowed` is only meaningful for enum/list, but type is {}",
                self.ty
            ));
        }
        if self.ty == Type::List && self.separators.is_empty() {
            e.push(format!(
                "{at}: type is list but no `separators` are declared, so a value cannot be split"
            ));
        }
        if !self.separators.is_empty() && self.ty != Type::List {
            e.push(format!(
                "{at}: `separators` are only meaningful for list, but type is {}",
                self.ty
            ));
        }
        e
    }
}

impl<T: 'static + Serialize, C: 'static + Serialize> Input<T, C> {
    /// Project this declaration down to the portable [`Record`] the contract
    /// describes.
    pub fn to_record(&self) -> Record {
        Record {
            key: self.key.to_string(),
            ty: self.ty.as_str().to_string(),
            default: self
                .default
                .as_ref()
                .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null)),
            allowed: self.allowed.iter().map(|s| s.to_string()).collect(),
            separators: self.separators.iter().map(|c| c.to_string()).collect(),
            required: self.required,
            stability: self.stability.as_str().to_string(),
            since: self.since.to_json(),
            deprecation: self.deprecation.as_ref().map(|d| d.to_json()),
            group: non_empty(self.group),
            maps_to: self.maps_to.map(|c| c.0.to_string()),
            example: non_empty(self.example),
            observe: self.observe.map(|s| s.to_string()),
            reviewed: self.reviewed.map(|d| d.to_string()),
            summary: non_empty(self.summary),
            modified: None,
            source: None,
            env: self.env.map(|e| EnvBinding {
                name: e.name.to_string(),
                aliases: e.aliases.iter().map(|s| s.to_string()).collect(),
            }),
            arg: self.arg.map(|a| ArgBinding {
                long: a.long.map(str::to_string),
                short: a.short.map(|c| c.to_string()),
                // Derived here so no other language has to re-derive it.
                arity: self.arity(),
                value_name: non_empty(a.value_name),
                negatable: a.negatable,
                repeatable: a.repeatable,
                position: a.position,
            }),
            subcommands: self
                .subcommands
                .iter()
                .map(|c| {
                    // The same conversion `default` already goes through, so
                    // a token is derived here rather than requiring a second,
                    // separately-maintained "to string" mechanism.
                    match serde_json::to_value(c).ok() {
                        Some(serde_json::Value::String(s)) => s,
                        other => panic!(
                            "subcommand value did not serialise to a plain string: {other:?} \
                             — a subcommand enum should derive Serialize with rename_all = \
                             snake_case, the same convention LogLevel and Tristate use"
                        ),
                    }
                })
                .collect(),
        }
    }
}

/// An input's identity is `snake_case`: transport-free, so it reads the same in
/// every language a binding is generated for.
fn valid_key(key: &str) -> Result<(), String> {
    let mut chars = key.chars();
    match chars.next() {
        None => return Err("is empty".into()),
        Some(c) if c.is_ascii_lowercase() => {}
        Some(c) => {
            return Err(format!(
                "starts with `{c}`; must start with a lowercase letter"
            ))
        }
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_') {
            return Err(format!(
                "contains `{c}`; keys are snake_case (lowercase, digits, underscore)"
            ));
        }
    }
    Ok(())
}

/// Environment names are portable only within `[A-Za-z_][A-Za-z0-9_]*`. A name
/// with a space, a dash, or a leading digit cannot be set from a POSIX shell.
fn valid_env_name(name: &str) -> Result<(), String> {
    let mut chars = name.chars();
    match chars.next() {
        None => return Err("is empty".into()),
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        Some(c) => {
            return Err(format!(
                "starts with `{c}`; must start with a letter or underscore"
            ))
        }
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!(
                "contains `{c}`; only letters, digits and underscore are portable"
            ));
        }
    }
    Ok(())
}

/// Long flags are kebab-case and are stored without their leading dashes, so a
/// declaration cannot disagree with itself about how many dashes to write.
fn valid_long(long: &str) -> Result<(), String> {
    if long.starts_with('-') {
        return Err("must be written without leading dashes".into());
    }
    let mut chars = long.chars();
    match chars.next() {
        None => return Err("is empty".into()),
        Some(c) if c.is_ascii_lowercase() => {}
        Some(c) => {
            return Err(format!(
                "starts with `{c}`; must start with a lowercase letter"
            ))
        }
    }
    if long.starts_with("no-") {
        return Err("starts with `no-`, which collides with the negation of another flag".into());
    }
    for c in chars {
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(format!(
                "contains `{c}`; long flags are kebab-case (lowercase, digits, dashes)"
            ));
        }
    }
    Ok(())
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}
