#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId {
    pub name: String,
    pub backend: BackendKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Dnf,
    LocalFile,
    AppImage,
    // Future expansions:
    // Apt,
    // Pacman,
    // Flatpak,
    // Snap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchScope {
    Local,
    Dnf,
}

impl Default for SearchScope {
    fn default() -> Self {
        Self::Local
    }
}

#[derive(Debug, Clone)]
pub struct Package {
    pub id: PackageId,
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub size_bytes: Option<u64>,
    pub repo: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    Installed,
    UpgradeAvailable,
    NotInstalled,
}

impl Package {
    pub fn status(&self) -> PackageStatus {
        match (&self.installed_version, &self.available_version) {
            (Some(i), Some(a)) if i != a => PackageStatus::UpgradeAvailable,
            (Some(_), _) => PackageStatus::Installed,
            (None, _) => PackageStatus::NotInstalled,
        }
    }
}
