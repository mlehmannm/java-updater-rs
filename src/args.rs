//! Arguments.
//!
//! This module contains the definition for the available command-line parameter.

use clap::Parser;

#[derive(Debug, Parser)]
#[clap(author)]
pub(crate) struct Args {
    /// Sets a custom config file
    #[clap(short, long, value_name = "file")]
    pub(crate) config: Option<String>,
    /// Whether to really execute the command
    #[clap(short = 'n', long, action)]
    pub(crate) dry_run: bool,
    /// Suppress unnecessary information
    #[clap(short = 'q', long, action)]
    pub(crate) quiet: bool,
    /// Change level of verbosity (apply multiple times to increase level)
    #[clap(short, long, action = clap::ArgAction::Count)]
    pub(crate) verbose: u8,
    /// Print version information
    #[clap(short = 'V', long, action)]
    pub(crate) version: bool,
}

#[cfg(test)]
mod tests {

    use super::*;
    use test_log::test;

    #[test]
    fn no_args() {
        let args = Args::try_parse_from(["program"]).unwrap();
        assert_eq!(args.config, None);
    }

    #[test]
    fn config_without_file() {
        let args = Args::try_parse_from(["program", "--config"]);
        assert!(args.is_err());
    }

    #[test]
    fn config_with_file() {
        let args = Args::try_parse_from(["program", "--config", "file"]).unwrap();
        assert_eq!(args.config, Some("file".into()));
    }
}
