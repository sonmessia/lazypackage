# Domain Model — lazypackage

> **Living Document**

The data model keeps versions as raw strings — package status is always **recomputed** (computed), not stored separately.

## Domain model (`core::domain`)

```rust
pub struct PackageId {
    pub name: String,
    pub backend: BackendKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BackendKind { Dnf, LocalFile, AppImage }
// expand later: Apt, Pacman, Flatpak, Snap

pub struct Package {
    pub id: PackageId,
    pub installed_version: Option<String>, // keep as raw string
    pub available_version: Option<String>, // rpm/deb versions don't share the same comparison algorithm
    pub size_bytes: Option<u64>,
    pub repo: Option<String>,
    pub summary: String,
}

pub enum PackageStatus { Installed, UpgradeAvailable, NotInstalled }

impl Package {
    pub fn status(&self) -> PackageStatus {
        match (&self.installed_version, &self.available_version) {
            (Some(i), Some(a)) if i != a => PackageStatus::UpgradeAvailable,
            (Some(_), _) => PackageStatus::Installed,
            (None, _) => PackageStatus::NotInstalled,
        }
    }
}
```

> **Version Note:** `rpmvercmp` (rpm) and `dpkg --compare-versions` (deb) use different algorithms. **Do not implement a generic `Ord` for versions in `core`**. Comparison is provided by each backend via `PackageSource::compare_versions`.

---

## Trait boundaries (`core::traits`)

```rust
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
```

### Design Notes

- **`dnf`** implements both `PackageSource` + `Installer`.
- **`local`** only implements `Installer` (no "list by repo").
- **`appimage`** needs a separate trait for desktop integration (`.desktop` entry, icon) — do not force it into `PackageSource`.
- Requires **`#[async_trait]`** (or manually `Pin<Box<dyn Future>>`) because raw `async fn` in traits is **not dyn-compatible** — mandatory for `Vec<Box<dyn PackageSource>>` when merging multiple backends at once.

---

## Cache Decorator

Do not modify original backend code — wrap it with an outer cache layer:

```rust
pub struct Cached<S: PackageSource> {
    inner: S,
    ttl: Duration,
}

impl<S: PackageSource> PackageSource for Cached<S> {
    // cache list_installed/search, invalidate after ttl
}
```

---

## Privilege escalation is a separate seam

Do not call `sudo`/`pkexec` directly in `dnf.rs`/`local.rs` — accept `Arc<dyn PrivilegeEscalator>` via constructor.

> **Security:** Do not keep passwords in application memory; let `pkexec`/polkit handle the prompt itself.
