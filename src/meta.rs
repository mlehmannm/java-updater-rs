//! Installation metadata.
//!
//! This module contains the installation metadata read from a file within the installation directory.

use anyhow::Result;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use tracing::instrument;

/// Name of the metadata directory within the installation directory.
pub(crate) const METADATA_DIR: &str = ".java-updater";

/// Name of the metadata file within the metadata directory.
pub(crate) const METADATA_FILE: &str = "meta";

/// Struct to hold the metadata for an installation.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Metadata {
    /// The checksum of the downloaded package
    pub(crate) checksum: String,
    /// Additional properties
    #[serde(default, skip_serializing_if = "default")]
    pub(crate) props: HashMap<String, String>,
    /// The vendor of the installation (Azul, Eclipse, etc.)
    pub(crate) vendor: String,
    /// The version of the installation
    pub(crate) version: Version,
}

// Helper to determine if the given field has the default value.
#[doc(hidden)]
fn default<F: Default + PartialEq>(f: &F) -> bool {
    *f == F::default()
}

impl Metadata {
    /// Creates a new `Metadata`.
    pub(crate) fn new(vendor: impl Into<String>, version: Version, checksum: impl Into<String>) -> Self {
        Self {
            checksum: checksum.into(),
            props: HashMap::new(),
            vendor: vendor.into(),
            version,
        }
    }

    /// Loads the `Metadata` from the given filename.
    #[instrument(err(level = "trace"), level = "trace")]
    pub(crate) fn load<P>(filename: P) -> Result<Self>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let metadata_file = File::open(filename)?;

        let de = serde_yaml::Deserializer::from_reader(metadata_file);
        let value = serde_yaml::Value::deserialize(de)?;
        let metadata: Metadata = serde_yaml::from_value(value)?;

        Ok(metadata)
    }

    /// Saves the `Metadata` to the given filename.
    #[instrument(err(level = "trace"), level = "trace")]
    pub(crate) fn save<P>(&self, filename: P) -> Result<()>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        let metadata_file = File::create(filename)?;
        let mut metadata_ser = serde_yaml::Serializer::new(&metadata_file);
        self.serialize(&mut metadata_ser)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tempfile::tempdir;
    use test_log::test;

    #[test]
    fn save_and_load() {
        // prepare
        let tempdir = tempdir().unwrap();
        let dir = tempdir.path();
        let file = dir.join(METADATA_FILE);

        // test
        let mut md = Metadata::new("whatever", Version::parse("1.2.3").unwrap(), "abcd".to_string());
        md.props.insert("k".to_string(), "v".to_string());
        md.save(&file).unwrap();
        let md_loaded = Metadata::load(&file).unwrap();
        assert_eq!(md, md_loaded);
    }
}
