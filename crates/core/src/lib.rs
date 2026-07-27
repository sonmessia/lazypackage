//! # lazypackage-core
//!
//! The pure Domain / Core layer of the `lazypackage` package-manager tool.
//!
//! ## Architecture
//!
//! This crate implements the innermost ring of a **Hexagonal (Clean)
//! Architecture** and is the single source of truth for:
//!
//! - **Domain models** — `Package`, `ArchivePackage`, `InstallRequest`, …
//! - **Port definitions** (Traits) — abstract interfaces that Infrastructure
//!   adapters must implement
//! - **Use-case services** — orchestration logic that drives the ports
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                      Infrastructure                          │
//! │   (crates/backends: AptBackend, DnfBackend, PacmanBackend,   │
//! │    HttpDownloader, TarExtractor, Sha256Verifier, …)          │
//! │   ┌──────────────────────────────────────────────────────┐   │
//! │   │                 Application / TUI                    │   │
//! │   │         (crates/tui: ratatui rendering, events)      │   │
//! │   │   ┌──────────────────────────────────────────────┐   │   │
//! │   │   │             lazypackage-core   ◄─────────────────┤   │
//! │   │   │   domain  │  traits  │  services             │   │   │
//! │   │   └──────────────────────────────────────────────┘   │   │
//! │   └──────────────────────────────────────────────────────┘   │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Hybrid Installation Strategy
//!
//! `lazypackage` supports two complementary installation paths, both
//! first-class citizens of the domain:
//!
//! | Path | Description | Key Types |
//! |------|-------------|-----------|
//! | **System PM** | Delegates to `apt`, `dnf`, `pacman`, `zypper` | [`SystemManagerService`], [`SystemPackageManager`] |
//! | **Direct archive** | Download → SHA-256 verify → extract `.deb`/`.rpm` | [`DirectInstallService`], [`Downloader`], [`Extractor`] |
//!
//! The [`InstallSource`] enum in an [`InstallRequest`] tells the application
//! layer which path to route the request to.
//!
//! ## Dependency Rules
//!
//! - **No I/O**: this crate never touches the network, filesystem, or terminal.
//! - **No runtime**: no direct Tokio/async-std imports. Async is expressed
//!   via `std::future::Future` bounds on trait methods.
//! - **No panics**: `unwrap()` / `expect()` are forbidden outside `#[cfg(test)]`.
//!   All fallibility flows through `Result<T, PackageError>`.

pub mod domain;
pub mod services;
pub mod traits;

// ── Domain re-exports ─────────────────────────────────────────────────────────
pub use domain::{
    ArchivePackage, BackendKind, InstallRequest, InstallSource, Package, PackageError,
    PackageStatus,
};

// ── Trait (Port) re-exports ───────────────────────────────────────────────────
pub use traits::{
    ChecksumVerifier, Downloader, Extractor, PackageRepository, SystemPackageManager,
};

// ── Service (Use-Case) re-exports ─────────────────────────────────────────────
pub use services::{DirectInstallService, SystemManagerService};
