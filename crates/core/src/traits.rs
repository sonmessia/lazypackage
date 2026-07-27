use crate::domain::{Package, PackageId};
use async_trait::async_trait;

#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("Failed to parse output: {0}")]
    ParseError(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Dependency conflict: {0}")]
    DependencyConflict(String),
    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, BackendError>;

#[async_trait]
pub trait PackageSource: Send + Sync {
    async fn list_installed(&self) -> Result<Vec<Package>>;
    async fn search(&self, query: &str) -> Result<Vec<Package>>;
    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering;
}

#[async_trait]
pub trait Installer: Send + Sync {
    async fn install(&self, id: &PackageId) -> Result<()>;
    async fn remove(&self, id: &PackageId) -> Result<()>;
}

#[async_trait]
pub trait PrivilegeEscalator: Send + Sync {
    async fn run_privileged(&self, cmd: std::process::Command) -> Result<std::process::Output>;
}
