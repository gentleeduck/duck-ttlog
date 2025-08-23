use ratatui::{
  layout::Rect,
  style::{Color, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Paragraph, Wrap},
  Frame,
};

pub struct Monotoring {
  pub log_count: u32,
  pub log_rate: f32,
  pub snapshot_count: u32,
  pub uptime: chrono::Duration,
}

impl Monotoring {
  pub fn render_monotoring(f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
      .title("Memory & Interning")
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Cyan));

    let text = Text::from(vec![
      Line::from(vec![
        Span::styled("Interner size:     ", Style::default().fg(Color::White)),
        Span::styled(format!("{:>8.1} MB", 0), Style::default().fg(Color::Green)),
      ]),
      Line::from(vec![
        Span::styled("Strings interned:  ", Style::default().fg(Color::White)),
        Span::styled(format!("{:>8}", 0), Style::default().fg(Color::Green)),
      ]),
      Line::from(vec![
        Span::styled("Snapshot size avg: ", Style::default().fg(Color::White)),
        Span::styled(format!("{:>8.1} MB", 0), Style::default().fg(Color::Green)),
      ]),
      Line::from(vec![
        Span::styled("Flush interval:    ", Style::default().fg(Color::White)),
        Span::styled(format!("{:>8}s", 0), Style::default().fg(Color::Green)),
      ]),
    ]);

    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

    f.render_widget(paragraph, area);
  }
}
