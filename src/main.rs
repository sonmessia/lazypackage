/// Entry point for `lazypackage`.
///
/// Wires together:
/// - [`DnfBackend`] (system package manager adapter)
/// - [`App`] (TUI state machine)
/// - `tokio::sync::mpsc` channel for async backend results → main thread
/// - crossterm event polling loop
use std::sync::Arc;

use crossterm::event::{KeyCode, KeyModifiers};
use lazypackage_backends::DnfBackend;
use lazypackage_core::{services::SystemManagerService, Package, PackageError};
use lazypackage_tui::{
    app::{App, AppMode, ConfirmAction, FocusedPanel, LogEntry},
    event::{poll_event, AppEvent},
    tui, ui,
};
use tokio::sync::mpsc;

// ── Message type for async backend results ────────────────────────────────────

enum BackendMsg {
    PackagesLoaded(Result<Vec<Package>, PackageError>),
    AllPackagesLoaded(Result<Vec<Package>, PackageError>),
    InstallDone(String, Result<(), PackageError>),
    RemoveDone(String, Result<(), PackageError>),
    UpgradeDone(String, Result<(), PackageError>),
    UpgradeAllDone(Result<(), PackageError>),
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Build backend (unit struct — no constructor needed).
    let backend: Arc<DnfBackend> = Arc::new(DnfBackend);

    // Detect backend name synchronously.
    let backend_name = {
        let svc = SystemManagerService::new(&*backend);
        svc.backend_kind().to_string()
    };

    let mut app = App::new(backend_name);
    let mut terminal = tui::init()?;

    let (tx, mut rx) = mpsc::channel::<BackendMsg>(32);

    // Kick off initial package load.
    app.is_loading = true;
    app.add_log(LogEntry::Info("Loading packages...".into()));
    spawn_list_installed(tx.clone(), Arc::clone(&backend));

    // ── Main event loop ───────────────────────────────────────────────────────
    loop {
        // Draw the current frame.
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Drain all pending backend messages.
        while let Ok(msg) = rx.try_recv() {
            handle_backend_msg(&mut app, msg, &tx, &backend).await;
        }

        // Poll for a terminal event (≤ 16 ms).
        if let Some(ev) = poll_event() {
            match ev {
                AppEvent::Key(k) => handle_key(&mut app, k, &tx, &backend).await,
                AppEvent::Resize(_, _) | AppEvent::Tick => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    tui::restore()?;
    Ok(())
}

// ── Async task helpers ────────────────────────────────────────────────────────

fn spawn_list_installed(tx: mpsc::Sender<BackendMsg>, backend: Arc<DnfBackend>) {
    tokio::spawn(async move {
        let svc = SystemManagerService::new(&*backend);
        let result = svc.list_installed().await;
        let _ = tx.send(BackendMsg::PackagesLoaded(result)).await;
    });
}

fn spawn_list_all(tx: mpsc::Sender<BackendMsg>, backend: Arc<DnfBackend>) {
    tokio::spawn(async move {
        let result = backend.list_all().await;
        let _ = tx.send(BackendMsg::AllPackagesLoaded(result)).await;
    });
}

async fn refresh_packages(app: &mut App, tx: &mpsc::Sender<BackendMsg>, backend: &Arc<DnfBackend>) {
    app.is_loading = true;
    spawn_list_installed(tx.clone(), Arc::clone(backend));
}

// ── Backend message handler ───────────────────────────────────────────────────

async fn handle_backend_msg(
    app: &mut App,
    msg: BackendMsg,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
) {
    match msg {
        BackendMsg::PackagesLoaded(result) => {
            app.is_loading = false;
            match result {
                Ok(pkgs) => {
                    let count = pkgs.len();
                    app.set_packages(pkgs);
                    app.add_log(LogEntry::Success(format!(
                        "Loaded {} installed packages.",
                        count
                    )));
                    // Now kick off loading all available packages in the background.
                    app.add_log(LogEntry::Info(
                        "Fetching all available packages in background...".into(),
                    ));
                    spawn_list_all(tx.clone(), Arc::clone(backend));
                }
                Err(e) => {
                    app.add_log(LogEntry::Error(format!("Failed to load packages: {}", e)));
                }
            }
        }
        BackendMsg::AllPackagesLoaded(result) => match result {
            Ok(pkgs) => {
                let count = pkgs.len();
                app.merge_packages(pkgs);
                app.add_log(LogEntry::Success(format!(
                    "Loaded {} total available packages.",
                    count
                )));
            }
            Err(e) => {
                app.add_log(LogEntry::Error(format!(
                    "Failed to load all available packages: {}",
                    e
                )));
            }
        },
        BackendMsg::InstallDone(name, result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    app.add_log(LogEntry::Success(format!("'{}' installed.", name)));
                    refresh_packages(app, tx, backend).await;
                }
                Err(e) => {
                    app.add_log(LogEntry::Error(format!("Install '{}' failed: {}", name, e)));
                }
            }
        }
        BackendMsg::RemoveDone(name, result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    app.add_log(LogEntry::Success(format!("'{}' removed.", name)));
                    refresh_packages(app, tx, backend).await;
                }
                Err(e) => {
                    app.add_log(LogEntry::Error(format!("Remove '{}' failed: {}", name, e)));
                }
            }
        }
        BackendMsg::UpgradeDone(name, result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    app.add_log(LogEntry::Success(format!("'{}' upgraded.", name)));
                    refresh_packages(app, tx, backend).await;
                }
                Err(e) => {
                    app.add_log(LogEntry::Error(format!("Upgrade '{}' failed: {}", name, e)));
                }
            }
        }
        BackendMsg::UpgradeAllDone(result) => {
            app.is_loading = false;
            match result {
                Ok(()) => {
                    app.add_log(LogEntry::Success("All packages upgraded.".into()));
                    refresh_packages(app, tx, backend).await;
                }
                Err(e) => {
                    app.add_log(LogEntry::Error(format!("Upgrade-all failed: {}", e)));
                }
            }
        }
    }
}

