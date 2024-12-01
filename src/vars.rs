//! Variable resolvers.
//!
//! This module contains basic support for variable resolvers and an implementation of the same to resolve environment variables.

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::rc::Rc;
use thiserror::Error;
use tracing::instrument;

/// The error type for operations interacting with variables.
#[derive(Debug, Error)]
pub(crate) enum VarError {
    /// The specified variable is not present.
    #[error("variable '{0}' not found")]
    NotPresent(String),
}

/// Trait for variable resolvers.
pub(crate) trait VarResolver: fmt::Debug {
    /// Resolves the variable with the given name.
    fn resolve_var(&self, var_name: &str) -> Result<String, VarError>;
}

/// [`VarResolver`] implementation that simply returns the variable as-is.
#[derive(Debug)]
pub(crate) struct AsIsVarResolver;

impl VarResolver for AsIsVarResolver {
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        Ok(format!("${{{v}}}"))
    }
}

/// [`VarResolver`] implementation for environment variables.
#[derive(Debug)]
pub(crate) struct EnvVarResolver;

impl VarResolver for EnvVarResolver {
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        if let Some(v) = v.strip_prefix("env.") {
            if let Ok(val) = env::var(v) {
                return Ok(val);
            }
        };

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// [`VarResolver`] implementation that combines other variable resolvers.
#[derive(Debug)]
pub(crate) struct CombinedVarResolver {
    resolvers: Vec<Rc<dyn VarResolver>>,
}

impl CombinedVarResolver {
    /// Constructs a new `CombinedVarResolver` with the given variable resolvers.
    pub(crate) fn new<I>(resolvers: I) -> Self
    where
        I: IntoIterator<Item = Rc<dyn VarResolver>>,
    {
        Self {
            resolvers: Vec::from_iter(resolvers),
        }
    }
}

impl VarResolver for CombinedVarResolver {
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        for resolver in &self.resolvers {
            if let Ok(value) = resolver.resolve_var(v) {
                return Ok(value);
            }
        }

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// [`VarResolver`] implementation for simple variables.
#[derive(Debug)]
pub(crate) struct SimpleVarResolver<'a> {
    vars: HashMap<&'a str, String>,
}

impl<'a> SimpleVarResolver<'a> {
    /// Constructs a new `SimpleVarResolver`.
    pub(crate) fn new() -> Self {
        Self { vars: HashMap::new() }
    }

    /// Registers the value for the given variable name.
    pub(crate) fn insert<V: Into<String>>(&mut self, name: &'a str, val: V) {
        self.vars.insert(name, val.into());
    }
}

impl VarResolver for SimpleVarResolver<'_> {
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        match self.vars.get(v) {
            Some(value) => Ok(value.clone()),
            _ => Err(VarError::NotPresent(v.to_owned())),
        }
    }
}

/// Expands variables in strings with the help of variable resolvers.
#[derive(Debug)]
pub(crate) struct VarExpander {
    // The array with variable resolvers.
    #[doc(hidden)]
    resolvers: Vec<Box<dyn VarResolver>>,
}

impl VarExpander {
    /// Constructs a new `VarExpander` with the given variable resolvers.
    pub(crate) fn new<I>(resolvers: I) -> Self
    where
        I: IntoIterator<Item = Box<dyn VarResolver>>,
    {
        Self {
            resolvers: Vec::from_iter(resolvers),
        }
    }

    /// Expands all known variables in the given string.
    #[instrument(level = "trace", ret)]
    pub(crate) fn expand<'a, S>(&self, s: &'a S) -> Result<Cow<'a, str>, VarError>
    where
        S: ?Sized + AsRef<str> + fmt::Debug,
    {
        let resolved = shellexpand::env_with_context(s, |s| self.resolve(s));
        resolved.map_err(|err| err.cause)
    }

    // Provides the context for `expand`.
    #[doc(hidden)]
    fn resolve(&self, v: &str) -> Result<Option<String>, VarError> {
        for r in &self.resolvers {
            let v = r.resolve_var(v);
            match v {
                Ok(v) => return Ok(Some(v)),
                _ => continue,
            };
        }

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// Expands all known variables in the given string with the given variable resolvers.
#[instrument(level = "trace", ret, skip(resolvers))]
pub(crate) fn expand<'a, S, I>(s: &'a S, mut resolvers: I) -> Result<Cow<'a, str>, VarError>
where
    S: ?Sized + AsRef<str> + fmt::Debug,
    I: Iterator<Item = &'a dyn VarResolver>,
{
    let resolved = shellexpand::env_with_context(s, |s| {
        for r in resolvers.by_ref() {
            let v = r.resolve_var(s);
            match v {
                Ok(v) => return Ok(Some(v)),
                _ => continue,
            };
        }

        Err(VarError::NotPresent(s.to_owned()))
    });
    resolved.map_err(|err| err.cause)
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn as_is_var_resolver() {
        let resolver = AsIsVarResolver;
        let resolved = resolver.resolve_var("abc.def").unwrap();
        assert_eq!(resolved, "${abc.def}");
    }

    #[test]
    fn env_var_resolver_known_var() {
        env::set_var("MY_SHELL_VAR1", "MY_SHELL_VAL");
        let resolver = EnvVarResolver;
        let resolved = resolver.resolve_var("env.MY_SHELL_VAR1").unwrap();
        assert_eq!(resolved, "MY_SHELL_VAL");
    }

    #[test]
    fn env_var_resolver_unknown_var() {
        env::remove_var("MY_SHELL_VAR2");
        let resolver = EnvVarResolver;
        let resolved = resolver.resolve_var("env.MY_SHELL_VAR2");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "env.MY_SHELL_VAR2"),
        };
        assert!(failed);
    }

    #[test]
    fn simple_var_resolver_known_var() {
        let mut resolver = Box::new(SimpleVarResolver::new());
        resolver.insert("foo.bar", "baz");
        let resolved = resolver.resolve_var("foo.bar").unwrap();
        assert_eq!(resolved, "baz");
    }

    #[test]
    fn simple_var_resolver_unknown_var() {
        let mut resolver = Box::new(SimpleVarResolver::new());
        resolver.insert("foo.bar", "baz");
        let resolved = resolver.resolve_var("foo.buz");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "foo.buz"),
        };
        assert!(failed);
    }

    fn var_expander() -> VarExpander {
        let mut simple_var_resolver = Box::new(SimpleVarResolver::new());
        simple_var_resolver.insert("foo.bar", "baz");
        let var_resolvers: [Box<dyn VarResolver>; 2] = [simple_var_resolver, Box::new(EnvVarResolver)];
        VarExpander::new(var_resolvers)
    }

    #[test]
    fn vars_resolver_known_env_var() {
        env::set_var("MY_SHELL_VAR3", "MY_SHELL_VAL");
        let expanded = var_expander().expand("${env.MY_SHELL_VAR3}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("MY_SHELL_VAL"));
    }

    #[test]
    fn vars_resolver_unknown_env_var() {
        env::remove_var("MY_SHELL_VAR4");
        let expanded = var_expander().expand("${env.MY_SHELL_VAR4}");
        let failed = match expanded {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "env.MY_SHELL_VAR4"),
        };
        assert!(failed);
    }

    #[test]
    fn vars_resolver_known_simple_var() {
        let expanded = var_expander().expand("${foo.bar}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("baz"));
    }

    #[test]
    fn vars_resolver_unknown_simple_var() {
        let expanded = var_expander().expand("${foo.buz}");
        let failed = match expanded {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "foo.buz"),
        };
        assert!(failed);
    }
}
