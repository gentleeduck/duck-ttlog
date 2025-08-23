mod app;
mod main_widget;
mod monotring;
mod stats_widget;
mod tabs_widget;
mod times;
mod widget;

use ratatui::{
  crossterm::event::{self, Event, KeyCode},
  layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Style},
  symbols::line::ROUNDED,
  text::{Line, Span, Text},
  widgets::{Block, BorderType, Borders, Paragraph},
  Frame,
};

use crate::{
  main_widget::MainWidget, stats_widget::StatsWidget, tabs_widget::ListWidget, widget::Widget,
};

fn main() -> color_eyre::Result<()> {
  color_eyre::install()?;
  let terminal = ratatui::init();
  let result = app_run(terminal);
  ratatui::restore();
  result
}

fn app_run(mut terminal: ratatui::DefaultTerminal) -> color_eyre::Result<()> {
  let mut main = MainWidget::new();
  let mut list = ListWidget::new();
  let mut stats = StatsWidget::new();

  loop {
    terminal.draw(|f| reader_ui(f, &mut main, &mut list, &mut stats))?;

    if event::poll(std::time::Duration::from_millis(100))? {
      match event::read()? {
        // Global checking for q pressing to quite.
        Event::Key(k) => {
          list.on_key(k);
          stats.on_key(k);
        },
        Event::Mouse(m) => {
          list.on_mouse(m);
          stats.on_mouse(m);
        },
        _ => {},
      }
    }
  }
}

pub fn reader_ui(
  f: &mut Frame<'_>,
  main: &mut MainWidget,
  list: &mut ListWidget,
  stats: &mut StatsWidget,
) {
  let area = f.area();

  let mut b = Block::default()
    .title("") // we’ll render custom title manually
    .borders(Borders::ALL)
    .border_type(BorderType::Rounded)
    .border_style(Style::default().fg(Color::White));

  // Inner content (after borders)
  let inner_area = area.inner(Margin {
    vertical: 1,
    horizontal: 1,
  });

  // Split vertically: header (3 rows) + content (rest)
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(inner_area);

  let first_layer = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(chunks[0]);

  main.render(f, &b, area);
  // Render list in the second chunk (main content)
  list.render(f, &mut b, first_layer[0], true);
  stats.render(f, &mut b, first_layer[1], false);
}
