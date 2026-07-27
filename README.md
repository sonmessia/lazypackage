# 📦 lazypackage

**A fast and lightweight terminal Linux package manager**

[![CI](https://img.shields.io/badge/CI-TODO-blue.svg)](#)
[![Crates.io](https://img.shields.io/badge/crates.io-TODO-orange.svg)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## 📖 Overview

**lazypackage** is a TUI-based (Terminal User Interface) package manager supporting multiple backends (dnf, apt, pacman, flatpak, snap, local RPM/DEB, AppImage).

Built with Rust, the project adopts **The Elm Architecture (TEA)** pattern for state management, providing a predictable, secure, and user-friendly experience right in the Linux terminal.

## ✨ Key Features

- 🔄 **Multi-backend package management**: Supports dnf, local files, AppImage (expanding to apt, pacman, flatpak, snap).
- 🖥️ **Modern TUI**: Built with `ratatui` for a smooth and fast experience.
- 🏗️ **TEA Architecture**: Centralized state management, clear separation between UI and backend logic, easily testable.
- ⚡ **Smart Cache**: Built-in caching mechanism with configurable TTL to improve response times.
- 🔐 **Multiple Privilege Escalation Methods**: Securely supports `pkexec` or `sudo` for operations requiring root privileges.
- ⌨️ **Customizable Keybindings**: Fully configurable via the `config.toml` file.
- 📦 **Multi-select**: Select multiple packages with Space for batch installation or removal.

## 🖼️ Screenshots

<!-- TODO: Add screenshot/demo GIF when TUI is complete -->

_Coming soon..._

## 🛠️ Installation

**System Requirements:**

- Rust 1.75+
- (Recommended) Nerd Font for the best icon display on the terminal

**Build from source:**

```bash
git clone https://github.com/Chicken_Yi/lazypackage.git
cd lazypackage
cargo build --release
```

Execute the binary at `target/release/lazypackage`.

## 🚀 Quick Start

lazypackage provides an intuitive interface that is completely keyboard-driven.

👉 **Check out the full list of shortcuts in the [Keybindings Guide](docs/guides/keybindings.md).**

## ⚙️ Configuration

The default configuration file is stored at `~/.config/lazypackage/config.toml`.

You can modify UI settings, keybindings, and backends. See more details in the [Configuration Guide](docs/guides/configuration.md).

## 🏛️ Architecture

**lazypackage** strictly follows module boundary separation principles.

- The User Interface (UI) only reads the state (`AppState`), and absolutely never calls backends directly.
- All side effects are treated as data via the `Command` enum.

**Dependency Diagram:**

```text
main.rs
  ├── tui       ──┐
  └── backends  ──┴──> core
```

_Note: `tui` and `backends` DO NOT depend on each other._

👉 See more at [Architecture Overview](docs/architecture/overview.md).

## 🤝 Contributing

We always welcome community contributions! Please check out [CONTRIBUTING.md](docs/guides/CONTRIBUTING.md) for more details on the pull request process and project structure.

## 📄 License

The project is distributed under the **MIT** license.  
Copyright © 2026 Chicken_Yi.
