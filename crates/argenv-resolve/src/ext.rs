//! [`InputResolveExt`] — [`Input`]'s runtime-reading methods, added from
//! outside the data crate.
//!
//! `argenv::Input<T, C>` cannot have methods that take a [`Resolution`]
//! defined on it directly: `Resolution` lives in this crate, which already
//! depends on `argenv` for `Input` — a dependency back the other way would
//! be circular. An extension trait is the standard, idiomatic way around
//! this in Rust: implemented here, for a type declared elsewhere, callable
//! with ordinary method syntax the moment this trait is in scope (which
//! `use argenv_resolve::*;` already brings in for every consumer).

use crate::{EnvSource, ProcessEnv, Resolution};
use argenv::{FromRaw, Input};

/// Runtime-reading methods for [`Input`], added by this crate.
///
/// Implemented for every `Input<T, C>` whose `T` implements [`FromRaw`] —
/// which is every `Input` that could ever have a value read out of it.
/// `use argenv_resolve::*;` brings this into scope automatically; you will
/// not usually name it directly, just call `.get_from(&resolution)` etc.
pub trait InputResolveExt<T> {
    /// Every environment name this input answers to.
    fn env_names(&self) -> Vec<&'static str>;

    /// Read and parse from the process environment. `None` means absent
    /// **or** invalid — never a silent empty string.
    fn get(&self) -> Option<T>;

    /// Read and parse from `env`.
    ///
    /// Use this to check an environment being assembled for a child
    /// process, to read a snapshot captured elsewhere, or to test without
    /// mutating global process state.
    fn get_in(&self, env: &impl EnvSource) -> Option<T>;

    /// Read and parse from a resolved invocation, honouring precedence
    /// across both bindings.
    fn get_from(&self, resolution: &Resolution) -> Option<T>;

    /// Whether the input is present in the process environment.
    fn is_set(&self) -> bool;

    /// Whether the input is present in `env`.
    fn is_set_in(&self, env: &impl EnvSource) -> bool;

    /// The raw, unparsed value from the process environment, if set.
    fn raw(&self) -> Option<String>;

    /// The raw, unparsed value from `env`, if set.
    fn raw_in(&self, env: &impl EnvSource) -> Option<String>;
}

impl<T, C> InputResolveExt<T> for Input<T, C>
where
    T: 'static + FromRaw,
{
    fn env_names(&self) -> Vec<&'static str> {
        self.env
            .into_iter()
            .flat_map(|e| std::iter::once(e.name).chain(e.aliases.iter().copied()))
            .collect()
    }

    fn get(&self) -> Option<T> {
        self.get_in(&ProcessEnv)
    }

    fn get_in(&self, env: &impl EnvSource) -> Option<T> {
        InputResolveExt::raw_in(self, env).and_then(|s| T::from_raw(&s))
    }

    fn get_from(&self, resolution: &Resolution) -> Option<T> {
        resolution.raw(self.key).and_then(T::from_raw)
    }

    fn is_set(&self) -> bool {
        self.is_set_in(&ProcessEnv)
    }

    fn is_set_in(&self, env: &impl EnvSource) -> bool {
        InputResolveExt::env_names(self)
            .into_iter()
            .any(|n| env.get(n).is_some())
    }

    fn raw(&self) -> Option<String> {
        self.raw_in(&ProcessEnv)
    }

    fn raw_in(&self, env: &impl EnvSource) -> Option<String> {
        InputResolveExt::env_names(self)
            .into_iter()
            .find_map(|n| env.get(n))
    }
}

/// [`get_from_or_default`](InputResolveDefaultExt::get_from_or_default) — a
/// separate trait because it needs `T: Clone`, which not every `Input` value
/// type provides, the same split `argenv::Input`'s own impl blocks already
/// made before this trait existed.
pub trait InputResolveDefaultExt<T> {
    /// Read from a resolved invocation, falling back to the declared default.
    fn get_from_or_default(&self, resolution: &Resolution) -> Option<T>;
}

impl<T, C> InputResolveDefaultExt<T> for Input<T, C>
where
    T: 'static + FromRaw + Clone,
{
    fn get_from_or_default(&self, resolution: &Resolution) -> Option<T> {
        self.get_from(resolution).or_else(|| self.default.clone())
    }
}
