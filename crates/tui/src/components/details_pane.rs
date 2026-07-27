use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

pub struct DetailsPane {
    pub is_focused: bool,
    pub active_tab: usize,
}

impl DetailsPane {
    pub fn new() -> Self {
        Self {
            is_focused: false,
            active_tab: 0,
        }
    }
}

impl Component for DetailsPane {
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    fn update(&mut self, _action: &Action) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let border_color = if self.is_focused {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER_UNFOCUSED
        };

        let titles = vec!["Info", "Dependencies", "Files"];
        let tabs = Tabs::new(titles)
            .select(self.active_tab)
            .style(Style::default().fg(Theme::TEXT))
            .highlight_style(Style::default().fg(Theme::ACCENT).bold())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Details")
                    .border_style(Style::default().fg(border_color)),
            );

        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Vertical)
            .constraints([
                ratatui::layout::Constraint::Length(3),
                ratatui::layout::Constraint::Min(0),
            ])
            .split(area);

        f.render_widget(tabs, chunks[0]);

        let text = if let Some(idx) = state.selected_package_index {
            if let Some(p) = state.packages.get(idx) {
                format!(
                    "Name: {}\nVersion: {:?}\nSummary: {}\nRepo: {:?}",
                    p.id.name,
                    p.installed_version
                        .as_deref()
                        .or(p.available_version.as_deref()),
                    p.summary,
                    p.repo
                )
            } else {
                "No package selected".to_string()
            }
        } else {
            "No package selected".to_string()
        };

        let content = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(content, chunks[1]);
    }
}
