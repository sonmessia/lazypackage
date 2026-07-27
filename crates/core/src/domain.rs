//! # Module `domain`
//!
//! Pure domain models and error types for the package-management core.
//!
//! ## Two Package Concepts
//!
//! This module distinguishes between two fundamentally different representations
//! of a "package":
//!
//! | Type            | Used for                                      |
//! |-----------------|-----------------------------------------------|
//! | [`Package`]     | Results from `apt`/`dnf`/`pacman` queries    |
//! | [`ArchivePackage`] | Metadata from a custom registry for direct archive installation |
//!
//! This separation avoids polluting the system-PM model with fields that are
//! only meaningful for direct-archive installs (download URL, checksum, …)
//! and vice versa.

use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// BackendKind — which system package manager owns the package
// ─────────────────────────────────────────────────────────────────────────────

/// Identifies which system package-manager backend owns a package.
///
/// Each variant corresponds to a concrete adapter in the `backends` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendKind {
    Apt,
    Dnf,
    Pacman,
    Zypper,
}

impl BackendKind {
    /// Human-readable identifier used in UI labels and log messages.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PackageStatus — installation state on the current machine
// ─────────────────────────────────────────────────────────────────────────────

/// Installation state of a package as reported by the system package manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PackageStatus {
    /// Package is installed and up-to-date.
    Installed,
    /// Package is installed, but a newer version is available in the repository.
    UpgradeAvailable,
    /// Package is not present on the system.
    NotInstalled,
}

impl fmt::Display for PackageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Installed => "installed",
            Self::UpgradeAvailable => "upgrade available",
            Self::NotInstalled => "not installed",
        };
        f.write_str(s)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Package — system-managed package (apt / dnf / pacman / zypper)
// ─────────────────────────────────────────────────────────────────────────────

/// A package as reported by a system package manager.
///
/// This is the **primary display type** used in the TUI's package list.
/// It does not carry installation URLs or checksums — those concerns belong
/// to the system package manager or to [`ArchivePackage`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// Human-readable package name (e.g. `"vim"`).
    pub name: String,

    /// Version string as reported by the backend (e.g. `"9.1.2-1"`).
    pub version: String,

    /// Short one-line description for the TUI details panel.
    pub description: String,

    /// Current installation state on this machine.
    pub status: PackageStatus,

    /// Which system package manager owns this package.
    pub backend: BackendKind,

    /// Repository or channel the package originates from (e.g. `"universe"`).
    pub repo: Option<String>,

    /// Installed or download size in bytes, if known.
    pub size_bytes: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// ArchivePackage — package metadata from a custom / standalone registry
// ─────────────────────────────────────────────────────────────────────────────

/// Metadata for a package held in a **custom (non-OS) registry**.
///
/// Used exclusively by the **direct archive install** path
/// (`InstallSource::RemoteArchive` / `InstallSource::LocalArchive`).
/// The infrastructure layer fetches this from a registry manifest or API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePackage {
    /// Unique name used to look up the package.
    pub name: String,

    /// Semantic version string (e.g. `"1.2.3"`).
    pub version: String,

    /// Short description.
    pub description: String,

    /// URL from which the archive can be downloaded (`.deb`, `.rpm`, `.tar.gz`, …).
    pub download_url: String,

    /// Expected SHA-256 hex digest of the downloaded archive (lowercase, 64 chars).
    pub checksum: String,

    /// Names of other [`ArchivePackage`]s that must be installed first.
    pub dependencies: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// InstallSource — "how should this package be installed?"
// ─────────────────────────────────────────────────────────────────────────────

