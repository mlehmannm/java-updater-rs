//! Variable resolvers.
//!
//! This module contains basic support for variable resolvers and an implementation of the same to resolve environment variables.

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::rc::Rc;

/// The error type for operations interacting with variables.
#[cfg_attr(test, derive(PartialEq))]
#[derive(Debug, thiserror::Error)]
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
    #[tracing::instrument(level = "trace", ret)]
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        for resolver in &self.resolvers {
            if let Ok(value) = resolver.resolve_var(v) {
                return Ok(value);
            }
        }

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// [`VarResolver`] implementation for environment variables from the operationg system.
#[derive(Debug)]
pub(crate) struct OsEnvVarResolver;

impl VarResolver for OsEnvVarResolver {
    #[tracing::instrument(level = "trace", ret)]
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        if let Ok(val) = env::var(v) {
            return Ok(val);
        }

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// [`VarResolver`] that first removes the given prefix from the variable and then delegates to the given resolver.
#[derive(Debug)]
pub(crate) struct PrefixedVarResolver {
    resolver: Rc<dyn VarResolver>,
    prefix: String,
}

impl PrefixedVarResolver {
    /// Constructs a new `PrefixedVarResolver` for the given variable resolver.
    pub(crate) fn new(prefix: impl Into<String>, resolver: Rc<dyn VarResolver>) -> Self {
        Self {
            prefix: prefix.into(),
            resolver,
        }
    }
}

impl VarResolver for PrefixedVarResolver {
    #[tracing::instrument(level = "trace", ret)]
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        if let Some(v) = v.strip_prefix(&self.prefix) {
            return self.resolver.resolve_var(v);
        };

        Err(VarError::NotPresent(v.to_owned()))
    }
}

/// [`VarResolver`] implementation for Rust environment constants.
#[derive(Debug)]
pub(crate) struct RustEnvVarResolver;

impl VarResolver for RustEnvVarResolver {
    #[tracing::instrument(level = "trace", ret)]
    fn resolve_var(&self, v: &str) -> Result<String, VarError> {
        match v {
            "JU_ARCH" => Ok(env::consts::ARCH.to_string()),
            "JU_FAMILY" => Ok(env::consts::FAMILY.to_string()),
            "JU_OS" => Ok(env::consts::OS.to_string()),
            _ => Err(VarError::NotPresent(v.to_owned())),
        }
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
    #[tracing::instrument(level = "trace", ret)]
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
    resolver: CombinedVarResolver,
}

impl VarExpander {
    /// Constructs a new `VarExpander` with the given variable resolvers.
    pub(crate) fn new<I>(resolvers: I) -> Self
    where
        I: IntoIterator<Item = Rc<dyn VarResolver>>,
    {
        Self {
            resolver: CombinedVarResolver::new(resolvers),
        }
    }

    /// Expands all known variables in the given string.
    #[tracing::instrument(level = "trace", ret)]
    pub(crate) fn expand<'a, S>(&self, s: &'a S) -> Result<Cow<'a, str>, VarError>
    where
        S: ?Sized + AsRef<str> + fmt::Debug,
    {
        self.expand_inner(s.as_ref()).map(Cow::Owned)
    }

    // Expands all known variables in the given string.
    fn expand_inner(&self, s: &str) -> Result<String, VarError> {
        let expanded = shellexpand::env_with_context(s, |s| self.resolve(s)) //
            .map_err(|err| err.cause)? //
            .to_string();

        if expanded == s {
            return Ok(expanded);
        }

        self.expand_inner(&expanded)
    }

