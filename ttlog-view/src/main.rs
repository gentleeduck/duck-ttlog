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

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
  DefaultTerminal,
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

fn main() -> Result<()> {
  color_eyre::install()?;
  let terminal = ratatui::init();
  let result = app_run(terminal);
  ratatui::restore();
  result
}

struct AppState {
  pub focused_widget: u8,
}

fn app_run(mut terminal: DefaultTerminal) -> Result<()> {
  // Try to load data quickly, but don't block if it fails
  let (snapshots, logs_info, log_events) = match try_load_data_quickly() {
    Ok(data) => data,
    Err(_) => {
      // If loading fails or takes too long, start with empty data
      (Vec::new(), crate::logs::LogsInfo {
        total_size: 0,
        total_size_formatted: "0 B".to_string(),
        total_files: 0,
        bin_files: crate::logs::BinFilesInfo {
          count: 0,
          total_size: 0,
          total_size_formatted: "0 B".to_string(),
          min_size: None,
          min_size_formatted: None,
          max_size: None,
          max_size_formatted: None,
          avg_size: 0,
          avg_size_formatted: "0 B".to_string(),
          files: Vec::new(),
        },
        log_files: crate::logs::LogFilesInfo {
          count: 0,
          total_size: 0,
          total_size_formatted: "0 B".to_string(),
          total_lines: 0,
          total_events: 0,
          files: Vec::new(),
        },
        directory_path: "./tmp".to_string(),
      }, Vec::new())
    }
  };

  let mut app_state = AppState { focused_widget: 0 };
  let mut main = MainWidget::new();
  let mut list = ListWidget::new();
  let mut logs = LogsWidget::new(&log_events);
  let mut logs_chart = LogsChartWidget::new(&log_events);
  let mut snapshots_widget = SnapshotWidget::new(&snapshots);
  let mut events_graph = EventsGraphWidget::new();
  let mut system_info = SystemInfoWidget::new(&logs_info);

  list.focused = app_state.focused_widget == list.id;
  logs.focused = app_state.focused_widget == logs.id;
  logs_chart.focused = app_state.focused_widget == logs_chart.id;
  snapshots_widget.focused = app_state.focused_widget == snapshots_widget.id;
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
        &mut snapshots_widget,
        &mut events_graph,
        &mut system_info,
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
              snapshots_widget.focused = app_state.focused_widget == snapshots_widget.id;
              events_graph.focused = app_state.focused_widget == events_graph.id;
              system_info.focused = app_state.focused_widget == system_info.id;
            },
            KeyCode::BackTab => {
              app_state.focused_widget = (app_state.focused_widget + 6 - 1) % 6;
              list.focused = app_state.focused_widget == list.id;
              logs.focused = app_state.focused_widget == logs.id;
              logs_chart.focused = app_state.focused_widget == logs_chart.id;
              snapshots_widget.focused = app_state.focused_widget == snapshots_widget.id;
              events_graph.focused = app_state.focused_widget == events_graph.id;
              system_info.focused = app_state.focused_widget == system_info.id;
            },
            _ => {},
          }
          list.on_key(k);
          logs.on_key(k);
          logs_chart.on_key(k);
          snapshots_widget.on_key(k);
          events_graph.on_key(k);
          system_info.on_key(k);
        },
        Event::Mouse(m) => {
          list.on_mouse(m);
          logs.on_mouse(m);
          logs_chart.on_mouse(m);
          snapshots_widget.on_mouse(m);
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
  f.render_widget(b, area);
}

