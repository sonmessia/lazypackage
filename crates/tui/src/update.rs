use crate::state::AppState;
use crossterm::event::{KeyCode, KeyEvent};
use lazypackage_core::action::{Action, BackendOp, Command};

pub fn update(state: &mut AppState, action: Action) -> Vec<Command> {
    let mut commands = Vec::new();

    match action {
        Action::KeyPressed(KeyEvent { code, .. }) => match code {
            KeyCode::Char('q') => {
                commands.push(Command::Quit);
            }
            KeyCode::Char('i') => {
                if let Some(idx) = state.selected_package_index {
                    if let Some(pkg) = state.packages.get(idx) {
                        commands.push(Command::RunBackend {
                            backend: pkg.id.backend,
                            op: BackendOp::Install(pkg.id.clone()),
                        });
                        state
                            .log_messages
                            .push(format!("Requested install for {}", pkg.id.name));
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(idx) = state.selected_package_index {
                    if let Some(pkg) = state.packages.get(idx) {
                        commands.push(Command::RunBackend {
                            backend: pkg.id.backend,
                            op: BackendOp::Remove(pkg.id.clone()),
                        });
                        state
                            .log_messages
                            .push(format!("Requested remove for {}", pkg.id.name));
                    }
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(idx) = state.selected_package_index {
                    if idx + 1 < state.packages.len() {
                        state.selected_package_index = Some(idx + 1);
                    }
                } else if !state.packages.is_empty() {
                    state.selected_package_index = Some(0);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(idx) = state.selected_package_index {
                    if idx > 0 {
                        state.selected_package_index = Some(idx - 1);
                    }
                }
            }
            KeyCode::Char(' ') => {
                if let Some(idx) = state.selected_package_index {
                    if let Some(pkg) = state.packages.get(idx) {
                        if state.selected_packages.contains(&pkg.id) {
                            state.selected_packages.remove(&pkg.id);
                        } else {
                            state.selected_packages.insert(pkg.id.clone());
                        }
                    }
                }
            }
            _ => {}
        },
        Action::BackendResult(_backend, Ok(packages)) => {
            state.packages = packages;
            if !state.packages.is_empty() && state.selected_package_index.is_none() {
                state.selected_package_index = Some(0);
            }
            state
                .log_messages
                .push("Backend action completed successfully".to_string());
        }
        Action::BackendResult(_backend, Err(err)) => {
            state.error_message = Some(err.clone());
            state.log_messages.push(format!("Backend error: {}", err));
        }
        Action::InstallRequested(id) => {
            commands.push(Command::RunBackend {
                backend: id.backend,
                op: BackendOp::Install(id),
            });
        }
        Action::RemoveRequested(id) => {
            commands.push(Command::RunBackend {
                backend: id.backend,
                op: BackendOp::Remove(id),
            });
        }
        Action::UpgradeRequested(_) => {
            // Handle upgrade requested
        }
        Action::SearchChanged(query) => {
            state.search_query = query;
        }
        Action::Tick => {}
        Action::Quit => {
            commands.push(Command::Quit);
        }
    }

    commands
}
