//! # Module `services`
//!
//! **Use-Case layer** — orchestrates domain objects and ports to fulfil
//! concrete application requirements.
//!
//! ## Two Services
//!
//! | Service                  | Handles                           | Ports used                              |
//! |--------------------------|-----------------------------------|-----------------------------------------|
//! | [`SystemManagerService`] | `InstallSource::SystemManager`    | [`SystemPackageManager`]                |
//! | [`DirectInstallService`] | `RemoteArchive` / `LocalArchive`  | `PackageRepository` + `Downloader` + `ChecksumVerifier` + `Extractor` |
//!
//! The **application layer** (TUI event loop or CLI dispatcher) is responsible
//! for selecting the right service based on `InstallRequest.source`. This keeps
//! each service focused and independently testable.

use crate::domain::{
    ArchivePackage, BackendKind, InstallRequest, InstallSource, Package, PackageError,
};
use crate::traits::{
    ChecksumVerifier, Downloader, Extractor, PackageRepository, SystemPackageManager,
};

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers (pure, no I/O)
// ─────────────────────────────────────────────────────────────────────────────

/// Returns a canonical temp path for a downloaded archive.
///
/// Keeps naming logic in a single place; the actual directory is created
/// by the infrastructure adapter at write time.
fn temp_path_for(package_name: &str) -> String {
    format!("/tmp/lazypackage/{}.archive", package_name)
}

/// Default installation prefix used by [`DirectInstallService`].
const DEFAULT_INSTALL_PREFIX: &str = "/usr/local";

// ─────────────────────────────────────────────────────────────────────────────
// Guard helper — reusable name validation
// ─────────────────────────────────────────────────────────────────────────────

