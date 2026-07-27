use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lazypackage_core::action::{Action, Command};
use lazypackage_tui::{update, AppLayout, AppState};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::time::Duration;
use tokio::sync::mpsc;

// ============================================================================
// Orchestration layer (TODO: implement)
// ============================================================================
//
// This is the integration point between the TUI and the package-manager
// backends. The functions below are called whenever `update()` emits a
// `Command`. Fill them in to support each backend you want to expose.
//
// Suggested approach:
//   - Detect which package managers are available at startup.
//   - Store them as `Arc<dyn Installer + PackageSource>` values.
//   - Route each `Command` to the correct backend instance.

/// Bootstrap: send the initial list of installed packages into the TUI.
///
/// Called once at startup. Implementations should spawn an async task that
/// calls `backend.list_installed()` and sends an
/// `Action::InstalledPackagesLoaded(backend_kind, result)` back through `tx`.
async fn load_initial_packages(_tx: mpsc::Sender<Action>) {
    // TODO: detect available backends, call list_installed(), send result
    todo!("implement initial package loading")
}

/// Dispatch a `Command` emitted by the TUI update function.
///
/// Implementations should match on the command variant and route it to the
/// appropriate backend, then send the result back as an `Action`.
async fn dispatch(cmd: Command, _tx: mpsc::Sender<Action>) {
    match cmd {
        Command::Quit => { /* handled in the event loop */ }
        Command::RefreshInstalled { backend } => {
            // TODO: call backend.list_installed() and send Action::InstalledPackagesLoaded
            let _ = backend;
            todo!("implement RefreshInstalled")
        }
        Command::SearchRemote { backend, query } => {
            // TODO: call backend.search(&query) and send Action::SearchResult
            let _ = (backend, query);
            todo!("implement SearchRemote")
        }
        Command::Install { id } => {
            // TODO: call backend.install(&id), then refresh installed list,
            //       send Action::OperationResult on error
            let _ = id;
            todo!("implement Install")
        }
        Command::Remove { id } => {
            // TODO: call backend.remove(&id), then refresh installed list,
            //       send Action::OperationResult on error
            let _ = id;
            todo!("implement Remove")
        }
        Command::Upgrade { id } => {
            // TODO: call backend.upgrade(&id), then refresh installed list,
            //       send Action::OperationResult on error
            let _ = id;
            todo!("implement Upgrade")
        }
    }
}

// ============================================================================
// Entry point
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Restore terminal on panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // 2. Setup terminal
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. Application state & layout
    let mut app_state = AppState::new();
    let mut layout = AppLayout::new();

    // 4. Action channel (TUI → orchestration → TUI)
    let (tx, mut rx) = mpsc::channel::<Action>(32);

    // 5. Load initial data
    load_initial_packages(tx.clone()).await;

    // 6. Crossterm event pump
    let tx_input = tx.clone();
    tokio::spawn(async move {
        loop {
            if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                if let Ok(Event::Key(key)) = event::read() {
                    let _ = tx_input.send(Action::KeyPressed(key)).await;
                }
            } else {
                let _ = tx_input.send(Action::Tick).await;
            }
        }
    });

    // 7. Main event loop
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| {
            layout.draw(f, &app_state);
        })?;

        if let Some(action) = rx.recv().await {
            let commands = update(&mut app_state, action);

            for command in commands {
                if command == Command::Quit {
                    should_quit = true;
                } else {
                    let tx_cmd = tx.clone();
                    tokio::spawn(async move {
                        dispatch(command, tx_cmd).await;
                    });
                }
            }
        }
    }

    // 8. Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
