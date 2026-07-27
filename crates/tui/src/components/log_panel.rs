use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub struct LogPanel;

impl LogPanel {
    pub fn new() -> Self {
        Self
    }
}

impl Component for LogPanel {
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    fn update(&mut self, _action: &Action) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let text = state
            .log_messages
            .last()
            .cloned()
            .unwrap_or_else(|| "$ ".to_string());
        let para = Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Theme::BORDER_UNFOCUSED)),
        );
        f.render_widget(para, area);
    }
}