fn validate_name(name: &str) -> Result<(), PackageError> {
    if name.trim().is_empty() {
        Err(PackageError::InvalidArgument(
            "package name must not be empty".to_owned(),
        ))
    } else {
        Ok(())
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SERVICE 1: SystemManagerService
// ═════════════════════════════════════════════════════════════════════════════

/// Thin orchestration wrapper around a [`SystemPackageManager`] port.
///
/// Adds domain-level input validation (e.g. empty-name guard) and a
/// consistent error vocabulary on top of the raw adapter calls.
///
/// ## Why a Wrapper?
///
/// Even though many calls are simple delegations today, the service layer is
/// the right place to add future cross-cutting concerns such as:
///
/// - Logging / tracing (without `tracing` in core — the outer layer injects it)
/// - Pre/post-operation hooks (e.g. refreshing the package list after install)
/// - Retry / circuit-breaker policies
///
/// # Generic Parameters
///
/// | `M` | Bound: [`SystemPackageManager`] — the concrete backend adapter |
pub struct SystemManagerService<'a, M: SystemPackageManager> {
    manager: &'a M,
}

impl<'a, M: SystemPackageManager> SystemManagerService<'a, M> {
    /// Creates a new service wrapping `manager`.
    pub fn new(manager: &'a M) -> Self {
        Self { manager }
    }

    /// Returns the [`BackendKind`] of the underlying manager (synchronous).
    pub fn backend_kind(&self) -> BackendKind {
        self.manager.backend_kind()
    }

    // ── Query operations ──────────────────────────────────────────────────

    /// Lists all packages currently installed on the system.
    pub async fn list_installed(&self) -> Result<Vec<Package>, PackageError> {
        self.manager.list_installed().await
    }

    /// Searches the remote repository for packages matching `query`.
    ///
    /// # Errors
    ///
    /// - [`PackageError::InvalidArgument`] if `query` is blank.
    /// - Any error propagated from the backend.
    pub async fn search(&self, query: &str) -> Result<Vec<Package>, PackageError> {
        if query.trim().is_empty() {
            return Err(PackageError::InvalidArgument(
                "search query must not be empty".to_owned(),
            ));
        }
        self.manager.search(query).await
    }

    // ── Mutating operations ───────────────────────────────────────────────

    /// Installs `name` via the system package manager.
    ///
    /// # Errors
    ///
    /// - [`PackageError::InvalidArgument`] if `name` is blank.
    /// - Any error propagated from the backend.
    pub async fn install(&self, name: &str) -> Result<(), PackageError> {
        validate_name(name)?;
        self.manager.install(name).await
    }

    /// Removes (uninstalls) `name` via the system package manager.
    pub async fn remove(&self, name: &str) -> Result<(), PackageError> {
        validate_name(name)?;
        self.manager.remove(name).await
    }

    /// Upgrades `name` to the latest version available in the repository.
    pub async fn upgrade(&self, name: &str) -> Result<(), PackageError> {
        validate_name(name)?;
        self.manager.upgrade(name).await
    }

    /// Upgrades **all** installed packages that have a newer version available.
    pub async fn upgrade_all(&self) -> Result<(), PackageError> {
        self.manager.upgrade_all().await
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// SERVICE 2: DirectInstallService
// ═════════════════════════════════════════════════════════════════════════════

/// Orchestrates the **direct archive install** use-case.
///
/// Handles both remote archives (download → verify → extract) and local
/// archives (optionally verify → extract). Routes incoming
/// [`InstallRequest`]s based on their [`InstallSource`] variant.
///
/// # Generic Parameters
///
/// | `R` | [`PackageRepository`] — custom registry for metadata & install state |
/// | `D` | [`Downloader`]         — fetches remote archives                     |
/// | `V` | [`ChecksumVerifier`]   — computes SHA-256 digests                    |
/// | `E` | [`Extractor`]          — unpacks archives into the filesystem        |
pub struct DirectInstallService<'a, R, D, V, E>
where
    R: PackageRepository,
    D: Downloader,
    V: ChecksumVerifier,
    E: Extractor,
{
    repo: &'a R,
    downloader: &'a D,
    verifier: &'a V,
    extractor: &'a E,
}

impl<'a, R, D, V, E> DirectInstallService<'a, R, D, V, E>
where
    R: PackageRepository,
    D: Downloader,
    V: ChecksumVerifier,
    E: Extractor,
{
    /// Wires the four required ports together.
    pub fn new(repo: &'a R, downloader: &'a D, verifier: &'a V, extractor: &'a E) -> Self {
        Self {
            repo,
            downloader,
            verifier,
            extractor,
        }
    }

    // ── Public use-case entry point ───────────────────────────────────────

    /// Installs a package using the strategy encoded in `request.source`.
    ///
    /// | `request.source`        | Action                                      |
    /// |-------------------------|---------------------------------------------|
    /// | `SystemManager`         | Returns [`PackageError::InvalidArgument`] — route to [`SystemManagerService`] instead |
    /// | `RemoteArchive`         | Guard → (opt.) deps → download → verify → extract |
    /// | `LocalArchive`          | (opt.) verify → extract                     |
    ///
    /// # Errors
    ///
    /// See the individual route methods for per-source error documentation.
    pub async fn install_package(&self, request: &InstallRequest) -> Result<(), PackageError> {
        validate_name(&request.package_name)?;

        match &request.source {
            InstallSource::SystemManager => Err(PackageError::InvalidArgument(
                "SystemManager install requests must be routed to SystemManagerService".to_owned(),
            )),

            InstallSource::RemoteArchive { url, checksum } => {
                self.install_remote(
                    &request.package_name,
                    url,
                    checksum,
                    request.install_dependencies,
                )
                .await
            }

            InstallSource::LocalArchive {
                path,
                expected_checksum,
            } => {
                self.install_local(&request.package_name, path, expected_checksum.as_deref())
                    .await
            }
        }
    }

    // ── Remote archive route ──────────────────────────────────────────────

    /// Handles `InstallSource::RemoteArchive`.
    ///
    /// ## Steps
    ///
    /// 1. **Guard**: check if already installed via the repository ledger.
    /// 2. **Dependencies** (optional): iterate `ArchivePackage::dependencies`,
    ///    skip those already installed, install the rest.
    /// 3. **Download**: fetch the archive to a temp path.
    /// 4. **Verify checksum**: compare SHA-256 with the expected value.
    /// 5. **Extract**: unpack into `DEFAULT_INSTALL_PREFIX`.
    async fn install_remote(
        &self,
        name: &str,
        url: &str,
        expected_checksum: &str,
        install_deps: bool,
    ) -> Result<(), PackageError> {
        // ── Step 1: Guard — already installed? ───────────────────────────
        if self.repo.is_installed(name).await? {
            return Err(PackageError::AlreadyInstalled(name.to_owned()));
        }

        // ── Step 2: Optional dependency resolution ────────────────────────
        if install_deps {
            // Look up registry metadata to find the dependency list.
            // If the package isn't in the registry, we skip dep resolution
            // (the URL and checksum were supplied directly by the caller).
            if let Some(pkg_meta) = self.repo.find_by_name(name).await? {
                self.install_dependencies(&pkg_meta).await?;
            }
        }

        // ── Steps 3–5: Download → verify → extract ────────────────────────
        let dest = temp_path_for(name);
        self.download_verify_extract(name, url, expected_checksum, &dest)
            .await
    }

    // ── Local archive route ───────────────────────────────────────────────

    /// Handles `InstallSource::LocalArchive`.
    ///
    /// ## Steps
    ///
    /// 1. **Verify checksum** (only if `expected_checksum` is `Some`).
    /// 2. **Extract**: unpack into `DEFAULT_INSTALL_PREFIX`.
    ///
    /// The "already installed" guard is intentionally **omitted** for local
    /// archives: a user choosing a local file may be intentionally
    /// force-reinstalling or downgrading.
    async fn install_local(
        &self,
        name: &str,
        path: &str,
        expected_checksum: Option<&str>,
    ) -> Result<(), PackageError> {
        // ── Step 1: Optional checksum verification ────────────────────────
        if let Some(checksum) = expected_checksum {
            self.verify_checksum(name, path, checksum).await?;
        }

        // ── Step 2: Extract ───────────────────────────────────────────────
        self.do_extract(name, path).await
    }

    // ── Shared pipeline helpers ───────────────────────────────────────────

    /// Installs each dependency in `pkg.dependencies` that is not already
    /// present, using the same download → verify → extract pipeline.
    ///
    /// Transitive (nested) dependencies are **not** resolved here to avoid
    /// infinite recursion on circular metadata. A full dependency-resolution
    /// graph belongs in a future `DependencyResolver` service.
    async fn install_dependencies(&self, pkg: &ArchivePackage) -> Result<(), PackageError> {
        for dep_name in &pkg.dependencies {
            if self.repo.is_installed(dep_name).await? {
                continue; // Already present — skip.
            }

            let dep_meta = self
                .repo
                .find_by_name(dep_name)
                .await?
                .ok_or_else(|| PackageError::PackageNotFound(dep_name.clone()))?;

            let dest = temp_path_for(&dep_meta.name);
            self.download_verify_extract(
                &dep_meta.name,
                &dep_meta.download_url,
                &dep_meta.checksum,
                &dest,
            )
            .await?;
        }
        Ok(())
    }

    /// Core pipeline: download → verify checksum → extract.
    async fn download_verify_extract(
        &self,
        name: &str,
        url: &str,
        expected_checksum: &str,
        dest: &str,
    ) -> Result<(), PackageError> {
        // Download
        self.downloader
            .download(url, dest)
            .await
            .map_err(|e| match e {
                PackageError::NetworkError(msg) => {
                    PackageError::NetworkError(format!("failed to download `{}`: {}", name, msg))
                }
                other => other,
            })?;

        // Verify checksum
        self.verify_checksum(name, dest, expected_checksum).await?;

        // Extract
        self.do_extract(name, dest).await
    }

    /// Computes the SHA-256 digest of `file_path` and compares it to
    /// `expected`. Returns [`PackageError::ChecksumMismatch`] on divergence.
    async fn verify_checksum(
        &self,
        name: &str,
        file_path: &str,
        expected: &str,
    ) -> Result<(), PackageError> {
        let actual = self.verifier.sha256_hex(file_path).await?;
        if actual != expected {
            return Err(PackageError::ChecksumMismatch {
                package: name.to_owned(),
                expected: expected.to_owned(),
                actual,
            });
        }
        Ok(())
    }

    /// Calls the extractor port with a contextualised error message.
    async fn do_extract(&self, name: &str, archive_path: &str) -> Result<(), PackageError> {
        self.extractor
            .extract(archive_path, DEFAULT_INSTALL_PREFIX)
            .await
            .map_err(|e| match e {
                PackageError::ExtractionError(msg) => {
                    PackageError::ExtractionError(format!("failed to extract `{}`: {}", name, msg))
                }
                other => other,
            })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PackageStatus;
    use std::future::Future;

    // ═════════════════════════════════════════════════════════════════════
    // Test doubles (stubs / fakes)
    // ═════════════════════════════════════════════════════════════════════

    // ── SystemPackageManager stubs ────────────────────────────────────────

    /// Always-succeeds system manager.
    struct OkSystemManager {
        kind: BackendKind,
        packages: Vec<Package>,
    }

    impl OkSystemManager {
        fn apt(packages: Vec<Package>) -> Self {
            Self {
                kind: BackendKind::Apt,
                packages,
            }
        }
    }

    impl SystemPackageManager for OkSystemManager {
        fn backend_kind(&self) -> BackendKind {
            self.kind
        }

        fn list_installed(
            &self,
        ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
            let list = self.packages.clone();
            async move { Ok(list) }
        }

        fn search(
            &self,
            query: &str,
        ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
            let results: Vec<Package> = self
                .packages
                .iter()
                .filter(|p| p.name.contains(query))
                .cloned()
                .collect();
            async move { Ok(results) }
        }

        fn install(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }

        fn remove(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }

        fn upgrade(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }

        fn upgrade_all(&self) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }
    }

    /// Always-fails system manager.
    struct ErrSystemManager {
        kind: BackendKind,
        error: PackageError,
    }

    impl ErrSystemManager {
        fn apt_backend_err(msg: &str) -> Self {
            Self {
                kind: BackendKind::Apt,
                error: PackageError::BackendError {
                    backend: "apt".into(),
                    message: msg.to_owned(),
                },
            }
        }
    }

    impl SystemPackageManager for ErrSystemManager {
        fn backend_kind(&self) -> BackendKind {
            self.kind
        }

        fn list_installed(
            &self,
        ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }

        fn search(
            &self,
            _query: &str,
        ) -> impl Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }

        fn install(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }

        fn remove(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }

        fn upgrade(
            &self,
            _name: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }

        fn upgrade_all(&self) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            let err = self.error.clone();
            async move { Err(err) }
        }
    }

    // ── PackageRepository stub ────────────────────────────────────────────

    struct FakeRepo {
        available: Vec<ArchivePackage>,
        installed: Vec<String>,
    }

    impl FakeRepo {
        fn new(available: Vec<ArchivePackage>, installed: Vec<String>) -> Self {
            Self {
                available,
                installed,
            }
        }

        fn empty() -> Self {
            Self::new(vec![], vec![])
        }
    }

    impl PackageRepository for FakeRepo {
        fn find_by_name(
            &self,
            name: &str,
        ) -> impl Future<Output = Result<Option<ArchivePackage>, PackageError>> + Send + '_
        {
            let result = self.available.iter().find(|p| p.name == name).cloned();
            async move { Ok(result) }
        }

        fn is_installed(
            &self,
            name: &str,
        ) -> impl Future<Output = Result<bool, PackageError>> + Send + '_ {
            let found = self.installed.iter().any(|n| n == name);
            async move { Ok(found) }
        }
    }

    // ── Downloader stubs ──────────────────────────────────────────────────

    struct OkDownloader;

    impl Downloader for OkDownloader {
        fn download(
            &self,
            _url: &str,
            _dest_path: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }
    }

    struct ErrDownloader;

    impl Downloader for ErrDownloader {
        fn download(
            &self,
            _url: &str,
            _dest_path: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Err(PackageError::NetworkError("connection refused".into())) }
        }
    }

    // ── ChecksumVerifier stubs ────────────────────────────────────────────

    struct FixedVerifier(String);

    impl ChecksumVerifier for FixedVerifier {
        fn sha256_hex(
            &self,
            _file_path: &str,
        ) -> impl Future<Output = Result<String, PackageError>> + Send + '_ {
            let hex = self.0.clone();
            async move { Ok(hex) }
        }
    }

    // ── Extractor stub ────────────────────────────────────────────────────

    struct OkExtractor;

    impl Extractor for OkExtractor {
        fn extract(
            &self,
            _archive_path: &str,
            _install_path: &str,
        ) -> impl Future<Output = Result<(), PackageError>> + Send + '_ {
            async move { Ok(()) }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    const CHECKSUM: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn make_pkg(name: &str) -> Package {
        Package {
            name: name.to_owned(),
            version: "1.0.0".into(),
            description: "test".into(),
            status: PackageStatus::NotInstalled,
            backend: BackendKind::Apt,
            repo: None,
            size_bytes: None,
        }
    }

    fn make_archive_pkg(name: &str) -> ArchivePackage {
        ArchivePackage {
            name: name.to_owned(),
            version: "1.0.0".into(),
            description: "test".into(),
            download_url: format!("https://example.com/{}.deb", name),
            checksum: CHECKSUM.to_owned(),
            dependencies: vec![],
        }
    }

    // ═════════════════════════════════════════════════════════════════════
    // SystemManagerService Tests
    // ═════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn sys_search_rejects_empty_query() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(matches!(
            svc.search("").await,
            Err(PackageError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn sys_search_rejects_whitespace_query() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(matches!(
            svc.search("   ").await,
            Err(PackageError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn sys_install_rejects_empty_name() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(matches!(
            svc.install("").await,
            Err(PackageError::InvalidArgument(_))
        ));
    }

    #[tokio::test]
    async fn sys_list_installed_returns_packages() {
        let pkgs = vec![make_pkg("vim"), make_pkg("git")];
        let mgr = OkSystemManager::apt(pkgs.clone());
        let svc = SystemManagerService::new(&mgr);
        let result = svc.list_installed().await.unwrap();
        assert_eq!(result, pkgs);
    }

    #[tokio::test]
    async fn sys_search_filters_by_name() {
        let mgr = OkSystemManager::apt(vec![make_pkg("vim"), make_pkg("git")]);
        let svc = SystemManagerService::new(&mgr);
        let result = svc.search("vim").await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "vim");
    }

    #[tokio::test]
    async fn sys_backend_kind_matches_manager() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert_eq!(svc.backend_kind(), BackendKind::Apt);
    }

    #[tokio::test]
    async fn sys_install_delegates_to_backend() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(svc.install("vim").await.is_ok());
    }

    #[tokio::test]
    async fn sys_install_propagates_backend_error() {
        let mgr = ErrSystemManager::apt_backend_err("dpkg lock held");
        let svc = SystemManagerService::new(&mgr);
        assert!(matches!(
            svc.install("vim").await,
            Err(PackageError::BackendError { .. })
        ));
    }

    #[tokio::test]
    async fn sys_remove_delegates_to_backend() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(svc.remove("vim").await.is_ok());
    }

    #[tokio::test]
    async fn sys_upgrade_delegates_to_backend() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(svc.upgrade("vim").await.is_ok());
    }

    #[tokio::test]
    async fn sys_upgrade_all_delegates_to_backend() {
        let mgr = OkSystemManager::apt(vec![]);
        let svc = SystemManagerService::new(&mgr);
        assert!(svc.upgrade_all().await.is_ok());
    }

    // ═════════════════════════════════════════════════════════════════════
    // DirectInstallService Tests
    // ═════════════════════════════════════════════════════════════════════

    fn direct_svc<'a>(
        repo: &'a FakeRepo,
        dl: &'a OkDownloader,
        vf: &'a FixedVerifier,
        ex: &'a OkExtractor,
    ) -> DirectInstallService<'a, FakeRepo, OkDownloader, FixedVerifier, OkExtractor> {
        DirectInstallService::new(repo, dl, vf, ex)
    }

    // ── Source routing ────────────────────────────────────────────────────

    #[tokio::test]
    async fn direct_rejects_system_manager_source() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req = InstallRequest::system("vim").unwrap();
        assert!(matches!(
            svc.install_package(&req).await,
            Err(PackageError::InvalidArgument(_))
        ));
    }

    // ── Remote archive: guard ─────────────────────────────────────────────

    #[tokio::test]
    async fn direct_remote_already_installed_returns_error() {
        let repo = FakeRepo::new(vec![], vec!["vim".into()]);
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("vim", "http://x.com/vim.deb", CHECKSUM, false).unwrap();
        assert!(
            matches!(svc.install_package(&req).await, Err(PackageError::AlreadyInstalled(ref n)) if n == "vim")
        );
    }

    // ── Remote archive: network failure ───────────────────────────────────

    #[tokio::test]
    async fn direct_remote_propagates_network_error() {
        let repo = FakeRepo::empty();
        let dl = ErrDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = DirectInstallService::new(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("vim", "http://x.com/vim.deb", CHECKSUM, false).unwrap();
        assert!(matches!(
            svc.install_package(&req).await,
            Err(PackageError::NetworkError(_))
        ));
    }

    // ── Remote archive: checksum mismatch ─────────────────────────────────

    #[tokio::test]
    async fn direct_remote_checksum_mismatch_returns_error() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier("bad_checksum".into()); // Returns wrong digest
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("vim", "http://x.com/vim.deb", CHECKSUM, false).unwrap();
        assert!(matches!(
            svc.install_package(&req).await,
            Err(PackageError::ChecksumMismatch { .. })
        ));
    }

    // ── Remote archive: happy path ────────────────────────────────────────

    #[tokio::test]
    async fn direct_remote_happy_path() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("vim", "http://x.com/vim.deb", CHECKSUM, false).unwrap();
        assert!(svc.install_package(&req).await.is_ok());
    }

    // ── Remote archive: dependency already installed → skip ────────────────

    #[tokio::test]
    async fn direct_remote_skips_installed_dependency() {
        let mut pkg = make_archive_pkg("openssl");
        pkg.dependencies.push("libssl".into()); // dep declared

        let repo = FakeRepo::new(
            vec![pkg],
            vec!["libssl".into()], // dep already installed
        );
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("openssl", "http://x.com/openssl.deb", CHECKSUM, true)
                .unwrap();
        assert!(svc.install_package(&req).await.is_ok());
    }

    // ── Remote archive: dependency not found in registry ──────────────────

    #[tokio::test]
    async fn direct_remote_missing_dependency_returns_not_found() {
        let mut pkg = make_archive_pkg("openssl");
        pkg.dependencies.push("libssl".into());

        // libssl is NOT in the registry
        let repo = FakeRepo::new(vec![pkg], vec![]);
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("openssl", "http://x.com/openssl.deb", CHECKSUM, true)
                .unwrap();
        assert!(matches!(
            svc.install_package(&req).await,
            Err(PackageError::PackageNotFound(ref n)) if n == "libssl"
        ));
    }

    // ── Remote archive: dependency full happy path ─────────────────────────

    #[tokio::test]
    async fn direct_remote_installs_dependency_then_main_package() {
        let dep = make_archive_pkg("libssl");
        let mut pkg = make_archive_pkg("openssl");
        pkg.dependencies.push("libssl".into());

        let repo = FakeRepo::new(vec![dep, pkg], vec![]); // nothing installed yet
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req =
            InstallRequest::remote_archive("openssl", "http://x.com/openssl.deb", CHECKSUM, true)
                .unwrap();
        assert!(svc.install_package(&req).await.is_ok());
    }

    // ── Local archive: happy path with checksum ────────────────────────────

    #[tokio::test]
    async fn direct_local_with_checksum_happy_path() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier(CHECKSUM.into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req = InstallRequest::local_archive("vim", "/tmp/vim.deb", Some(CHECKSUM.to_owned()))
            .unwrap();
        assert!(svc.install_package(&req).await.is_ok());
    }

    // ── Local archive: happy path without checksum ─────────────────────────

    #[tokio::test]
    async fn direct_local_without_checksum_skips_verification() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier("irrelevant".into()); // Should NOT be called
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req = InstallRequest::local_archive("vim", "/tmp/vim.deb", None).unwrap();
        // If verifier were called, it would return "irrelevant" != CHECKSUM → error.
        // Getting Ok here confirms the verifier was NOT called.
        assert!(svc.install_package(&req).await.is_ok());
    }

    // ── Local archive: checksum mismatch ──────────────────────────────────

    #[tokio::test]
    async fn direct_local_checksum_mismatch_blocks_extraction() {
        let repo = FakeRepo::empty();
        let dl = OkDownloader;
        let vf = FixedVerifier("wrong_digest".into());
        let ex = OkExtractor;
        let svc = direct_svc(&repo, &dl, &vf, &ex);

        let req = InstallRequest::local_archive("vim", "/tmp/vim.deb", Some(CHECKSUM.to_owned()))
            .unwrap();
        assert!(matches!(
            svc.install_package(&req).await,
            Err(PackageError::ChecksumMismatch { .. })
        ));
    }
}