// Ultra-fast startup with minimal data loading
fn try_load_data_quickly() -> Result<(Vec<crate::snapshots::SnapshotFile>, crate::logs::LogsInfo, Vec<crate::logs::ResolvedLog>), Box<dyn std::error::Error>> {
  use std::time::{Duration, Instant};
  
  let start_time = Instant::now();
  eprintln!("[STARTUP] Starting quick data load...");
  
  // Load snapshots first (usually faster)
  let snapshots_start = Instant::now();
  let snapshots = match Snapshots::read_snapshots("./tmp") {
    Ok(snapshots) => {
      eprintln!("[STARTUP] Snapshots loaded in {:.2}ms", snapshots_start.elapsed().as_secs_f64() * 1000.0);
      snapshots
    },
    Err(e) => {
      eprintln!("[STARTUP] Failed to load snapshots: {}", e);
      Vec::new()
    }
  };
  
  // For logs, try a MUCH more aggressive approach
  let logs_start = Instant::now();
  let (logs_info, log_events) = match try_load_logs_ultra_fast() {
    Ok(data) => {
      eprintln!("[STARTUP] Logs loaded in {:.2}ms", logs_start.elapsed().as_secs_f64() * 1000.0);
      data
    },
    Err(e) => {
      eprintln!("[STARTUP] Failed to load logs: {}", e);
      (create_empty_logs_info(), Vec::new())
    }
  };
  
  eprintln!("[STARTUP] Total startup time: {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
  Ok((snapshots, logs_info, log_events))
}

// Ultra-fast log loading - only load what we absolutely need
fn try_load_logs_ultra_fast() -> Result<(crate::logs::LogsInfo, Vec<crate::logs::ResolvedLog>), Box<dyn std::error::Error>> {
  use std::time::Instant;
  
  let start_time = Instant::now();
  
  // Step 1: Get logs info (directory scan) - usually fast
  let info_start = Instant::now();
  let logs_info = match Logs::instance().get_logs_info("./tmp") {
    Ok(info) => {
      eprintln!("[STARTUP] Logs info loaded in {:.2}ms", info_start.elapsed().as_secs_f64() * 1000.0);
      info
    },
    Err(_) => return Ok((create_empty_logs_info(), Vec::new())),
  };
  
  // Step 2: Load only a SMALL sample of logs for immediate display
  let sample_start = Instant::now();
  let sample_logs = match load_log_sample("./tmp/ttlog.log", 100) { // Only 100 logs!
    Ok(logs) => {
      eprintln!("[STARTUP] Sample logs ({} entries) loaded in {:.2}ms", logs.len(), sample_start.elapsed().as_secs_f64() * 1000.0);
      logs
    },
    Err(_) => Vec::new(),
  };
  
  eprintln!("[STARTUP] Ultra-fast log loading completed in {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
  Ok((logs_info, sample_logs))
}

// Load only a small sample of logs for immediate display
fn load_log_sample(path: &str, limit: usize) -> Result<Vec<crate::logs::ResolvedLog>, Box<dyn std::error::Error>> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};
  
  let file = File::open(path)?;
  let reader = BufReader::with_capacity(64 * 1024, file);
  let mut events = Vec::with_capacity(limit);
  let mut count = 0;
  
  for line in reader.lines() {
    if count >= limit {
      break; // Stop after getting enough samples
    }
    
    let line = line?;
    if line.trim().is_empty() {
      continue;
    }
    
    // Use the fast parsing method
    if let Ok(log_event) = serde_json::from_str::<crate::logs::LogFileEvent>(&line) {
      let event = crate::logs::ResolvedLog {
        level: ttlog::event::LogLevel::from_u8(&log_event.level),
        timestamp: crate::utils::Utils::format_timestamp(log_event.timestamp),
        thread_id: log_event.thread_id,
        message: log_event.message,
        target: log_event.target,
        kv: log_event.kv,
        file: log_event.file,
        position: log_event.position,
      };
      events.push(event);
      count += 1;
    }
  }
  
  Ok(events)
}

fn create_empty_logs_info() -> crate::logs::LogsInfo {
  crate::logs::LogsInfo {
    total_size: 0,
    total_size_formatted: "0 B".to_string(),
    total_files: 0,
    bin_files: crate::logs::BinFilesInfo {
      count: 0,
      total_size: 0,
      total_size_formatted: "0 B".to_string(),
      min_size: None,
      min_size_formatted: None,
      max_size: None,
      max_size_formatted: None,
      avg_size: 0,
      avg_size_formatted: "0 B".to_string(),
      files: Vec::new(),
    },
    log_files: crate::logs::LogFilesInfo {
      count: 0,
      total_size: 0,
      total_size_formatted: "0 B".to_string(),
      total_lines: 0,
      total_events: 0,
      files: Vec::new(),
    },
    directory_path: "./tmp".to_string(),
  }
}

// The old implementation that was causing the slowdown
fn try_load_data_quickly_old() -> Result<(Vec<crate::snapshots::SnapshotFile>, crate::logs::LogsInfo, Vec<crate::logs::ResolvedLog>), Box<dyn std::error::Error>> {
  use std::time::{Duration, Instant};
  
  let start_time = Instant::now();
  let timeout = Duration::from_millis(500); // 500ms timeout
  
  // Try to load snapshots quickly
  let snapshots = if start_time.elapsed() < timeout {
    match Snapshots::read_snapshots("./tmp") {
      Ok(snapshots) => snapshots,
      Err(_) => Vec::new(), // Continue with empty data if fails
    }
  } else {
    Vec::new()
  };
  
  // Try to load logs quickly
  let (logs_info, log_events) = if start_time.elapsed() < timeout {
    match Logs::get_complete_logs_data_static("./tmp", "./tmp/ttlog.log") {
      Ok(data) => data,
      Err(_) => {
        // Continue with empty data if fails
        (create_empty_logs_info(), Vec::new())
      }
    }
  } else {
    (create_empty_logs_info(), Vec::new())
  };
  
  Ok((snapshots, logs_info, log_events))
}
