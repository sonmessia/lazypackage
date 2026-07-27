use ratatui::style::Color;

pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Cyan;
    pub const INSTALLED: Color = Color::Green;
    pub const UPGRADABLE: Color = Color::Yellow;
    pub const REMOVED: Color = Color::Red;
    pub const DISABLED: Color = Color::DarkGray;
    pub const BORDER_FOCUSED: Color = Color::Cyan;
    pub const BORDER_UNFOCUSED: Color = Color::DarkGray;
    pub const TEXT: Color = Color::Reset;
}
