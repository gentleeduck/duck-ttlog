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

const LOG_DIRECTORY: &str = "./tmp";

use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
  layout::{Constraint, Direction, Layout, Margin},
  style::{Color, Style},
  widgets::{Block, BorderType, Borders},
  DefaultTerminal, Frame,
};

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

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
  pub last_directory_check: Instant,
  pub directory_modified_time: std::time::SystemTime,
  pub needs_refresh: bool,
  pub is_refreshing: bool,
  pub last_refresh_time: Option<Instant>,
}

// Global state for file monitoring
static FILE_MONITOR: std::sync::OnceLock<Arc<Mutex<FileMonitor>>> = std::sync::OnceLock::new();

#[derive(Debug, Clone)]
struct FileStats {
  pub line_count: usize,
  pub last_modified: std::time::SystemTime,
  pub events_per_sec: f64,
  pub recent_additions: Vec<(Instant, usize)>, // (timestamp, lines_added)
}

struct FileMonitor {
  pub monitored_files: HashMap<String, FileStats>,
  pub total_events_per_sec: f64,
  pub last_update: Instant,
  pub dashboard_needs_refresh: bool,
  pub last_log_file_count: usize,
  pub last_bin_file_count: usize,
  pub last_total_file_count: usize,
}

impl FileMonitor {
  fn new() -> Self {
    Self {
      monitored_files: HashMap::new(),
      total_events_per_sec: 0.0,
      last_update: Instant::now(),
      dashboard_needs_refresh: false,
      last_log_file_count: 0,
      last_bin_file_count: 0,
      last_total_file_count: 0,
    }
  }

  fn get_total_events_per_sec(&self) -> f64 {
    self.total_events_per_sec
  }

  fn check_and_reset_dashboard_refresh(&mut self) -> bool {
    let needs_refresh = self.dashboard_needs_refresh;
    self.dashboard_needs_refresh = false;
    needs_refresh
  }

  fn update_file_counts(&mut self, log_count: usize, bin_count: usize, total_count: usize) {
    let log_changed = log_count != self.last_log_file_count;
    let bin_changed = bin_count != self.last_bin_file_count;
    let total_changed = total_count != self.last_total_file_count;

    if log_changed || bin_changed || total_changed {
      // Only trigger refresh if enough time has passed to prevent glitches
      let time_since_last = self.last_update.elapsed();
      if time_since_last.as_millis() >= 500 {
        // Minimum 500ms between refreshes
        self.dashboard_needs_refresh = true;
        self.last_update = Instant::now();
      }
      self.last_log_file_count = log_count;
      self.last_bin_file_count = bin_count;
      self.last_total_file_count = total_count;
    }
  }

  fn update_file_stats(&mut self, file_path: &str, new_line_count: usize) {
    let now = Instant::now();
    let system_time = std::time::SystemTime::now();

    if let Some(stats) = self.monitored_files.get_mut(file_path) {
      let lines_added = new_line_count.saturating_sub(stats.line_count);
      if lines_added > 0 {
        stats.recent_additions.push((now, lines_added));
        // Only trigger refresh if enough time has passed to prevent glitches
        let time_since_last = self.last_update.elapsed();
        if time_since_last.as_millis() >= 500 {
          // Minimum 500ms between refreshes
          self.dashboard_needs_refresh = true;
          self.last_update = now;
        }
      }
      stats
        .recent_additions
        .retain(|(timestamp, _)| now.duration_since(*timestamp).as_secs() <= 10);

      // Calculate events per second based on recent additions
      let total_recent_lines: usize = stats.recent_additions.iter().map(|(_, count)| count).sum();
      let time_span = if let Some((oldest, _)) = stats.recent_additions.first() {
        now.duration_since(*oldest).as_secs_f64().max(1.0)
      } else {
        1.0
      };
      stats.events_per_sec = total_recent_lines as f64 / time_span;
      stats.line_count = new_line_count;
      stats.last_modified = system_time;
    } else {
      self.monitored_files.insert(
        file_path.to_string(),
        FileStats {
          line_count: new_line_count,
          last_modified: system_time,
          events_per_sec: 0.0,
          recent_additions: Vec::new(),
        },
      );
    }

    // Update total events per second
    self.total_events_per_sec = self
      .monitored_files
      .values()
      .map(|stats| stats.events_per_sec)
      .sum();

    self.last_update = now;
  }
}

