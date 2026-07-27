//! # Module `traits`
//!
//! Port definitions (Traits) — the abstract boundary between the pure Domain
//! layer and any concrete Infrastructure implementation.
//!
//! ## Two Groups of Ports
//!
//! The traits here are split into two independent groups that mirror the two
//! installation strategies declared in [`crate::domain::InstallSource`]:
//!
//! ### Group 1 — System Package Manager Ports
//!
//! [`SystemPackageManager`] is a **single unified trait** that abstracts over
//! the native OS package managers (apt, dnf, pacman, zypper). Concrete
//! adapters in the `backends` crate implement this by shelling out to the
//! respective CLI tools.
//!
//! ### Group 2 — Direct Archive Install Ports
//!
//! Four focused traits, each with a single responsibility:
//!
//! | Trait                 | Responsibility                                    |
//! |-----------------------|---------------------------------------------------|
//! | [`PackageRepository`] | Look up `ArchivePackage` metadata from a registry |
//! | [`Downloader`]        | Fetch a remote archive to a local temp path       |
//! | [`ChecksumVerifier`]  | Compute the SHA-256 digest of a local file        |
//! | [`Extractor`]         | Unpack an archive into the installation prefix    |
//!
//! ## Async Design
//!
//! All async methods use **return-position `impl Trait`** (RPIT):
//!
//! ```text
//! fn method(&self) -> impl Future<Output = Result<…>> + Send + '_;
//! ```
//!
//! This is zero-overhead (monomorphised at call sites) and requires no
//! external macro (`async_trait`). The `Send` bound allows the returned
//! future to be spawned on multi-threaded executors such as Tokio.
//!
//! All traits carry `Send + Sync + 'static` super-trait bounds so they can be
//! stored in `Arc<dyn Trait>` or sent across thread boundaries.

use std::future::Future;

use crate::domain::{ArchivePackage, BackendKind, Package, PackageError};

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 1: System Package Manager Port
// ═════════════════════════════════════════════════════════════════════════════

