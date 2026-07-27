# TUI Specification

## 1. Overview

`lazypackage` utilizes a TUI (Text User Interface) designed based on `ratatui` and `crossterm`.
The goal is to provide a modern, clear, and performance-focused User Experience (UX), ensuring that operations are not hindered by complex interactions. The UI adheres to the principle of not storing application state-changing logic, but merely listening and rendering.

## 2. Layout specification

Below is the general layout diagram of the main screen:

```text
┌─────────────────────────────────────────────────────────┐
│  lazypackage                              ? help       │  ← Top bar
├──────────┬──────────────────────┬───────────────────────┤
│          │                      │                       │
│ Sidebar  │    Package Table     │    Details Pane       │
│ (categories) │    (packages)      │    (detail tabs)      │
│          │                      │                       │
│          │                      │                       │
├──────────┴──────────────────────┴───────────────────────┤
│  $ dnf install nodejs                            [OK]   │  ← Log panel
├─────────────────────────────────────────────────────────┤
│  j/k: move       i: install  /: search    q: quit       │  ← Footer
└─────────────────────────────────────────────────────────┘
```

## 3. Component specification

Details about each component in the layout:

- **Top bar**: Displays the application title, dynamic running status, and a hint to open help. Do not display the application version number here.
- **Sidebar**: Displays navigation categories such as: All, Installed, Upgradable, or by backends (dnf, flatpak, etc.). Supports navigating back and forth.
- **Package Table**: Table displaying primary information of the software package list.
  - Columns include: Checkbox, Status Icon, Name, Version, Repo, Size.
  - Has explicit size constraints and supports sorting features.
- **Details Pane**: Detailed information panel for the currently selected package.
  - Uses the `ratatui::Tabs` widget to divide into tabs: Info, Dependencies, Files.
- **Log Panel**: Located at the bottom section.
  - Displays 1 line of the actual underlying system command being called, mandatory for transparency.
- **Footer**: Bottom bar hinting context-sensitive keyboard shortcuts.

## 4. Design rules

| Category          | Design Rules                                                                                                            |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Icons**         | Use Nerd Font glyphs or 1-width Unicode characters. **DO NOT** use color emoji.                                         |
| **Focus state**   | The border of the currently focused panel must use an accent color. Unfocused panels use gray.                          |
| **Status colors** | Installed packages: Green. Packages with updates: Yellow. Uninstalled packages: Gray. DO NOT use plain text like I/U/-. |
| **Multi-select**  | The multi-select checkbox must be clearly separated from the package's status badge.                                    |
| **Tabs**          | Use `ratatui::Tabs`, do not use complex nested Boxes to draw tabs manually.                                             |

## 5. Keybinding specification

| Action         | Shortcut  | Function                                               |
| -------------- | --------- | ------------------------------------------------------ |
| **Navigation** | `j` / `k` | Move up/down the list.                                 |
|                | `Tab`     | Switch focus back and forth between panels.            |
|                | `h` / `l` | Switch back and forth between Sidebar and Detail Pane. |
| **Actions**    | `i`       | Install the currently selected package (Install).      |
|                | `r`       | Remove the currently selected package (Remove).        |
|                | `u`       | Upgrade the currently selected package (Upgrade).      |
|                | `/`       | Search.                                                |
|                | `Space`   | Select / Deselect package (Multi-select).              |
|                | `Enter`   | Confirm action.                                        |
| **System**     | `q`       | Quit application.                                      |
|                | `?`       | Show help screen.                                      |
|                | `F5`      | Refresh data.                                          |

## 6. State management

- **UI only reads `AppState`**: The interface only has permission to read from `AppState`; it must not directly mutate the global state or call backends.
- **Component Trait**: Components must implement a common trait with the functions: `handle_key`, `update`, `draw`.
- **Local State**: Local states (such as scroll offset, search bar content) will be stored inside each component's struct instead of being put into `AppState`.

## 7. Color palette

Use semantic colors:

- **Accent**: Applied to currently focused items/panels.
- **Green**: Represents successful installation, positive status.
- **Yellow**: Warning, update required (Upgradable).
- **Red**: Error, delete operation.
- **Gray**: Disabled status, uninstalled, auxiliary information.

## 8. Responsive behavior

The application must be able to resize its layout appropriately when the terminal window size changes:

- When the width narrows, the Sidebar column can be hidden or shrunk to a fixed size.
- The Package Table flexibly changes the widths of the `Name` and `Description` columns proportionally (percentage).
- The Detail Pane will automatically wrap text if the width is too narrow.
