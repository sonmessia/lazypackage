# ⌨️ lazypackage Keybindings Cheat Sheet

`lazypackage` is designed to be completely keyboard-driven for maximum efficiency, inspired by tools like **lazygit** and **lazydocker**.

Pressing **`?`** inside `lazypackage` will display a context-sensitive **Help Popup Modal** at any time.

---

## 🌐 Global Shortcuts & Panel Focus

| Key | Description |
| :--- | :--- |
| **`1` / `2` / `3` / `4`** | Jump directly to Panel (`1`: Categories Sidebar, `2`: Package Table, `3`: Details, `4`: Logs) |
| **`h` / `l`** or `Left` / `Right` | Switch layout panel focus left / right |
| **`?`** | Toggle Help & Keymaps Cheat Sheet Modal |
| **`/`** | Focus search input bar |
| **`Tab`** or `Shift+Tab` | Toggle search scope between **Local** installed packages and **DNF Remote** repositories |
| **`Esc`** | Exit search mode / clear filter / close Help modal |
| **`q`** | Quit `lazypackage` |

---

## 📦 Panel 2: Package Table Controls

When the **Package Table** panel is focused (`[2] Packages`):

| Key | Description |
| :--- | :--- |
| **`j` / `k`** or `Down` / `Up` | Navigate down / up package list |
| **`Ctrl + d`** or `PageDown` | Scroll half-page down (10 items) |
| **`Ctrl + u`** or `PageUp` | Scroll half-page up (10 items) |
| **`g` / `G`** or `Home` / `End` | Jump to Top / Bottom of package list |
| **`i`** | Install selected package (`dnf install`) |
| **`r` / `d`** | Remove / uninstall selected package (`dnf remove`) |
| **`Space`** | Select / deselect package (toggle multi-select checkbox `[✓]`) |
| **`a`** | Select all / Deselect all filtered packages |

---

## 📁 Panel 1: Categories Sidebar Controls

When the **Categories Sidebar** panel is focused (`[1] Categories`):

| Key | Description |
| :--- | :--- |
| **`j` / `k`** or `Down` / `Up` | Move down / up category list (`All`, `Installed`, `Upgradable`) |
| **`Enter` / `Space`** | Apply selected category filter to the package table |
| **`g` / `G`** | Jump to top / bottom of category list |

---

## ℹ️ Panel 3: Details Pane Controls

When the **Package Details** panel is focused (`[3] Package Details`):

| Key | Description |
| :--- | :--- |
| **`[` / `]`** | Switch details sub-tab (`ℹ Info` ↔ `🔗 Dependencies` ↔ `📄 Files`) |
| **`j` / `k`** or `Down` / `Up` | Scroll details text down / up |
| **`Ctrl + d` / `Ctrl + u`** | Scroll details text half-page down / up |
| **`g`** | Jump to top of details content |

---

## 📜 Panel 4: Command Logs Controls

When the **Logs Panel** is focused (`[4] Logs`):

| Key | Description |
| :--- | :--- |
| **`j` / `k`** or `Down` / `Up` | Scroll log messages history down / up |
| **`Ctrl + d` / `Ctrl + u`** | Scroll log history half-page down / up |
| **`c`** | Clear command log history |

---

*Note: Custom keybinding configurations via `config.toml` will be supported in upcoming releases. Refer to the [Configuration Guide](configuration.md) for updates.*
