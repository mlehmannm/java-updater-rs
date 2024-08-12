//! Package.
//!
//! This module contains the code to download and unpack a java package.

use crate::checksum::{self, ChecksumWrite};
use crate::meta::*;
use anyhow::anyhow;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tracing::{error, instrument, trace, warn};

/// Struct to hold all necessary data to download and unpack a java package.
pub(crate) struct Package {
    checksum: String,
    ext: String,
    path: PathBuf,
    url: String,
}

impl Package {
    /// Creates a new `Package`.
    pub(crate) fn new(path: impl Into<PathBuf>, ext: impl Into<String>, url: impl Into<String>, checksum: impl Into<String>) -> Self {
        Self {
            checksum: checksum.into(),
            path: path.into(),
            url: url.into(),
            ext: ext.into(),
        }
    }

    /// Provide (download annd unpack) the package.
    pub(crate) fn provide(&self) -> anyhow::Result<()> {
        let pkg = self.download()?;
        self.unpack(&pkg)
    }

    // Download the package.
    #[instrument(level = "trace", skip(self))]
    fn download(&self) -> anyhow::Result<PathBuf> {
        let metadata_dir = self.path.join(METADATA_DIR);
        let mut dest = metadata_dir.join(&self.checksum);
        dest.set_extension(&self.ext);

        // check if already downloaded
        if dest.exists() && checksum::checksum(&dest)? == self.checksum {
            return Ok(dest.to_path_buf());
        }

        // make request
        let client = reqwest::blocking::Client::new();
        let mut response = client
            .get(&self.url) //
            .header(reqwest::header::ACCEPT, "application/octet-stream") //
            .send()?;

        // download file
        fs::create_dir_all(&metadata_dir)?;
        trace!(pkg = %dest.display());
        let dest_file = File::create(&dest)?;
        let mut checksum_write = ChecksumWrite::new(dest_file);
        let bytes_written = response.copy_to(&mut checksum_write)?;
        trace!(bytes_written);
        let checksum_calculated = checksum_write.checksum()?;
        trace!(checksum_calculated);

        // calculate/verify checksum
        if self.checksum.to_lowercase() != checksum_calculated {
            return Err(anyhow::Error::msg("hashes differ"));
        }

        Ok(dest.to_path_buf())
    }

    // Unpacks the package and replaces the old installation with the new installation.
    #[cfg(not(windows))]
    #[instrument(level = "trace", skip(self))]
    fn unpack(&self, pkg: &Path) -> anyhow::Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let tmp = self.path.join(METADATA_DIR).join(&self.checksum);

