# Backend Specification

## 1. Overview

The Backend is the module responsible for communicating (I/O) with the actual underlying package manager system. Each backend in the system will implement core traits from the `core` library. The architecture ensures each backend owns its specific logic (e.g., version comparison, root privilege requests).

## 2. Backend List

| Backend    | BackendKind | Implement PackageSource? | Implement Installer? | Status  |
| ---------- | ----------- | ------------------------ | -------------------- | ------- |
| dnf        | Dnf         | Yes                      | Yes                  | MVP     |
| local file | LocalFile   | No (Installer only)      | Yes                  | Phase 2 |
| AppImage   | AppImage    | No (needs custom trait)  | Partial              | Phase 2 |
| apt        | Apt         | Yes                      | Yes                  | Planned |
| pacman     | Pacman      | Yes                      | Yes                  | Planned |
| flatpak    | Flatpak     | Yes                      | Yes                  | Planned |
| snap       | Snap        | Yes                      | Yes                  | Planned |

## 3. DNF Backend Specification

The primary backend in the MVP version, specified in detail:

- **Commands used**:
  - `dnf list installed`: Get the list of installed packages.
  - `dnf list available`: Get the list of available packages.
  - `dnf search`: Search packages by keyword.
  - `dnf info`: Get detailed package information.
  - `dnf install -y`: Install automatically.
  - `dnf remove -y`: Remove automatically.
- **Output parsing**: Output from `dnf` will be parsed line-by-line. Terminal dnf text wrapping cases must be carefully handled to correctly extract name, version, and repo.
- **Version comparison**: Use the `rpmvercmp` algorithm to compare version strings exactly according to the RPM standard.
- **Privilege**: Requires `root` permissions (via PrivilegeEscalator) for `install` and `remove` operations.
- **Error cases**: Must explicitly catch and handle: package does not exist, network errors, permission denied, dependency conflicts.
- **Cache strategy**: Cache the results of `list_installed` and `search`. The cache Time-To-Live (TTL) is retrieved from the configuration.

## 4. Local File Backend Specification

Intended for installations from downloaded files (.rpm, .deb):

- Only implement the `Installer` trait, **do not** implement `PackageSource`.
- **Input**: Path to `.rpm`, `.deb` files.
- **Execution**:
  - Call `dnf install <path>` (on RedHat/Fedora).
  - Or call `dpkg -i <path>` (on Debian/Ubuntu).
- **Version comparison**: No version comparison needed.

## 5. AppImage Backend Specification

Manage standalone AppImage executables:

- Needs a distinct trait to support desktop integration (creating `.desktop` files, extracting icons).
- **Not** forced into the standard `PackageSource` trait due to its different nature.
- **Core operations**: Download, make executable, create desktop entry, and remove.

## 6. Trait mapping

Backends will implement different traits based on their capabilities:

- **PackageSource methods**: `list_installed`, `search`, `compare_versions`.
- **Installer methods**: `install`, `remove`.
- **PrivilegeEscalator**: Separated from the backend, injected via the constructor for reusability and easy mocking during tests.

## 7. Checklist for adding a new backend

When adding a new backend (like `apt`, `pacman`), developers must follow 6 steps (refer to the development guide documentation). This includes:

1. Create a new module in the `backends` crate.
2. Define the struct and implement necessary traits (`PackageSource`, `Installer`).
3. Handle the corresponding output parsing.
4. Handle the system-specific version comparison logic.
5. Handle privilege escalation if needed.
6. Register the backend with the core system.