fn app_run(mut terminal: DefaultTerminal) -> Result<()> {
  // Initialize file monitor
  let file_monitor = FILE_MONITOR.get_or_init(|| Arc::new(Mutex::new(FileMonitor::new())));

  // Discover and start monitoring all log files
  let log_files = discover_log_files(LOG_DIRECTORY)?;

  // Initialize file monitor with current counts
  if let Ok((initial_log_files, initial_bin_files)) = discover_all_files(LOG_DIRECTORY) {
    if let Ok(mut monitor) = file_monitor.lock() {
      monitor.last_log_file_count = initial_log_files.len();
      monitor.last_bin_file_count = initial_bin_files.len();
      monitor.last_total_file_count = initial_log_files.len() + initial_bin_files.len();
    }
  }

  // Start file monitoring thread
  start_file_monitoring_thread(file_monitor.clone(), log_files.clone());

  // Try to load data quickly, but don't block if it fails
  let (mut snapshots, mut logs_info, mut log_events) = match try_load_data_quickly() {
    Ok(data) => data,
    Err(_) => {
      // If loading fails or takes too long, start with empty data
      (
        Vec::new(),
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
          directory_path: LOG_DIRECTORY.to_string(),
        },
        Vec::new(),
      )
    },
  };

  let mut app_state = AppState {
    focused_widget: 0,
    last_directory_check: Instant::now(),
    directory_modified_time: get_directory_modified_time(LOG_DIRECTORY)
      .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
    needs_refresh: false,
    is_refreshing: false,
    last_refresh_time: None,
  };
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

  // Removed rng as we now use real monitoring data
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
            KeyCode::Char('r') | KeyCode::F(5) => {
              // Force refresh - reload all data
              app_state.needs_refresh = true;
            },
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

    // Check for immediate dashboard refresh needs from file monitor
    let file_monitor_needs_refresh = if let Ok(mut monitor) = file_monitor.lock() {
      monitor.check_and_reset_dashboard_refresh()
    } else {
      false
    };

    if file_monitor_needs_refresh {
      app_state.needs_refresh = true;
    }

    // Check for directory changes or force refresh (less frequent fallback)
    let now = Instant::now();
    if now.duration_since(app_state.last_directory_check).as_secs() >= 1 || app_state.needs_refresh
    {
      if let Some(new_modified_time) =
        check_directory_changes(LOG_DIRECTORY, app_state.directory_modified_time)
      {
        app_state.directory_modified_time = new_modified_time;
        app_state.needs_refresh = true;
      }
      app_state.last_directory_check = now;
    }

    // Handle refresh if needed
    if app_state.needs_refresh {
      app_state.is_refreshing = true;
      app_state.last_refresh_time = Some(Instant::now());

      // Clear any cached data to ensure fresh reload
      let logs_instance = Logs::instance();
      logs_instance.clear_cache();

      // Refresh dashboard data
      if let Ok((new_snapshots, new_logs_info, new_log_events)) = try_load_data_quickly() {
        // Update the data variables
        snapshots = new_snapshots;
        logs_info = new_logs_info;
        log_events = new_log_events;

        // Update ALL widgets with new data (except events graph which updates on tick)
        main = MainWidget::new();
        list = ListWidget::new();
        snapshots_widget = SnapshotWidget::new(&snapshots);
        logs = LogsWidget::new(&log_events);
        logs_chart = LogsChartWidget::new(&log_events);
        system_info = SystemInfoWidget::new(&logs_info);
        // Note: events_graph is NOT recreated here - it updates only on tick

        // Maintain focus state for ALL widgets
        list.focused = app_state.focused_widget == list.id;
        snapshots_widget.focused = app_state.focused_widget == snapshots_widget.id;
        logs.focused = app_state.focused_widget == logs.id;
        logs_chart.focused = app_state.focused_widget == logs_chart.id;
        system_info.focused = app_state.focused_widget == system_info.id;
        events_graph.focused = app_state.focused_widget == events_graph.id;

        // Rediscover and restart monitoring for new log files
        if let Ok(new_log_files) = discover_log_files(LOG_DIRECTORY) {
          start_file_monitoring_thread(file_monitor.clone(), new_log_files);
        }
      } else {
      }

      app_state.is_refreshing = false;
      app_state.needs_refresh = false;
    }

    // Get real events/sec from file monitoring
    let events_per_sec = if let Ok(monitor) = file_monitor.lock() {
      monitor.get_total_events_per_sec() as usize
    } else {
      0
    };

    // call the tick with real data
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

