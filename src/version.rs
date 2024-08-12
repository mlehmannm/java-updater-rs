//! Version.
//!
//! This module contains the version information.

use std::fmt;

/// Structure to hold the version information.
#[derive(Debug)]
pub(crate) struct Version {
    /// The name of the package.
    pub(crate) pkg_name: String,
    /// The version of the package.
    pub(crate) pkg_version: String,
    /// The value that `git describe` returned.
    pub(crate) git_describe: String,
    /// The version of the rust compiler.
    pub(crate) rustc_semver: String,
}

impl Default for Version {
    fn default() -> Self {
        let pkg_name = env!("CARGO_PKG_NAME");
        let pkg_version = env!("CARGO_PKG_VERSION");
        let git_describe = env!("VERGEN_GIT_DESCRIBE");
        let rustc_semver = env!("VERGEN_RUSTC_SEMVER");

        Self {
            pkg_name: pkg_name.to_string(),
            pkg_version: pkg_version.to_string(),
            git_describe: git_describe.to_string(),
            rustc_semver: rustc_semver.to_string(),
        }
    }
}

/// Display this Version.
impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

/// String conversion.
impl From<&Version> for String {
    fn from(value: &Version) -> String {
        let pkg_name = &value.pkg_name;
        let pkg_version = &value.pkg_version;
        let git_describe = &value.git_describe;
        let rustc_semver = &value.rustc_semver;
        format!("{pkg_name} {pkg_version} (git/{git_describe}) (rustc/{rustc_semver})")
    }
}
