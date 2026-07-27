use crate::state::AppState;
use crossterm::event::{KeyCode, KeyEvent};
use lazypackage_core::action::{Action, BackendOp, Command};
use lazypackage_core::domain::{ActivePanel, BackendKind, SearchScope};

pub fn update(state: &mut AppState, action: Action) -> Vec<Command> {
    let mut commands = Vec::new();

    match action {
        Action::KeyPressed(KeyEvent { code, .. }) => {
            if state.is_search_mode {
                match code {
                    KeyCode::Tab | KeyCode::BackTab => {
                        state.search_scope = match state.search_scope {
                            SearchScope::Local => SearchScope::Dnf,
                            SearchScope::Dnf => SearchScope::Local,
                        };
                        state.clamp_selection();
                        state.log_messages.push(format!(
                            "Switched search scope to [{:?}]",
                            state.search_scope
                        ));
                    }
                    KeyCode::Esc => {
                        state.is_search_mode = false;
                    }
                    KeyCode::Enter => {
                        state.is_search_mode = false;
                        if state.search_scope == SearchScope::Dnf
                            && !state.search_query.trim().is_empty()
                        {
                            state.is_loading = true;
                            state.log_messages.push(format!(
                                "Searching DNF repository for '{}'...",
                                state.search_query
                            ));
                            commands.push(Command::RunBackend {
                                backend: BackendKind::Dnf,
                                op: BackendOp::Search(state.search_query.clone()),
                            });
                        }
                    }
                    KeyCode::Backspace => {
                        state.search_query.pop();
                        state.clamp_selection();
                    }
                    KeyCode::Char(c) => {
                        state.search_query.push(c);
                        state.clamp_selection();
                    }
                    KeyCode::Down => {
                        let count = state.filtered_packages().len();
                        if let Some(idx) = state.selected_package_index {
                            if idx + 1 < count {
                                state.selected_package_index = Some(idx + 1);
                            }
                        } else if count > 0 {
                            state.selected_package_index = Some(0);
                        }
                    }
                    KeyCode::Up => {
                        if let Some(idx) = state.selected_package_index {
                            if idx > 0 {
                                state.selected_package_index = Some(idx - 1);
                            }
                        }
                    }
                    _ => {}
                }
            } else {
                // Layout panel switching keys (1..4, h/l, Left/Right)
                match code {
                    KeyCode::Char('1') => {
                        state.active_panel = ActivePanel::Sidebar;
                        state.log_messages.push("Focused [1] Sidebar".to_string());
                    }
                    KeyCode::Char('2') => {
                        state.active_panel = ActivePanel::PackageTable;
                        state.log_messages.push("Focused [2] Packages Table".to_string());
                    }
                    KeyCode::Char('3') => {
                        state.active_panel = ActivePanel::Details;
                        state.log_messages.push("Focused [3] Details Pane".to_string());
                    }
                    KeyCode::Char('4') => {
                        state.active_panel = ActivePanel::Logs;
                        state.log_messages.push("Focused [4] Logs Panel".to_string());
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        state.active_panel = state.active_panel.prev();
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        state.active_panel = state.active_panel.next();
                    }
                    KeyCode::Char('[') => {
                        state.details_tab = if state.details_tab == 0 { 2 } else { state.details_tab - 1 };
                        state.log_messages.push(format!("Details tab: {}", state.details_tab + 1));
                    }
                    KeyCode::Char(']') => {
                        state.details_tab = (state.details_tab + 1) % 3;
                        state.log_messages.push(format!("Details tab: {}", state.details_tab + 1));
                    }
                    KeyCode::Char('/') => {
                        state.is_search_mode = true;
                    }
                    KeyCode::Tab | KeyCode::BackTab => {
                        state.search_scope = match state.search_scope {
                            SearchScope::Local => SearchScope::Dnf,
                            SearchScope::Dnf => SearchScope::Local,
                        };
                        state.clamp_selection();
                        state.log_messages.push(format!(
                            "Switched search scope to [{:?}]",
                            state.search_scope
                        ));
                        if state.search_scope == SearchScope::Dnf
                            && !state.search_query.trim().is_empty()
                            && state.dnf_search_results.is_empty()
                        {
                            state.is_loading = true;
                            state.log_messages.push(format!(
                                "Searching DNF repository for '{}'...",
                                state.search_query
                            ));
                            commands.push(Command::RunBackend {
                                backend: BackendKind::Dnf,
                                op: BackendOp::Search(state.search_query.clone()),
                            });
                        }
                    }
                    KeyCode::Esc => {
                        if !state.search_query.is_empty() {
                            state.search_query.clear();
                            state.clamp_selection();
                        }
                    }
                    KeyCode::Char('q') => {
                        commands.push(Command::Quit);
                    }
                    _ => {
                        // Panel-specific controls
                        match state.active_panel {
                            ActivePanel::Sidebar => match code {
                                KeyCode::Char('j') | KeyCode::Down => {
                                    state.sidebar_index = (state.sidebar_index + 1) % 3;
                                    let categories = ["All", "Installed", "Upgradable"];
                                    state.current_category = categories[state.sidebar_index].to_string();
                                    state.clamp_selection();
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    state.sidebar_index = if state.sidebar_index == 0 { 2 } else { state.sidebar_index - 1 };
                                    let categories = ["All", "Installed", "Upgradable"];
                                    state.current_category = categories[state.sidebar_index].to_string();
                                    state.clamp_selection();
                                }
                                KeyCode::Enter | KeyCode::Char(' ') => {
                                    let categories = ["All", "Installed", "Upgradable"];
                                    state.current_category = categories[state.sidebar_index].to_string();
                                    state.clamp_selection();
                                    state.log_messages.push(format!("Filter category: {}", state.current_category));
                                }
                                _ => {}
                            },
                            ActivePanel::PackageTable => match code {
                                KeyCode::Char('i') => {
                                    if let Some(pkg) = state.selected_package() {
                                        let pkg_id = pkg.id.clone();
                                        state.is_loading = true;
                                        commands.push(Command::RunBackend {
                                            backend: pkg_id.backend,
                                            op: BackendOp::Install(pkg_id.clone()),
                                        });
                                        state
                                            .log_messages
                                            .push(format!("Requested install for {}", pkg_id.name));
                                    }
                                }
                                KeyCode::Char('r') => {
                                    if let Some(pkg) = state.selected_package() {
                                        let pkg_id = pkg.id.clone();
                                        state.is_loading = true;
                                        commands.push(Command::RunBackend {
                                            backend: pkg_id.backend,
                                            op: BackendOp::Remove(pkg_id.clone()),
                                        });
                                        state
                                            .log_messages
                                            .push(format!("Requested remove for {}", pkg_id.name));
                                    }
                                }
                                KeyCode::Char('j') | KeyCode::Down => {
                                    let count = state.filtered_packages().len();
                                    if let Some(idx) = state.selected_package_index {
                                        if idx + 1 < count {
                                            state.selected_package_index = Some(idx + 1);
                                        }
                                    } else if count > 0 {
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
                                    if let Some(pkg) = state.selected_package() {
                                        let pkg_id = pkg.id.clone();
                                        if state.selected_packages.contains(&pkg_id) {
                                            state.selected_packages.remove(&pkg_id);
                                        } else {
                                            state.selected_packages.insert(pkg_id);
                                        }
                                    }
                                }
                                _ => {}
                            },
                            ActivePanel::Details => match code {
                                KeyCode::Char('j') | KeyCode::Down => {
                                    state.details_scroll = state.details_scroll.saturating_add(1);
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    state.details_scroll = state.details_scroll.saturating_sub(1);
                                }
                                _ => {}
                            },
                            ActivePanel::Logs => match code {
                                KeyCode::Char('j') | KeyCode::Down => {
                                    state.log_scroll = state.log_scroll.saturating_add(1);
                                }
                                KeyCode::Char('k') | KeyCode::Up => {
                                    state.log_scroll = state.log_scroll.saturating_sub(1);
                                }
                                _ => {}
                            },
                        }
                    }
                }
            }
        }
        Action::BackendResult(_backend, Ok(packages)) => {
            state.is_loading = false;
            state.installed_packages = packages;
            for pkg in &mut state.dnf_search_results {
                if let Some(installed_pkg) = state.installed_packages.iter().find(|i| i.id.name == pkg.id.name) {
                    pkg.installed_version = installed_pkg.installed_version.clone();
                } else {
                    pkg.installed_version = None;
                }
            }
            state.clamp_selection();
            state
                .log_messages
                .push("Loaded installed packages".to_string());
        }
        Action::BackendResult(_backend, Err(err)) => {
            state.is_loading = false;
            state.error_message = Some(err.clone());
            state.log_messages.push(format!("Backend error: {}", err));
        }
        Action::SearchResult(_backend, Ok(packages)) => {
            state.is_loading = false;
            let mut enriched = packages;
            for pkg in &mut enriched {
                if let Some(installed_pkg) = state.installed_packages.iter().find(|i| i.id.name == pkg.id.name) {
                    pkg.installed_version = installed_pkg.installed_version.clone();
                }
            }
            state.dnf_search_results = enriched;
            state.clamp_selection();
            state
                .log_messages
                .push(format!("DNF search finished: {} packages found", state.dnf_search_results.len()));
        }
        Action::SearchResult(_backend, Err(err)) => {
            state.is_loading = false;
            state.error_message = Some(err.clone());
            state.log_messages.push(format!("DNF search error: {}", err));
        }
        Action::SetActivePanel(panel) => {
            state.active_panel = panel;
        }
        Action::NextPanel => {
            state.active_panel = state.active_panel.next();
        }
        Action::PrevPanel => {
            state.active_panel = state.active_panel.prev();
        }
        Action::ToggleSearchScope => {
            state.search_scope = match state.search_scope {
                SearchScope::Local => SearchScope::Dnf,
                SearchScope::Dnf => SearchScope::Local,
            };
            state.clamp_selection();
        }
        Action::SetSearchScope(scope) => {
            state.search_scope = scope;
            state.clamp_selection();
        }
        Action::ExecuteSearch => {
            if state.search_scope == SearchScope::Dnf && !state.search_query.trim().is_empty() {
                state.is_loading = true;
                state
                    .log_messages
                    .push(format!("Searching DNF repository for '{}'...", state.search_query));
                commands.push(Command::RunBackend {
                    backend: BackendKind::Dnf,
                    op: BackendOp::Search(state.search_query.clone()),
                });
            }
        }
        Action::InstallRequested(id) => {
            state.is_loading = true;
            commands.push(Command::RunBackend {
                backend: id.backend,
                op: BackendOp::Install(id),
            });
        }
        Action::RemoveRequested(id) => {
            state.is_loading = true;
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
            state.clamp_selection();
        }
        Action::Tick => {}
        Action::Quit => {
            commands.push(Command::Quit);
        }
    }

    commands
}
