# lazypackage Documentation 🚀

> **Welcome to the lazypackage Living Documentation!**  
> Whether you are a curious user, a new developer looking to make your first open-source contribution, or a QA engineer, you are in the right place. 
> 
> *Note: `lazypackage` is developed with heavy assistance from AI (AI-generated code). This entire `docs/` directory is designed as a "Living Document"—it continuously evolves alongside the code to help human contributors and AI assistants stay perfectly aligned.*

---

## 1. What is lazypackage? 🤔

If you use Linux, you interact with package managers daily. However, the current landscape has a frustrating gap:
- **CLI Package Managers (dnf, apt, pacman)**: Incredibly fast, but require memorizing exact package names and flags. Hard to discover new software.
- **GUI App Stores (GNOME Software, Discover)**: Great for visual discovery, but often bloated, slow, and resource-heavy. They force you to leave your terminal.

**`lazypackage`** bridges this gap. It is a **fast, lightweight Terminal User Interface (TUI) package manager** that gives you the best of both worlds:
- The speed and efficiency of the terminal.
- The visual discoverability and ease-of-use of a GUI (navigate with keyboard, search interactively, multi-select).

## 2. Documentation Structure

This directory is organized to help you quickly find what you need:

```text
docs/
├── README.md                  ← You are here! The landing page.
├── specs/                     ← What the system must do (Requirements)
│   ├── product-requirements.md
│   ├── backend-spec.md
│   └── tui-spec.md
├── architecture/              ← How the system is built (For Developers)
│   ├── overview.md
│   ├── domain-model.md
│   ├── tea-runtime.md
│   └── error-handling.md
├── guides/                    ← How to help build it (For Contributors & QA)
│   ├── development-guide.md
│   ├── configuration.md
│   ├── keybindings.md
│   └── CONTRIBUTING.md
└── changelog.md               ← Version History
```

### 🎯 Specifications
- [Product Requirements (PRD)](specs/product-requirements.md): Vision, features, and MVP scope.
- [Backend Spec](specs/backend-spec.md): Specs for package manager backends.
- [TUI Spec](specs/tui-spec.md): Interface specs and keybindings.

### 🏗️ Architecture
- [Architecture Overview](architecture/overview.md): The Elm Architecture (TEA) and crate boundaries. Start here to understand the code!
- [Domain Model](architecture/domain-model.md): Core data structures.
- [TEA Runtime](architecture/tea-runtime.md): Async tasks and the MPSC loop.
- [Error Handling](architecture/error-handling.md): Guidelines for `anyhow` and `thiserror`.

### 🤝 Guides & QA
- [Contributing & QA Guide](guides/CONTRIBUTING.md): **Must read for new contributors and QA**. Covers how to set up the environment, run tests (including UI snapshots), and the risks associated with AI-generated code.
- [Development Guide](guides/development-guide.md): Coding rules and patterns.
- [Configuration](guides/configuration.md): Customizing the app.
- [Keybindings](guides/keybindings.md): Default keyboard shortcuts.

## 3. Join the Community! 🌟
We believe open source should be friendly and accessible. Explore the docs, and if you see something wrong or want to tackle your first issue, don't hesitate! Submit a PR or open an issue. Let's make Linux package management beautiful together.
