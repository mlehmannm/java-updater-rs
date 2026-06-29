//! Version.
//!
//! This module contains the version information.

/// Structure to hold the version information.
#[derive(Debug)]
#[expect(clippy::struct_field_names)]
pub(crate) struct Version {
    /// The dirty state of the Git repository during build.
    pub(crate) git_dirty: bool,
    /// The commit SHA of the Git repository during build.
    pub(crate) git_sha: String,
    /// The name of the package.
    pub(crate) pkg_name: String,
    /// The version of the package.
    pub(crate) pkg_version: String,
    /// The version of the rust compiler use to compile the package.
    pub(crate) rustc_semver: String,
}

impl Default for Version {
    fn default() -> Self {
        let git_dirty = env!("VERGEN_GIT_DIRTY");
        Self {
            git_dirty: git_dirty == "true",
            git_sha: env!("VERGEN_GIT_SHA").to_string(),
            pkg_name: env!("CARGO_PKG_NAME").to_string(),
            pkg_version: env!("CARGO_PKG_VERSION").to_string(),
            rustc_semver: env!("VERGEN_RUSTC_SEMVER").to_string(),
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
        let git_dirty_suffix = if value.git_dirty { "-dirty" } else { "" };
        let git_sha = &value.git_sha;
        let pkg_name = &value.pkg_name;
        let pkg_version = &value.pkg_version;
        let rustc_semver = &value.rustc_semver;
        format!("{pkg_name} {pkg_version} (git/{git_sha}{git_dirty_suffix}) (rustc/{rustc_semver})")
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn default() {
        let version = Version::default();
        assert_eq!(version.git_dirty, env!("VERGEN_GIT_DIRTY") == "true");
        assert_eq!(version.git_sha, env!("VERGEN_GIT_SHA"));
        assert_eq!(version.pkg_name, env!("CARGO_PKG_NAME"));
        assert_eq!(version.pkg_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(version.rustc_semver, env!("VERGEN_RUSTC_SEMVER"));
    }
}
