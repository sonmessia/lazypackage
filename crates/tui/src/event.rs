use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;

/// Events the main loop processes each iteration.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Resize(u16, u16),
    /// Fired when no crossterm event arrives within the poll window (≈16 ms).
    Tick,
}

/// Non-blocking event poll (up to 16 ms).
///
/// Returns `Some(AppEvent::Tick)` when no terminal event is available so the
/// caller always gets a chance to repaint and process channel messages.
pub fn poll_event() -> Option<AppEvent> {
    if event::poll(Duration::from_millis(16)).unwrap_or(false) {
        match event::read() {
            Ok(CrosstermEvent::Key(k)) => Some(AppEvent::Key(k)),
            Ok(CrosstermEvent::Resize(w, h)) => Some(AppEvent::Resize(w, h)),
            _ => None,
        }
    } else {
        Some(AppEvent::Tick)
    }
}
