//! Notification.
//!
//! This module contains the notification command infrastructure.

use crate::NotifyCommandConfig;
use crate::vars::VarExpander;
use std::collections::HashMap;
use std::process::{Command, Stdio};
use tracing::error;

/// A list specifying general categories of notification.
#[derive(Clone, Debug)]
pub(crate) enum NotifyKind {
    /// Failure
    Failure,
    /// Success
    Success,
}

// The struct that holds the notify command.
#[derive(Clone, Debug)]
pub(crate) struct NotifyCommand {
    // The arguments for the executable.
    args: Vec<String>,
    // The working directory for the executable.
    directory: Option<String>,
    // The environment for the executable.
    env: HashMap<String, String>,
    // The kind of notification.
    kind: Option<NotifyKind>,
    // The path to the executable.
    path: String,
}

impl NotifyCommand {
    /// Creates a new `NotifyCommand` out of the given `NotifyCommandConfig`.
    pub(crate) fn from_config(config: &NotifyCommandConfig) -> Self {
        Self {
            args: config.args.clone(),
            directory: config.directory.clone(),
            env: HashMap::new(),
            kind: None,
            path: config.path.clone(),
        }
    }

    /// Inserts or updates the kind of notification.
    pub(crate) fn kind(&mut self, kind: NotifyKind) -> &mut Self {
        self.kind = Some(kind);

        self
    }

    /// Inserts or updates an explicit environment variable mapping.
    pub(crate) fn env(&mut self, key: &str, val: &str) -> &mut Self {
        self.env.insert(key.to_string(), val.to_string());

        self
    }

    /// Executes (and consumes) the notify command.
    pub(crate) fn execute(self, var_expander: &VarExpander) {
        if let Err(err) = self.execute_inner(var_expander) {
            match self.kind {
                Some(NotifyKind::Failure) => error!(?err, "failed to execute notify (on failure) command"),
                Some(NotifyKind::Success) => error!(?err, "failed to execute notify (on success) command"),
                None => error!(?err, "failed to execute notify command"),
            }
        }
    }

    // Executes the notify command internally.
    #[tracing::instrument(err, level = "trace")]
    fn execute_inner(&self, var_expander: &VarExpander) -> anyhow::Result<()> {
        // prepare command
        let path = var_expander.expand(&self.path)?;
        let mut cmd = Command::new(path.as_ref());
        for arg in &self.args {
            let arg = var_expander.expand(arg)?;
            cmd.arg(arg.as_ref());
        }
        if let Some(ref dir) = self.directory {
            let dir = var_expander.expand(dir)?;
            cmd.current_dir(dir.as_ref());
        }
        for (key, val) in &self.env {
            let val = var_expander.expand(val)?;
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
