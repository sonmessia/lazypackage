use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use lazypackage_core::domain::PackageStatus;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Style, Stylize},
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
            .style(Style::default().bold());

        let rows: Vec<Row> = state
            .packages
            .iter()
            .map(|p| {
                let selected = if state.selected_packages.contains(&p.id) {
                    "[x]"
                } else {
                    "[ ]"
                };
                let (st, color) = match p.status() {
                    PackageStatus::Installed => ("●", Theme::INSTALLED),
                    PackageStatus::UpgradeAvailable => ("▲", Theme::UPGRADABLE),
                    PackageStatus::NotInstalled => ("○", Theme::DISABLED),
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
                    Cell::from(selected),
                    Cell::from(st).style(Style::default().fg(color)),
                    Cell::from(name),
                    Cell::from(version),
                    Cell::from(repo),
                    Cell::from(size),
                ])
            })
            .collect();

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
                .title("Packages")
                .border_style(Style::default().fg(border_color)),
        )
        .highlight_style(Style::default().bg(Theme::BORDER_UNFOCUSED).fg(Theme::TEXT));

        f.render_stateful_widget(table, area, &mut self.state);
    }
}
