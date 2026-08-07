//! # lazypackage-backends
//!
//! Infrastructure adapters — concrete implementations of the port traits
//! defined in [`lazypackage_core`].
//!
//! ## Available Adapters
//!
//! | Module         | Struct         | Backend                          |
//! |----------------|----------------|----------------------------------|
//! | [`dnf`]        | [`DnfBackend`] | Fedora / RHEL / CentOS Stream    |
//!
//! More adapters (APT, Pacman, Zypper) will be added here as separate modules.

pub mod dnf;

pub use dnf::DnfBackend;
