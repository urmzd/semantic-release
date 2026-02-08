use std::process::Command;

use crate::error::ReleaseError;

/// A shell command to run as a lifecycle hook.
#[derive(Debug, Clone)]
pub struct HookCommand {
    pub command: String,
}

/// Runs lifecycle hooks at various points in the release process.
pub trait HookRunner: Send + Sync {
    fn run(&self, hooks: &[HookCommand]) -> Result<(), ReleaseError>;
}

/// Default hook runner that executes commands via the system shell.
pub struct ShellHookRunner;

impl HookRunner for ShellHookRunner {
    fn run(&self, hooks: &[HookCommand]) -> Result<(), ReleaseError> {
        for hook in hooks {
            let status = Command::new("sh")
                .arg("-c")
                .arg(&hook.command)
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