// Discover log files in the directory
fn discover_log_files(directory: &str) -> Result<Vec<String>, std::io::Error> {
  use std::fs;

  let mut log_files = Vec::new();

  if let Ok(entries) = fs::read_dir(directory) {
    for entry in entries.flatten() {
      if let Some(file_name) = entry.file_name().to_str() {
        if file_name.ends_with(".log") {
          if let Some(path_str) = entry.path().to_str() {
            log_files.push(path_str.to_string());
          }
        }
      }
    }
  }

  // Always include the main log file if it exists
  let main_log = format!("{}/ttlog.log", directory);
  if Path::new(&main_log).exists() && !log_files.contains(&main_log) {
    log_files.push(main_log);
  }

  Ok(log_files)
}

// Discover both log and bin files in the directory
fn discover_all_files(directory: &str) -> Result<(Vec<String>, Vec<String>), std::io::Error> {
  use std::fs;

  let mut log_files = Vec::new();
  let mut bin_files = Vec::new();

  if let Ok(entries) = fs::read_dir(directory) {
    for entry in entries.flatten() {
      if let Some(file_name) = entry.file_name().to_str() {
        if let Some(path_str) = entry.path().to_str() {
          if file_name.ends_with(".log") {
            log_files.push(path_str.to_string());
          } else if file_name.ends_with(".bin") {
            bin_files.push(path_str.to_string());
          }
        }
      }
    }
  }

  // Always include the main log file if it exists
  let main_log = format!("{}/ttlog.log", directory);
  if Path::new(&main_log).exists() && !log_files.contains(&main_log) {
    log_files.push(main_log);
  }

  // Sort both vectors for consistent comparison
  log_files.sort();
  bin_files.sort();

  Ok((log_files, bin_files))
}

// Enhanced file monitoring with better change detection and user feedback
fn start_file_monitoring_thread(file_monitor: Arc<Mutex<FileMonitor>>, log_files: Vec<String>) {
  std::thread::spawn(move || {
    let mut current_log_files = log_files;
    let mut consecutive_errors = 0;
    const MAX_CONSECUTIVE_ERRORS: u32 = 10;

    loop {
      // Rediscover all files to detect new .log and .bin files
      if let Ok((discovered_log_files, discovered_bin_files)) = discover_all_files(LOG_DIRECTORY) {
        let total_files = discovered_log_files.len() + discovered_bin_files.len();

        // Check if file counts have changed
        if let Ok(mut monitor) = file_monitor.lock() {
          monitor.update_file_counts(
            discovered_log_files.len(),
            discovered_bin_files.len(),
            total_files,
          );
        }

        // Update current log files if they've changed
        if discovered_log_files != current_log_files {
          if let Ok(mut monitor) = file_monitor.lock() {
            monitor.dashboard_needs_refresh = true;
          }
          current_log_files = discovered_log_files;
        }
      } else {
        consecutive_errors += 1;
        if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
          consecutive_errors = 0; // Reset to prevent spam
        }
      }

      // Check each log file for line count changes
      for file_path in &current_log_files {
        if let Ok(line_count) = count_lines_in_file(file_path) {
          if let Ok(mut monitor) = file_monitor.lock() {
            let old_count = monitor
              .monitored_files
              .get(file_path)
              .map(|s| s.line_count)
              .unwrap_or(0);
            monitor.update_file_stats(file_path, line_count);
            let new_count = monitor
              .monitored_files
              .get(file_path)
              .map(|s| s.line_count)
              .unwrap_or(0);
            if new_count != old_count {}
          }
        } else {
          consecutive_errors += 1;
          if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
            consecutive_errors = 0;
          }
        }
      }

      // Sleep for 500ms to prevent TUI glitches while still being responsive
      std::thread::sleep(std::time::Duration::from_millis(500));
    }
  });
}

