# lazypackage Configuration

## 1. Configuration File Location

The default configuration file for `lazypackage` is located at:
`~/.config/lazypackage/config.toml`

If your system defines the `XDG_CONFIG_HOME` environment variable, lazypackage will use it instead of `~/.config/`, with the full path being:
`$XDG_CONFIG_HOME/lazypackage/config.toml`

## 2. Full Configuration

Below is a complete `config.toml` file including all options and their default values:

```toml
[general]
# Default package manager backend to use.
# Supported: "dnf", "apt", "pacman", "local", "appimage"
default_backend = "dnf"

# Time-to-live for cached data (in seconds).
# Package information will be stored for this duration to speed up queries.
cache_ttl_seconds = 300

[privilege]
# Privilege escalation method for installing, updating, or removing packages.
# Valid values: "pkexec" (recommended for desktop), "sudo" (for terminal use)
method = "pkexec"

[keybindings]
# Custom keybindings for interacting with the interface.
navigate_down = "j"
navigate_up = "k"
install = "i"
search = "/"
quit = "q"
```

## 3. The [general] Section

- `default_backend`: Determines which package manager backend the application initializes on startup. This option is important as it decides which packages will be displayed.
- `cache_ttl_seconds`: To avoid repeated system commands causing UI lag, the application caches results. 300 seconds (5 minutes) is the default, balancing data freshness with performance.

## 4. The [privilege] Section

When performing system changes (install, remove), root privileges are required.

- `method = "pkexec"`: Displays a Desktop environment password prompt (Polkit). Best suited for users on GUI workstations.
- `method = "sudo"`: Requests the password directly in the terminal. Suitable for SSH users, servers, or headless environments.

## 5. The [keybindings] Section

Available keybinding actions:

| Action          | Description                  | Default |
| :-------------- | :--------------------------- | :------ |
| `navigate_down` | Move down in the list        | `j`     |
| `navigate_up`   | Move up in the list          | `k`     |
| `install`       | Install the selected package | `i`     |
| `search`        | Activate search mode         | `/`     |
| `quit`          | Quit the application         | `q`     |

## 6. Configuration Examples

**Fedora User (Desktop)**

```toml
[general]
default_backend = "dnf"
cache_ttl_seconds = 300

[privilege]
method = "pkexec"
```

**Ubuntu User (Server / SSH)**

```toml
[general]
default_backend = "apt"

[privilege]
method = "sudo"
```

**Arch Linux User**

```toml
[general]
default_backend = "pacman"

[privilege]
method = "pkexec"
```

**Minimal Configuration (Use all defaults)**
Just create an empty file or only specify the settings you want to override; the system will automatically use defaults for any missing configuration.

```toml
[general]
default_backend = "dnf"
```
