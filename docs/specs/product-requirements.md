# Product Requirements Document (PRD)

## 1. Product Overview

`lazypackage` is a TUI package manager for the Linux operating system. The core goal of the project is to provide a unified, fast, and smooth command-line interface (TUI) to interact with various package managers (dnf, apt, pacman, flatpak, snap, local RPM/DEB, AppImage) within a single terminal window.

## 2. Target Audience

- **Linux power users**: Those who are familiar with and prefer using the terminal.
- **System administrators**: Need to manage multiple machines and packages visually and quickly.
- **Developers**: Need to frequently search for, install, and remove software packages during their workflow.

## 3. Problems to Solve

- Each Linux distribution (distro) has its own package manager (dnf, apt, pacman, etc.) with different command syntaxes, causing confusion and inconvenience.
- GUI package managers (like GNOME Software, Discover) are often slow, consume significant system resources, and are unsuitable for server/headless environments.
- There is currently a lack of a modern, unified TUI tool to efficiently manage all these software backends.

## 4. Functional Requirements

- **FR-001**: List installed software packages from all configured backends.
- **FR-002**: Search for software packages by name and description.
- **FR-003**: Install packages (supports single and batch).
- **FR-004**: Remove packages (supports single and batch).
- **FR-005**: Display package details (version, size, repo, summary/description).
- **FR-006**: Display a list of packages with available updates.
- **FR-007**: Safe privilege escalation via `pkexec` or `sudo` when necessary.
- **FR-008**: Cache query results with a configurable Time-To-Live (TTL) to improve response speed.
- **FR-009**: Support customizable keybindings via a configuration file.
- **FR-010**: Multi-select to perform batch operations.
- **FR-011**: Log panel to display the actual system command running, ensuring transparency.
- **FR-012**: Sidebar to categorize packages by backend or status (installed, update available, etc.).

## 5. Non-Functional Requirements

- **NFR-001**: Startup time < 500ms (excluding backend query time).
- **NFR-002**: UI responsive — the application must not block the UI while the backend is executing heavy/network tasks.
- **NFR-003**: Memory footprint < 50MB when managing lists of up to 10,000 packages.
- **NFR-004**: Terminal compatibility — supports 256 colors, optionally supports Nerd Font.
- **NFR-005**: Graceful degradation — the application continues to function if one backend fails or is unavailable.
- **NFR-006**: Panic recovery — restores normal terminal state when the application crashes.

## 6. Minimum Viable Product (MVP) Scope

- **Supported Backend**: `dnf` only.
- **Features**: List installed packages, search, install, remove, display details (detail view).
- **Defer**: Support for local files, AppImage, multi-backend (supporting multiple backends simultaneously), and config file will be implemented in later phases.

## 7. Extended Scope (Future)

- Add support for backends: `apt`, `pacman`, `flatpak`, `snap`, local RPM/DEB, AppImage.

## 8. Technical Constraints

- **Language**: Rust
- **TUI Interface**: Use `ratatui` + `crossterm`
- **Asynchronous Processing (Async)**: `tokio`
- **Architecture**: Use TEA (The Elm Architecture) pattern. (See details at [architecture/overview.md](../architecture/overview.md))
- **License**: MIT, Author: Chicken_Yi
