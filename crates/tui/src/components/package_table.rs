use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use lazypackage_core::domain::PackageStatus;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

pub struct PackageTable {
    pub is_focused: bool,
    pub state: TableState,
}

impl PackageTable {
    pub fn new() -> Self {
        let mut state = TableState::default();
        state.select(Some(0));
        Self {
            is_focused: true,
            state,
        }
    }
}

impl Component for PackageTable {
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        // App-level key handling handles j/k and space to update state directly
        // because state.selected_package_index is in AppState.
        None
    }

    fn update(&mut self, _action: &Action) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        self.state.select(state.selected_package_index);

        let border_color = if self.is_focused {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER_UNFOCUSED
        };

        let header = Row::new(vec!["Sel", "St", "Name", "Version", "Repo", "Size"])
            .style(Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD));

        let filtered = state.filtered_packages();
        let rows: Vec<Row> = filtered
            .iter()
            .map(|p| {
                let is_checked = state.selected_packages.contains(&p.id);
                let selected_cell = if is_checked {
                    Cell::from("[✓]").style(
                        Style::default()
                            .fg(Theme::SUCCESS)
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    )
                } else {
                    Cell::from("[ ]").style(Style::default().fg(Theme::MUTED))
                };

                let (st, color) = match p.status() {
                    PackageStatus::Installed => ("●", Theme::INSTALLED),
                    PackageStatus::UpgradeAvailable => ("▲", Theme::UPGRADABLE),
                    PackageStatus::NotInstalled => ("○", Theme::MUTED),
                };

                let name = p.id.name.clone();
                let version = p
                    .installed_version
                    .as_deref()
                    .or(p.available_version.as_deref())
                    .unwrap_or("")
                    .to_string();
                let repo = p.repo.as_deref().unwrap_or("").to_string();
                let size = p.size_bytes.map(|s| s.to_string()).unwrap_or_default();

                Row::new(vec![
                    selected_cell,
                    Cell::from(st).style(Style::default().fg(color)),
                    Cell::from(name).style(Style::default().add_modifier(ratatui::style::Modifier::BOLD)),
                    Cell::from(version).style(Style::default().fg(Theme::SUCCESS)),
                    Cell::from(repo).style(Style::default().fg(Theme::SECONDARY)),
                    Cell::from(size).style(Style::default().fg(Theme::TEXT_MUTED)),
                ])
            })
            .collect();

        let scope_name = match state.search_scope {
            lazypackage_core::domain::SearchScope::Local => "Local",
            lazypackage_core::domain::SearchScope::Dnf => "DNF Remote",
        };

        let total = state.active_packages().len();

        let title_prefix = if self.is_focused { "[2] 📦 Packages" } else { "2: Packages" };

        let title_text = if state.search_query.is_empty() {
            format!(" {} [{}] ({}) ", title_prefix, scope_name, total)
        } else {
            format!(" {} [{}] ({}/{}) ", title_prefix, scope_name, filtered.len(), total)
        };

        let table = Table::new(
            rows,
            [
                Constraint::Length(5),
                Constraint::Length(3),
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(15),
                Constraint::Percentage(10),
            ],
        )
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(ratatui::text::Span::styled(
                    title_text,
                    Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD),
                ))
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_symbol("▶ ")
        .highlight_style(
            Style::default()
                .bg(Theme::SELECTION_BG)
                .fg(Theme::SELECTION_FG)
                .add_modifier(ratatui::style::Modifier::BOLD),
        );

        f.render_stateful_widget(table, area, &mut self.state);

        let scrollbar = ratatui::widgets::Scrollbar::new(
            ratatui::widgets::ScrollbarOrientation::VerticalRight,
        )
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .style(Style::default().fg(Theme::ACCENT));

        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(filtered.len().saturating_sub(1))
            .position(state.selected_package_index.unwrap_or(0));

        f.render_stateful_widget(
            scrollbar,
            area.inner(&ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}
