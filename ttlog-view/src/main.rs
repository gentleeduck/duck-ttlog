mod app;
mod logs_graph_widget;
mod logs_widget;
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
  logs_graph_widget::LogsGraphWidget, logs_widget::LogsWidget, main_widget::MainWidget,
  stats_widget::StatsWidget, tabs_widget::ListWidget, widget::Widget,
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
  // let mut stats = StatsWidget::new();
  let mut logs = LogsWidget::new();
  let mut logs_graph = LogsGraphWidget::new();

  loop {
    terminal.draw(|f| reader_ui(f, &mut main, &mut list, &mut logs, &mut logs_graph))?;

    if event::poll(std::time::Duration::from_millis(100))? {
      match event::read()? {
        // Global checking for q pressing to quite.
        Event::Key(k) if matches!(k.code, KeyCode::Char('q')) => break Ok(()),
        Event::Key(k) => {
          list.on_key(k);
          // logs.on_key(k);
          logs_graph.on_key(k);
        },
        Event::Mouse(m) => {
          list.on_mouse(m);
          // logs.on_mouse(m);
          logs_graph.on_mouse(m);
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
  logs: &mut LogsWidget,
  logs_graph: &mut LogsGraphWidget,
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
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(inner_area);

  let first_layer = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(chunks[0]);

  let second_layer = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(first_layer[1]);

  main.render(f, &b, area);

  list.render(f, first_layer[0], true);
  logs.render(f, second_layer[0], true);
  logs_graph.render(f, second_layer[1], true);
}
