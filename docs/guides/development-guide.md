# lazypackage Development Guide

## 1. Project Overview

`lazypackage` is a terminal-based user interface (TUI) package manager for Linux, written in Rust. The project focuses on interactivity, speed, and user experience, allowing users to search, install, update, and remove software packages easily and intuitively right from the terminal.

Tech stack:

- **Language:** Rust
- **Architecture:** The Elm Architecture (TEA)
- **TUI Framework:** `ratatui` combined with `crossterm`
- **Async:** `tokio`
- **Error Handling:** `thiserror` (for core/backends) and `anyhow` (for the main binary)

## 2. Source Code Structure

The project is split into multiple crates within a Cargo workspace to ensure clear module boundaries:

- **core:** Contains core logic and shared definitions.
  - `domain.rs`: Domain data structures (Package, PackageStatus).
  - `traits.rs`: Interfaces/traits defining contracts between components.
  - `action.rs`: Action (event) definitions for the TEA architecture.

- **backends:** Implements specific package managers.
  - `dnf.rs`: Integration with the DNF package manager (Fedora/RHEL).
  - `local.rs`: Manages locally installed packages.
  - `appimage.rs`: Manages AppImage files.
  - `process.rs`: Utilities for executing system subcommands.
  - `privilege.rs`: Handles privilege escalation (pkexec, sudo).
  - `cache.rs`: Static caching system for package information.

- **tui:** Handles the entire user interface, never calls backends directly.
  - `layout.rs`: Arranges display areas on screen.
  - `theme.rs`: Defines colors and interface styling.
  - `components/*`: Individual UI widgets (Package List, Details, Status Bar).
  - `update.rs`: Updates application state based on received Actions.

- **binary:** Application entry point.
  - `main.rs`: Wires everything together, initializes tokio runtime, sets up the terminal, and runs the main event loop.

## 3. Data Flow

The application follows the TEA architecture, where the interface is a pure function of state. Here is the data flow when a user presses `i` to install a package:

1. `crossterm` captures the keyboard event (KeyEvent for the `i` key).
2. The event is sent to an `mpsc` channel (multi-producer, single-consumer).
3. The main loop receives the event, creates `Action::InstallRequested(package_id)`, and passes it to the `update()` function.
4. `update()` processes this Action, updates the UI state (e.g., displays "Installing..."), and returns `Command::RunBackend(backend_task)`.
5. `main.rs` (runtime) checks the returned Command and spawns a `tokio task` to call the corresponding backend's `Installer::install()`.
6. When the backend completes, the result is sent back to the main loop via the mpsc channel as `Action::BackendResult(Result)`.
7. `update()` receives `Action::BackendResult`, updates `AppState` again (changes package status to "Installed" or displays an error).
8. `render()` is called with the latest `AppState` to redraw the entire UI.

## 4. Dependency Management

Key dependencies:

- **ratatui + crossterm:** Provides powerful tools for building TUI interfaces with cross-platform terminal interaction.
- **tokio:** Async runtime, essential for keeping the UI responsive when system commands take a long time.
- **async-trait:** Enables defining traits with async functions.
- **thiserror:** Helps define custom error types safely and readably for core and backends.
- **anyhow:** Flexible and convenient error handling at the application level (binary).
- **serde + toml:** Used for parsing and managing configuration files.
- **insta:** Used for snapshot testing, ensuring UI and logic don't change unexpectedly.

## 5. Patterns & Conventions

- **The Elm Architecture (TEA):** All state changes go through `update(state, action) -> (state, command)`. UI does not directly execute side-effect logic.
- **No derived data storage:** Don't store data that can be computed from base state to avoid data inconsistency.
- **Cached\<S\> Decorator Pattern:** Used to wrap a cache around slow backend calls.
- **PrivilegeEscalator Injection:** Allows injecting the privilege escalation method (sudo, pkexec) into backends instead of hardcoding.
- **Component Trait:** Defines a common standard for UI widgets, making it easy to create new ones and reuse code.
- **Adding New Widgets:** To add a new widget, implement the `Component` trait, add render logic to `layout.rs`, and add event handling in `update.rs` if needed.

## 6. Debug & Troubleshooting

- **Debugging TUI apps:** Since `stdout` is used by `ratatui` to draw the interface, you cannot use `println!`. Instead, use logging libraries (like `tracing` or `log`) to write logs to a separate file (e.g., `lazypackage.log`).
- **Testing without real backends:** You can provide mock structs implementing `BackendTrait` to test the interface without changing the real system configuration.
- **UI display issues:** If the interface breaks, check the constraints in `layout.rs` (Constraint). If keys don't work, check the event reception flow from `crossterm`.

## 7. Build & Release

- Build the release version (highest optimization):

  ```bash
  cargo build --release
  ```

- The executable (binary) will be created at: `target/release/lazypackage`
- **Future direction:** The project will provide pre-built packages for major Linux distributions like Fedora (RPM), Debian (DEB), and support via AUR for Arch Linux.
