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

        // check structure of response

        let Some(response) = response.as_array() else {
            return Err(anyhow!("response has not the expected structure"));
        };
        let arch = self.arch();
        let response = response
            .iter()
            .find(|r| {
                let r_arch = r["binary"]["architecture"].as_str().unwrap_or_default();
                r_arch == arch // Direct comparison after normalization
            })
            .ok_or_else(|| anyhow!("no package found for architecture {arch}"))?;

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

    // Returns the requested architecture for the package, normalized for the API.
    fn arch(&self) -> String {
        let arch = self.arch.trim();

        let arch = if arch.is_empty() { env::consts::ARCH } else { arch };

        match arch.to_lowercase().as_str() {
            "amd64" | "x86_64" => "x64".to_string(),
            "i686" | "x86" => "x32".to_string(),
            arch => arch.to_string(),
        }
    }

    // Returns the requested operating system for the package.
    fn os(&self) -> String {
        let os = self.os.trim();
        if os.is_empty() { env::consts::OS.to_lowercase() } else { os.to_lowercase() }
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
        if version.is_empty() { "17".to_string() } else { version.to_lowercase() }
    }
}

/// The response to the [`MetadataRequest`].
pub(super) struct MetadataResponse {
    pub(super) checksum: String,
    pub(super) url: String,
    pub(super) version: Version,
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn test_normalize_i686_architecture() {
        let request = MetadataRequest {
            arch: "i686".to_string(),
            os: "windows".to_string(),
            package_type: "jdk".to_string(),
            version: "17".to_string(),
        };
        assert_eq!("x32", request.arch());
    }

    #[test]
    fn test_normalize_x86_64_architecture() {
        let request = MetadataRequest {
            arch: "x86_64".to_string(),
            os: "windows".to_string(),
            package_type: "jdk".to_string(),
            version: "17".to_string(),
        };
        assert_eq!("x64", request.arch());
    }

    #[test]
    fn test_query_x32_architecture() {
        let request = MetadataRequest {
            arch: "x32".to_string(),
            os: "windows".to_string(),
            package_type: "jdk".to_string(),
            version: "17".to_string(),
        };
        let _result = request.query().expect("Failed to query");
        // TODO check response
    }

    #[test]
    fn test_query_x64_architecture() {
        let request = MetadataRequest {
            arch: "x64".to_string(),
            os: "windows".to_string(),
            package_type: "jdk".to_string(),
            version: "25".to_string(),
        };
        let _result = request.query().expect("Failed to query");
        // TODO check response
    }

    #[test]
    fn test_query_aarch64_architecture() {
        let request = MetadataRequest {
            arch: "aarch64".to_string(),
            os: "linux".to_string(),
            package_type: "jdk".to_string(),
            version: "17".to_string(),
        };
        let _result = request.query().expect("Failed to query");
        // TODO check response
    }
}