// ── Key dispatch ──────────────────────────────────────────────────────────────

async fn handle_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
) {
    match &app.mode {
        AppMode::Normal => handle_normal_key(app, key, tx, backend).await,
        AppMode::Search => handle_search_key(app, key),
        AppMode::Confirm(_) => handle_confirm_key(app, key, tx, backend).await,
        AppMode::SudoPrompt => handle_sudo_key(app, key, tx, backend).await,
        AppMode::ShowHelp => handle_help_key(app, key),
    }
}

async fn handle_normal_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
) {
    match key.code {
        // Quit
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true
        }

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => app.go_top(),
        KeyCode::Char('G') => app.go_bottom(),

        // Log scroll (when Log panel has focus)
        KeyCode::Char('J') if app.focused_panel == FocusedPanel::Log => app.log_scroll_down(),
        KeyCode::Char('K') if app.focused_panel == FocusedPanel::Log => app.log_scroll_up(),

        // Panel focus
        KeyCode::Tab => {
            app.focused_panel = match app.focused_panel {
                FocusedPanel::PackageList => FocusedPanel::Details,
                FocusedPanel::Details => FocusedPanel::Log,
                FocusedPanel::Log => FocusedPanel::PackageList,
            };
        }

        // Modes / overlays
        KeyCode::Char('/') => app.mode = AppMode::Search,
        KeyCode::Char('?') => app.mode = AppMode::ShowHelp,

        // Refresh
        KeyCode::Char('r') => {
            app.add_log(LogEntry::Info("Refreshing...".into()));
            refresh_packages(app, tx, backend).await;
        }

        // Package actions — prompt for confirmation first
        KeyCode::Char('i') => {
            if let Some(pkg) = app.selected_package() {
                let name = pkg.name.clone();
                app.mode = AppMode::Confirm(ConfirmAction::Install(name));
            }
        }
        KeyCode::Char('d') => {
            if let Some(pkg) = app.selected_package() {
                let name = pkg.name.clone();
                app.mode = AppMode::Confirm(ConfirmAction::Remove(name));
            }
        }
        KeyCode::Char('u') => {
            if let Some(pkg) = app.selected_package() {
                let name = pkg.name.clone();
                app.mode = AppMode::Confirm(ConfirmAction::Upgrade(name));
            }
        }
        KeyCode::Char('U') => {
            app.mode = AppMode::Confirm(ConfirmAction::UpgradeAll);
        }

        _ => {}
    }
}

fn handle_search_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search_query.clear();
            app.apply_filter();
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            app.apply_filter();
            app.mode = AppMode::Normal;
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_filter();
        }
        _ => {}
    }
}

