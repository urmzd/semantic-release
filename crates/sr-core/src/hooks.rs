use std::collections::HashMap;
use std::process::Command;

use crate::error::ReleaseError;

/// A shell command to run as a lifecycle hook.
#[derive(Debug, Clone)]
pub struct HookCommand {
    pub command: String,
}

/// Release context passed to hooks as environment variables.
#[derive(Debug, Clone, Default)]
pub struct HookContext {
    pub env: HashMap<String, String>,
}

impl HookContext {
    /// Set an environment variable in the context.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

/// Runs lifecycle hooks at various points in the release process.
pub trait HookRunner: Send + Sync {
    fn run(&self, hooks: &[HookCommand], ctx: &HookContext) -> Result<(), ReleaseError>;
}

/// Default hook runner that executes commands via the system shell.
pub struct ShellHookRunner;

impl HookRunner for ShellHookRunner {
    fn run(&self, hooks: &[HookCommand], ctx: &HookContext) -> Result<(), ReleaseError> {
        for hook in hooks {
            let status = Command::new("sh")
                .arg("-c")
                .arg(&hook.command)
                .envs(&ctx.env)
                .status()
                .map_err(|e| ReleaseError::Hook {
                    command: format!("{}: {e}", hook.command),
                })?;

            if !status.success() {
                return Err(ReleaseError::Hook {
                    command: hook.command.clone(),
                });
            }
        }
        Ok(())
    }
}
