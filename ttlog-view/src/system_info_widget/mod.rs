use crossterm::event::{KeyEvent, MouseEvent};

use ratatui::{
  layout::Rect,
  style::{Color, Style},
  widgets::{Block, BorderType, Borders},
  Frame,
};

use crate::widget::Widget;
pub struct SystemInfoWidget {
  pub id: u8,
  pub title: &'static str,
  pub snapshots: Vec<u8>,
  pub area: Option<Rect>,
  pub focused: bool,
}

impl SystemInfoWidget {
  pub fn new() -> Self {
    Self {
      id: 4,
      title: "~ System Info ~",
      snapshots: Vec::new(),
      area: None,
      focused: false,
    }
  }
}

impl Widget for SystemInfoWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
      .title(self.title)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    f.render_widget(block, area);
  }

  fn on_key(&mut self, key: KeyEvent) {
    if !self.focused {
      return;
    }

    match key.code {
      _ => {},
    }
  }

  fn on_mouse(&mut self, me: MouseEvent) {}
}