/// **Port**: unified interface over any Linux system package manager.
///
/// Concrete adapters (e.g. `AptBackend`, `DnfBackend`, `PacmanBackend`)
/// in the `backends` crate implement this trait by spawning the appropriate
/// CLI subprocess and parsing its output.
///
/// The core service layer never knows which package manager is active —
/// it only calls methods on this trait.
pub trait SystemPackageManager: Send + Sync + 'static {
    // ── Identity ──────────────────────────────────────────────────────────

    /// Returns the [`BackendKind`] this adapter represents.
    ///
    /// This is a synchronous, infallible method used for display labels
    /// (e.g. showing "apt" or "pacman" in the TUI sidebar).
    fn backend_kind(&self) -> BackendKind;

    // ── Query ─────────────────────────────────────────────────────────────

    /// Lists all packages currently installed on the system.
    ///
    /// # Errors
    ///
    /// - [`PackageError::BackendError`] — the CLI subprocess failed.
    /// - [`PackageError::PrivilegeError`] — insufficient permissions.
    fn list_installed(
        &self,
    ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_;

    /// Searches the remote repository for packages matching `query`.
    ///
    /// The matching strategy (exact name, prefix, fuzzy, …) is
    /// backend-specific.
    ///
    /// # Errors
    ///
    /// - [`PackageError::BackendError`] — the CLI subprocess failed.
    /// - [`PackageError::NetworkError`] — repository metadata could not be
    ///   fetched (e.g. cache stale and network unavailable).
    fn search(
        &self,
        query: &str,
    ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_;

    // ── Mutating operations ───────────────────────────────────────────────

    /// Installs the package named `name`.
    ///
    /// The backend is responsible for dependency resolution, privilege
    /// escalation and any confirmation dialogs.
    ///
    /// # Errors
    ///
    /// - [`PackageError::PackageNotFound`]
    /// - [`PackageError::DependencyConflict`]
    /// - [`PackageError::PrivilegeError`]
    /// - [`PackageError::BackendError`]
    fn install(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<(), PackageError>> + Send + '_;

    /// Removes (uninstalls) the package named `name`.
    ///
    /// # Errors
    ///
    /// Same as [`install`][SystemPackageManager::install].
    fn remove(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<(), PackageError>> + Send + '_;

    /// Upgrades the package named `name` to the latest available version.
    ///
    /// # Errors
    ///
    /// Same as [`install`][SystemPackageManager::install].
    fn upgrade(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<(), PackageError>> + Send + '_;

    /// Upgrades **all** installed packages that have newer versions available.
    ///
    /// Equivalent to `apt upgrade`, `dnf upgrade`, `pacman -Syu`, …
    ///
    /// # Errors
    ///
    /// Same as [`install`][SystemPackageManager::install].
    fn upgrade_all(&self) -> impl Future<Output = Result<(), PackageError>> + Send + '_;
}

// ═════════════════════════════════════════════════════════════════════════════
// GROUP 2: Direct Archive Install Ports
// ═════════════════════════════════════════════════════════════════════════════

// ── Port 2a — PackageRepository ──────────────────────────────────────────────

/// **Port**: abstracts over a custom package registry that stores
/// [`ArchivePackage`] metadata (URLs, checksums, dependency lists).
///
/// Typical implementations: a remote HTTPS JSON API, a local SQLite database,
/// or a static TOML/YAML manifest file bundled with the tool.
pub trait PackageRepository: Send + Sync + 'static {
    /// Returns the full [`ArchivePackage`] metadata for the given `name`, or
    /// `Ok(None)` if no such package exists in the registry.
    ///
    /// Returning `Ok(None)` instead of an error is intentional: the *absence*
    /// of a package is a normal, expected outcome that the service layer maps
    /// to [`PackageError::PackageNotFound`].
    ///
    /// # Errors
    ///
    /// - [`PackageError::NetworkError`] — registry API is unreachable.
    /// - [`PackageError::StorageError`] — local database I/O failure.
    fn find_by_name(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Option<ArchivePackage>, PackageError>> + Send + '_;

    /// Returns `true` if a package with the given `name` is currently
    /// tracked as installed in the registry's installation ledger.
    ///
    /// # Errors
    ///
    /// - [`PackageError::StorageError`] — ledger read failure.
    fn is_installed(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<bool, PackageError>> + Send + '_;
}

// ── Port 2b — Downloader ─────────────────────────────────────────────────────

/// **Port**: abstracts over any file-fetching mechanism.
///
/// The core layer only says *what* to download and *where* to put it.
/// The adapter decides *how* (HTTP client library, `curl` subprocess,
/// torrent, …).
pub trait Downloader: Send + Sync + 'static {
    /// Downloads the resource at `url` and writes it atomically to `dest_path`.
    ///
    /// # Errors
    ///
    /// - [`PackageError::NetworkError`] — connection failure, timeout, or
    ///   non-2xx HTTP response.
    /// - [`PackageError::StorageError`] — cannot create or write `dest_path`.
    fn download(
        &self,
        url: &str,
        dest_path: &str,
    ) -> impl Future<Output = Result<(), PackageError>> + Send + '_;
}

// ── Port 2c — ChecksumVerifier ────────────────────────────────────────────────

/// **Port**: abstracts over digest computation for a file on disk.
///
/// Keeping this as a separate, injectable trait (rather than linking a SHA-256
/// library directly into the core) means the service layer can be tested with
/// a stub that returns a pre-configured digest without touching the filesystem.
pub trait ChecksumVerifier: Send + Sync + 'static {
    /// Computes the SHA-256 hex digest of the file at `file_path`.
    ///
    /// Returns a lowercase 64-character hexadecimal string on success.
    ///
    /// # Errors
    ///
    /// - [`PackageError::StorageError`] — file not found or I/O failure during
    ///   reading.
    fn sha256_hex(
        &self,
        file_path: &str,
    ) -> impl Future<Output = Result<String, PackageError>> + Send + '_;
}

// ── Port 2d — Extractor ───────────────────────────────────────────────────────

/// **Port**: abstracts over unpacking and installing an archive into the
/// system.
///
/// The core layer does not know the archive format (`.deb`, `.rpm`,
/// `.tar.gz`, `.pkg.tar.zst`, …). The adapter in `backends` handles format
/// detection, unpacking, and any post-install hooks.
pub trait Extractor: Send + Sync + 'static {
    /// Unpacks `archive_path` and installs its contents under `install_path`.
    ///
    /// # Errors
    ///
    /// - [`PackageError::ExtractionError`] — corrupt archive, unsupported
    ///   format, or post-install hook failure.
    /// - [`PackageError::StorageError`] — cannot create the target directory.
    /// - [`PackageError::PrivilegeError`] — the target path requires elevated
    ///   permissions.
    fn extract(
        &self,
        archive_path: &str,
        install_path: &str,
    ) -> impl Future<Output = Result<(), PackageError>> + Send + '_;
}
