use crate::components::{
    details_pane::DetailsPane, log_panel::LogPanel, package_table::PackageTable, sidebar::Sidebar,
    Component,
};
use crate::state::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
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

        let top_bar = Paragraph::new("lazypackage                                ? help")
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(top_bar, chunks[0]);

        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20), // Sidebar
                Constraint::Percentage(50), // Table
                Constraint::Percentage(30), // Details
            ])
            .split(chunks[1]);

        self.sidebar.draw(f, main_chunks[0], state);
        self.table.draw(f, main_chunks[1], state);
        self.details.draw(f, main_chunks[2], state);

        self.log_panel.draw(f, chunks[2], state);

        let footer = Paragraph::new(
            "j/k: navigate  i: install  r: remove  u: upgrade  /: search  Space: select  q: quit",
        );
        f.render_widget(footer, chunks[3]);
    }
}
