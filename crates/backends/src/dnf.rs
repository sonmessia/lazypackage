//! # Module `dnf`
//!
//! Infrastructure adapter — concrete implementation of [`SystemPackageManager`]
//! for **Fedora / RHEL / CentOS Stream** using the `dnf` CLI.
//!
//! ## Strategy
//!
//! Every method shells out to `/usr/bin/dnf` (or whatever `dnf` resolves to on
//! `$PATH`) and parses its stdout.  No native library bindings are used; this
//! keeps the build dependency footprint minimal and guarantees compatibility
//! with any DNF version that honours the same CLI surface.
//!
//! ## Privilege Escalation
//!
//! Mutating operations (`install`, `remove`, `upgrade`, `upgrade_all`) require
//! root.  The adapter **prepends `sudo`** automatically, delegating the actual
//! privilege challenge to the system PAM/sudo configuration.  If the caller is
//! already root, `sudo` is a no-op on most systems.
//!
//! ## Output Parsing
//!
//! | Operation        | Command                                              | Parsing strategy                                        |
//! |------------------|------------------------------------------------------|---------------------------------------------------------|
//! | `list_installed` | `dnf list --installed`                               | Skip header, split `name.arch  version  repo`           |
//! | `search`         | `dnf repoquery --queryformat ... <query>` (parallel) | Tab-delimited: `name\tversion\tsummary\treponame`        |
//!
//! ### `list_installed` output format
//!
//! ```text
//! Installed Packages
//! bash.x86_64                          5.2.26-3.fc40          @System
//! vim-enhanced.x86_64                  2:9.1.158-1.fc40       @updates
//! ```
//!
//! Each data line has three whitespace-separated fields:
//! 1. `name.arch` — we strip the `.arch` suffix to get the bare package name.
//! 2. `version`   — kept verbatim (may include epoch prefix `2:`).
//! 3. `repo`      — repository or `@System` for core system packages.
//!
//! ### `search` / `repoquery` output format
//!
//! We use `dnf repoquery` with `--queryformat` to emit machine-readable,
//! tab-delimited lines.  This gives us name, version, and summary **in a
//! single subprocess call** — far cheaper than running `dnf info` per package.
//!
//! ```text
//! bash\t5.2.26-3.fc40\tThe GNU Bourne Again shell\tfedora
//! vim-enhanced\t9.1.158-1.fc40\tA version of the vim editor…\tupdates
//! ```
//!
//! Simultaneously, `dnf list --installed` runs in parallel to build an
//! installed-name set, which is used to set [`PackageStatus`] accurately.

use std::process::Stdio;

use tokio::process::Command;

use lazypackage_core::{BackendKind, Package, PackageError, PackageStatus, SystemPackageManager};

// ─────────────────────────────────────────────────────────────────────────────
// DnfBackend
// ─────────────────────────────────────────────────────────────────────────────

/// Concrete adapter that drives the system `dnf` binary.
///
/// # Construction
///
/// ```rust,ignore
/// let backend = DnfBackend::new();
/// let svc = SystemManagerService::new(&backend);
/// let pkgs = svc.list_installed().await?;
/// ```
///
/// # Thread Safety
///
/// `DnfBackend` is `Send + Sync` — it holds no mutable state.  All state
/// lives on the stack inside each async method call.
#[derive(Debug, Clone, Default)]
pub struct DnfBackend;

impl DnfBackend {
    /// Creates a new [`DnfBackend`] instance.
    ///
    /// This is a zero-cost constructor — no I/O is performed at this point.
    pub fn new() -> Self {
        Self
    }

    // ── Private helpers ───────────────────────────────────────────────────

    /// Constructs a `BackendError` tagged with `"dnf"`.
    fn backend_err(msg: impl Into<String>) -> PackageError {
        PackageError::BackendError {
            backend: "dnf".to_owned(),
            message: msg.into(),
        }
    }

