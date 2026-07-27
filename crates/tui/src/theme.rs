use ratatui::style::Color;

pub struct Theme;

impl Theme {
    // Lazygit / Lazydocker color scheme
    pub const ACCENT: Color = Color::Cyan;
    pub const SECONDARY: Color = Color::LightMagenta;
    pub const SUCCESS: Color = Color::LightGreen;
    pub const WARNING: Color = Color::LightYellow;
    pub const ERROR: Color = Color::LightRed;
    pub const MUTED: Color = Color::DarkGray;

    pub const INSTALLED: Color = Color::Green;
    pub const UPGRADABLE: Color = Color::Yellow;
    pub const REMOVED: Color = Color::Red;
    pub const DISABLED: Color = Color::DarkGray;

    pub const BORDER_FOCUSED: Color = Color::Green; // Lazygit uses green for focused borders
    pub const BORDER_UNFOCUSED: Color = Color::Rgb(80, 90, 105);
    pub const HEADER_BG: Color = Color::Rgb(30, 40, 55);

    pub const SELECTION_BG: Color = Color::Rgb(45, 60, 85);
    pub const SELECTION_FG: Color = Color::LightCyan;

    pub const TEXT: Color = Color::Reset;
    pub const TEXT_MUTED: Color = Color::Gray;
    pub const KEY_BINDING: Color = Color::Yellow;
}
