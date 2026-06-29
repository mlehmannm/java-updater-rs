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
use std::rc::Rc;

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
    #[tracing::instrument(err, level = "trace")]
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
    pub(crate) fn expand_directory(config: &Rc<Self>) -> String {
        // setup variable resolver(s) and expander
        let env_var_resolver = PrefixedVarResolver::new("env.", Rc::new(OsEnvVarResolver));
        let var_resolvers: Vec<Rc<dyn VarResolver>> = vec![config.clone(), Rc::new(env_var_resolver), Rc::new(RustEnvVarResolver), Rc::new(AsIsVarResolver)];
        let var_expander = VarExpander::new(var_resolvers);

        // expand all known variables and leave unknown variables as-is
        let directory = &config.directory;
        var_expander.expand(directory).unwrap_or(Cow::Borrowed(directory)).to_string()
    }
}

impl VarResolver for InstallationConfig {
    fn resolve_var(&self, var_name: &str) -> Result<String, VarError> {
        let value = match var_name {
            "JU_CONFIG_ARCH" => &self.architecture,
            "JU_CONFIG_DIRECTORY" => &self.directory,
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
    use std::env;
    use std::path::Path;
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
    fn expand_directory() {
        let architecture = env::consts::ARCH.to_string();
        let os = env::consts::OS.to_string();
        let directory = "${JU_CONFIG_ARCH}/${JU_CONFIG_TYPE}/${JU_CONFIG_VENDOR}/${JU_CONFIG_VERSION}/${JU_OS}/${JU_UNSUPPORTED}".to_string();
        let config = InstallationConfig {
            architecture: architecture.clone(),
            directory: directory.clone(),
            package_type: "jdk".to_string(),
            vendor: "eclipse".to_string(),
            version: "17".to_string(),
            ..Default::default()
        };
        let config = Rc::new(config);
        let actual = InstallationConfig::expand_directory(&config);
        let expected = format!("{architecture}/jdk/eclipse/17/{os}/${{JU_UNSUPPORTED}}");
        assert_eq!(expected, actual);
    }

    #[test]
    fn resolve_vars() {
        let architecture = env::consts::ARCH.to_string();
        let directory = "${JU_CONFIG_ARCH}/${JU_CONFIG_TYPE}/${JU_CONFIG_VENDOR}/${JU_CONFIG_VERSION}/${JU_OS}".to_string();
        let package_type = "jdk".to_string();
        let vendor = "eclipse".to_string();
        let version = "17".to_string();
        let config = InstallationConfig {
            architecture: architecture.clone(),
            directory: directory.clone(),
            package_type: package_type.clone(),
            vendor: vendor.clone(),
            version: version.clone(),
            ..Default::default()
        };
        assert_eq!(Ok(architecture), config.resolve_var("JU_CONFIG_ARCH"));
        assert_eq!(Ok(package_type), config.resolve_var("JU_CONFIG_TYPE"));
        assert_eq!(Ok(directory), config.resolve_var("JU_CONFIG_DIRECTORY"));
        assert_eq!(Ok(vendor), config.resolve_var("JU_CONFIG_VENDOR"));
        assert_eq!(Ok(version), config.resolve_var("JU_CONFIG_VERSION"));
    }

    #[test]
    fn expand_vars() {
        let architecture = env::consts::ARCH.to_string();
        let os = env::consts::OS.to_string();
        let directory = "${JU_CONFIG_ARCH}/${JU_CONFIG_TYPE}/${JU_CONFIG_VENDOR}/${JU_CONFIG_VERSION}/${JU_OS}".to_string();
        let package_type = "jdk".to_string();
        let vendor = "eclipse".to_string();
        let version = "17".to_string();
        let expanded_directory = format!("{architecture}/{package_type}/{vendor}/{version}/{os}");
        let config = InstallationConfig {
            architecture: architecture.clone(),
            directory: directory.clone(),
            package_type: package_type.clone(),
            vendor: vendor.clone(),
            version: version.clone(),
            ..Default::default()
        };

        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("JU_OS", os);
        let var_resolvers: Vec<Rc<dyn VarResolver>> = vec![Rc::new(simple_var_resolver), Rc::new(config)];
        let var_expander = VarExpander::new(var_resolvers);

        assert_eq!(Ok(architecture), var_expander.expand("${JU_CONFIG_ARCH}").map(Cow::into_owned));
        assert_eq!(Ok(package_type), var_expander.expand("${JU_CONFIG_TYPE}").map(Cow::into_owned));
        assert_eq!(Ok(expanded_directory), var_expander.expand("${JU_CONFIG_DIRECTORY}").map(Cow::into_owned));
        assert_eq!(Ok(vendor), var_expander.expand("${JU_CONFIG_VENDOR}").map(Cow::into_owned));
        assert_eq!(Ok(version), var_expander.expand("${JU_CONFIG_VERSION}").map(Cow::into_owned));
    }

    #[cfg(feature = "notify")]
    #[derive(Debug, PartialEq)]
    struct FixtureCommand {
        args: Vec<String>,
        directory: Option<String>,
        path: String,
    }

    #[cfg(feature = "notify")]
    #[derive(Debug, PartialEq)]
    struct FixtureInstallation {
        architecture: String,
        directory: String,
        enabled: bool,
        expanded_directory: String,
        on_failure: Vec<FixtureCommand>,
        on_success: Vec<FixtureCommand>,
        on_update: Vec<FixtureCommand>,
        package_type: String,
        vendor: String,
        version: String,
    }

    #[cfg(feature = "notify")]
    fn fixture_command(command: &NotifyCommandConfig) -> FixtureCommand {
        FixtureCommand {
            args: command.args.clone(),
            directory: command.directory.clone(),
            path: command.path.clone(),
        }
    }

    #[cfg(feature = "notify")]
    fn notify_failure_command(path: &str, directory: Option<&str>) -> FixtureCommand {
        FixtureCommand {
            args: vec![
                "failed".to_string(),
                "${JU_ERROR}\n${JU_DIRECTORY} [${JU_VENDOR_NAME} (${JU_VENDOR_ID})]".to_string(),
            ],
            directory: directory.map(str::to_string),
            path: path.to_string(),
        }
    }

    #[cfg(feature = "notify")]
    fn notify_update_command(path: &str, directory: Option<&str>) -> FixtureCommand {
        FixtureCommand {
            args: vec![
                "updated".to_string(),
                "${JU_OLD_VERSION:-n/a} -> ${JU_NEW_VERSION}\n${JU_DIRECTORY} [${JU_VENDOR_NAME} (${JU_VENDOR_ID})]".to_string(),
            ],
            directory: directory.map(str::to_string),
            path: path.to_string(),
        }
    }

    #[cfg(feature = "notify")]
    fn powershell_command(script: &str) -> FixtureCommand {
        FixtureCommand {
            args: vec![script.to_string()],
            directory: None,
            path: "powershell.exe".to_string(),
        }
    }

    #[cfg(feature = "notify")]
    fn fixture_installations(filename: &str) -> Vec<FixtureInstallation> {
        let filename = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join(filename);
        let config = Config::load_from_file(filename).unwrap();
        config
            .installations
            .into_iter()
            .map(|config| {
                let config = Rc::new(config);
                let expanded_directory = InstallationConfig::expand_directory(&config);
                FixtureInstallation {
                    architecture: config.architecture.clone(),
                    directory: config.directory.clone(),
                    enabled: config.enabled,
                    expanded_directory,
                    on_failure: config.on_failure.iter().map(fixture_command).collect(),
                    on_success: config.on_success.iter().map(fixture_command).collect(),
                    on_update: config.on_update.iter().map(fixture_command).collect(),
                    package_type: config.package_type.clone(),
                    vendor: config.vendor.clone(),
                    version: config.version.clone(),
                }
            })
            .collect()
    }

    #[test]
    #[cfg(feature = "notify")]
    fn linux_fixture_contents() {
        let architecture = env::consts::ARCH;
        let notify_failure = notify_failure_command("notify-send", None);
        let notify_update = notify_update_command("notify-send", None);
        let expected = vec![
            FixtureInstallation {
                architecture: "i686".to_string(),
                directory: "tmp/${JU_CONFIG_VENDOR}/x86/${JU_CONFIG_VERSION}".to_string(),
                enabled: true,
                expanded_directory: "tmp/azul/x86/8".to_string(),
                on_failure: vec![notify_failure_command("notify-send", None)],
                on_success: vec![],
                on_update: vec![notify_update_command("notify-send", None)],
                package_type: "jdk".to_string(),
                vendor: "azul".to_string(),
                version: "8".to_string(),
            },
            FixtureInstallation {
                architecture: architecture.to_string(),
                directory: "tmp/${JU_CONFIG_VENDOR}/${JU_CONFIG_ARCH}/${JU_CONFIG_VERSION}".to_string(),
                enabled: true,
                expanded_directory: format!("tmp/eclipse/{architecture}/17"),
                on_failure: vec![notify_failure],
                on_success: vec![],
                on_update: vec![notify_update],
                package_type: "jdk".to_string(),
                vendor: "eclipse".to_string(),
                version: "17".to_string(),
            },
        ];
        assert_eq!(expected, fixture_installations("java-updater-linux.yml"));
    }

    #[test]
    #[cfg(feature = "notify")]
    fn windows_fixture_contents() {
        let architecture = env::consts::ARCH;
        let notify_send = r"D:\dev\src\notify-send\notify-send.exe";
        let notify_dir = Some("${env.windir}");
        let expected = vec![
            FixtureInstallation {
                architecture: "i686".to_string(),
                directory: "tmp/${JU_CONFIG_VENDOR}/x86/${JU_CONFIG_VERSION}".to_string(),
                enabled: true,
                expanded_directory: "tmp/azul/x86/8".to_string(),
                on_failure: vec![notify_failure_command(notify_send, notify_dir)],
                on_success: vec![],
                on_update: vec![notify_update_command(notify_send, notify_dir)],
                package_type: "jdk".to_string(),
                vendor: "azul".to_string(),
                version: "8".to_string(),
            },
            FixtureInstallation {
                architecture: architecture.to_string(),
                directory: "tmp/${JU_CONFIG_VENDOR}/${JU_CONFIG_ARCH}/${JU_CONFIG_VERSION}".to_string(),
                enabled: true,
                expanded_directory: format!("tmp/eclipse/{architecture}/17"),
                on_failure: vec![notify_failure_command(notify_send, notify_dir)],
                on_success: vec![
                    powershell_command("[System.Environment]::SetEnvironmentVariable('JAVA_HOME_${JU_CONFIG_VERSION}', '${JU_DIRECTORY}', 'User')"),
                    powershell_command("[System.Environment]::SetEnvironmentVariable('JAVA_HOME', '${JU_DIRECTORY}', 'User')"),
                ],
                on_update: vec![notify_update_command(notify_send, notify_dir)],
                package_type: "jdk".to_string(),
                vendor: "eclipse".to_string(),
                version: "17".to_string(),
            },
            FixtureInstallation {
                architecture: architecture.to_string(),
                directory: "tmp/${JU_CONFIG_VENDOR}/${JU_CONFIG_ARCH}/${JU_CONFIG_VERSION}".to_string(),
                enabled: true,
                expanded_directory: format!("tmp/eclipse/{architecture}/21"),
                on_failure: vec![notify_failure_command(notify_send, notify_dir)],
                on_success: vec![powershell_command(
                    "[System.Environment]::SetEnvironmentVariable('JAVA_HOME_${JU_CONFIG_VERSION}', '${JU_DIRECTORY}', 'User')",
                )],
                on_update: vec![notify_update_command(notify_send, notify_dir)],
                package_type: "jdk".to_string(),
                vendor: "eclipse".to_string(),
                version: "21".to_string(),
            },
        ];
        assert_eq!(expected, fixture_installations("java-updater-windows.yml"));
    }
}
