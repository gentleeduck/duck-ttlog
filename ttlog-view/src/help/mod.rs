use ratatui::{
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Clear, Paragraph, Wrap},
  Frame,
};

use crate::centered_rect;

pub fn render_help(f: &mut Frame<'_>) {
  let area = centered_rect(60, 70, f.area());

  let block = Block::default()
    .title("Help - TTLog Dashboard")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Yellow));

  let help_text = Text::from(vec![
    Line::from("Navigation:"),
    Line::from(""),
    Line::from(vec![
      Span::styled(
        "Tab/Shift+Tab",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("  - Switch between views"),
    ]),
    Line::from(vec![
      Span::styled(
        "↑/k, ↓/j",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("      - Navigate logs"),
    ]),
    Line::from(""),
    Line::from("Actions:"),
    Line::from(""),
    Line::from(vec![
      Span::styled(
        "q",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("             - Quit application"),
    ]),
    Line::from(vec![
      Span::styled(
        "h, F1",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("         - Toggle this help"),
    ]),
    Line::from(vec![
      Span::styled(
        "f",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("             - Cycle log level filter"),
    ]),
    Line::from(vec![
      Span::styled(
        "r",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("             - Refresh data"),
    ]),
    Line::from(""),
    Line::from("Views:"),
    Line::from(""),
    Line::from("• Overview    - System summary and key metrics"),
    Line::from("• Logs        - Detailed log viewer with filtering"),
    Line::from("• Metrics     - System resource monitoring"),
    Line::from("• Alerts      - Alert rules and notifications"),
    Line::from("• Performance - Throughput and latency stats"),
    Line::from("• Network     - Network activity and connections"),
    Line::from(""),
    Line::from("Press Esc or any key to close..."),
  ]);

  let paragraph = Paragraph::new(help_text)
    .block(block)
    .wrap(Wrap { trim: true });

  f.render_widget(Clear, area);
  f.render_widget(paragraph, area);
}
