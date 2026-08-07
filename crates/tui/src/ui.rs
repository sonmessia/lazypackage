use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, AppMode, ConfirmAction, FocusedPanel, LogEntry};
use lazypackage_core::PackageStatus;

// ── Entry point ───────────────────────────────────────────────────────────────

/// Draw the complete UI for one frame.
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Outer layout: title bar | content | bottom help bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title bar
            Constraint::Min(0),    // main content
            Constraint::Length(1), // bottom bar
        ])
        .split(area);

    render_title_bar(frame, app, main_chunks[0]);
    render_content(frame, app, main_chunks[1]);
    render_bottom_bar(frame, app, main_chunks[2]);

    // Overlay popups (drawn last so they appear on top).
    match &app.mode {
        AppMode::Search => render_search_popup(frame, app),
        AppMode::Confirm(action) => {
            let action = action.clone();
            render_confirm_popup(frame, &action, area);
        }
        AppMode::SudoPrompt => render_sudo_popup(frame, app, area),
        AppMode::ShowHelp => render_help_popup(frame, area),
        AppMode::Normal => {}
    }
}

// ── Title bar ─────────────────────────────────────────────────────────────────

fn render_title_bar(frame: &mut Frame, app: &App, area: Rect) {
    let loading = if app.is_loading { " [loading...]" } else { "" };
    let text = format!(
        "  lazypackage  [backend: {}]  [{} packages]{}",
        app.backend_name,
        app.filtered_packages.len(),
        loading,
    );
    let bar = Paragraph::new(text).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(bar, area);
}

// ── Content area ──────────────────────────────────────────────────────────────

fn render_content(frame: &mut Frame, app: &mut App, area: Rect) {
    // Horizontal split: 40 % package list | 60 % right pane
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_package_list(frame, app, cols[0]);

    // Right pane vertical split: 50 % details | 50 % log
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(cols[1]);

    render_details(frame, app, rows[0]);
    render_log(frame, app, rows[1]);
}

// ── Helper: status symbol & colour ───────────────────────────────────────────

fn status_symbol(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Installed => "[+]",
        PackageStatus::UpgradeAvailable => "[^]",
        PackageStatus::NotInstalled => "[ ]",
    }
}

fn status_color(status: PackageStatus) -> Color {
    match status {
        PackageStatus::Installed => Color::Green,
        PackageStatus::UpgradeAvailable => Color::Yellow,
        PackageStatus::NotInstalled => Color::Gray,
    }
}

// ── Package list panel ────────────────────────────────────────────────────────

fn render_package_list(frame: &mut Frame, app: &mut App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::PackageList;
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Packages ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Reserve first row for column header.
    let header_area = Rect { height: 1, ..inner };
    let list_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    let header = Paragraph::new(" STS  NAME                     VERSION").style(
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, header_area);

    let visible_rows = list_area.height as usize;
    app.update_scroll_offset(visible_rows.max(1));

    // Safely clamp start and end to avoid slicing panics.
    let start = app.scroll_offset.min(app.filtered_packages.len());
    let end = (start + visible_rows).min(app.filtered_packages.len());

    let items: Vec<ListItem> = app.filtered_packages[start..end]
        .iter()
        .enumerate()
        .map(|(offset, &pkg_idx)| {
            let pkg = &app.packages[pkg_idx];
            let abs_idx = start + offset;
            let is_selected = abs_idx == app.selected_idx;

            let symbol = status_symbol(pkg.status);
            let fg = status_color(pkg.status);

            // Truncate name/version to keep the row width sane.
            let max_name = (list_area.width as usize).saturating_sub(16).max(5);
            let name = truncate(&pkg.name, max_name);
            let version = truncate(&pkg.version, 12);

            let text = format!(" {:<4} {:<25} {}", symbol, name, version);

            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(fg)
            };

            ListItem::new(text).style(style)
        })
        .collect();

    frame.render_widget(List::new(items), list_area);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

