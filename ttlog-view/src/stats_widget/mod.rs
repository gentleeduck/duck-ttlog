use crossterm::event::{KeyEvent, MouseEvent};

use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Style},
  text::Span,
  widgets::{Block, BorderType, Borders, Paragraph},
  Frame,
};

use crate::{times::format_duration, widget::Widget};
pub struct StatsWidget {
  pub id: usize,
  pub title: &'static str,
  pub log_count: u32,
  pub log_rate: f32,
  pub snapshot_count: u32,
  pub uptime: chrono::Duration,
  pub area: Option<Rect>,
  pub focused: bool,
}

impl StatsWidget {
  pub fn new() -> Self {
    Self {
      id: 1,
      title: "Stats",
      uptime: chrono::Duration::seconds(0),
      snapshot_count: 100,
      log_rate: 100.0,
      log_count: 100,
      area: None,
      focused: false,
    }
  }
}

impl Widget for StatsWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
      .title("TTLog Stats")
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    // split horizontal space into 3 cols
    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
      ])
      .split(block.inner(area));

    let (d, h, m, s) = format_duration(self.uptime);

    let logs = Paragraph::new(Span::styled(
      format!("Logs: {}", self.log_count),
      Style::default().fg(Color::Green),
    ));

    let rate = Paragraph::new(Span::styled(
      format!("Rate: {:.1}/s", self.log_rate),
      Style::default().fg(Color::Green),
    ));

    let uptime = Paragraph::new(Span::styled(
      format!("Uptime: {d}:{h}:{m}:{s}"),
      Style::default().fg(Color::Green),
    ));

    // Render block first
    f.render_widget(block, area);

    // Then render items inside
    f.render_widget(logs, chunks[0]);
    f.render_widget(rate, chunks[1]);
    f.render_widget(uptime, chunks[2]);
  }

  fn on_key(&mut self, key: KeyEvent) {
    match key.code {
      _ => {},
    }
  }

  fn on_mouse(&mut self, me: MouseEvent) {}
}
