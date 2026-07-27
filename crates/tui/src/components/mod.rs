pub mod details_pane;
pub mod log_panel;
pub mod package_table;
pub mod sidebar;

use crate::state::AppState;
use crossterm::event::KeyEvent;
use lazypackage_core::action::Action;
use ratatui::layout::Rect;
use ratatui::Frame;

pub trait Component {
    fn handle_key(&mut self, key: KeyEvent) -> Option<Action>;
    fn update(&mut self, action: &Action);
    fn draw(&mut self, f: &mut Frame, area: Rect, state: &AppState);
}