// ── Details panel ─────────────────────────────────────────────────────────────

fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Details;
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Details ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if let Some(pkg) = app.selected_package() {
        let repo = pkg.repo.as_deref().unwrap_or("unknown");
        let size_str = format_bytes(pkg.size_bytes);
        let status_label = match pkg.status {
            PackageStatus::Installed => "Installed",
            PackageStatus::UpgradeAvailable => "Upgrade Available",
            PackageStatus::NotInstalled => "Not Installed",
        };

        let label_style = Style::default().fg(Color::DarkGray);
        let value_style = Style::default().fg(Color::White);

        let lines = vec![
            Line::from(vec![
                Span::styled("Name:    ", label_style),
                Span::styled(pkg.name.as_str(), value_style.add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::styled("Version: ", label_style),
                Span::styled(pkg.version.as_str(), value_style),
            ]),
            Line::from(vec![
                Span::styled("Status:  ", label_style),
                Span::styled(status_label, Style::default().fg(status_color(pkg.status))),
            ]),
            Line::from(vec![
                Span::styled("Repo:    ", label_style),
                Span::styled(repo, value_style),
            ]),
            Line::from(vec![
                Span::styled("Size:    ", label_style),
                Span::styled(size_str, value_style),
            ]),
            Line::from(vec![
                Span::styled("Backend: ", label_style),
                Span::styled(pkg.backend.to_string(), Style::default().fg(Color::Cyan)),
            ]),
            Line::from(""),
            Line::from(Span::styled("Description:", label_style)),
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(pkg.description.as_str(), value_style),
            ]),
        ];

        frame.render_widget(
            Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
            inner,
        );
    } else {
        frame.render_widget(
            Paragraph::new("No package selected")
                .style(Style::default().fg(Color::DarkGray))
                .alignment(Alignment::Center),
            inner,
        );
    }
}

fn format_bytes(bytes: Option<u64>) -> String {
    match bytes {
        Some(b) if b >= 1_048_576 => format!("{:.1} MB", b as f64 / 1_048_576.0),
        Some(b) if b >= 1_024 => format!("{:.1} KB", b as f64 / 1_024.0),
        Some(b) => format!("{} B", b),
        None => "unknown".to_string(),
    }
}

// ── Log panel ─────────────────────────────────────────────────────────────────

fn render_log(frame: &mut Frame, app: &App, area: Rect) {
    let focused = app.focused_panel == FocusedPanel::Log;
    let border_color = if focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(" Output Log ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height as usize;
    let total = app.log_entries.len();

    // Compute the visible window centred around log_scroll.
    let start = if total > visible_height {
        app.log_scroll
            .saturating_sub(visible_height.saturating_sub(1))
            .min(total.saturating_sub(visible_height))
    } else {
        0
    };
    let end = (start + visible_height).min(total);

    let lines: Vec<Line> = app.log_entries[start..end]
        .iter()
        .map(|entry| match entry {
            LogEntry::Info(s) => {
                Line::from(Span::styled(s.as_str(), Style::default().fg(Color::White)))
            }
            LogEntry::Success(s) => {
                Line::from(Span::styled(s.as_str(), Style::default().fg(Color::Green)))
            }
            LogEntry::Error(s) => {
                Line::from(Span::styled(s.as_str(), Style::default().fg(Color::Red)))
            }
            LogEntry::Command(s) => {
                Line::from(Span::styled(s.as_str(), Style::default().fg(Color::Cyan)))
            }
        })
        .collect();

    frame.render_widget(
        Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false }),
        inner,
    );
}

// ── Bottom help bar ───────────────────────────────────────────────────────────

