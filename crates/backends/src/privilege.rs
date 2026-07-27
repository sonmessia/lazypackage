use async_trait::async_trait;
use lazypackage_core::traits::{PrivilegeEscalator, Result};
use std::process::Output;

pub struct PkexecEscalator;

#[async_trait]
impl PrivilegeEscalator for PkexecEscalator {
    async fn run_privileged(&self, cmd: std::process::Command) -> Result<Output> {
        let t_cmd = tokio::process::Command::from(cmd);

        let mut pkexec = tokio::process::Command::new("pkexec");
        pkexec.arg(t_cmd.as_std().get_program());
        pkexec.args(t_cmd.as_std().get_args());

        crate::process::run_command(pkexec).await
    }
}

pub struct SudoEscalator;

#[async_trait]
impl PrivilegeEscalator for SudoEscalator {
    async fn run_privileged(&self, cmd: std::process::Command) -> Result<Output> {
        let t_cmd = tokio::process::Command::from(cmd);

        let mut sudo = tokio::process::Command::new("sudo");
        sudo.arg("--non-interactive");
        sudo.arg(t_cmd.as_std().get_program());
        sudo.args(t_cmd.as_std().get_args());

        crate::process::run_command(sudo).await
    }
}