    // Provides the context for `expand`.
    #[doc(hidden)]
    fn resolve(&self, v: &str) -> Result<Option<String>, VarError> {
        self.resolver.resolve_var(v).map(Option::Some)
    }
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
        let resolver = OsEnvVarResolver;
        let resolved = resolver.resolve_var("MY_SHELL_VAR1").unwrap();
        assert_eq!(resolved, "MY_SHELL_VAL");
    }

    #[test]
    fn env_var_resolver_unknown_var() {
        env::remove_var("MY_SHELL_VAR2");
        let resolver = OsEnvVarResolver;
        let resolved = resolver.resolve_var("env.MY_SHELL_VAR2");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "env.MY_SHELL_VAR2"),
        };
        assert!(failed);
    }

    #[test]
    fn rust_env_var_resolver_arch() {
        let resolver = RustEnvVarResolver;
        let resolved = resolver.resolve_var("JU_ARCH").unwrap();
        assert_eq!(resolved, env::consts::ARCH);
    }

    #[test]
    fn rust_env_var_resolver_family() {
        let resolver = RustEnvVarResolver;
        let resolved = resolver.resolve_var("JU_FAMILY").unwrap();
        assert_eq!(resolved, env::consts::FAMILY);
    }

    #[test]
    fn rust_env_var_resolver_os() {
        let resolver = RustEnvVarResolver;
        let resolved = resolver.resolve_var("JU_OS").unwrap();
        assert_eq!(resolved, env::consts::OS);
    }

    #[test]
    fn simple_var_resolver_known_var() {
        let mut resolver = SimpleVarResolver::new();
        resolver.insert("foo.bar", "baz");
        let resolved = resolver.resolve_var("foo.bar").unwrap();
        assert_eq!(resolved, "baz");
    }

    #[test]
    fn simple_var_resolver_unknown_var() {
        let mut resolver = SimpleVarResolver::new();
        resolver.insert("foo.bar", "baz");
        let resolved = resolver.resolve_var("foo.buz");
        let failed = match resolved {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "foo.buz"),
        };
        assert!(failed);
    }

    #[allow(clippy::similar_names)]
    fn var_expander() -> VarExpander {
        let mut foo_resolver = SimpleVarResolver::new();
        foo_resolver.insert("foo", "bar");
        let mut baz_resolver = SimpleVarResolver::new();
        baz_resolver.insert("baz", "${foo}");
        let mut buz_resolver = SimpleVarResolver::new();
        buz_resolver.insert("buz", "${buz}");
        let env_var_resolver = PrefixedVarResolver::new("env.", Rc::new(OsEnvVarResolver));
        let var_resolvers: [Rc<dyn VarResolver>; 4] = [Rc::new(foo_resolver), Rc::new(baz_resolver), Rc::new(buz_resolver), Rc::new(env_var_resolver)];
        VarExpander::new(var_resolvers)
    }

    #[test]
    fn var_expander_known_env_var() {
        env::set_var("MY_SHELL_VAR3", "MY_SHELL_VAL");
        let expanded = var_expander().expand("${env.MY_SHELL_VAR3}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("MY_SHELL_VAL"));
    }

    #[test]
    fn var_expander_unknown_env_var() {
        env::remove_var("MY_SHELL_VAR4");
        let expanded = var_expander().expand("${env.MY_SHELL_VAR4}");
        let failed = match expanded {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "env.MY_SHELL_VAR4"),
        };
        assert!(failed);
    }

    #[test]
    fn var_expander_known_simple_var() {
        let expanded = var_expander().expand("${foo}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("bar"));
    }

    #[test]
    fn var_expander_known_simple_var_buz() {
        let expanded = var_expander().expand("${buz}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("${buz}"));
    }

    #[test]
    fn var_expander_known_simple_var_nested() {
        let expanded = var_expander().expand("${baz}").unwrap();
        assert_eq!(expanded, Cow::Borrowed("bar"));
    }

    #[test]
    fn var_expander_unknown_simple_var() {
        let expanded = var_expander().expand("${xyz}");
        let failed = match expanded {
            Ok(_) => false,
            Err(err) => matches!(err, VarError::NotPresent(name) if name == "xyz"),
        };
        assert!(failed);
    }
}
