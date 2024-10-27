//! Configuration.
//!
//! This module contains the configuration read from a YAML file.

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer};
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
    /// The command(s) executed on each run.
    #[cfg(feature = "notify")]
    #[serde(default, rename = "on-run")]
    pub(crate) on_run: Vec<NotifyCommandConfig>,
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

    impl<'de> Visitor<'de> for UintOrString {
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
    fn installation_version_as_uint() {
        let config = r"
          vendor: azul
          directory: tmp/azul/x86/8
          type: jdk
          architecture: i686
          version: 8
        ";
        let config: InstallationConfig = serde_yaml::from_str(config).unwrap();
        assert_eq!("8", config.version);
    }

    #[test]
    fn installation_version_as_string() {
        let config = r#"
          vendor: azul
          directory: tmp/azul/x86/8
          type: jdk
          architecture: i686
          version: "8"
        "#;
        let config: InstallationConfig = serde_yaml::from_str(config).unwrap();
        assert_eq!("8", config.version);
    }
}
