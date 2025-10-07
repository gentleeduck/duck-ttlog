mod events_graph_widget;
mod logs;
mod logs_chart_widget;
mod logs_widget;
mod main_widget;
mod snapshot_widget;
mod snapshots;
mod system_info_widget;
mod tabs_widget;
mod utils;
mod widget;

use ratatui::{
  crossterm::event::{self, Event, KeyCode},
  layout::{Constraint, Direction, Layout, Margin},
  style::{Color, Style},
  widgets::{Block, BorderType, Borders},
  Frame,
};

use rand::Rng;

use crate::{
  events_graph_widget::EventsGraphWidget, logs::Logs, logs_chart_widget::LogsChartWidget,
  logs_widget::LogsWidget, main_widget::MainWidget, snapshot_widget::SnapshotWidget,
  snapshots::Snapshots, system_info_widget::SystemInfoWidget, tabs_widget::ListWidget,
  widget::Widget,
};

fn main() -> color_eyre::Result<()> {
  color_eyre::install()?;
  let terminal = ratatui::init();
  let result = app_run(terminal);
  ratatui::restore();
  result
}

struct AppState {
  pub focused_widget: u8,
}

fn app_run(mut terminal: ratatui::DefaultTerminal) -> color_eyre::Result<()> {
  let logs_vec = Logs::get_logs("tmp/ttlog.log");
  let snapshots = Snapshots::read_snapshots("./tmp").unwrap_or_default();

  let mut app_state = AppState { focused_widget: 0 };
  let mut main = MainWidget::new();
  let mut list = ListWidget::new();
  let mut logs = LogsWidget::new(&logs_vec);
  let mut logs_chart = LogsChartWidget::new(&logs_vec);
  let mut snapshots = SnapshotWidget::new(&snapshots);
  // let mut snapshots_events = LogsWidget::new(&logs_vec);
  let mut events_graph = EventsGraphWidget::new();
  let mut system_info = SystemInfoWidget::new();

  list.focused = app_state.focused_widget == list.id;
  logs.focused = app_state.focused_widget == logs.id;
  logs_chart.focused = app_state.focused_widget == logs_chart.id;
  snapshots.focused = app_state.focused_widget == snapshots.id;
  events_graph.focused = app_state.focused_widget == events_graph.id;
  system_info.focused = app_state.focused_widget == system_info.id;

  let mut rng = rand::thread_rng();
  loop {
    terminal.draw(|f| {
      reader_ui(
        f,
        &mut main,
        &mut list,
        &mut logs,
        &mut logs_chart,
        &mut snapshots,
        &mut events_graph,
        &mut system_info,
        // &mut snapshots_events,
      )
    })?;

    if event::poll(std::time::Duration::from_millis(100))? {
      match event::read()? {
        // Global checking for q pressing to quite.
        Event::Key(k) => {
          match k.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Tab => {
              app_state.focused_widget = (app_state.focused_widget + 1) % 6;
              list.focused = app_state.focused_widget == list.id;
              logs.focused = app_state.focused_widget == logs.id;
              logs_chart.focused = app_state.focused_widget == logs_chart.id;
              snapshots.focused = app_state.focused_widget == snapshots.id;
              events_graph.focused = app_state.focused_widget == events_graph.id;
              system_info.focused = app_state.focused_widget == system_info.id;
            },
            KeyCode::BackTab => {
              app_state.focused_widget = (app_state.focused_widget + 6 - 1) % 6;
              list.focused = app_state.focused_widget == list.id;
              logs.focused = app_state.focused_widget == logs.id;
              logs_chart.focused = app_state.focused_widget == logs_chart.id;
              snapshots.focused = app_state.focused_widget == snapshots.id;
              events_graph.focused = app_state.focused_widget == events_graph.id;
              system_info.focused = app_state.focused_widget == system_info.id;
            },
            _ => {},
          }
          list.on_key(k);
          logs.on_key(k);
          logs_chart.on_key(k);
          snapshots.on_key(k);
          // snapshots_events.on_key(k);
          events_graph.on_key(k);
          system_info.on_key(k);
        },
        Event::Mouse(m) => {
          list.on_mouse(m);
          logs.on_mouse(m);
          logs_chart.on_mouse(m);
          snapshots.on_mouse(m);
          // snapshots_events.on_mouse(m);
          events_graph.on_mouse(m);
          system_info.on_mouse(m);
        },
        _ => {},
      }
    }

    // Example: simulate external events/sec
    let events_per_sec = rng.gen_range(300000..800000);

    // call the tick
    events_graph.on_tick(events_per_sec);
  }
}

pub fn reader_ui(
  f: &mut Frame<'_>,
  main: &mut MainWidget,
  list: &mut ListWidget,
  logs: &mut LogsWidget,
  logs_chart: &mut LogsChartWidget,
  snapshots: &mut SnapshotWidget,
  events_graph: &mut EventsGraphWidget,
  system_info: &mut SystemInfoWidget,
  // snapshots_events: &mut LogsWidget,
) {
  let area = f.area();

  let b = Block::default()
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

  let left_side_l_1 = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(chunks[0]);

  let lef_side_l_2 = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(left_side_l_1[1]);

  let right_side_l_1 = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Percentage(33),
      Constraint::Percentage(23),
      Constraint::Percentage(44),
    ])
    .split(chunks[1]);

  main.render(f, &b, area);

  // Render Left Side Widgets
  list.render(f, left_side_l_1[0]);
  logs.render(f, lef_side_l_2[0]);
  logs_chart.render(f, lef_side_l_2[1]);

  // Render Right Side Widgets
  events_graph.render(f, right_side_l_1[0]);
  system_info.render(f, right_side_l_1[1]);
  snapshots.render(f, right_side_l_1[2]);
}
