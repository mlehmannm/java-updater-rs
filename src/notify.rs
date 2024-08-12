//! Notification.
//!
//! This module contains the notification command infrastructure.

use crate::vars::VarsResolver;
use crate::NotifyCommandConfig;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tracing::*;

// The struct that holds the notify command.
#[derive(Clone, Debug)]
pub(crate) struct NotifyCommand {
    // The path to the executable.
    path: String,
    // The arguments for the executable.
    args: Vec<String>,
    // The envorinment for the executable.
    env: HashMap<String, String>,
    // The working directory for the executable.
    directory: Option<String>,
}

impl NotifyCommand {
    /// Creates a new `NotifyCommand` out of the given `NotifyCommandConfig`.
    pub(crate) fn from_config(config: &NotifyCommandConfig) -> Self {
        Self {
            args: config.args.clone(),
            directory: config.directory.clone(),
            env: HashMap::new(),
            path: config.path.clone(),
        }
    }

    /// Inserts or updates an explicit environment variable mapping.
    pub(crate) fn env(&mut self, key: &str, val: &str) -> &mut Self {
        self.env.insert(key.to_string(), val.to_string());

        self
    }

    /// Executes (and consumes) the notify command.
    pub(crate) fn execute(self, vars_resolver: VarsResolver) {
        if let Err(err) = self._execute(vars_resolver) {
            error!(?err, "failed to execute notify command");
        };
    }

    // Executes the notify command internally.
    #[instrument(err, level = "trace")]
    fn _execute(&self, vars_resolver: VarsResolver) -> anyhow::Result<()> {
        // prepare command
        let path = vars_resolver.resolve(&self.path)?;
        let mut cmd = Command::new(path.as_ref());
        for arg in &self.args {
            let arg = vars_resolver.resolve(arg)?;
            cmd.arg(arg.as_ref());
        }
        if let Some(ref dir) = self.directory {
            let dir = vars_resolver.resolve(dir)?;
            cmd.current_dir(dir.as_ref());
        }
        for (key, val) in &self.env {
            let val = vars_resolver.resolve(val)?;
            cmd.env(key, val.as_ref());
        }
        cmd.stdin(Stdio::null()); // disconnect from self
        cmd.stderr(Stdio::null()); // disconnect from self
        cmd.stdout(Stdio::null()); // disconnect from self

        // execute command (use spawn to not block this thread)
        cmd.spawn()?;

        Ok(())
    }
}
