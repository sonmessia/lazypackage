use lazypackage_core::traits::{BackendError, Result};
use std::process::Output;
use tokio::process::Command;

pub async fn run_command(mut cmd: Command) -> Result<Output> {
    cmd.output()
        .await
        .map_err(|e| BackendError::CommandFailed(e.to_string()))
}
