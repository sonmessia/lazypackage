use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use ratatui::{
    layout::Rect,
    style::Style,
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

        let title_text = if self.is_focused {
            " [3] ℹ Package Details "
        } else {
            " 3: Package Details "
        };

        let titles = vec![" ℹ Info ", " 🔗 Dependencies ", " 📄 Files "];
        let tabs = Tabs::new(titles)
            .select(state.details_tab)
            .style(Style::default().fg(Theme::TEXT_MUTED))
            .highlight_style(
                Style::default()
                    .fg(Theme::SELECTION_FG)
                    .bg(Theme::SELECTION_BG)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(ratatui::text::Span::styled(
                        title_text,
                        Style::default()
                            .fg(if self.is_focused { Theme::BORDER_FOCUSED } else { Theme::ACCENT })
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ))
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

        let lines = if let Some(p) = state.selected_package() {
            let (st_str, st_color) = match p.status() {
                lazypackage_core::domain::PackageStatus::Installed => ("● Installed", Theme::INSTALLED),
                lazypackage_core::domain::PackageStatus::UpgradeAvailable => ("▲ Upgrade Available", Theme::UPGRADABLE),
                lazypackage_core::domain::PackageStatus::NotInstalled => ("○ Not Installed", Theme::MUTED),
            };

            let ver = p
                .installed_version
                .as_deref()
                .or(p.available_version.as_deref())
                .unwrap_or("N/A");

            let repo = p.repo.as_deref().unwrap_or("N/A");

            let summary = if p.summary.is_empty() {
                "N/A"
            } else {
                &p.summary
            };

            vec![
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Name:    ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(&p.id.name, Style::default().fg(Theme::TEXT).add_modifier(ratatui::style::Modifier::BOLD)),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Status:  ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(st_str, Style::default().fg(st_color).add_modifier(ratatui::style::Modifier::BOLD)),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Version: ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(ver, Style::default().fg(Theme::SUCCESS)),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Backend: ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(format!("{:?}", p.id.backend), Style::default().fg(Theme::WARNING)),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Repo:    ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(repo, Style::default().fg(Theme::SECONDARY)),
                ]),
                ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("Summary: ", Style::default().fg(Theme::ACCENT).add_modifier(ratatui::style::Modifier::BOLD)),
                    ratatui::text::Span::styled(summary, Style::default().fg(Theme::TEXT)),
                ]),
            ]
        } else {
            vec![ratatui::text::Line::from(ratatui::text::Span::styled(
                "No package selected",
                Style::default().fg(Theme::MUTED),
            ))]
        };

        let content = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::LEFT | Borders::RIGHT)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(content, chunks[1]);
    }
}
