use crate::components::Component;
use crate::state::AppState;
use crate::theme::Theme;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use lazypackage_core::domain::ActivePanel;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub struct HelpPopup;

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpPopup {
    pub fn new() -> Self {
        Self
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

impl Component for HelpPopup {
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Action> {
        None
    }

    fn update(&mut self, _action: &Action) {}

    fn draw(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let popup_area = self.centered_rect(70, 75, area);

        let (panel_title, panel_shortcuts) = match state.active_panel {
            ActivePanel::Sidebar => (
                "[1] Sidebar (Categories)",
                vec![
                    ("j / Down", "Move down category list"),
                    ("k / Up", "Move up category list"),
                    ("Enter / Space", "Filter packages by selected category"),
                    ("g / G", "Jump to Top / Bottom"),
                ],
            ),
            ActivePanel::PackageTable => (
                "[2] Package Table",
                vec![
                    ("j / Down", "Navigate down package list"),
                    ("k / Up", "Navigate up package list"),
                    ("g / G", "Jump to Top / Bottom of package list"),
                    ("Ctrl+d / u", "Scroll half page down / up"),
                    ("i", "Install selected package"),
                    ("r / d", "Remove selected package"),
                    ("Space", "Select / Deselect package (toggle checkbox)"),
                    ("a", "Select all / Deselect all packages"),
                    ("Tab", "Toggle search scope (Local ↔ DNF Remote)"),
                ],
            ),
            ActivePanel::Details => (
                "[3] Package Details Pane",
                vec![
                    ("[ / ]", "Switch details tab (Info ↔ Dependencies ↔ Files)"),
                    ("j / Down", "Scroll details info down"),
                    ("k / Up", "Scroll details info up"),
                ],
            ),
            ActivePanel::Logs => (
                "[4] Command Logs Panel",
                vec![
                    ("j / Down", "Scroll command logs down"),
                    ("k / Up", "Scroll command logs up"),
                    ("c", "Clear command log history"),
                ],
            ),
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled(
                    format!(" ❓ Keymaps & Shortcuts - {} ", panel_title),
                    Style::default()
                        .fg(Theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "── Panel Shortcuts ──────────────────────────────────────────────────────────",
                Style::default().fg(Theme::MUTED),
            )]),
        ];

        for (key, desc) in panel_shortcuts {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:16}", key),
                    Style::default()
                        .fg(Theme::KEY_BINDING)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Style::default().fg(Theme::TEXT)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "── Global Layout Shortcuts ───────────────────────────────────────────────────",
            Style::default().fg(Theme::MUTED),
        )]));

        let global_shortcuts = vec![
            ("1 / 2 / 3 / 4", "Direct jump to Panel (1:Sidebar 2:Packages 3:Details 4:Logs)"),
            ("h / l", "Switch active layout panel Left / Right"),
            ("/", "Focus search input bar"),
            ("Tab", "Toggle search scope (Local ↔ DNF Remote)"),
            ("?", "Toggle this Help window"),
            ("q / Esc", "Quit application / Exit search"),
        ];

        for (key, desc) in global_shortcuts {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:16}", key),
                    Style::default()
                        .fg(Theme::SECONDARY)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc, Style::default().fg(Theme::TEXT)),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            " Press [?] or [Esc] to close ",
            Style::default()
                .fg(Theme::SUCCESS)
                .add_modifier(Modifier::BOLD),
        )]));

        let paragraph = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    " Help & Keymaps Cheat Sheet ",
                    Style::default()
                        .fg(Theme::BORDER_FOCUSED)
                        .add_modifier(Modifier::BOLD),
                ))
                .border_style(Style::default().fg(Theme::BORDER_FOCUSED)),
        );

        f.render_widget(Clear, popup_area);
        f.render_widget(paragraph, popup_area);
    }
}
