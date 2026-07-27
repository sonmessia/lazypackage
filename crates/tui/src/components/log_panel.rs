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
        let msg = state
            .log_messages
            .last()
            .cloned()
            .unwrap_or_else(|| "Ready".to_string());

        let (tag, tag_color) = if msg.contains("error") || msg.contains("Err") || msg.contains("failed") {
            ("[ERROR]", Theme::ERROR)
        } else if msg.contains("Searching") || msg.contains("Requested") {
            ("[SEARCH]", Theme::WARNING)
        } else if msg.contains("completed") || msg.contains("finished") || msg.contains("Loaded") {
            ("[SUCCESS]", Theme::SUCCESS)
        } else {
            ("[INFO]", Theme::ACCENT)
        };

        let spans = vec![
            ratatui::text::Span::styled(tag, Style::default().fg(tag_color).add_modifier(ratatui::style::Modifier::BOLD)),
            ratatui::text::Span::raw(" "),
            ratatui::text::Span::styled(msg, Style::default().fg(Theme::TEXT)),
        ];

        let is_focused = state.active_panel == lazypackage_core::domain::ActivePanel::Logs;
        let border_color = if is_focused {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER_UNFOCUSED
        };

        let title_text = if is_focused {
            " [4] 📜 Logs "
        } else {
            " 4: Logs "
        };

        let para = Paragraph::new(ratatui::text::Line::from(spans)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(ratatui::text::Span::styled(
                    title_text,
                    Style::default()
                        .fg(if is_focused { Theme::BORDER_FOCUSED } else { Theme::ACCENT })
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ))
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(para, area);
    }
}
