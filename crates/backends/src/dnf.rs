use async_trait::async_trait;
use lazypackage_core::domain::{BackendKind, Package, PackageId};
use lazypackage_core::traits::{
    BackendError, Installer, PackageSource, PrivilegeEscalator, Result,
};
use std::sync::Arc;
use tokio::process::Command;

pub struct Dnf {
    escalator: Arc<dyn PrivilegeEscalator>,
}

impl Dnf {
    pub fn new(escalator: Arc<dyn PrivilegeEscalator>) -> Self {
        Self { escalator }
    }
}

#[async_trait]
impl PackageSource for Dnf {
    async fn list_installed(&self) -> Result<Vec<Package>> {
        let mut cmd = Command::new("dnf");
        cmd.arg("list").arg("--installed").arg("-q");

        let output = crate::process::run_command(cmd).await?;
        if !output.status.success() {
            return Err(BackendError::CommandFailed(
                "dnf list installed failed".into(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            if line.contains("Installed Packages") {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let name = parts[0].split('.').next().unwrap_or(parts[0]).to_string();
                let version = parts[1].to_string();
                let repo = parts[2].to_string();

                packages.push(Package {
                    id: PackageId {
                        name,
                        backend: BackendKind::Dnf,
                    },
                    installed_version: Some(version),
                    available_version: None,
                    size_bytes: None,
                    repo: Some(repo),
                    summary: "".to_string(),
                });
            }
        }

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let mut cmd = Command::new("dnf");
        cmd.arg("search").arg("-q").arg(query);

        let output = crate::process::run_command(cmd).await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut packages = Vec::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("Matched")
                || trimmed.starts_with("Name Exactly Matched:")
                || trimmed.starts_with("Name & Summary Matched:")
                || trimmed.starts_with("Installed Packages")
            {
                continue;
            }

            let (name_arch, summary) = if let Some((n, s)) = trimmed.split_once('\t') {
                (n.trim(), s.trim())
            } else if let Some((n, s)) = trimmed.split_once(':') {
                (n.trim(), s.trim())
            } else if let Some((n, s)) = trimmed.split_once("  ") {
                (n.trim(), s.trim())
            } else {
                continue;
            };

            let name = name_arch
                .split('.')
                .next()
                .unwrap_or(name_arch)
                .trim()
                .to_string();

            if !name.is_empty() {
                packages.push(Package {
                    id: PackageId {
                        name,
                        backend: BackendKind::Dnf,
                    },
                    installed_version: None,
                    available_version: None,
                    size_bytes: None,
                    repo: None,
                    summary: summary.to_string(),
                });
            }
        }

        Ok(packages)
    }

    fn compare_versions(&self, a: &str, b: &str) -> std::cmp::Ordering {
        a.cmp(b)
    }
}

#[async_trait]
impl Installer for Dnf {
    async fn install(&self, id: &PackageId) -> Result<()> {
        let mut cmd = std::process::Command::new("dnf");
        cmd.arg("install").arg("-y").arg(&id.name);

        let output = self.escalator.run_privileged(cmd).await?;
        if !output.status.success() {
            return Err(BackendError::CommandFailed(
                "Failed to install package".into(),
            ));
        }
        Ok(())
    }

    async fn remove(&self, id: &PackageId) -> Result<()> {
        let mut cmd = std::process::Command::new("dnf");
        cmd.arg("remove").arg("-y").arg(&id.name);

        let output = self.escalator.run_privileged(cmd).await?;
        if !output.status.success() {
            return Err(BackendError::CommandFailed(
                "Failed to remove package".into(),
            ));
        }
        Ok(())
    }
}
