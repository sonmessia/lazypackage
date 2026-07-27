# Error Handling Strategy — lazypackage

> **Living Document**

The error handling strategy across the lazypackage system ensures that errors are meaningful, easy to debug, and user-friendly, especially in the TUI interface.

## Error Handling Layers

| Layer              | Library     | Usage                                                                                                               |
| ------------------ | ----------- | ------------------------------------------------------------------------------------------------------------------- |
| `core`, `backends` | `thiserror` | Domain-specific errors: `BackendError::CommandFailed`, `BackendError::ParseError`, `BackendError::PermissionDenied` |
| Binary (`main.rs`) | `anyhow`    | Aggregate all errors to `anyhow::Result` at the outermost layer                                                     |

> **`anyhow` does not leak into `core`/`backends`** — keeping those 2 crates independently reusable.

## Backend Error Types (BackendError)

To ensure system resilience, `backends` use the `BackendError` enumeration (via `thiserror`) with clear variants:

- **CommandFailed**: Underlying command (like `dnf`, `apt`) returned a non-zero exit code.
- **ParseError**: Cannot parse output from CLI (e.g., text output format changed).
- **PermissionDenied**: Insufficient execution privileges, usually due to polkit/sudo rejection.
- **NetworkError**: Network issue when fetching repositories.
- **DependencyConflict**: Dependency conflicts that cannot be resolved automatically (often due to manual installations, etc.).

## Error Flow Through the System

1. Backend executes a command and encounters a failure, producing a `BackendError`.
2. The error is sent via `Action::BackendResult(Err(error_string))` into the main event loop.
3. Within the `update()` function, the state catches the error and saves it to `AppState.error_message`.
4. When `render()` is called, the TUI will reflect this error to the user.

## Displaying Errors in TUI

- **Log panel**: Displays details of the running command and the detailed error string (if any).
- **Status bar (Footer/Top bar)**: Displays a short summary message ("Network error", "Root privileges required").

## Panic Hook (Required)

`main.rs` **must** set a panic hook to restore the terminal before running the event loop:

```rust
let original_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |panic_info| {
    // Restore the terminal before printing the panic
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    original_hook(panic_info);
}));
```

> Without this step, a mid-flight panic leaves the terminal in a broken state (raw mode, alternate screen). Restoring the terminal ensures the user's terminal is not broken after the app crashes.

## Guide to Adding New Error Types

1. Define a new variant for the error type in `core` or `backends` using `thiserror`.
2. Map errors from lower modules to the appropriate domain-error type using `?`.
3. In the UI, adjust the render function to display the error usefully (e.g., alongside suggestions for the User).
