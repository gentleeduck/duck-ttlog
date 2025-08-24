use ratatui::{
  style::{Color, Style},
  widgets::{Block, BorderType, Borders},
};

use crate::widget::Widget;

pub struct LogsWidget {
  id: u8,
  title: &'static str,
}

impl LogsWidget {
  pub fn new() -> Self {
    Self {
      id: 2,
      title: "~ Logs ~",
    }
  }
}

impl Widget for LogsWidget {
  fn render(&mut self, f: &mut ratatui::Frame<'_>, area: ratatui::prelude::Rect, focused: bool) {
    let block = Block::default()
      .title(self.title)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(block, area);
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {}

  fn on_mouse(&mut self, me: crossterm::event::MouseEvent) {}
}
