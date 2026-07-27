use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use lazypackage_core::action::Action;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct Sidebar {
    pub is_focused: bool,
    pub categories: Vec<String>,
    pub state: ListState,
}

impl Sidebar {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            is_focused: false,
            categories: vec![
                "All".to_string(),
                "Installed".to_string(),
                "Upgradable".to_string(),
            ],
            state,
        }
    }
}

impl Component for Sidebar {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.is_focused {
            return None;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                let i = match self.state.selected() {
                    Some(i) => {
                        if i >= self.categories.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let i = match self.state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.categories.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.state.select(Some(i));
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, _action: &Action) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, _state: &AppState) {
        let items: Vec<ListItem> = self
            .categories
            .iter()
            .map(|c| {
                let icon = match c.as_str() {
                    "All" => "🌐",
                    "Installed" => "📦",
                    "Upgradable" => "🚀",
                    _ => "📁",
                };
                ListItem::new(format!(" {} {}", icon, c))
            })
            .collect();

        let border_color = if self.is_focused {
            Theme::BORDER_FOCUSED
        } else {
            Theme::BORDER_UNFOCUSED
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(ratatui::text::Span::styled(
                        " 📁 Categories ",
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

        f.render_stateful_widget(list, area, &mut self.state);

        let scrollbar = ratatui::widgets::Scrollbar::new(
            ratatui::widgets::ScrollbarOrientation::VerticalRight,
        )
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .track_symbol(Some("│"))
        .thumb_symbol("█")
        .style(Style::default().fg(Theme::ACCENT));

        let mut scrollbar_state = ratatui::widgets::ScrollbarState::new(self.categories.len().saturating_sub(1))
            .position(self.state.selected().unwrap_or(0));

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
