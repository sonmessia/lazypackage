use crate::components::{
    details_pane::DetailsPane, log_panel::LogPanel, package_table::PackageTable, sidebar::Sidebar,
    Component,
};
use crate::state::AppState;
use crate::theme::Theme;
use lazypackage_core::domain::{ActivePanel, SearchScope};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct AppLayout {
    pub sidebar: Sidebar,
    pub table: PackageTable,
    pub details: DetailsPane,
    pub log_panel: LogPanel,
}

impl Default for AppLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl AppLayout {
    pub fn new() -> Self {
        Self {
            sidebar: Sidebar::new(),
            table: PackageTable::new(),
            details: DetailsPane::new(),
            log_panel: LogPanel::new(),
        }
    }

    pub fn draw(&mut self, f: &mut Frame, state: &AppState) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top bar
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Log panel
                Constraint::Length(1), // Footer
            ])
            .split(f.size());

        let loading = if state.is_loading {
            vec![Span::styled(
                " ⏳ Working...",
                Style::default().fg(Theme::WARNING).add_modifier(Modifier::BOLD),
            )]
        } else {
            vec![]
        };

        let scope_badge = match state.search_scope {
            SearchScope::Local => Span::styled(
                " [LOCAL] ",
                Style::default()
                    .fg(Theme::SUCCESS)
                    .bg(Theme::HEADER_BG)
                    .add_modifier(Modifier::BOLD),
            ),
            SearchScope::Dnf => Span::styled(
                " [DNF REMOTE] ",
                Style::default()
                    .fg(Theme::WARNING)
                    .bg(Theme::HEADER_BG)
                    .add_modifier(Modifier::BOLD),
            ),
        };

        let mut header_spans = vec![
            Span::styled(
                " 📦 lazypackage",
                Style::default()
                    .fg(Theme::SECONDARY)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            scope_badge,
        ];

        if state.is_search_mode {
            let scope_hint = match state.search_scope {
                SearchScope::Local => " (Scope: Local)",
                SearchScope::Dnf => " (Scope: DNF)",
            };
            header_spans.push(Span::styled(
                " 🔍 Search: ",
                Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD),
            ));
            header_spans.push(Span::styled(
                &state.search_query,
                Style::default()
                    .fg(Theme::KEY_BINDING)
                    .add_modifier(Modifier::BOLD),
            ));
            header_spans.push(Span::styled("█", Style::default().fg(Theme::ACCENT)));
            header_spans.push(Span::styled(
                format!("{} (Tab: scope | Enter: search | Esc: exit)", scope_hint),
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        } else if !state.search_query.is_empty() {
            header_spans.push(Span::styled(
                " 🔍 Filter: ",
                Style::default().fg(Theme::ACCENT),
            ));
            header_spans.push(Span::styled(
                format!("\"{}\"", state.search_query),
                Style::default().fg(Theme::KEY_BINDING),
            ));
            header_spans.push(Span::styled(
                " (Press / to edit, Tab: scope, Esc: clear)",
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        } else {
            header_spans.push(Span::styled(
                " (Press / to search, Tab to switch scope)",
                Style::default().fg(Theme::TEXT_MUTED),
            ));
        }

        header_spans.extend(loading);

        let top_border_color = if state.is_search_mode {
            Theme::BORDER_FOCUSED
        } else if !state.search_query.is_empty() {
            Theme::ACCENT
        } else {
            Theme::BORDER_UNFOCUSED
        };

        let top_bar = Paragraph::new(Line::from(header_spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " lazypackage TUI ",
                    Style::default().fg(Theme::ACCENT).add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(top_border_color)),
        );
        f.render_widget(top_bar, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Sidebar
                Constraint::Percentage(50), // Table
                Constraint::Percentage(30), // Details
            ])
            .split(chunks[1]);

        self.sidebar.is_focused = state.active_panel == ActivePanel::Sidebar;
        self.table.is_focused = state.active_panel == ActivePanel::PackageTable;
        self.details.is_focused = state.active_panel == ActivePanel::Details;

        self.sidebar.draw(f, main_chunks[0], state);
        self.table.draw(f, main_chunks[1], state);
        self.details.draw(f, main_chunks[2], state);

        self.log_panel.draw(f, chunks[2], state);

        let keybindings: Vec<(&str, &str)> = if state.is_search_mode {
            match state.search_scope {
                SearchScope::Local => vec![
                    ("Tab", "switch scope"),
                    ("Enter/Esc", "confirm"),
                    ("Backspace", "delete"),
                    ("Up/Down", "select"),
                ],
                SearchScope::Dnf => vec![
                    ("Enter", "search DNF"),
                    ("Tab", "switch scope"),
                    ("Esc", "exit"),
                    ("Backspace", "delete"),
                    ("Up/Down", "select"),
                ],
            }
        } else {
            match state.active_panel {
                ActivePanel::Sidebar => vec![
                    ("1-4 / h/l", "switch layout"),
                    ("j/k", "select category"),
                    ("Enter/Space", "filter"),
                    ("/", "search"),
                    ("q", "quit"),
                ],
                ActivePanel::PackageTable => vec![
                    ("1-4 / h/l", "switch layout"),
                    ("j/k", "navigate"),
                    ("i", "install"),
                    ("r", "remove"),
                    ("Tab", "scope"),
                    ("/", "search"),
                    ("Space", "select"),
                    ("q", "quit"),
                ],
                ActivePanel::Details => vec![
                    ("1-4 / h/l", "switch layout"),
                    ("[ / ]", "switch tab"),
                    ("j/k", "scroll info"),
                    ("/", "search"),
                    ("q", "quit"),
                ],
                ActivePanel::Logs => vec![
                    ("1-4 / h/l", "switch layout"),
                    ("j/k", "scroll logs"),
                    ("/", "search"),
                    ("q", "quit"),
                ],
            }
        };

        let mut footer_spans = Vec::new();
        for (i, (key, desc)) in keybindings.iter().enumerate() {
            if i > 0 {
                footer_spans.push(Span::styled(" | ", Style::default().fg(Theme::MUTED)));
            }
            footer_spans.push(Span::styled(
                format!("[{}]", key),
                Style::default()
                    .fg(Theme::KEY_BINDING)
                    .add_modifier(Modifier::BOLD),
            ));
            footer_spans.push(Span::styled(
                format!(" {}", desc),
                Style::default().fg(Theme::TEXT),
            ));
        }

        let footer = Paragraph::new(Line::from(footer_spans));
        f.render_widget(footer, chunks[3]);
    }
}