async fn handle_confirm_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
) {
    match key.code {
        KeyCode::Enter => {
            if let AppMode::Confirm(action) = app.mode.clone() {
                app.mode = AppMode::Normal;
                // Check if we already have cached sudo credentials.
                // `sudo -vn` exits 0 if valid, non-zero if password needed.
                let needs_password = !check_sudo_cached().await;
                if needs_password {
                    // Open sudo prompt instead of running directly.
                    app.pending_action = Some(action);
                    app.sudo_input.clear();
                    app.mode = AppMode::SudoPrompt;
                } else {
                    execute_action(app, action, tx, backend, None).await;
                }
            }
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

/// Handles key events while in SudoPrompt mode.
async fn handle_sudo_key(
    app: &mut App,
    key: crossterm::event::KeyEvent,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
) {
    match key.code {
        KeyCode::Esc => {
            // Cancel: clear sensitive data and go back.
            app.sudo_input.clear();
            app.pending_action = None;
            app.mode = AppMode::Normal;
        }
        KeyCode::Enter => {
            if let Some(action) = app.pending_action.take() {
                let password = app.sudo_input.clone();
                app.sudo_input.clear();
                app.mode = AppMode::Normal;
                execute_action(app, action, tx, backend, Some(password)).await;
            } else {
                app.sudo_input.clear();
                app.mode = AppMode::Normal;
            }
        }
        KeyCode::Backspace => {
            app.sudo_input.pop();
        }
        KeyCode::Char(c) => {
            app.sudo_input.push(c);
        }
        _ => {}
    }
}

fn handle_help_key(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('?') | KeyCode::Esc => app.mode = AppMode::Normal,
        _ => {}
    }
}

// ── Action execution ──────────────────────────────────────────────────────────

/// Checks whether `sudo` has a valid cached credential (i.e. no password
/// needed right now). Uses `sudo -vn` which exits 0 if cached, 1 if not.
async fn check_sudo_cached() -> bool {
    tokio::process::Command::new("sudo")
        .args(["-vn"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn execute_action(
    app: &mut App,
    action: ConfirmAction,
    tx: &mpsc::Sender<BackendMsg>,
    backend: &Arc<DnfBackend>,
    sudo_password: Option<String>,
) {
    match action {
        ConfirmAction::Install(name) => {
            app.add_log(LogEntry::Command(format!("Installing '{}'...", name)));
            app.is_loading = true;
            let tx = tx.clone();
            let b = Arc::clone(backend);
            let pkg_name = name.clone();
            let pw = sudo_password.clone();
            tokio::spawn(async move {
                let result = b
                    .run_dnf_privileged(&["install", "-y", &pkg_name], pw.as_deref())
                    .await
                    .map(|_| ());
                let _ = tx.send(BackendMsg::InstallDone(pkg_name, result)).await;
            });
        }
        ConfirmAction::Remove(name) => {
            app.add_log(LogEntry::Command(format!("Removing '{}'...", name)));
            app.is_loading = true;
            let tx = tx.clone();
            let b = Arc::clone(backend);
            let pkg_name = name.clone();
            let pw = sudo_password.clone();
            tokio::spawn(async move {
                let result = b
                    .run_dnf_privileged(&["remove", "-y", &pkg_name], pw.as_deref())
                    .await
                    .map(|_| ());
                let _ = tx.send(BackendMsg::RemoveDone(pkg_name, result)).await;
            });
        }
        ConfirmAction::Upgrade(name) => {
            app.add_log(LogEntry::Command(format!("Upgrading '{}'...", name)));
            app.is_loading = true;
            let tx = tx.clone();
            let b = Arc::clone(backend);
            let pkg_name = name.clone();
            let pw = sudo_password.clone();
            tokio::spawn(async move {
                let result = b
                    .run_dnf_privileged(&["upgrade", "-y", &pkg_name], pw.as_deref())
                    .await
                    .map(|_| ());
                let _ = tx.send(BackendMsg::UpgradeDone(pkg_name, result)).await;
            });
        }
        ConfirmAction::UpgradeAll => {
            app.add_log(LogEntry::Command("Upgrading all packages...".into()));
            app.is_loading = true;
            let tx = tx.clone();
            let b = Arc::clone(backend);
            let pw = sudo_password.clone();
            tokio::spawn(async move {
                let result = b
                    .run_dnf_privileged(&["upgrade", "-y"], pw.as_deref())
                    .await
                    .map(|_| ());
                let _ = tx.send(BackendMsg::UpgradeAllDone(result)).await;
            });
        }
    }
}