/// The installation strategy chosen by the UI or CLI layer.
///
/// The core's service layer routes the install workflow based on this value:
///
/// | Variant          | Routed to          | Flow                                    |
/// |------------------|--------------------|-----------------------------------------|
/// | `SystemManager`  | `SystemManagerService` | `apt install` / `dnf install` / … |
/// | `RemoteArchive`  | `DirectInstallService` | download → verify SHA-256 → extract |
/// | `LocalArchive`   | `DirectInstallService` | (opt. verify SHA-256) → extract     |
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallSource {
    /// Delegate entirely to the OS package manager.
    /// The backend (`AptBackend`, `DnfBackend`, …) handles resolution,
    /// download and installation internally.
    SystemManager,

    /// Download an archive from `url`, verify it's SHA-256 `checksum`,
    /// then extract/install it.
    RemoteArchive {
        /// Download URL of the archive file.
        url: String,
        /// Expected to lowercase 64-character SHA-256 hex digest.
        checksum: String,
    },

    /// Install from an archive that already exists on disk.
    ///
    /// If `expected_checksum` is `Some`, integrity is verified before
    /// extraction. Pass `None` only when you trust the local file source
    /// (e.g. a package produced by your own build pipeline).
    LocalArchive {
        /// Absolute path to the archive on disk.
        path: String,
        /// Optional expected SHA-256 hex digest for integrity verification.
        expected_checksum: Option<String>,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// InstallRequest — validated value object from UI / CLI
// ─────────────────────────────────────────────────────────────────────────────

/// A validated install request emitted by the UI or CLI layer.
///
/// Always construct through one of the provided constructors so that
/// invariants (non-empty name) are enforced at the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallRequest {
    /// Name of the package to install (non-empty).
    pub package_name: String,

    /// Describes where and how to obtain the package.
    pub source: InstallSource,

    /// When `true` and the source supports it, direct dependencies are
    /// resolved and installed before the main package.
    ///
    /// Meaningful only for `RemoteArchive` (where the registry carries
    /// dependency metadata). Ignored for `SystemManager` (the OS backend
    /// handles dependency resolution itself) and `LocalArchive`.
    pub install_dependencies: bool,
}

impl InstallRequest {
    // ── Constructors ──────────────────────────────────────────────────────

    /// General constructor — validates `package_name` and returns an error
    /// if it is blank.
    pub fn new(
        package_name: impl Into<String>,
        source: InstallSource,
        install_dependencies: bool,
    ) -> Result<Self, PackageError> {
        let package_name = package_name.into();
        if package_name.trim().is_empty() {
            return Err(PackageError::InvalidArgument(
                "package name must not be empty".to_owned(),
            ));
        }
        Ok(Self {
            package_name,
            source,
            install_dependencies,
        })
    }

    /// Convenience constructor for a **system package manager** install.
    ///
    /// Dependency resolution is handled by the OS backend — the
    /// `install_dependencies` flag is set to `false` and ignored at
    /// the service level.
    pub fn system(name: impl Into<String>) -> Result<Self, PackageError> {
        Self::new(name, InstallSource::SystemManager, false)
    }

    /// Convenience constructor for a **remote archive** install.
    pub fn remote_archive(
        name: impl Into<String>,
        url: impl Into<String>,
        checksum: impl Into<String>,
        install_dependencies: bool,
    ) -> Result<Self, PackageError> {
        Self::new(
            name,
            InstallSource::RemoteArchive {
                url: url.into(),
                checksum: checksum.into(),
            },
            install_dependencies,
        )
    }

    /// Convenience constructor for a **local archive** install.
    ///
    /// Pass `expected_checksum = Some(hex)` to enable integrity verification.
    pub fn local_archive(
        name: impl Into<String>,
        path: impl Into<String>,
        expected_checksum: Option<String>,
    ) -> Result<Self, PackageError> {
        Self::new(
            name,
            InstallSource::LocalArchive {
                path: path.into(),
                expected_checksum,
            },
            false,
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PackageError — exhaustive error taxonomy
// ─────────────────────────────────────────────────────────────────────────────

/// All error conditions that can arise during package-management operations.
///
/// Each variant maps to a specific, recoverable (or informative) failure mode.
/// Outer layers (TUI, CLI) match on these variants to display actionable
/// messages without exposing raw OS error text.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackageError {
    /// The downloaded file's SHA-256 digest does not match the expected value.
    /// The archive is discarded; the system is never written to.
    #[error("checksum mismatch for `{package}`: expected `{expected}`, got `{actual}`")]
    ChecksumMismatch {
        package: String,
        expected: String,
        actual: String,
    },

    /// No package with the given name could be found in any known source.
    #[error("package `{0}` was not found")]
    PackageNotFound(String),

    /// The package conflicts with one or more already-installed packages.
    #[error("dependency conflict for `{package}`: {reason}")]
    DependencyConflict { package: String, reason: String },

    /// A filesystem or database operation failed.
    #[error("storage error: {0}")]
    StorageError(String),

    /// A network or download operation failed.
    #[error("network error: {0}")]
    NetworkError(String),

    /// An archive could not be extracted or a file could not be installed.
    #[error("extraction error: {0}")]
    ExtractionError(String),

    /// A privilege-escalation step (sudo/pkexec) failed.
    #[error("privilege error: {0}")]
    PrivilegeError(String),

    /// The system package manager exited with a non-zero status or produced
    /// unexpected output.
    #[error("{backend} backend error: {message}")]
    BackendError { backend: String, message: String },

    /// The caller supplied an invalid argument (empty name, malformed URL, …).
    #[error("invalid argument: {0}")]
    InvalidArgument(String),

    /// Package is already installed; callers may treat this as a no-op or
    /// present a prompt to the user.
    #[error("package `{0}` is already installed")]
    AlreadyInstalled(String),

    /// Catch-all for unexpected conditions that do not fit a structured variant.
    #[error("unknown error: {0}")]
    Unknown(String),
}

// Compile-time assertion that PackageError implements std::fmt::Display.
const _: () = {
    fn _assert_display<T: fmt::Display>() {}
    fn _check() {
        _assert_display::<PackageError>();
    }
};

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── InstallRequest constructors ───────────────────────────────────────

    #[test]
    fn install_request_rejects_empty_name() {
        assert!(matches!(
            InstallRequest::system(""),
            Err(PackageError::InvalidArgument(_))
        ));
    }

    #[test]
    fn install_request_rejects_whitespace_only_name() {
        assert!(matches!(
            InstallRequest::system("   "),
            Err(PackageError::InvalidArgument(_))
        ));
    }

    #[test]
    fn install_request_system_convenience() {
        let req = InstallRequest::system("vim").unwrap();
        assert_eq!(req.package_name, "vim");
        assert_eq!(req.source, InstallSource::SystemManager);
        assert!(!req.install_dependencies);
    }

    #[test]
    fn install_request_remote_archive_convenience() {
        let req =
            InstallRequest::remote_archive("vim", "https://example.com/vim.deb", "abc123", true)
                .unwrap();
        assert!(matches!(req.source, InstallSource::RemoteArchive { .. }));
        assert!(req.install_dependencies);
    }

    #[test]
    fn install_request_local_archive_with_checksum() {
        let req = InstallRequest::local_archive("vim", "/tmp/vim.deb", Some("abc123".to_owned()))
            .unwrap();
        assert!(
            matches!(&req.source, InstallSource::LocalArchive { expected_checksum: Some(cs), .. } if cs == "abc123")
        );
    }

    #[test]
    fn install_request_local_archive_without_checksum() {
        let req = InstallRequest::local_archive("vim", "/tmp/vim.deb", None).unwrap();
        assert!(matches!(
            &req.source,
            InstallSource::LocalArchive {
                expected_checksum: None,
                ..
            }
        ));
    }

    // ── BackendKind ───────────────────────────────────────────────────────

    #[test]
    fn backend_kind_display_names() {
        assert_eq!(BackendKind::Apt.to_string(), "apt");
        assert_eq!(BackendKind::Dnf.to_string(), "dnf");
        assert_eq!(BackendKind::Pacman.to_string(), "pacman");
        assert_eq!(BackendKind::Zypper.to_string(), "zypper");
    }

    // ── PackageStatus ─────────────────────────────────────────────────────

    #[test]
    fn package_status_display() {
        assert_eq!(PackageStatus::Installed.to_string(), "installed");
        assert_eq!(
            PackageStatus::UpgradeAvailable.to_string(),
            "upgrade available"
        );
        assert_eq!(PackageStatus::NotInstalled.to_string(), "not installed");
    }

    // ── PackageError ──────────────────────────────────────────────────────

    #[test]
    fn package_error_checksum_mismatch_display_contains_all_fields() {
        let err = PackageError::ChecksumMismatch {
            package: "vim".into(),
            expected: "abc123".into(),
            actual: "def456".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("vim"));
        assert!(msg.contains("abc123"));
        assert!(msg.contains("def456"));
    }

    #[test]
    fn package_error_backend_error_display_contains_backend_name() {
        let err = PackageError::BackendError {
            backend: "apt".into(),
            message: "lock file held by another process".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("apt"));
        assert!(msg.contains("lock file"));
    }
}