fn render_bottom_bar(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        AppMode::Normal => {
            " j/k: nav  /: search  i: install  d: remove  u: upgrade  U: upgrade-all  r: refresh  Tab: focus  q: quit  ?: help"
        }
        AppMode::Search => " Type to search  |  ESC: cancel  |  Enter: apply",
        AppMode::Confirm(_) => " Enter: confirm  |  q / ESC: cancel",
        AppMode::SudoPrompt => " Type sudo password  |  Enter: authenticate  |  ESC: cancel",
        AppMode::ShowHelp => " ? / ESC: close help",
    };
    let bar = Paragraph::new(text).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(bar, area);
}

// ── Search popup ──────────────────────────────────────────────────────────────

fn render_search_popup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let popup = centered_rect(42, 5, area);

    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Search ");

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        Line::from(vec![
            Span::styled("Query: ", Style::default().fg(Color::DarkGray)),
            Span::styled(app.search_query.as_str(), Style::default().fg(Color::White)),
            Span::styled("_", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ESC: cancel   Enter: apply",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

// ── Sudo password popup ───────────────────────────────────────────────────────

fn render_sudo_popup(frame: &mut Frame, app: &App, area: Rect) {
    // Show what action is pending above the password field.
    let action_label = match app.pending_action.as_ref() {
        Some(ConfirmAction::Install(n)) => format!(" Install \"{}\"  –  requires sudo", n),
        Some(ConfirmAction::Remove(n)) => format!(" Remove \"{}\"  –  requires sudo", n),
        Some(ConfirmAction::Upgrade(n)) => format!(" Upgrade \"{}\"  –  requires sudo", n),
        Some(ConfirmAction::UpgradeAll) => " Upgrade ALL packages  –  requires sudo".to_owned(),
        None => " Sudo authentication required".to_owned(),
    };

    let popup = centered_rect(52, 8, area);
    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Sudo Password ");

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let lines = vec![
        Line::from(Span::styled(
            action_label,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Password: ", Style::default().fg(Color::DarkGray)),
            Span::styled("_", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Enter: confirm   ESC: cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

// ── Confirm popup ─────────────────────────────────────────────────────────────

fn render_confirm_popup(frame: &mut Frame, action: &ConfirmAction, area: Rect) {
    let popup = centered_rect(46, 6, area);

    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm ");

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let msg = match action {
        ConfirmAction::Install(n) => format!(" Install \"{}\"?", n),
        ConfirmAction::Remove(n) => format!(" Remove \"{}\"?", n),
        ConfirmAction::Upgrade(n) => format!(" Upgrade \"{}\"?", n),
        ConfirmAction::UpgradeAll => " Upgrade ALL packages?".to_string(),
    };

    let lines = vec![
        Line::from(Span::styled(
            msg,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  [Enter] Yes     [q] No",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

// ── Help popup ────────────────────────────────────────────────────────────────

fn render_help_popup(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(52, 18, area);

    frame.render_widget(Clear, popup);

    let block = Block::new()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" Keybindings ");

    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let bindings: &[(&str, &str)] = &[
        ("j / k", "Navigate list up / down"),
        ("g / G", "Jump to top / bottom"),
        ("/", "Open search"),
        ("i", "Install selected package"),
        ("d", "Remove selected package"),
        ("u", "Upgrade selected package"),
        ("U", "Upgrade all packages"),
        ("r", "Refresh package list"),
        ("Tab", "Switch panel focus"),
        ("q", "Quit"),
        ("ESC", "Cancel / close overlay"),
        ("?", "Toggle this help"),
    ];

    let lines: Vec<Line> = bindings
        .iter()
        .map(|(key, desc)| {
            Line::from(vec![
                Span::styled(format!("  {:<12}", key), Style::default().fg(Color::Cyan)),
                Span::styled(*desc, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

// ── Geometry helper ───────────────────────────────────────────────────────────

/// Returns a [`Rect`] of `width × height` centred inside `r`.
fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let x = r.x + r.width.saturating_sub(width) / 2;
    let y = r.y + r.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(r.width),
        height: height.min(r.height),
    }
}
