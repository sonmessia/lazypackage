use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use lazypackage_backends::dnf::Dnf;
use lazypackage_backends::privilege::SudoEscalator;
use lazypackage_core::action::{Action, BackendOp, Command};
use lazypackage_core::domain::BackendKind;
use lazypackage_core::traits::{Installer, PackageSource};
use lazypackage_tui::{update, AppLayout, AppState};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup panic hook to restore terminal
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

    // 3. Application State & Layout
    let mut app_state = AppState::new();
    let mut layout = AppLayout::new();

    // 4. Communication channel
    let (tx, mut rx) = mpsc::channel::<Action>(32);

    // 5. Initialize backends (Dnf for MVP)
    let escalator = Arc::new(SudoEscalator);
    let dnf = Arc::new(Dnf::new(escalator));

    // Load initial installed packages
    let tx_clone = tx.clone();
    let dnf_clone = dnf.clone();
    tokio::spawn(async move {
        match dnf_clone.list_installed().await {
            Ok(packages) => {
                let _ = tx_clone
                    .send(Action::BackendResult(BackendKind::Dnf, Ok(packages)))
                    .await;
            }
            Err(e) => {
                let _ = tx_clone
                    .send(Action::BackendResult(BackendKind::Dnf, Err(e.to_string())))
                    .await;
            }
        }
    });

    // Event loop task for crossterm
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

    // 6. Main Event Loop
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| {
            layout.draw(f, &app_state);
        })?;

        if let Some(action) = rx.recv().await {
            let commands = update(&mut app_state, action);

            for command in commands {
                match command {
                    Command::Quit => {
                        should_quit = true;
                    }
                    Command::RunBackend { backend, op } => {
                        if backend == BackendKind::Dnf {
                            let tx_cmd = tx.clone();
                            let dnf_cmd = dnf.clone();
                            tokio::spawn(async move {
                                match op {
                                    BackendOp::Install(id) => {
                                        if let Err(e) = dnf_cmd.install(&id).await {
                                            let _ = tx_cmd
                                                .send(Action::BackendResult(
                                                    BackendKind::Dnf,
                                                    Err(e.to_string()),
                                                ))
                                                .await;
                                        } else {
                                            // Refresh list on success
                                            if let Ok(packages) = dnf_cmd.list_installed().await {
                                                let _ = tx_cmd
                                                    .send(Action::BackendResult(
                                                        BackendKind::Dnf,
                                                        Ok(packages),
                                                    ))
                                                    .await;
                                            }
                                        }
                                    }
                                    BackendOp::Remove(id) => {
                                        if let Err(e) = dnf_cmd.remove(&id).await {
                                            let _ = tx_cmd
                                                .send(Action::BackendResult(
                                                    BackendKind::Dnf,
                                                    Err(e.to_string()),
                                                ))
                                                .await;
                                        } else {
                                            if let Ok(packages) = dnf_cmd.list_installed().await {
                                                let _ = tx_cmd
                                                    .send(Action::BackendResult(
                                                        BackendKind::Dnf,
                                                        Ok(packages),
                                                    ))
                                                    .await;
                                            }
                                        }
                                    }
                                    BackendOp::ListInstalled => {
                                        match dnf_cmd.list_installed().await {
                                            Ok(packages) => {
                                                let _ = tx_cmd
                                                    .send(Action::BackendResult(
                                                        BackendKind::Dnf,
                                                        Ok(packages),
                                                    ))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_cmd
                                                    .send(Action::BackendResult(
                                                        BackendKind::Dnf,
                                                        Err(e.to_string()),
                                                    ))
                                                    .await;
                                            }
                                        }
                                    }
                                    BackendOp::Search(query) => {
                                        match dnf_cmd.search(&query).await {
                                            Ok(packages) => {
                                                let _ = tx_cmd
                                                    .send(Action::SearchResult(
                                                        BackendKind::Dnf,
                                                        Ok(packages),
                                                    ))
                                                    .await;
                                            }
                                            Err(e) => {
                                                let _ = tx_cmd
                                                    .send(Action::SearchResult(
                                                        BackendKind::Dnf,
                                                        Err(e.to_string()),
                                                    ))
                                                    .await;
                                            }
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }
    }

    // 7. Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
