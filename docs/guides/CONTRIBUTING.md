# Contributing Guide

Welcome to **lazypackage**! Thank you for taking the time to contribute to the project. Your participation helps make the project better and more useful for the community.

Before getting started, please read our [Code of Conduct](CODE_OF_CONDUCT.md) (coming soon) to ensure a friendly and respectful working environment.

---

## 1. How to Contribute

There are many ways you can contribute to lazypackage:

### Bug Reports

If you discover a bug, please create a new Issue. Include the following information so we can reproduce it:

- Linux distribution and version you're using.
- lazypackage version.
- Steps to reproduce the bug.
- Expected result and actual result.
- Any logs or screenshots (if applicable).

### Feature Requests

If you have a new idea, create an Issue with the `enhancement` label. Please describe clearly:

- The problem you're facing or why this feature is needed.
- Proposed solution or how the feature should work.

### Pull Requests (PR)

If you want to directly fix a bug or add a feature:

1. Fork the repository.
2. Create a new branch.
3. Make your changes.
4. Submit a Pull Request to the `main` branch of the original repository.

---

## 2. Development Environment Setup

### Prerequisites

- **Rust**: Version 1.75 or later with `cargo`.
- **Operating System**: A Linux system. For full backend testing, you need `dnf` installed on your system.

### Clone & Build

```bash
# Clone repository
git clone https://github.com/Chicken_Yi/lazypackage.git
cd lazypackage

# Build the entire project
cargo build
```

### Running Tests

```bash
cargo test --workspace
```

### Running the TUI in Development

To run the application directly from source:

```bash
cargo run
```

---

## 3. Code Conventions

### Rust Code Style

- Ensure code follows formatting standards: run `cargo fmt` before committing.
- Code must have no linter warnings: run `cargo clippy --workspace -- -D warnings`.

### Commit Message Format

We follow [Conventional Commits](https://www.conventionalcommits.org/). Use these prefixes:

- `feat:`: Add a new feature.
- `fix:`: Fix a bug.
- `docs:`: Update documentation.
- `refactor:`: Improve code (no logic or feature changes).
- `test:`: Add or fix tests.
- `chore:`: Other maintenance tasks (e.g., update dependencies).

### Branch Naming

- New feature: `feature/<feature-name>`
- Bug fix: `fix/<bug-name>`
- Documentation update: `docs/<doc-name>`

---

## 4. Architecture — What You Need to Know

lazypackage follows The Elm Architecture (TEA). See [architecture/overview.md](../architecture/overview.md) for full details.

**Core principles:**

- **Dependency direction:** `main` → `{tui, backends}` → `core`.
- The `tui` and `backends` crates **MUST NEVER** depend on each other.
- The `core` crate contains central logic: **NO** I/O, **NO** async runtime, **NO** UI coupling.

---

## 5. Adding a New Backend

If you want to add support for a new package manager (e.g., apt, pacman, flatpak, snap), refer to the 6-step checklist in the [Architecture Documentation](../architecture/overview.md).

---

## 6. QA & Testing Guide (AI Context)

Because we use AI-generated code, robust QA and human review are our safety nets.

### 6.1. Automated Testing
- **Every PR** involving logic or new features must include corresponding tests.
- To run all tests in the workspace:
  ```bash
  cargo test --workspace
  ```
- To run tests with detailed logs (useful for tracing AI-generated errors):
  ```bash
  RUST_LOG=debug cargo test -- --nocapture
  ```

### 6.2. Snapshot Testing (UI)
Testing a terminal UI is hard. We use `insta` to take "snapshots" of what the UI should look like.
1. Run snapshot tests: `cargo test --test snapshot`
2. If you *intended* to change the UI, review and accept the new snapshot: `cargo insta review`

### 6.3. Manual Testing & Debugging
You cannot use `println!` to debug `lazypackage` because it breaks the terminal UI drawing. Instead:
1. Run it and redirect logs to a file: `RUST_LOG=info cargo run -- 2> debug.log`
2. In a second terminal, watch the logs: `tail -f debug.log`

### 6.4. AI Code Risk Areas
Reviewers and QA should be extra vigilant about:
- **Error Handling**: AI sometimes gets lazy and uses `.unwrap()`. We want graceful error handling using `?` (via `anyhow` and `thiserror`).
- **Concurrency**: Ensure async mpsc channels don't get blocked or dropped, which would freeze the TUI.
- **System Commands**: In `backends/src/process.rs`, we execute real OS commands. Watch out for parameter parsing bugs or command injection risks.

---

## 7. Review Process

- **Reviewers:** Every Pull Request must have at least 1 approval from a core project member.
- **CI/CD:** All CI pipelines must pass, including successful builds and all tests passing.
- **Clippy:** Code must not contain any warnings from `clippy`.

---

## 8. License

By submitting contributions to lazypackage, you agree that your contributions will be released under the [MIT License](https://opensource.org/licenses/MIT), the same license as the rest of the project.
