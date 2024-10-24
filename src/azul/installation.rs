use super::api::*;
use super::*;
use crate::colors::*;
use crate::meta::*;
#[cfg(feature = "notify")]
use crate::notify::*;
use crate::package::*;
use crate::vars::*;
use crate::vendor::*;
use anyhow::anyhow;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{instrument, trace, warn};

/// The installation contains everything to materialise a java package (JDK or JRE) to disc.
#[derive(Debug)]
pub(super) struct Installation {
    arch: String,
    os: String,
    package_type: String,
    path: PathBuf,
    vendor: Vendor,
    version: String,
    dry_run: bool,
    #[cfg(feature = "notify")]
    on_failure: Option<NotifyCommand>,
    #[cfg(feature = "notify")]
    on_update: Option<NotifyCommand>,
}

impl Installation {
    // Creates a new [Installation] out of the given [InstallationConfig].
    pub(super) fn from_config(basedir: &Path, config: &InstallationConfig) -> anyhow::Result<Self> {
        let vendor = Vendor::Azul;
        let path = Self::resolve_path(&vendor, basedir, config)?;
        #[cfg(feature = "notify")]
        let on_update = config.on_update.as_ref().map(NotifyCommand::from_config);
        #[cfg(feature = "notify")]
        let on_failure = config.on_failure.as_ref().map(NotifyCommand::from_config);

        Ok(Installation {
            arch: config.architecture.clone(),
            dry_run: false,
            os: env::consts::OS.to_string(),
            package_type: config.package_type.clone(),
            path,
            vendor,
            version: config.version.clone(),
            #[cfg(feature = "notify")]
            on_update,
            #[cfg(feature = "notify")]
            on_failure,
        })
    }

    // Resolves the complete path of the installation.
    fn resolve_path(vendor: &Vendor, basedir: &Path, config: &InstallationConfig) -> anyhow::Result<PathBuf> {
        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", config.architecture.to_string());
        simple_var_resolver.insert("env.JU_OS", env::consts::OS.to_string());
        simple_var_resolver.insert("env.JU_TYPE", config.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", vendor.name().to_string());
        simple_var_resolver.insert("env.JU_VERSION", config.version.to_string());
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(EnvVarResolver)];
        let vars_resolver = VarsResolver::new(var_resolvers);

        // resolve path
        let directory = vars_resolver.resolve(&config.directory)?;
        let path = basedir.join(directory.as_ref());
        let path = path::absolute(&path).unwrap_or(path);

        Ok(path)
    }

    /// Whether to perform the installation or not.
    pub(super) fn dry_run(&mut self, dry_run: bool) -> &mut Self {
        self.dry_run = dry_run;

        self
    }

    // Set up the installation.
    pub(super) fn setup(&self) {
        let metadata = self.load_metadata();
        let path = PATH_COLOR.paint(self.path.to_string_lossy());
        let old_version = metadata.as_ref().map(|metadata| metadata.version.clone()).ok();
        let old_version_str = old_version.as_ref().map_or("n/a".to_string(), ToString::to_string);
        let old_version_str = INFO_COLOR.paint(old_version_str);
        println!("Processing installation at {path} [{old_version_str}]");

        match self._setup(metadata.ok()) {
            Ok(Some(metadata)) => {
                let old_version = old_version.as_ref();
                let new_version = &metadata.version;
                if old_version != Some(new_version) {
                    let new_version = INFO_COLOR.paint(new_version.to_string());
                    if self.dry_run {
                        let not = ATTENTION_COLOR.paint("NOT");
                        println!("dry-run: {not} processing installation at  {path} [{old_version_str} -> {new_version}]");
                    } else {
                        println!("Processed installation at  {path} [{old_version_str} -> {new_version}]");
                        #[cfg(feature = "notify")]
                        self.notify_on_update(old_version, &metadata.version);
                    }
                } else if self.dry_run {
                    let not = ATTENTION_COLOR.paint("NOT");
                    println!("dry-run: {not} processing installation at  {path} [{old_version_str}]");
                } else {
                    println!("Processed installation at  {path} [{old_version_str}]");
                }
            }
            Ok(None) => {
                let version = INFO_COLOR.paint("n/a");
                if self.dry_run {
                    let not = ATTENTION_COLOR.paint("NOT");
                    println!("dry-run: {not} processing installation at  {path} [{version}]");
                } else {
                    println!("Processed installation at  {path} [{version}]");
                }
            }
            Err(err) => {
                let err_str = ATTENTION_COLOR.paint(format!("err = {err:?}"));
                eprintln!("Failed to process installation at {path}!\r\n\t{err_str}");
                #[cfg(feature = "notify")]
                self.notify_on_failure(old_version.as_ref(), err);
            }
        };
    }

    // Set up the installation internally.
    #[instrument(level = "trace", skip(self))]
    fn _setup(&self, metadata: Option<Metadata>) -> anyhow::Result<Option<Metadata>> {
        let latest = self.query_latest()?;
        let download = if let Some(ref metadata) = metadata {
            if latest.version > metadata.version {
                true
            } else {
                latest.checksum != metadata.checksum
            }
        } else {
            true
        };

        let metadata = if download {
            let metadata = Metadata::new(self.vendor.id(), latest.version, &latest.checksum);

            if self.dry_run {
                return Ok(Some(metadata));
            }

            // download/unpack the package
            let package = Package::new(&self.path, ARCHIVE_TYPE, &latest.url, &latest.checksum);
            package.provide()?;

            self.save_metadata(&metadata)?;
            Some(metadata)
        } else {
            trace!(path = %self.path.display(), "no download necessary");
            metadata
        };

        Ok(metadata)
    }

