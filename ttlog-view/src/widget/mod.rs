use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{layout::Rect, Frame};

/// Trait every widget will implement
pub trait Widget {
  /// Render into the given area. `focused` can be used to show focus style.
  fn render(&mut self, f: &mut Frame<'_>, area: Rect);
  /// Key events delivered to widget when it is focused.
  fn on_key(&mut self, key: KeyEvent);
  /// Mouse events delivered to widget (coords are global Frame coords).
  fn on_mouse(&mut self, me: MouseEvent);
}
