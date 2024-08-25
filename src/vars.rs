//! Variable resolvers.
//!
//! This module contains basic support for variable resolvers and an implementation of the same to resolve environment variables.

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fmt;
use thiserror::Error;
use tracing::*;

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

impl<'a> VarResolver for SimpleVarResolver<'a> {
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        match self.vars.get(v) {
            Some(value) => Ok(value.clone()),
            _ => Err(VarError::NotPresent(v.to_owned())),
        }
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

/// Resolver to resolve variables in strings with the help of other variable resolvers.
#[derive(Debug)]
pub(crate) struct VarsResolver {
    // The array with variable resolvers.
    #[doc(hidden)]
    resolvers: Vec<Box<dyn VarResolver>>,
}

impl VarsResolver {
    /// Constructs a new `VarsResolver` with the given variable resolvers.
    pub(crate) fn new<I>(resolvers: I) -> VarsResolver
    where
        I: IntoIterator<Item = Box<dyn VarResolver>>,
    {
        VarsResolver {
            resolvers: Vec::from_iter(resolvers),
        }
    }

    /// Resolves all known variables in the given string.
    #[instrument(level = "trace", ret)]
    pub(crate) fn resolve<'a, S>(&self, s: &'a S) -> Result<Cow<'a, str>, VarError>
    where
        S: ?Sized + AsRef<str> + fmt::Debug,
    {
        let resolved = shellexpand::env_with_context(s, |s| self._resolve_var(s));
        resolved.map_err(|err| err.cause)
    }

    // Provides the context for `resolve`.
    #[doc(hidden)]
    fn _resolve_var(&self, v: &str) -> Result<Option<String>, VarError> {
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

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

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

    fn vars_resolver() -> VarsResolver {
        let mut simple_var_resolver = Box::new(SimpleVarResolver::new());
        simple_var_resolver.insert("foo.bar", "baz");
        let env_var_resolver = Box::new(EnvVarResolver);
        let var_resolvers: [Box<dyn VarResolver>; 2] = [env_var_resolver, simple_var_resolver];
        VarsResolver::new(var_resolvers)
    }

    #[test]
    fn vars_resolver_known_env_var() {
        env::set_var("MY_SHELL_VAR3", "MY_SHELL_VAL");
        let resolved = vars_resolver().resolve("${env.MY_SHELL_VAR3}").unwrap();
        assert_eq!(resolved, Cow::Borrowed("MY_SHELL_VAL"));
    }

    #[test]
    fn vars_resolver_unknown_env_var() {
        env::remove_var("MY_SHELL_VAR4");
        let resolved = vars_resolver().resolve("${env.MY_SHELL_VAR4}");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "env.MY_SHELL_VAR4"),
        };
        assert!(failed);
    }

    #[test]
    fn vars_resolver_known_simple_var() {
        let resolved = vars_resolver().resolve("${foo.bar}").unwrap();
        assert_eq!(resolved, Cow::Borrowed("baz"));
    }

    #[test]
    fn vars_resolver_unknown_simple_var() {
        let resolved = vars_resolver().resolve("${foo.buz}");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "foo.buz"),
        };
        assert!(failed);
    }
}