// Fast line counting for monitoring
fn count_lines_in_file(file_path: &str) -> Result<usize, std::io::Error> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};

  let file = File::open(file_path)?;
  let reader = BufReader::with_capacity(64 * 1024, file);
  let mut count = 0;

  for line in reader.lines() {
    line?; // Ensure line is valid
    count += 1;
  }

  Ok(count)
}

// Get directory modification time
fn get_directory_modified_time(directory: &str) -> Result<std::time::SystemTime, std::io::Error> {
  use std::fs;

  let metadata = fs::metadata(directory)?;
  metadata.modified()
}

// Check if directory has been modified since last check
fn check_directory_changes(
  directory: &str,
  last_modified: std::time::SystemTime,
) -> Option<std::time::SystemTime> {
  if let Ok(current_modified) = get_directory_modified_time(directory) {
    if current_modified > last_modified {
      return Some(current_modified);
    }
  }
  None
}

// Ultra-fast startup with minimal data loading
fn try_load_data_quickly() -> Result<
  (
    Vec<crate::snapshots::SnapshotFile>,
    crate::logs::LogsInfo,
    Vec<crate::logs::ResolvedLog>,
  ),
  Box<dyn std::error::Error>,
> {
  // Load snapshots first (usually faster)
  let snapshots = match Snapshots::read_snapshots("./tmp") {
    Ok(snapshots) => {
      // Snapshots loaded successfully
      snapshots
    },
    Err(_e) => {
      // Failed to load snapshots, using empty list
      Vec::new()
    },
  };

  // For logs, try a MUCH more aggressive approach
  let (logs_info, log_events) = match try_load_logs_ultra_fast() {
    Ok(data) => {
      // Logs loaded successfully
      data
    },
    Err(_e) => {
      // Failed to load logs info, using empty data
      (create_empty_logs_info(), Vec::new())
    },
  };

  // Startup completed
  Ok((snapshots, logs_info, log_events))
}

// Ultra-fast log loading - only load what we absolutely need
fn try_load_logs_ultra_fast(
) -> Result<(crate::logs::LogsInfo, Vec<crate::logs::ResolvedLog>), Box<dyn std::error::Error>> {
  // Step 1: Get logs info (directory scan) - FORCE FRESH RELOAD to avoid cache issues
  let logs_info = match Logs::get_logs_info_static(LOG_DIRECTORY) {
    Ok(info) => {
      // Logs info loaded successfully
      info
    },
    Err(_) => return Ok((create_empty_logs_info(), Vec::new())),
  };

  // Step 2: Try to load full dataset for charts, fallback to sample for startup speed
  let main_log_file = format!("{}/ttlog.log", LOG_DIRECTORY);
  let logs = match try_load_full_logs_fast(&main_log_file) {
    Ok(logs) => {
      // Full logs loaded successfully
      logs
    },
    Err(_) => {
      // Fallback to sample if full loading fails
      match load_log_sample(&main_log_file, 100) {
        Ok(sample_logs) => sample_logs,
        Err(_) => Vec::new(),
      }
    },
  };

  // Ultra-fast log loading completed
  Ok((logs_info, logs))
}

// Try to load full logs efficiently with timeout
fn try_load_full_logs_fast(
  path: &str,
) -> Result<Vec<crate::logs::ResolvedLog>, Box<dyn std::error::Error>> {
  use std::fs::File;
  use std::io::{BufRead, BufReader};
  use std::time::{Duration, Instant};

  let start_time = Instant::now();
  let timeout = Duration::from_millis(3000); // 3 second timeout

  // Load all logs from the file efficiently
  let file = File::open(path)?;
  let reader = BufReader::with_capacity(256 * 1024, file); // 256KB buffer
  let mut events = Vec::new();
  let mut processed_lines = 0;

  for line in reader.lines() {
    // Check timeout periodically
    if processed_lines % 1000 == 0 && start_time.elapsed() > timeout {
      return Err("Loading timeout exceeded".into());
    }

    let line = line?;
    if line.trim().is_empty() {
      continue;
    }

    // Parse log line efficiently
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
    }

    processed_lines += 1;
  }

  events.shrink_to_fit();
  Ok(events)
}

// Load only a small sample of logs for immediate display
fn load_log_sample(
  path: &str,
  limit: usize,
) -> Result<Vec<crate::logs::ResolvedLog>, Box<dyn std::error::Error>> {
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
    directory_path: LOG_DIRECTORY.to_string(),
  }
}