    /// Runs a `dnf` command **without** privilege escalation and returns the
    /// captured stdout on success.
    ///
    /// # Errors
    ///
    /// - [`PackageError::BackendError`] if the subprocess could not be spawned
    ///   or exited with a non-zero status.
    async fn run_dnf(&self, args: &[&str]) -> Result<String, PackageError> {
        let output = Command::new("dnf")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| Self::backend_err(format!("failed to spawn dnf: {e}")))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| Self::backend_err(format!("dnf output is not valid UTF-8: {e}")))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(Self::backend_err(format!(
                "dnf exited with {}: {}",
                output.status,
                stderr.trim()
            )))
        }
    }

    /// Runs a `dnf` command **with `sudo`** and returns the captured stdout.
    ///
    /// If `sudo_password` is `Some`, it is piped to `sudo -S` via stdin so
    /// that non-interactive invocations (TUI, CI) can authenticate without
    /// a controlling terminal.
    ///
    /// Pass `None` when the process already has cached sudo credentials or
    /// is running as root.
    ///
    /// # Errors
    ///
    /// - [`PackageError::PrivilegeError`] if `sudo` is unavailable or the
    ///   password is incorrect.
    /// - [`PackageError::BackendError`] for other failures.
    pub async fn run_dnf_privileged(
        &self,
        args: &[&str],
        sudo_password: Option<&str>,
    ) -> Result<String, PackageError> {
        let mut full_args: Vec<&str> = if sudo_password.is_some() {
            // -S: read password from stdin  -p "": suppress the password prompt
            vec!["-S", "-p", "", "dnf"]
        } else {
            vec!["dnf"]
        };
        full_args.extend_from_slice(args);

        let mut cmd = Command::new("sudo");
        cmd.args(&full_args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if sudo_password.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| PackageError::PrivilegeError(format!("failed to spawn sudo: {e}")))?;

        // Write the password followed by a newline to stdin.
        if let Some(pw) = sudo_password {
            use tokio::io::AsyncWriteExt as _;
            if let Some(stdin) = child.stdin.take() {
                let mut stdin = stdin;
                stdin
                    .write_all(format!("{pw}\n").as_bytes())
                    .await
                    .map_err(|e| {
                        PackageError::PrivilegeError(format!("failed to write sudo password: {e}"))
                    })?;
                // Drop stdin to signal EOF to sudo.
            }
        }

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| PackageError::PrivilegeError(format!("sudo wait failed: {e}")))?;

        if output.status.success() {
            String::from_utf8(output.stdout)
                .map_err(|e| Self::backend_err(format!("dnf output is not valid UTF-8: {e}")))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = stderr.trim();
            if msg.contains("incorrect password")
                || msg.contains("Sorry, try again")
                || msg.contains("Authentication failure")
            {
                return Err(PackageError::PrivilegeError(
                    "incorrect sudo password".to_owned(),
                ));
            }
            if msg.contains("sudo:") || msg.contains("not allowed") {
                return Err(PackageError::PrivilegeError(msg.to_owned()));
            }
            Err(Self::backend_err(format!(
                "dnf exited with {}: {}",
                output.status, msg
            )))
        }
    }
    /// Lists **all** packages available in the configured repositories,
    /// including those not currently installed.
    ///
    /// ## Strategy
    ///
    /// Two subprocesses run concurrently:
    /// 1. `dnf repoquery --queryformat ... --available` — all repo packages.
    /// 2. `dnf list --installed` — installed package names for status tagging.
    ///
    /// Packages that appear in the installed list receive
    /// [`PackageStatus::Installed`]; all others receive
    /// [`PackageStatus::NotInstalled`].
    ///
    /// # Performance note
    ///
    /// This can return tens of thousands of packages and may take several
    /// seconds.  Call it from a background task so the UI remains responsive.
    pub async fn list_all(&self) -> Result<Vec<Package>, PackageError> {
        const FMT: &str = "%{name}\t%{version}-%{release}\t%{summary}\t%{reponame}\n";

        let repoquery_args = [
            "repoquery",
            "--queryformat",
            FMT,
            "--color=never",
            "--available",
        ];
        let list_args = ["list", "--installed", "--color=never"];

        let (repoquery_result, installed_result) =
            tokio::join!(self.run_dnf(&repoquery_args), self.run_dnf(&list_args));

        let installed_names: std::collections::HashSet<String> = match installed_result {
            Ok(stdout) => parse_dnf_list_installed(&stdout, BackendKind::Dnf)
                .into_iter()
                .map(|p| p.name)
                .collect(),
            Err(_) => std::collections::HashSet::new(),
        };

        let repoquery_stdout = repoquery_result.unwrap_or_default();
        let packages = parse_dnf_repoquery(&repoquery_stdout, &installed_names, BackendKind::Dnf);
        Ok(packages)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SystemPackageManager implementation
// ─────────────────────────────────────────────────────────────────────────────

impl SystemPackageManager for DnfBackend {
    // ── Identity ──────────────────────────────────────────────────────────

    fn backend_kind(&self) -> BackendKind {
        BackendKind::Dnf
    }

    // ── Query operations ──────────────────────────────────────────────────

    /// Lists all packages currently installed on the system via
    /// `dnf list --installed`.
    ///
    /// # Output parsing
    ///
    /// The first line ("Installed Packages") is a header and is skipped.
    /// Each subsequent non-empty line is split on whitespace:
    ///
    /// ```text
    /// bash.x86_64   5.2.26-3.fc40   @System
    /// ```
    ///
    /// Field 0 → `name.arch` — the part before the **last** `.` is the name.
    /// Field 1 → version string.
    /// Field 2 → repository (optional, defaults to empty string if missing).
    fn list_installed(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
        async move {
            let stdout = self.run_dnf(&["list", "--installed"]).await?;
            let packages = parse_dnf_list_installed(&stdout, BackendKind::Dnf);
            Ok(packages)
        }
    }

    /// Searches the remote repository for packages matching `query`.
    ///
    /// ## Implementation
    ///
    /// Two subprocesses run **concurrently** via `tokio::join!`:
    ///
    /// 1. `dnf repoquery --queryformat "%{name}\t%{version}-%{release}\t%{summary}\t%{reponame}\n"
    ///    --whatprovides "*<query>*" --color=never`
    ///    — returns name, version, summary, and repo for every matching package
    ///    **from the remote repository** in a single call.
    ///
    /// 2. `dnf list --installed --color=never`
    ///    — returns the set of currently installed package names.
    ///
    /// The two results are merged: packages found in the installed set receive
    /// [`PackageStatus::Installed`]; all others receive [`PackageStatus::NotInstalled`].
    ///
    /// Returns [`PackageError::InvalidArgument`] if `query` is blank.
    fn search(
        &self,
        query: &str,
    ) -> impl std::future::Future<Output = Result<Vec<Package>, PackageError>> + Send + '_ {
        let query = query.to_owned();
        async move {
            if query.trim().is_empty() {
                return Err(PackageError::InvalidArgument(
                    "search query must not be empty".to_owned(),
                ));
            }

            // Build the repoquery format string. Tab-delimited fields:
            //   %{name} \t %{version}-%{release} \t %{summary} \t %{reponame}
            // We use a literal sentinel "\t" tab character as field separator.
            const FMT: &str = "%{name}\t%{version}-%{release}\t%{summary}\t%{reponame}\n";

            // Wrap query in glob wildcards so repoquery matches any package
            // whose name *contains* the query, not just exact-name matches.
            // e.g. "vim" → "*vim*" returns vim, vim-enhanced, vim-common, …
            let glob_query = format!("*{}*", query);

            // Bind args slices to named variables so their lifetimes outlive tokio::join!.
            let repoquery_args = [
                "repoquery",
                "--queryformat",
                FMT,
                "--color=never",
                &glob_query,
            ];
            let list_args = ["list", "--installed", "--color=never"];

            // Run both queries concurrently.
            let (repoquery_result, installed_result) =
                tokio::join!(self.run_dnf(&repoquery_args), self.run_dnf(&list_args),);

            // The installed list is best-effort: if it fails we degrade gracefully
            // (all results show NotInstalled rather than returning an error).
            let installed_names: std::collections::HashSet<String> = match installed_result {
                Ok(installed_stdout) => {
                    parse_dnf_list_installed(&installed_stdout, BackendKind::Dnf)
                        .into_iter()
                        .map(|p| p.name)
                        .collect()
                }
                Err(_) => std::collections::HashSet::new(),
            };

            // Non-zero exit from repoquery with no matches is not an error.
            let repoquery_stdout = repoquery_result.unwrap_or_default();

            let packages =
                parse_dnf_repoquery(&repoquery_stdout, &installed_names, BackendKind::Dnf);
            Ok(packages)
        }
    }

    // ── Mutating operations ───────────────────────────────────────────────

    /// Installs `name` via `sudo dnf install -y <name>`.
    fn install(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), PackageError>> + Send + '_ {
        let name = name.to_owned();
        async move {
            if name.trim().is_empty() {
                return Err(PackageError::InvalidArgument(
                    "package name must not be empty".to_owned(),
                ));
            }

            let stdout = self.run_dnf_privileged(&["install", "-y", &name], None).await?;

            // DNF prints "Nothing to do." when the package is already installed.
            if stdout.contains("Nothing to do") {
                return Err(PackageError::AlreadyInstalled(name));
            }

            // DNF reports "No match for argument: <name>" when not found.
            if stdout.contains("No match for argument") || stdout.contains("Error: Unable to find")
            {
                return Err(PackageError::PackageNotFound(name));
            }

            Ok(())
        }
    }

    /// Removes `name` via `sudo dnf remove -y <name>`.
    fn remove(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), PackageError>> + Send + '_ {
        let name = name.to_owned();
        async move {
            if name.trim().is_empty() {
                return Err(PackageError::InvalidArgument(
                    "package name must not be empty".to_owned(),
                ));
            }

            self.run_dnf_privileged(&["remove", "-y", &name], None).await?;
            Ok(())
        }
    }

    /// Upgrades `name` to the latest available version via
    /// `sudo dnf upgrade -y <name>`.
    fn upgrade(
        &self,
        name: &str,
    ) -> impl std::future::Future<Output = Result<(), PackageError>> + Send + '_ {
        let name = name.to_owned();
        async move {
            if name.trim().is_empty() {
                return Err(PackageError::InvalidArgument(
                    "package name must not be empty".to_owned(),
                ));
            }

            let stdout = self.run_dnf_privileged(&["upgrade", "-y", &name], None).await?;

            // If there is nothing to upgrade, DNF exits 0 and prints "Nothing to do."
            // This is not an error — we treat it as a successful no-op.
            let _ = stdout;

            Ok(())
        }
    }

    /// Upgrades all installed packages via `sudo dnf upgrade -y`.
    fn upgrade_all(
        &self,
    ) -> impl std::future::Future<Output = Result<(), PackageError>> + Send + '_ {
        async move {
            self.run_dnf_privileged(&["upgrade", "-y"], None).await?;
            Ok(())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pure parsing helpers (no I/O — unit-testable in isolation)
// ─────────────────────────────────────────────────────────────────────────────

/// Parses the stdout of `dnf list --installed` into a [`Vec<Package>`].
///
/// Expected format per data line:
/// ```text
/// name.arch   version   repo
/// ```
///
/// The `repo` field may be absent on some DNF versions; it is treated as
/// `None` when missing.
fn parse_dnf_list_installed(stdout: &str, backend: BackendKind) -> Vec<Package> {
    stdout
        .lines()
        // Skip the "Installed Packages" header and any blank lines.
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("Installed Packages")
        })
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name_arch = fields.next()?;
            let version = fields.next()?.to_owned();
            let repo = fields.next().map(|r| r.to_owned());

            // Strip the architecture suffix: "bash.x86_64" → "bash"
            let name = strip_arch_suffix(name_arch).to_owned();

            Some(Package {
                name,
                version,
                description: String::new(), // `dnf list` does not include descriptions.
                status: PackageStatus::Installed,
                backend,
                repo,
                size_bytes: None,
            })
        })
        .collect()
}

