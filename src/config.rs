//! Configuration.
//!
//! This module contains the configuration read from a YAML file.

use crate::vars::*;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
use std::borrow::Cow;
use std::env;
use std::fmt;
use std::fs::File;
use std::marker::PhantomData;
use std::path::Path;
use tracing::instrument;

/// Name of the default configuration file.
pub(crate) const CONFIG_FILENAME: &str = "java-updater.yml";

/// The struct that holds the configuration loaded from a YAML file.
#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    /// List with installation configurations.
    #[serde(default)]
    pub(crate) installations: Vec<InstallationConfig>,
}

impl Config {
    /// Loads the configuration from the given filename.
    #[instrument(err, level = "trace")]
    pub(crate) fn load_from_file<P>(filename: P) -> anyhow::Result<Self>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let config_file = File::open(filename)?;

        let de = serde_yaml::Deserializer::from_reader(config_file);
        let value = serde_yaml::Value::deserialize(de)?;
        let config: Config = serde_yaml::from_value(value)?;

        Ok(config)
    }
}

/// The configuration for an installation.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallationConfig {
    /// The architecture of the installation.
    #[serde(default = "installation_architecture_default")]
    pub(crate) architecture: String,
    /// The directory of the installation.
    pub(crate) directory: String,
    /// Whether the installation is enabled.
    #[serde(default = "installation_enabled_default")]
    pub(crate) enabled: bool,
    /// The package type of the installation (JDK or JRE).
    #[serde(rename = "type")]
    pub(crate) package_type: String,
    /// The vendor of the installation (Azul, Eclipse, etc.)
    pub(crate) vendor: String,
    /// The major version of the installation (17, 21, etc.)
    #[serde(deserialize_with = "installation_version_deser")]
    pub(crate) version: String,
    /// The command(s) executed on failure.
    #[cfg(feature = "notify")]
    #[serde(default, rename = "on-failure")]
    pub(crate) on_failure: Vec<NotifyCommandConfig>,
    /// The command(s) executed on success.
    #[cfg(feature = "notify")]
    #[serde(default, rename = "on-success")]
    pub(crate) on_success: Vec<NotifyCommandConfig>,
    /// The command(s) executed on update.
    #[cfg(feature = "notify")]
    #[serde(default, rename = "on-update")]
    pub(crate) on_update: Vec<NotifyCommandConfig>,
}

// Returns the default value for [InstallationConfig::architecture].
#[doc(hidden)]
#[inline]
fn installation_architecture_default() -> String {
    env::consts::ARCH.to_string()
}

// Returns the default value for [InstallationConfig::enabled].
#[doc(hidden)]
#[inline]
fn installation_enabled_default() -> bool {
    true
}

// Deserializes the field [InstallationConfig::version] from either unsigned integer or string.
// see https://serde.rs/string-or-struct.html
#[doc(hidden)]
fn installation_version_deser<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct UintOrString(PhantomData<fn() -> String>);

    impl Visitor<'_> for UintOrString {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("unsigned integer or string")
        }

        fn visit_str<E>(self, value: &str) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }

        fn visit_u64<E>(self, value: u64) -> Result<String, E>
        where
            E: de::Error,
        {
            Ok(value.to_string())
        }
    }

    deserializer.deserialize_any(UintOrString(PhantomData))
}

impl InstallationConfig {
    /// Returns [`Installation::directory`] where all known variables are expanded.
    pub(crate) fn expand_directory(&self) -> String {
        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("JU_ARCH", env::consts::ARCH.to_string());
        simple_var_resolver.insert("JU_CONFIG_ARCH", self.architecture.to_string());
        simple_var_resolver.insert("JU_CONFIG_TYPE", self.package_type.to_string());
        simple_var_resolver.insert("JU_CONFIG_VENDOR", self.vendor.to_string());
        simple_var_resolver.insert("JU_CONFIG_VERSION", self.version.to_string());
        simple_var_resolver.insert("JU_OS", env::consts::OS.to_string());
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(EnvVarResolver), Box::new(AsIsVarResolver)];
        let vars_resolver = VarExpander::new(var_resolvers);

        // expand all known variables and leave unknown variables as-is
        let directory = &self.directory;
        vars_resolver.expand(directory).unwrap_or(Cow::Borrowed(directory)).to_string()
    }
}

impl VarResolver for InstallationConfig {
    fn resolve_var(&self, var_name: &str) -> Result<String, VarError> {
        let value = match var_name {
            "JU_CONFIG_ARCH" => &self.architecture,
            "JU_CONFIG_TYPE" => &self.package_type,
            "JU_CONFIG_VENDOR" => &self.vendor,
            "JU_CONFIG_VERSION" => &self.version,
            _ => return Err(VarError::NotPresent(var_name.to_owned())),
        };

        Ok(value.clone())
    }
}

/// The configuration for a notify command.
#[cfg(feature = "notify")]
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NotifyCommandConfig {
    /// The path to the executable.
    pub(crate) path: String,
    /// The arguments for the executable.
    #[serde(default)]
    pub(crate) args: Vec<String>,
    /// The optional working directory for the executable.
    pub(crate) directory: Option<String>,
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn parse_version_as_uint() {
        let config = r"
          vendor: azul
          architecture: i686
          directory: tmp/azul/x86/8
          type: jdk
          version: 8
        ";
        let config: InstallationConfig = serde_yaml::from_str(config).unwrap();
        assert_eq!("8", config.version);
    }

    #[test]
    fn parse_version_as_string() {
        let config = r#"
          vendor: azul
          architecture: i686
          directory: tmp/azul/x86/8
          type: jdk
          version: "8"
        "#;
        let config: InstallationConfig = serde_yaml::from_str(config).unwrap();
        assert_eq!("8", config.version);
    }

    #[test]
    fn resolve_directory() {
        let architecture = env::consts::ARCH.to_string();
        let os = env::consts::OS.to_string();
        let config = InstallationConfig {
            architecture: architecture.clone(),
            directory: "${JU_CONFIG_ARCH}/\
                        ${JU_CONFIG_TYPE}/\
                        ${JU_CONFIG_VENDOR}/\
                        ${JU_CONFIG_VERSION}/\
                        ${JU_OS}/\
                        ${JU_NOT_SUPPORTED}"
                .to_string(),
            package_type: "jdk".to_string(),
            vendor: "eclipse".to_string(),
            version: "8".to_string(),
            ..Default::default()
        };
        let actual = config.expand_directory();
        let expected = format!("{architecture}/jdk/eclipse/8/{os}/${{JU_NOT_SUPPORTED}}");
        assert_eq!(expected, actual);
    }
}
