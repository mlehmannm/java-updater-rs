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
    // Query the API for all relevant data.
    pub(super) fn query(&self) -> anyhow::Result<MetadataResponse> {
        let url = self.query_url()?;
        trace!(?url);
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url) //
            .header(reqwest::header::ACCEPT, "application/json") //
            .send()?;
        let bytes = response.bytes()?;
        let mut de = serde_json::Deserializer::from_slice(&bytes);
        let response: serde_json::Value = Deserialize::deserialize(&mut de)?;
        trace!("response = {response:#?}");

        // check structure of response (1)
        let Some(response) = response.as_array() else {
            return Err(anyhow!("response has not the expected structure"));
        };
        // check structure of response (2)
        let response = if response.len() == 1 {
            &response[0]
        } else {
            return Err(anyhow!("response is ambiguous {}", response.len()));
        };

        // TODO check that the response corresponds to the request (the query for x86 returns packages for x64 too)

        // url

        let Some(url) = response["binary"]["package"]["link"].as_str() else {
            return Err(anyhow!("field 'link' not present in response"));
        };

        // checksum

        let Some(checksum) = response["binary"]["package"]["checksum"].as_str() else {
            return Err(anyhow!("field 'checksum' not present in response"));
        };

        // version

        let Some(version) = response["version"].as_object() else {
            return Err(anyhow!("field 'version' not present in response"));
        };
        let Some(major) = version["major"].as_u64() else {
            return Err(anyhow!("major part not present in 'version'"));
        };
        let Some(minor) = version["minor"].as_u64() else {
            return Err(anyhow!("minor part not present in 'version'"));
        };
        let Some(security) = version["security"].as_u64() else {
            return Err(anyhow!("security part not present in 'version'"));
        };
        let version = Version::new(major, minor, security);

        Ok(MetadataResponse {
            checksum: checksum.to_string(),
            url: url.to_string(),
            version,
        })
    }

    // Build the query URL to search for packages.
    fn query_url(&self) -> anyhow::Result<Url> {
        let mut version = self.version();
        version.push('/');
        let url = Url::parse(API_URL)?;
        let url = url.join(&version)?;
        let mut url = url.join("hotspot/")?;
        url.query_pairs_mut()
            .append_pair("architecture", &self.arch())
            .append_pair("image_type", &self.package_type())
            .append_pair("os", &self.os())
            .append_pair("vendor", "eclipse");

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

/// The response to the [`MetadataRequest`].
pub(super) struct MetadataResponse {
    pub(super) checksum: String,
    pub(super) url: String,
    pub(super) version: Version,
}