    // Load local metadata.
    #[instrument(level = "trace", skip(self))]
    fn load_metadata(&self) -> anyhow::Result<Metadata> {
        let filename = self.path.join(METADATA_DIR).join(METADATA_FILE);
        let metadata = Metadata::load(filename)?;
        if self.vendor.id() != metadata.vendor {
            return Err(anyhow!("vendors differ (expected: {}, got: {})", self.vendor.id(), metadata.vendor));
        }

        Ok(metadata)
    }

    // Query latest metadata.
    #[instrument(level = "trace", skip(self))]
    fn query_latest(&self) -> anyhow::Result<MetadataResponse> {
        let req = MetadataRequest {
            arch: self.arch.clone(),
            os: self.os.clone(),
            package_type: self.package_type.clone(),
            version: self.version.clone(),
        };
        req.query()
    }

    // Saves local metadata.
    #[instrument(level = "trace", skip(self))]
    fn save_metadata(&self, metadata: &Metadata) -> anyhow::Result<()> {
        let metadata_dir = self.path.join(METADATA_DIR);
        fs::create_dir_all(&metadata_dir)?;
        let filename = metadata_dir.join(METADATA_FILE);
        metadata.save(filename)
    }

    // Notify in case of update.
    #[cfg(feature = "notify")]
    #[instrument(level = "trace", skip(self))]
    fn notify_on_update(&self, old: Option<&semver::Version>, new: &semver::Version) {
        let Some(command) = &self.on_update else {
            return;
        };

        let path = self.path.to_string_lossy();

        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", self.arch.to_string());
        simple_var_resolver.insert("env.JU_INSTALLATION", path.to_string());
        simple_var_resolver.insert("env.JU_NEW_VERSION", new.to_string());
        if let Some(old) = old {
            simple_var_resolver.insert("env.JU_OLD_VERSION", old.to_string());
        }
        simple_var_resolver.insert("env.JU_TYPE", self.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", self.vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", self.vendor.name().to_string());
        let env_var_resolver = EnvVarResolver;
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(env_var_resolver)];
        let vars_resolver = VarsResolver::new(var_resolvers);

        // setup command
        let mut command = command.clone();
        command.kind(NotifyKind::Success);
        command.env("JU_ARCH", &self.arch);
        command.env("JU_INSTALLATION", &path);
        command.env("JU_NEW_VERSION", &new.to_string());
        if let Some(old) = old {
            command.env("JU_OLD_VERSION", &old.to_string());
        }
        command.env("JU_TYPE", &self.package_type);
        command.env("JU_VENDOR_ID", self.vendor.id());
        command.env("JU_VENDOR_NAME", self.vendor.name());

        // execute command
        trace!(?command, "executing on-update command");
        command.execute(vars_resolver);
    }

    // Notify in case of failure.
    #[cfg(feature = "notify")]
    #[instrument(level = "trace", skip(self))]
    fn notify_on_failure(&self, old: Option<&semver::Version>, err: anyhow::Error) {
        let Some(command) = &self.on_failure else {
            return;
        };

        let path = self.path.to_string_lossy();

        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", self.arch.to_string());
        simple_var_resolver.insert("env.JU_ERROR", err.to_string());
        simple_var_resolver.insert("env.JU_INSTALLATION", path.to_string());
        if let Some(old) = old {
            simple_var_resolver.insert("env.JU_OLD_VERSION", old.to_string());
        }
        simple_var_resolver.insert("env.JU_TYPE", self.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", self.vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", self.vendor.name().to_string());
        let env_var_resolver = EnvVarResolver;
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(env_var_resolver)];
        let vars_resolver = VarsResolver::new(var_resolvers);

        // setup command
        let mut command = command.clone();
        command.kind(NotifyKind::Failure);
        command.env("JU_ARCH", &self.arch);
        command.env("JU_ERROR", &err.to_string());
        command.env("JU_INSTALLATION", &path);
        if let Some(old) = old {
            command.env("JU_OLD_VERSION", &old.to_string());
        }
        command.env("JU_TYPE", &self.package_type);
        command.env("JU_VENDOR_ID", self.vendor.id());
        command.env("JU_VENDOR_NAME", self.vendor.name());

        // execute command
        trace!(?command, "executing on-failure command");
        command.execute(vars_resolver);
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn resolve_path_failure() {
        let vendor = Vendor::Azul;
        let config = InstallationConfig {
            directory: "${XYZ}".to_string(),
            ..Default::default()
        };
        let basedir = env::current_dir().unwrap();
        let result = Installation::resolve_path(&vendor, &basedir, &config);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_path_success() {
        let architecture = env::consts::ARCH.to_string();
        let os = env::consts::OS.to_string();
        let vendor = Vendor::Azul;
        let config = InstallationConfig {
            architecture: architecture.clone(),
            directory: "${env.JU_ARCH}/${env.JU_OS}/${env.JU_TYPE}/${env.JU_VENDOR_ID}/${env.JU_VENDOR_NAME}/${env.JU_VERSION}".to_string(),
            package_type: "jdk".to_string(),
            vendor: vendor.id().to_string(),
            version: "8".to_string(),
            ..Default::default()
        };
        let basedir = env::current_dir().unwrap();
        let actual = Installation::resolve_path(&vendor, &basedir, &config).unwrap();
        let expected = basedir.join(format!("{architecture}/{os}/jdk/{}/{}/8", vendor.id(), vendor.name()));
        assert_eq!(expected, actual);
    }
}