        // remove left-overs from last run, if there are any
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }

        // check, if current installation is in use
        let lib = self.path.join("lib");
        if lib.exists() {
            let mut lib_renamed = self.path.join("lib");
            lib_renamed.set_extension(&self.checksum);

            if let Err(err) = fs::rename(&lib, &lib_renamed) {
                error!(?err, "installation is still in use");
                println!("installation is still in use");
                return Err(anyhow::Error::new(err));
            }

            // revert rename
            let _ = fs::rename(lib_renamed, lib);
        }

        // unpack new installation to tmp directory
        let pkg_file = File::open(pkg)?;
        let mut archive = Archive::new(GzDecoder::new(pkg_file));
        for entry in archive.entries()? {
            let mut entry = entry?;

            // skip entry with dangerous name
            let Ok(name) = entry.path() else {
                let path_bytes = &entry.path_bytes();
                let name = String::from_utf8_lossy(path_bytes);
                warn!(name = %name, "skipping dangerous name");
                continue;
            };

            // skip elements without at least two components
            let mut components = name.components();
            if name.components().count() <= 1 {
                if !entry.header().entry_type().is_dir() {
                    warn!(name = %name.display(), "skipping unusual name");
                }
                continue;
            }

            // remove the first component
            components.next();
            let name = components.as_path();

            let name = tmp.join(name);
            trace!("unpacking {name:?}");

            if name.is_dir() {
                fs::create_dir_all(name)?;
            } else {
                if let Some(p) = name.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                entry.unpack(name)?;
            }
        }

        let java_exe = tmp.join("bin").join("java");
        if !java_exe.exists() {
            return Err(anyhow!("failed to verify installation"));
        }

        // TODO further verify installation in tmp by calling java -version ?

        // delete current installation
        let metadata_dir = OsStr::new(METADATA_DIR);
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;

            let path = entry.path();
            let Some(name) = path.file_name() else {
                continue;
            };

            // skip metadata directory
            if name == metadata_dir {
                continue;
            }

            // remove
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }

        // move new installation
        for entry in fs::read_dir(&tmp)? {
            let entry = entry?;

            let from = entry.path();
            let Some(name) = from.file_name() else {
                continue;
            };

            let to = self.path.join(name);

            fs::rename(from, to)?;
        }

        // cleanup tmp directory
        if let Err(err) = fs::remove_dir_all(tmp) {
            warn!(?err, "failed to delete tmp directory");
        }

        Ok(())
    }

    // Unpacks the package and replaces the old installation with the new installation.
    #[allow(clippy::permissions_set_readonly_false)]
    #[cfg(windows)]
    #[instrument(level = "trace", skip(self))]
    fn unpack(&self, pkg: &Path) -> anyhow::Result<()> {
        let tmp = self.path.join(METADATA_DIR).join(&self.checksum);

        // remove leftovers from last run, if there are any
        if tmp.exists() {
            fs::remove_dir_all(&tmp)?;
        }

        // check, if current installation is in use
        let lib = self.path.join("lib");
        if lib.exists() {
            let mut lib_renamed = self.path.join("lib");
            lib_renamed.set_extension(&self.checksum);

            if let Err(err) = fs::rename(&lib, &lib_renamed) {
                error!(?err, "installation is still in use");
                println!("installation is still in use");
                return Err(anyhow::Error::new(err));
            }

            // revert rename
            let _ = fs::rename(lib_renamed, lib);
        }

        // unpack new installation to tmp directory
        let pkg_file = File::open(pkg)?;
        let mut zip = zip::ZipArchive::new(pkg_file)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            let Some(name) = file.enclosed_name() else {
                warn!(name = file.name(), "skipping dangerous name");
                continue;
            };

            // skip elements without at least two components
            let mut components = name.components();
            if name.components().count() <= 1 {
                if !file.is_dir() {
                    warn!(name = %name.display(), "skipping unusual name");
                }
                continue;
            }

            // remove the first component
            components.next();
            let name = components.as_path();

            let name = tmp.join(name);
            trace!("unpacking {name:?}");

            if file.is_dir() {
                fs::create_dir_all(name)?;
            } else {
                if let Some(p) = name.parent() {
                    if !p.exists() {
                        fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = fs::File::create(name)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        let java_exe = tmp.join("bin").join("java.exe");
        if !java_exe.exists() {
            return Err(anyhow!("failed to verify installation"));
        }

        // TODO further verify installation in tmp by calling java -version ?

        // delete current installation
        let metadata_dir = OsStr::new(METADATA_DIR);
        for entry in fs::read_dir(&self.path)? {
            let entry = entry?;

            let path = entry.path();
            let Some(name) = path.file_name() else {
                continue;
            };

            // skip metadata directory
            if name == metadata_dir {
                continue;
            }

            // Windows won't delete directories/files marked read-only
            let metadata = entry.metadata()?;
            let mut perms = metadata.permissions();
            if perms.readonly() {
                perms.set_readonly(false);
                fs::set_permissions(&path, perms)?;
            }

            // remove
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                fs::remove_file(path)?;
            }
        }

        // move new installation
        for entry in fs::read_dir(&tmp)? {
            let entry = entry?;

            let from = entry.path();
            let Some(name) = from.file_name() else {
                continue;
            };

            let to = self.path.join(name);

            fs::rename(from, to)?;
        }

        // cleanup tmp directory
        if let Err(err) = fs::remove_dir_all(tmp) {
            warn!(?err, "failed to delete tmp directory");
        }

        Ok(())
    }
}