/// Parses the stdout of `dnf search --quiet --color=never <query>` into a
/// [`Vec<Package>`].
///
/// Section header lines (starting with `=`) are skipped.  Data lines follow
/// the pattern:
///
/// ```text
/// name.arch : description text
/// ```
///
/// > **Note**: this function is kept for reference but is **no longer called**
/// > in production. The `search()` method now uses [`parse_dnf_repoquery`]
/// > which provides version information and accurate [`PackageStatus`].
#[allow(dead_code)]
fn parse_dnf_search(stdout: &str, backend: BackendKind) -> Vec<Package> {
    stdout
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('=')
        })
        .filter_map(|line| {
            // Split on " : " to separate the "name.arch" part from description.
            let (name_arch_part, description) = line.split_once(" : ")?;
            let name_arch = name_arch_part.trim();
            let description = description.trim().to_owned();

            let name = strip_arch_suffix(name_arch).to_owned();

            Some(Package {
                name,
                version: String::new(), // version is not included in `dnf search` output.
                description,
                status: PackageStatus::NotInstalled,
                backend,
                repo: None,
                size_bytes: None,
            })
        })
        .collect()
}

/// Parses the stdout of
/// `dnf repoquery --queryformat "%{name}\t%{version}-%{release}\t%{summary}\t%{reponame}\n" <query>`
/// into a [`Vec<Package>`], cross-referencing against `installed_names` to set
/// an accurate [`PackageStatus`].
///
/// ## Line format
///
/// Each non-empty output line is tab-delimited with exactly four fields:
///
/// ```text
/// bash\t5.2.26-3.fc40\tThe GNU Bourne Again shell\tfedora
/// ```
///
/// | Field | Content |
/// |-------|---------|
/// | 0     | Package name (no arch suffix) |
/// | 1     | `version-release` string |
/// | 2     | One-line summary / description |
/// | 3     | Repository name (e.g. `"fedora"`, `"updates"`) |
///
/// Lines with fewer than two tab-delimited fields are silently skipped.
///
/// ## Deduplication
///
/// `dnf repoquery` may return the same package from multiple repositories
/// (e.g. `fedora` and `updates`).  Duplicates (same name) are collapsed:
/// the entry from the repository with the highest lexicographic version is
/// kept.  When versions are equal the first occurrence wins.
fn parse_dnf_repoquery(
    stdout: &str,
    installed_names: &std::collections::HashSet<String>,
    backend: BackendKind,
) -> Vec<Package> {
    use std::collections::HashMap;

    // Collect unique packages, keeping the highest version per name.
    let mut seen: HashMap<String, Package> = HashMap::new();

    for line in stdout.lines() {
        // Skip truly blank lines (only whitespace), but do NOT trim the line
        // before splitting — a leading tab means an empty name field, which
        // we must detect and skip below.
        if line.trim().is_empty() {
            continue;
        }

        let mut parts = line.splitn(4, '\t');
        let name = match parts.next() {
            Some(n) if !n.trim().is_empty() => n.trim().to_owned(),
            _ => continue,
        };
        let version = parts.next().unwrap_or("").trim().to_owned();
        let description = parts.next().unwrap_or("").trim().to_owned();
        let repo = parts
            .next()
            .map(|r| r.trim().to_owned())
            .filter(|r| !r.is_empty());

        let status = if installed_names.contains(&name) {
            PackageStatus::Installed
        } else {
            PackageStatus::NotInstalled
        };

        let pkg = Package {
            name: name.clone(),
            version: version.clone(),
            description,
            status,
            backend,
            repo,
            size_bytes: None,
        };

        seen.entry(name)
            .and_modify(|existing| {
                // Keep entry with higher version string (lexicographic is sufficient
                // for `version-release` strings in practice).
                if version > existing.version {
                    *existing = pkg.clone();
                }
            })
            .or_insert(pkg);
    }

    let mut packages: Vec<Package> = seen.into_values().collect();
    // Sort by name for deterministic output.
    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

/// Strips the architecture suffix from a `name.arch` token.
///
/// DNF appends `.x86_64`, `.noarch`, `.aarch64`, etc. to package names.
/// This function removes the last dot-separated component **only when** it
/// looks like a known architecture identifier.
///
/// Known architecture suffixes:
/// `x86_64`, `i686`, `noarch`, `aarch64`, `ppc64le`, `s390x`, `armv7hl`
///
/// If the suffix is not recognised the original string is returned unchanged.
///
/// # Examples
///
/// ```no_run
/// // strip_arch_suffix is a private helper — see unit tests for live examples.
/// // assert_eq!(strip_arch_suffix("bash.x86_64"), "bash");
/// // assert_eq!(strip_arch_suffix("vim-enhanced.noarch"), "vim-enhanced");
/// // assert_eq!(strip_arch_suffix("some-pkg"), "some-pkg");  // no arch → unchanged
/// ```
fn strip_arch_suffix(name_arch: &str) -> &str {
    const KNOWN_ARCHES: &[&str] = &[
        "x86_64", "i686", "i586", "i386", "noarch", "aarch64", "ppc64le", "ppc64", "s390x",
        "armv7hl", "armv6hl", "armhfp",
    ];

    if let Some(dot_pos) = name_arch.rfind('.') {
        let suffix = &name_arch[dot_pos + 1..];
        if KNOWN_ARCHES.contains(&suffix) {
            return &name_arch[..dot_pos];
        }
    }
    name_arch
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── strip_arch_suffix ─────────────────────────────────────────────────

    #[test]
    fn strip_known_arch_x86_64() {
        assert_eq!(strip_arch_suffix("bash.x86_64"), "bash");
    }

    #[test]
    fn strip_known_arch_noarch() {
        assert_eq!(strip_arch_suffix("python3-pip.noarch"), "python3-pip");
    }

    #[test]
    fn strip_known_arch_aarch64() {
        assert_eq!(strip_arch_suffix("vim-enhanced.aarch64"), "vim-enhanced");
    }

    #[test]
    fn strip_unknown_suffix_unchanged() {
        // "fc40" is not an arch — should not be stripped.
        assert_eq!(strip_arch_suffix("some-pkg.fc40"), "some-pkg.fc40");
    }

    #[test]
    fn strip_no_dot_unchanged() {
        assert_eq!(strip_arch_suffix("bash"), "bash");
    }

    // ── parse_dnf_list_installed ──────────────────────────────────────────

    const SAMPLE_LIST_INSTALLED: &str = "\
Installed Packages
bash.x86_64                          5.2.26-3.fc40          @System
vim-enhanced.x86_64                  2:9.1.158-1.fc40       @updates
python3-pip.noarch                   23.3.2-1.fc40          @fedora
kernel.x86_64                        6.8.4-300.fc40         @updates
";

    #[test]
    fn parse_list_installed_count() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        assert_eq!(pkgs.len(), 4);
    }

    #[test]
    fn parse_list_installed_names() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        let names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"vim-enhanced"));
        assert!(names.contains(&"python3-pip"));
        assert!(names.contains(&"kernel"));
    }

    #[test]
    fn parse_list_installed_versions() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.version, "5.2.26-3.fc40");
        // Epoch version (2:) is preserved verbatim.
        let vim = pkgs.iter().find(|p| p.name == "vim-enhanced").unwrap();
        assert_eq!(vim.version, "2:9.1.158-1.fc40");
    }

    #[test]
    fn parse_list_installed_repos() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.repo.as_deref(), Some("@System"));
        let vim = pkgs.iter().find(|p| p.name == "vim-enhanced").unwrap();
        assert_eq!(vim.repo.as_deref(), Some("@updates"));
    }

    #[test]
    fn parse_list_installed_status_is_installed() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| p.status == PackageStatus::Installed));
    }

    #[test]
    fn parse_list_installed_backend_is_dnf() {
        let pkgs = parse_dnf_list_installed(SAMPLE_LIST_INSTALLED, BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| p.backend == BackendKind::Dnf));
    }

    #[test]
    fn parse_list_installed_skips_header() {
        // A header-only string should produce no packages.
        let pkgs = parse_dnf_list_installed("Installed Packages\n", BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn parse_list_installed_empty_input() {
        let pkgs = parse_dnf_list_installed("", BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn parse_list_installed_missing_repo_field() {
        // Some DNF builds omit the repo column.
        let input = "Installed Packages\nbash.x86_64  5.2.26-3.fc40\n";
        let pkgs = parse_dnf_list_installed(input, BackendKind::Dnf);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "bash");
        assert!(pkgs[0].repo.is_none());
    }

    // ── parse_dnf_search ──────────────────────────────────────────────────

    const SAMPLE_SEARCH: &str = "\
============================= Name Exactly Matched: vim ==============================
vim.x86_64 : Vi IMproved - enhanced vi editor
================================= Name Matched: vim ==================================
vim-common.x86_64 : The common files needed by any version of the VIM editor
vim-enhanced.x86_64 : A version of the vim editor which includes recent enhancements
vim-minimal.x86_64 : A minimal version of the VIM editor
";

    #[test]
    fn parse_search_count() {
        let pkgs = parse_dnf_search(SAMPLE_SEARCH, BackendKind::Dnf);
        assert_eq!(pkgs.len(), 4);
    }

    #[test]
    fn parse_search_names() {
        let pkgs = parse_dnf_search(SAMPLE_SEARCH, BackendKind::Dnf);
        let names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"vim"));
        assert!(names.contains(&"vim-common"));
        assert!(names.contains(&"vim-enhanced"));
        assert!(names.contains(&"vim-minimal"));
    }

    #[test]
    fn parse_search_descriptions() {
        let pkgs = parse_dnf_search(SAMPLE_SEARCH, BackendKind::Dnf);
        let vim = pkgs.iter().find(|p| p.name == "vim").unwrap();
        assert_eq!(vim.description, "Vi IMproved - enhanced vi editor");
    }

    #[test]
    fn parse_search_status_is_not_installed() {
        let pkgs = parse_dnf_search(SAMPLE_SEARCH, BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| p.status == PackageStatus::NotInstalled));
    }

    #[test]
    fn parse_search_skips_section_headers() {
        // Headers start with '='; none should appear in package names.
        let pkgs = parse_dnf_search(SAMPLE_SEARCH, BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| !p.name.starts_with('=')));
    }

    #[test]
    fn parse_search_empty_input() {
        let pkgs = parse_dnf_search("", BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn parse_search_no_matches_section_headers_only() {
        let input = "=========================== No matches found ===========================\n";
        let pkgs = parse_dnf_search(input, BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    // ── DnfBackend identity ───────────────────────────────────────────────

    #[test]
    fn backend_kind_is_dnf() {
        let backend = DnfBackend::new();
        assert_eq!(backend.backend_kind(), BackendKind::Dnf);
    }

    #[test]
    fn dnf_backend_is_default_constructible() {
        let _backend: DnfBackend = Default::default();
    }

    // ── parse_dnf_repoquery ───────────────────────────────────────────────

    /// Helper: build an installed set from a slice of name strings.
    fn installed_set(names: &[&str]) -> std::collections::HashSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    const SAMPLE_REPOQUERY: &str = "\
bash\t5.2.26-3.fc40\tThe GNU Bourne Again shell\tfedora\n\
vim-enhanced\t9.1.158-1.fc40\tA version of the vim editor which includes recent enhancements\tupdates\n\
vim-common\t9.1.158-1.fc40\tThe common files needed by any version of the VIM editor\tupdates\n\
python3-pip\t23.3.2-1.fc40\tA tool for installing and managing Python packages\tfedora\n\
";

    #[test]
    fn repoquery_count() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        assert_eq!(pkgs.len(), 4);
    }

    #[test]
    fn repoquery_names_sorted() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        let names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
        // Output must be sorted alphabetically.
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn repoquery_version_extracted() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.version, "5.2.26-3.fc40");
    }

    #[test]
    fn repoquery_description_extracted() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.description, "The GNU Bourne Again shell");
    }

    #[test]
    fn repoquery_repo_extracted() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.repo.as_deref(), Some("fedora"));
        let vim = pkgs.iter().find(|p| p.name == "vim-enhanced").unwrap();
        assert_eq!(vim.repo.as_deref(), Some("updates"));
    }

    #[test]
    fn repoquery_status_installed_when_in_installed_set() {
        let installed = installed_set(&["bash", "vim-common"]);
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed, BackendKind::Dnf);
        let bash = pkgs.iter().find(|p| p.name == "bash").unwrap();
        assert_eq!(bash.status, PackageStatus::Installed);
        let vim_common = pkgs.iter().find(|p| p.name == "vim-common").unwrap();
        assert_eq!(vim_common.status, PackageStatus::Installed);
    }

    #[test]
    fn repoquery_status_not_installed_when_absent_from_installed_set() {
        let installed = installed_set(&["bash"]);
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed, BackendKind::Dnf);
        let vim = pkgs.iter().find(|p| p.name == "vim-enhanced").unwrap();
        assert_eq!(vim.status, PackageStatus::NotInstalled);
    }

    #[test]
    fn repoquery_all_not_installed_with_empty_set() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| p.status == PackageStatus::NotInstalled));
    }

    #[test]
    fn repoquery_deduplication_keeps_higher_version() {
        // Same package appears from two repos: older from "fedora", newer from "updates".
        let input = "vim\t9.0.0-1.fc40\tVi IMproved editor\tfedora\n\
                     vim\t9.1.0-1.fc40\tVi IMproved editor\tupdates\n";
        let pkgs = parse_dnf_repoquery(input, &installed_set(&[]), BackendKind::Dnf);
        assert_eq!(pkgs.len(), 1, "duplicates must be collapsed");
        assert_eq!(pkgs[0].version, "9.1.0-1.fc40", "higher version must win");
        assert_eq!(pkgs[0].repo.as_deref(), Some("updates"));
    }

    #[test]
    fn repoquery_deduplication_same_version_keeps_first() {
        let input = "vim\t9.0.0-1.fc40\tDesc A\trepo-a\n\
                     vim\t9.0.0-1.fc40\tDesc B\trepo-b\n";
        let pkgs = parse_dnf_repoquery(input, &installed_set(&[]), BackendKind::Dnf);
        assert_eq!(pkgs.len(), 1);
        // Version is equal — first occurrence's repo wins.
        assert_eq!(pkgs[0].repo.as_deref(), Some("repo-a"));
    }

    #[test]
    fn repoquery_empty_input() {
        let pkgs = parse_dnf_repoquery("", &installed_set(&[]), BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn repoquery_skips_blank_lines() {
        let input = "\nbash\t5.2.26-3.fc40\tThe GNU Bourne Again shell\tfedora\n\n";
        let pkgs = parse_dnf_repoquery(input, &installed_set(&[]), BackendKind::Dnf);
        assert_eq!(pkgs.len(), 1);
    }

    #[test]
    fn repoquery_skips_lines_with_empty_name() {
        // A line where the name field is empty should be silently skipped.
        let input = "\t9.0.0-1\tDesc\trepo\n";
        let pkgs = parse_dnf_repoquery(input, &installed_set(&[]), BackendKind::Dnf);
        assert!(pkgs.is_empty());
    }

    #[test]
    fn repoquery_handles_missing_repo_field() {
        // Three fields only — repo is None.
        let input = "bash\t5.2.26-3.fc40\tThe GNU Bourne Again shell\n";
        let pkgs = parse_dnf_repoquery(input, &installed_set(&[]), BackendKind::Dnf);
        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name, "bash");
        assert!(pkgs[0].repo.is_none());
    }

    #[test]
    fn repoquery_backend_is_dnf() {
        let pkgs = parse_dnf_repoquery(SAMPLE_REPOQUERY, &installed_set(&[]), BackendKind::Dnf);
        assert!(pkgs.iter().all(|p| p.backend == BackendKind::Dnf));
    }
}
