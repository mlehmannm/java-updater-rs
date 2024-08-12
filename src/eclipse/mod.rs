//! Eclipse.
//!
//! This module contains the implementation to download and unpack a java package from Eclipse.

// https://api.adoptium.net/q/swagger-ui/

#[doc(hidden)]
mod api;
#[doc(hidden)]
mod installation;

use self::installation::*;
use crate::args::*;
use crate::config::InstallationConfig;
use std::env;
use std::path::{self, Path};

// Base URL for the API endpoint.
#[doc(hidden)]
const API_URL: &str = "https://api.adoptium.net/v3/assets/latest/";

// Archive type to be used on OSes other than Windows.
#[cfg(not(windows))]
#[doc(hidden)]
const ARCHIVE_TYPE: &str = "tar.gz";

// Archive type to be used on Windows.
#[cfg(windows)]
#[doc(hidden)]
const ARCHIVE_TYPE: &str = "zip";

/// Prepare and set up the installation.
pub(crate) fn setup(basedir: &Path, args: &Args, config: &InstallationConfig) {
    let mut installation = Installation::from_config(basedir, config);
    installation.dry_run(args.dry_run);
    installation.setup();
}
