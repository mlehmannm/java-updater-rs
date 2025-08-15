//! Version.
//!
//! This module contains the version information.

/// Structure to hold the version information.
#[derive(Debug)]
pub(crate) struct Version {
    /// The name of the package.
    pub(crate) pkg_name: String,
    /// The version of the package.
    #[expect(clippy::struct_field_names)]
    pub(crate) pkg_version: String,
    /// The value that `git describe` returned.
    pub(crate) git_describe: String,
    /// The version of the rust compiler.
    pub(crate) rustc_semver: String,
}

impl Default for Version {
    fn default() -> Self {
        let cargo_pkg_name = env!("CARGO_PKG_NAME");
        let cargo_pkg_version = env!("CARGO_PKG_VERSION");
        let vergen_git_describe = env!("VERGEN_GIT_DESCRIBE");
        let vergen_rustc_semver = env!("VERGEN_RUSTC_SEMVER");

        Self {
            git_describe: vergen_git_describe.to_string(),
            pkg_name: cargo_pkg_name.to_string(),
            pkg_version: cargo_pkg_version.to_string(),
            rustc_semver: vergen_rustc_semver.to_string(),
        }
    }
}

/// Display this Version.
impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

/// String conversion.
impl From<&Version> for String {
    fn from(value: &Version) -> String {
        let git_describe = &value.git_describe;
        let pkg_name = &value.pkg_name;
        let pkg_version = &value.pkg_version;
        let rustc_semver = &value.rustc_semver;
        format!("{pkg_name} {pkg_version} (git/{git_describe}) (rustc/{rustc_semver})")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn default() {
        let version = Version::default();
        assert_eq!(version.git_describe, env!("VERGEN_GIT_DESCRIBE"));
        assert_eq!(version.pkg_name, env!("CARGO_PKG_NAME"));
        assert_eq!(version.pkg_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(version.rustc_semver, env!("VERGEN_RUSTC_SEMVER"));
    }
}
