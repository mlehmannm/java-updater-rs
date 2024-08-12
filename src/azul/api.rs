use super::*;
use anyhow::anyhow;
use reqwest::Url;
use semver::Version;
use serde::Deserialize;
use std::env;
use tracing::trace;

/// The request to retrieve the metadata.
pub(super) struct MetadataRequest {
    pub(super) arch: String,
    pub(super) os: String,
    pub(super) package_type: String,
    pub(super) version: String,
}

impl MetadataRequest {
    // Query the Metadata API for all relevant data.
    pub(super) fn query(&self) -> anyhow::Result<MetadataResponse> {
        let (version, url, uuid) = self.query_packages()?;
        let checksum = Self::query_packages_uuid(&uuid)?;

        Ok(MetadataResponse { checksum, url, version })
    }

    // Query the Metadata API for the package that fulfills the parameter.
    fn query_packages(&self) -> anyhow::Result<(Version, String, String)> {
        let url = self.packages_query_url()?;
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url) //
            .header(reqwest::header::ACCEPT, "application/json") //
            .send()?;
        let bytes = response.bytes()?;
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let response: serde_json::Value = Deserialize::deserialize(&mut de)?;
        trace!("packages response = {response:#?}");

        // check structure of response (1)
        let Some(response) = response.as_array() else {
            return Err(anyhow!("response has not the expected structure"));
        };
        // check structure of response (2)
        let response = if [1, 2].contains(&response.len()) {
            &response[0]
        } else {
            return Err(anyhow!("response is ambiguous {}", response.len()));
        };

        // TODO check that the response corresponds to the request (the query for x86 returns packages for x64 too)

        // url

        let Some(url) = response["download_url"].as_str() else {
            return Err(anyhow!("field 'download_url' not present in response"));
        };

        // version

        let Some(version) = response["java_version"].as_array() else {
            return Err(anyhow!("field 'java_version' not present in response"));
        };
        let Some(major) = version[0].as_u64() else {
            return Err(anyhow!("major part not present in 'java_version'"));
        };
        let Some(minor) = version[1].as_u64() else {
            return Err(anyhow!("minor part not present in 'java_version'"));
        };
        let Some(patch) = version[2].as_u64() else {
            return Err(anyhow!("patch part not present in 'java_version'"));
        };
        let version = Version::new(major, minor, patch);

        // uuid

        let Some(uuid) = response["package_uuid"].as_str() else {
            return Err(anyhow!("field 'package_uuid' not present in response"));
        };

        Ok((version, url.to_string(), uuid.to_string()))
    }

    // Build the query URL to search for packages.
    fn packages_query_url(&self) -> anyhow::Result<Url> {
        let mut url = Url::parse(API_URL)?;
        url.query_pairs_mut()
            .append_pair("arch", &self.arch())
            .append_pair("archive_type", ARCHIVE_TYPE)
            .append_pair("java_version", &self.version())
            .append_pair("java_package_type", &self.package_type())
            .append_pair("os", &self.os()) //
            .append_pair("javafx_bundled", "true")
            .append_pair("latest", "true")
            .append_pair("release_status", "ga");

        Ok(url)
    }

    // Query the Metadata API for details for the package.
    fn query_packages_uuid(uuid: &str) -> anyhow::Result<String> {
        let url = Self::packages_uuid_query_url(uuid)?;
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url) //
            .header(reqwest::header::ACCEPT, "application/json") //
            .send()?;
        let bytes = response.bytes()?;
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let response: serde_json::Value = Deserialize::deserialize(&mut de)?;
        // checksum
        let Some(checksum) = response["sha256_hash"].as_str() else {
            return Err(anyhow!("field 'sha256_hash' not present in response"));
        };

        Ok(checksum.to_string())
    }

    // Build the query URL to get the package details.
    fn packages_uuid_query_url(uuid: &str) -> anyhow::Result<Url> {
        let url = Url::parse(API_URL)?;
        let url = url.join(uuid)?;

        Ok(url)
    }

    // Returns the requested architecture for the package.
    fn arch(&self) -> String {
        let arch = self.arch.trim();
        if arch.is_empty() {
            env::consts::ARCH.to_string()
        } else {
            arch.to_lowercase()
        }
    }

    // Returns the requested operating system for the package.
    fn os(&self) -> String {
        let os = self.os.trim();
        if os.is_empty() {
            env::consts::OS.to_string()
        } else {
            os.to_lowercase()
        }
    }

    // Returns the requested type for the package.
    fn package_type(&self) -> String {
        let package_type = self.package_type.trim();
        if package_type.is_empty() {
            return "jdk".to_string(); // default to JDK
        }

        let package_type = package_type.to_lowercase();
        match package_type.as_str() {
            "jdk" | "jre" => package_type,
            _ => "jdk".to_string(), // default to JDK
        }
    }

    // Returns the requested (major) version for the package.
    fn version(&self) -> String {
        let version = self.version.trim();
        if version.is_empty() {
            "17".to_string()
        } else {
            version.to_lowercase()
        }
    }
}

/// The response to the [MetadataRequest].
pub(super) struct MetadataResponse {
    pub(super) checksum: String,
    pub(super) url: String,
    pub(super) version: Version,
}
