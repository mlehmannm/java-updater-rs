//! Azul.
//!
//! This module contains the implementation to download and unpack a java package from Azul.

// https://docs.azul.com/core/install/metadata-api
// https://api.azul.com/metadata/v1/docs/swagger

#[doc(hidden)]
mod api;
#[doc(hidden)]
mod installation;

use self::installation::*;
use crate::args::*;
use crate::colors::*;
use crate::config::InstallationConfig;
use std::env;
use std::path::{self, Path};

// Base URL for the API endpoint.
#[doc(hidden)]
const API_URL: &str = "https://api.azul.com/metadata/v1/zulu/packages/";

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
    let mut installation = match Installation::from_config(basedir, config) {
        Ok(installation) => installation,
        Err(err) => {
            let err_str = ATTENTION_COLOR.paint(format!("err = {err:?}"));
            eprintln!("Failed to setup installation!\r\n\t{err_str}");
            return;
        }
    };

    installation //
        .dry_run(args.dry_run) //
        .setup();
}
