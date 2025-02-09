use super::api::*;
use super::*;
use crate::meta::*;
#[cfg(feature = "notify")]
use crate::notify::*;
use crate::package::*;
use crate::terminal::*;
use crate::vars::*;
use crate::vendor::*;
use anyhow::anyhow;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use tracing::{instrument, trace, warn};

/// The installation contains everything to materialise a java package (JDK or JRE) to disc.
#[derive(Debug)]
pub(super) struct Installation {
    config: Rc<InstallationConfig>,
    dry_run: bool,
    os: String,
    path: PathBuf,
    vendor: Vendor,
}

impl Installation {
    // Creates a new [Installation] out of the given [InstallationConfig].
    pub(super) fn from_config(basedir: &Path, config: InstallationConfig) -> Self {
        let path = basedir.join(config.expand_directory());
        let path = path::absolute(&path).unwrap_or(path);

        Self {
            config: Rc::new(config),
            dry_run: false,
            os: env::consts::OS.to_string(), // TODO do we really need this here?
            path,
            vendor: Vendor::Eclipse,
        }
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
                        println!("dry-run: {not} processing installation at {path} [{old_version_str} -> {new_version}]");
                    } else {
                        println!("Processed installation at {path} [{old_version_str} -> {new_version}]");
                        #[cfg(feature = "notify")]
                        self.notify_on_update(old_version, &metadata.version);
                        #[cfg(feature = "notify")]
                        self.notify_on_success(old_version, &metadata.version);
                    }
                } else if self.dry_run {
                    let not = ATTENTION_COLOR.paint("NOT");
                    println!("dry-run: {not} processing installation at {path} [{old_version_str}]");
                } else {
                    println!("Processed installation at {path} [{old_version_str}]");
                    #[cfg(feature = "notify")]
                    self.notify_on_success(old_version, &metadata.version);
                }
            }
            Ok(None) => {
                let version = INFO_COLOR.paint("n/a");
                if self.dry_run {
                    let not = ATTENTION_COLOR.paint("NOT");
                    println!("dry-run: {not} processing installation at {path} [{version}]");
                } else {
                    println!("Processed installation at {path} [{version}]");
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
            arch: self.config.architecture.clone(),
            os: self.os.clone(),
            package_type: self.config.package_type.clone(),
            version: self.config.version.clone(),
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

    // Notify in case of failure.
    #[cfg(feature = "notify")]
    #[instrument(level = "trace", skip(self))]
    fn notify_on_failure(&self, old: Option<&semver::Version>, err: anyhow::Error) {
        if self.config.on_failure.is_empty() {
            return;
        };

        let path = self.path.to_string_lossy();

        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", self.config.architecture.to_string());
        simple_var_resolver.insert("env.JU_ERROR", err.to_string());
        simple_var_resolver.insert("env.JU_DIRECTORY", path.to_string());
        if let Some(old) = old {
            simple_var_resolver.insert("env.JU_OLD_VERSION", old.to_string());
        }
        simple_var_resolver.insert("env.JU_TYPE", self.config.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", self.vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", self.vendor.name().to_string());
        let env_var_resolver = EnvVarResolver;
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(env_var_resolver)];
        let var_expander = VarExpander::new(var_resolvers);

        // process all commands
        let commands: Vec<NotifyCommand> = self.config.on_failure.iter().map(NotifyCommand::from_config).collect();
        for command in commands {
            // setup command
            let mut command = command.clone();
            command.kind(NotifyKind::Failure);
            command.env("JU_ARCH", &self.config.architecture);
            command.env("JU_ERROR", &err.to_string());
            command.env("JU_DIRECTORY", &path);
            if let Some(old) = old {
                command.env("JU_OLD_VERSION", &old.to_string());
            }
            command.env("JU_TYPE", &self.config.package_type);
            command.env("JU_VENDOR_ID", self.vendor.id());
            command.env("JU_VENDOR_NAME", self.vendor.name());

            // execute command
            trace!(?command, "executing on-failure command");
            command.execute(&var_expander);
        }
    }

    // Notify in case of success.
    #[cfg(feature = "notify")]
    #[instrument(level = "trace", skip(self))]
    fn notify_on_success(&self, old: Option<&semver::Version>, new: &semver::Version) {
        if self.config.on_success.is_empty() {
            return;
        };

        let path = self.path.to_string_lossy();

        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", self.config.architecture.to_string());
        simple_var_resolver.insert("env.JU_DIRECTORY", path.to_string());
        simple_var_resolver.insert("env.JU_NEW_VERSION", new.to_string());
        if let Some(old) = old {
            simple_var_resolver.insert("env.JU_OLD_VERSION", old.to_string());
        }
        simple_var_resolver.insert("env.JU_TYPE", self.config.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", self.vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", self.vendor.name().to_string());
        let env_var_resolver = EnvVarResolver;
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(env_var_resolver)];
        let var_expander = VarExpander::new(var_resolvers);

        // process all commands
        let commands: Vec<NotifyCommand> = self.config.on_success.iter().map(NotifyCommand::from_config).collect();
        for command in commands {
            // setup command
            let mut command = command.clone();
            command.kind(NotifyKind::Success);
            command.env("JU_ARCH", &self.config.architecture);
            command.env("JU_DIRECTORY", &path);
            command.env("JU_NEW_VERSION", &new.to_string());
            if let Some(old) = old {
                command.env("JU_OLD_VERSION", &old.to_string());
            }
            command.env("JU_TYPE", &self.config.package_type);
            command.env("JU_VENDOR_ID", self.vendor.id());
            command.env("JU_VENDOR_NAME", self.vendor.name());

            // execute command
            trace!(?command, "executing on-success command");
            command.execute(&var_expander);
        }
    }

    // Notify in case of update.
    #[cfg(feature = "notify")]
    #[instrument(level = "trace", skip(self))]
    fn notify_on_update(&self, old: Option<&semver::Version>, new: &semver::Version) {
        if self.config.on_update.is_empty() {
            return;
        };

        let path = self.path.to_string_lossy();

        // setup variable resolver(s)
        let mut simple_var_resolver = SimpleVarResolver::new();
        simple_var_resolver.insert("env.JU_ARCH", self.config.architecture.to_string());
        simple_var_resolver.insert("env.JU_DIRECTORY", path.to_string());
        simple_var_resolver.insert("env.JU_NEW_VERSION", new.to_string());
        if let Some(old) = old {
            simple_var_resolver.insert("env.JU_OLD_VERSION", old.to_string());
        }
        simple_var_resolver.insert("env.JU_TYPE", self.config.package_type.to_string());
        simple_var_resolver.insert("env.JU_VENDOR_ID", self.vendor.id().to_string());
        simple_var_resolver.insert("env.JU_VENDOR_NAME", self.vendor.name().to_string());
        let env_var_resolver = EnvVarResolver;
        let var_resolvers: Vec<Box<dyn VarResolver>> = vec![Box::new(simple_var_resolver), Box::new(env_var_resolver)];
        let var_expander = VarExpander::new(var_resolvers);

        // process all commands
        let commands: Vec<NotifyCommand> = self.config.on_update.iter().map(NotifyCommand::from_config).collect();
        for command in commands {
            // setup command
            let mut command = command.clone();
            command.kind(NotifyKind::Success);
            command.env("JU_ARCH", &self.config.architecture);
            command.env("JU_DIRECTORY", &path);
            command.env("JU_NEW_VERSION", &new.to_string());
            if let Some(old) = old {
                command.env("JU_OLD_VERSION", &old.to_string());
            }
            command.env("JU_TYPE", &self.config.package_type);
            command.env("JU_VENDOR_ID", self.vendor.id());
            command.env("JU_VENDOR_NAME", self.vendor.name());

            // execute command
            trace!(?command, "executing on-update command");
            command.execute(&var_expander);
        }
    }
}
