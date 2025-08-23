mod help;
use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  backend::{Backend, CrosstermBackend},
  layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
  style::{Color, Modifier, Style},
  symbols,
  text::{Line, Span, Text},
  widgets::{
    Axis, BarChart, Block, Borders, Cell, Chart, Clear, Dataset, Gauge, GraphType, List, ListItem,
    ListState, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Sparkline, Table,
    TableState, Tabs, Wrap,
  },
  Frame, Terminal,
};
use std::{
  collections::{HashMap, VecDeque},
  error::Error,
  io,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
struct LogEntry {
  id: u64,
  timestamp: String,
  level: LogLevel,
  component: String,
  message: String,
  duration_ms: Option<u32>,
  memory_kb: Option<u32>,
  thread_id: u16,
  source_file: String,
  line_number: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
  Fatal,
}

impl LogLevel {
  fn color(&self) -> Color {
    match self {
      LogLevel::Trace => Color::Rgb(128, 128, 128),
      LogLevel::Debug => Color::Cyan,
      LogLevel::Info => Color::Green,
      LogLevel::Warn => Color::Yellow,
      LogLevel::Error => Color::Red,
      LogLevel::Fatal => Color::Magenta,
    }
  }

  fn as_str(&self) -> &str {
    match self {
      LogLevel::Trace => "TRACE",
      LogLevel::Debug => "DEBUG",
      LogLevel::Info => "INFO",
      LogLevel::Warn => "WARN",
      LogLevel::Error => "ERROR",
      LogLevel::Fatal => "FATAL",
    }
  }

  fn priority(&self) -> u8 {
    match self {
      LogLevel::Trace => 0,
      LogLevel::Debug => 1,
      LogLevel::Info => 2,
      LogLevel::Warn => 3,
      LogLevel::Error => 4,
      LogLevel::Fatal => 5,
    }
  }
}

#[derive(Debug)]
struct SystemMetrics {
  cpu_usage: f64,
  memory_usage: f64,
  disk_io_read: u64,
  disk_io_write: u64,
  network_rx: u64,
  network_tx: u64,
  threads_active: u32,
  connections_open: u32,
}

#[derive(Debug)]
struct PerformanceStats {
  logs_processed: u64,
  logs_per_sec: f64,
  avg_processing_time_us: f64,
  errors_per_minute: f64,
  memory_allocated_mb: f64,
  gc_collections: u32,
  cache_hit_ratio: f64,
  queue_depth: u32,
}

#[derive(Debug)]
struct AlertRule {
  name: String,
  condition: String,
  threshold: f64,
  severity: LogLevel,
  triggered: bool,
  last_triggered: Option<Instant>,
}

#[derive(Debug, Clone)]
enum ViewMode {
  Overview,
  Logs,
  Metrics,
  Alerts,
  Performance,
  Network,
}

impl ViewMode {
  fn as_str(&self) -> &str {
    match self {
      ViewMode::Overview => "Overview",
      ViewMode::Logs => "Logs",
      ViewMode::Metrics => "Metrics",
      ViewMode::Alerts => "Alerts",
      ViewMode::Performance => "Performance",
      ViewMode::Network => "Network",
    }
  }
}

struct App {
  // Core data
  logs: Vec<LogEntry>,
  filtered_logs: Vec<LogEntry>,
  performance_history: VecDeque<(f64, f64)>, // (time, value) for charts
  metrics_history: VecDeque<SystemMetrics>,
  alerts: Vec<AlertRule>,

  // UI state
  current_view: ViewMode,
  view_tabs: Vec<ViewMode>,
  selected_tab: usize,
  show_help: bool,

  // Log view state
  log_list_state: ListState,
  log_scroll_state: ScrollbarState,
  log_filter: String,
  log_level_filter: Option<LogLevel>,
  component_filter: String,

  // Metrics state
  selected_metric: usize,

  // Performance state
  performance_stats: PerformanceStats,

  // Table state for various views
  table_state: TableState,

  // System state
  uptime: Duration,
  last_update: Instant,
  update_counter: u64,

  // Statistics
  log_stats: HashMap<LogLevel, u64>,
  component_stats: HashMap<String, u64>,
  hourly_stats: [u64; 24],
}

impl App {
  fn new() -> App {
    let mut app = App {
      logs: Vec::new(),
      filtered_logs: Vec::new(),
      performance_history: VecDeque::with_capacity(100),
      metrics_history: VecDeque::with_capacity(100),
      alerts: vec![
        AlertRule {
          name: "High Error Rate".to_string(),
          condition: "errors_per_minute > 10".to_string(),
          threshold: 10.0,
          severity: LogLevel::Error,
          triggered: false,
          last_triggered: None,
        },
        AlertRule {
          name: "Memory Usage".to_string(),
          condition: "memory_usage > 80%".to_string(),
          threshold: 80.0,
          severity: LogLevel::Warn,
          triggered: false,
          last_triggered: None,
        },
        AlertRule {
          name: "Queue Depth".to_string(),
          condition: "queue_depth > 1000".to_string(),
          threshold: 1000.0,
          severity: LogLevel::Warn,
          triggered: false,
          last_triggered: None,
        },
      ],
      current_view: ViewMode::Overview,
      view_tabs: vec![
        ViewMode::Overview,
        ViewMode::Logs,
        ViewMode::Metrics,
        ViewMode::Alerts,
        ViewMode::Performance,
        ViewMode::Network,
      ],
      selected_tab: 0,
      show_help: false,
      log_list_state: ListState::default(),
      log_scroll_state: ScrollbarState::new(0),
      log_filter: String::new(),
      log_level_filter: None,
      component_filter: String::new(),
      selected_metric: 0,
      performance_stats: PerformanceStats {
        logs_processed: 0,
        logs_per_sec: 0.0,
        avg_processing_time_us: 0.0,
        errors_per_minute: 0.0,
        memory_allocated_mb: 0.0,
        gc_collections: 0,
        cache_hit_ratio: 0.0,
        queue_depth: 0,
      },
      table_state: TableState::default(),
      uptime: Duration::from_secs(0),
      last_update: Instant::now(),
      update_counter: 0,
      log_stats: HashMap::new(),
      component_stats: HashMap::new(),
      hourly_stats: [0; 24],
    };

    // Generate initial data
    app.generate_initial_logs();
    app.filter_logs();
    app.update_statistics();

    app
  }

  fn generate_initial_logs(&mut self) {
    let components = [
      "Auth",
      "Database",
      "Cache",
      "API",
      "Worker",
      "Scheduler",
      "Network",
      "Storage",
      "Security",
      "Monitoring",
    ];
    let messages = [
      "Connection established successfully",
      "Query executed in {}ms",
      "Cache miss for key: {}",
      "Request processed",
      "Timeout occurred after {}ms",
      "Authentication failed for user",
      "Memory allocation: {}KB",
      "Background job started",
      "Configuration reloaded",
      "Health check passed",
      "Rate limit exceeded",
      "Database connection pool exhausted",
      "SSL handshake completed",
      "Backup operation started",
      "Performance threshold exceeded",
    ];

    for i in 0..1000 {
      let level = match rand::random_u64() % 100 {
        0..=60 => LogLevel::Info,
        61..=75 => LogLevel::Debug,
        76..=85 => LogLevel::Warn,
        86..=95 => LogLevel::Trace,
        96..=98 => LogLevel::Error,
        _ => LogLevel::Fatal,
      };

      let component = components[rand::random_usize() % components.len()];
      let message_template = messages[rand::random_usize() % messages.len()];

      let message = if message_template.contains("{}") {
        message_template.replace("{}", &(rand::random_u32() % 1000).to_string())
      } else {
        message_template.to_string()
      };

      let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
      let timestamp_offset = rand::random_u64() % 86400; // Random within last day
      let log_time = now.as_secs().saturating_sub(timestamp_offset);

      let timestamp = format!(
        "2025-08-23 {:02}:{:02}:{:02}",
        (log_time / 3600) % 24,
        (log_time / 60) % 60,
        log_time % 60
      );

      self.logs.push(LogEntry {
        id: i,
        timestamp,
        level,
        component: component.to_string(),
        message,
        duration_ms: if rand::random_f64() < 0.3 {
          Some(rand::random_u32() % 1000)
        } else {
          None
        },
        memory_kb: if rand::random_f64() < 0.2 {
          Some(rand::random_u32() % 10000)
        } else {
          None
        },
        thread_id: (rand::random_u16() % 20) + 1,
        source_file: format!("src/{}.rs", component.to_lowercase()),
        line_number: (rand::random_u32() % 500) + 1,
      });
    }

    // Sort by timestamp (newest first)
    self.logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
  }

  fn update(&mut self) {
    if self.last_update.elapsed() > Duration::from_millis(1000) {
      self.update_counter += 1;
      self.uptime = Duration::from_secs(self.update_counter);

      // Add new logs occasionally
      if rand::random_f64() < 0.7 {
        self.add_random_log();
      }

      // Update performance stats
      self.update_performance_stats();

      // Update system metrics
      self.update_system_metrics();

      // Check alerts
      self.check_alerts();

      // Update filtered logs
      self.filter_logs();
      self.update_statistics();

      self.last_update = Instant::now();
    }
  }

  fn add_random_log(&mut self) {
    let components = ["Auth", "Database", "Cache", "API", "Worker"];
    let messages = [
      "New connection established",
      "Request processed successfully",
      "Cache updated",
      "Error occurred during processing",
      "Background task completed",
    ];

    let level = match rand::random_u64() % 100 {
      0..=65 => LogLevel::Info,
      66..=80 => LogLevel::Debug,
      81..=90 => LogLevel::Warn,
      91..=97 => LogLevel::Error,
      98..=99 => LogLevel::Trace,
      _ => LogLevel::Fatal,
    };

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let timestamp = format!(
      "2025-08-23 {:02}:{:02}:{:02}",
      (now.as_secs() / 3600) % 24,
      (now.as_secs() / 60) % 60,
      now.as_secs() % 60
    );

    let component = components[rand::random_usize() % components.len()];

    let log = LogEntry {
      id: self.logs.len() as u64,
      timestamp,
      level,
      component: component.to_string(),
      message: messages[rand::random_usize() % messages.len()].to_string(),
      duration_ms: if rand::random_f64() < 0.3 {
        Some(rand::random_u32() % 500)
      } else {
        None
      },
      memory_kb: if rand::random_f64() < 0.2 {
        Some(rand::random_u32() % 5000)
      } else {
        None
      },
      thread_id: (rand::random_u16() % 20) + 1,
      source_file: format!("src/{}.rs", component.to_lowercase()),
      line_number: (rand::random_u32() % 500) + 1,
    };

    self.logs.insert(0, log);

    // Keep logs manageable
    if self.logs.len() > 5000 {
      self.logs.truncate(5000);
    }
  }

  fn update_performance_stats(&mut self) {
    // Simulate performance metrics
    self.performance_stats.logs_processed += rand::random_u64() % 100;
    self.performance_stats.logs_per_sec = (rand::random_f64() * 50.0) + 10.0;
    self.performance_stats.avg_processing_time_us = (rand::random_f64() * 1000.0) + 100.0;
    self.performance_stats.errors_per_minute = rand::random_f64() * 20.0;
    self.performance_stats.memory_allocated_mb = (rand::random_f64() * 500.0) + 100.0;
    self.performance_stats.gc_collections += rand::random_u32() % 3;
    self.performance_stats.cache_hit_ratio = 0.7 + (rand::random_f64() * 0.3);
    self.performance_stats.queue_depth = rand::random_u32() % 2000;

    // Add to history for charts
    let time = self.uptime.as_secs_f64();
    self
      .performance_history
      .push_back((time, self.performance_stats.logs_per_sec));
    if self.performance_history.len() > 100 {
      self.performance_history.pop_front();
    }
  }

  fn update_system_metrics(&mut self) {
    let metrics = SystemMetrics {
      cpu_usage: rand::random_f64() * 100.0,
      memory_usage: 20.0 + (rand::random_f64() * 70.0),
      disk_io_read: rand::random_u64() % 1000000,
      disk_io_write: rand::random_u64() % 500000,
      network_rx: rand::random_u64() % 10000000,
      network_tx: rand::random_u64() % 5000000,
      threads_active: 10 + (rand::random_u32() % 90),
      connections_open: rand::random_u32() % 1000,
    };

    self.metrics_history.push_back(metrics);
    if self.metrics_history.len() > 100 {
      self.metrics_history.pop_front();
    }
  }

  fn check_alerts(&mut self) {
    for alert in &mut self.alerts {
      let triggered = match alert.name.as_str() {
        "High Error Rate" => self.performance_stats.errors_per_minute > alert.threshold,
        "Memory Usage" => self
          .metrics_history
          .back()
          .map_or(false, |m| m.memory_usage > alert.threshold),
        "Queue Depth" => self.performance_stats.queue_depth as f64 > alert.threshold,
        _ => false,
      };

      if triggered && !alert.triggered {
        alert.last_triggered = Some(Instant::now());
      }
      alert.triggered = triggered;
    }
  }

  fn filter_logs(&mut self) {
    self.filtered_logs = self
      .logs
      .iter()
      .filter(|log| {
        // Level filter
        if let Some(ref level) = self.log_level_filter {
          if log.level.priority() < level.priority() {
            return false;
          }
        }

        // Component filter
        if !self.component_filter.is_empty() {
          if !log
            .component
            .to_lowercase()
            .contains(&self.component_filter.to_lowercase())
          {
            return false;
          }
        }

        // Message filter
        if !self.log_filter.is_empty() {
          if !log
            .message
            .to_lowercase()
            .contains(&self.log_filter.to_lowercase())
          {
            return false;
          }
        }

        true
      })
      .cloned()
      .collect();

    // Update scroll state
    self.log_scroll_state = self
      .log_scroll_state
      .content_length(self.filtered_logs.len());
  }

  fn update_statistics(&mut self) {
    self.log_stats.clear();
    self.component_stats.clear();

    for log in &self.logs {
      *self.log_stats.entry(log.level.clone()).or_insert(0) += 1;
      *self
        .component_stats
        .entry(log.component.clone())
        .or_insert(0) += 1;
    }
  }

  fn next_tab(&mut self) {
    self.selected_tab = (self.selected_tab + 1) % self.view_tabs.len();
    self.current_view = self.view_tabs[self.selected_tab].clone();
  }

  fn previous_tab(&mut self) {
    self.selected_tab = if self.selected_tab == 0 {
      self.view_tabs.len() - 1
    } else {
      self.selected_tab - 1
    };
    self.current_view = self.view_tabs[self.selected_tab].clone();
  }

  fn next_log(&mut self) {
    let i = match self.log_list_state.selected() {
      Some(i) => {
        if i >= self.filtered_logs.len().saturating_sub(1) {
          0
        } else {
          i + 1
        }
      },
      None => 0,
    };
    self.log_list_state.select(Some(i));
    self.log_scroll_state = self.log_scroll_state.position(i);
  }

  fn previous_log(&mut self) {
    let i = match self.log_list_state.selected() {
      Some(i) => {
        if i == 0 {
          self.filtered_logs.len().saturating_sub(1)
        } else {
          i - 1
        }
      },
      None => 0,
    };
    self.log_list_state.select(Some(i));
    self.log_scroll_state = self.log_scroll_state.position(i);
  }

  fn toggle_help(&mut self) {
    self.show_help = !self.show_help;
  }

  fn cycle_log_level_filter(&mut self) {
    self.log_level_filter = match self.log_level_filter {
      None => Some(LogLevel::Debug),
      Some(LogLevel::Debug) => Some(LogLevel::Info),
      Some(LogLevel::Info) => Some(LogLevel::Warn),
      Some(LogLevel::Warn) => Some(LogLevel::Error),
      Some(LogLevel::Error) => Some(LogLevel::Fatal),
      Some(LogLevel::Fatal) => None,
      Some(LogLevel::Trace) => Some(LogLevel::Debug),
    };
    self.filter_logs();
  }
}

// Simple random number generator
mod rand {
  use std::cell::Cell;
  use std::time::{SystemTime, UNIX_EPOCH};

  thread_local! {
      static SEED: Cell<u64> = Cell::new({
          SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
      });
  }

  fn next_u64() -> u64 {
    SEED.with(|seed| {
      let mut s = seed.get();
      s ^= s << 13;
      s ^= s >> 7;
      s ^= s << 17;
      seed.set(s);
      s
    })
  }

  pub fn random<T>() -> T
  where
    T: From<u64>,
  {
    T::from(next_u64())
  }

  pub fn random_f64() -> f64 {
    (next_u64() as f64) / ((u64::MAX as f64) + 1.0)
  }

  pub fn random_usize() -> usize {
    next_u64() as usize
  }

  pub fn random_u64() -> u64 {
    next_u64()
  }

  pub fn random_u32() -> u32 {
    (next_u64() & 0xFFFF_FFFF) as u32
  }

  pub fn random_u16() -> u16 {
    (next_u64() & 0xFFFF) as u16
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  // Setup terminal
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Create app
  let mut app = App::new();
  let res = run_app(&mut terminal, &mut app);

  // Restore terminal
  disable_raw_mode()?;
  execute!(
    terminal.backend_mut(),
    LeaveAlternateScreen,
    DisableMouseCapture
  )?;
  terminal.show_cursor()?;

  if let Err(err) = res {
    println!("{err:?}");
  }

  Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
  loop {
    app.update();
    terminal.draw(|f| ui(f, app))?;

    if event::poll(Duration::from_millis(100))? {
      if let Event::Key(key) = event::read()? {
        if key.kind == KeyEventKind::Press {
          match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char('h') | KeyCode::F(1) => app.toggle_help(),
            KeyCode::Tab => app.next_tab(),
            KeyCode::BackTab => app.previous_tab(),
            KeyCode::Down | KeyCode::Char('j') => app.next_log(),
            KeyCode::Up | KeyCode::Char('k') => app.previous_log(),
            KeyCode::Char('f') => app.cycle_log_level_filter(),
            KeyCode::Char('r') => {
              // Refresh/reload
              app.filter_logs();
              app.update_statistics();
            },
            KeyCode::Esc => app.show_help = false,
            _ => {},
          }
        }
      }
    }
  }
}

fn ui(f: &mut Frame<'_>, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
    .split(f.area());

  // Render tabs
  render_tabs(f, chunks[0], app);

  // Render current view
  match app.current_view {
    ViewMode::Overview => render_overview(f, chunks[1], app),
    ViewMode::Logs => render_logs_view(f, chunks[1], app),
    ViewMode::Metrics => render_metrics_view(f, chunks[1], app),
    ViewMode::Alerts => render_alerts_view(f, chunks[1], app),
    ViewMode::Performance => render_performance_view(f, chunks[1], app),
    ViewMode::Network => render_network_view(f, chunks[1], app),
  }

  // Help modal
  if app.show_help {
    help::render_help(f);
  }
}

fn render_tabs(f: &mut Frame<'_>, area: Rect, app: &App) {
  let tab_titles: Vec<Line> = app
    .view_tabs
    .iter()
    .map(|t| Line::from(format!(" {} ", t.as_str())))
    .collect();

  let tabs = Tabs::new(tab_titles)
    .block(
      Block::default()
        .borders(Borders::ALL)
        .title("TTLog Dashboard"),
    )
    .select(app.selected_tab)
    .style(Style::default().fg(Color::Cyan))
    .highlight_style(
      Style::default()
        .add_modifier(Modifier::BOLD)
        .bg(Color::DarkGray),
    );

  f.render_widget(tabs, area);
}

fn render_overview(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(8),
      Constraint::Length(12),
      Constraint::Min(0),
    ])
    .split(area);

  // Top stats
  let top_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(25); 4])
    .split(chunks[0]);

  render_stat_card(
    f,
    top_chunks[0],
    "Total Logs",
    &format!("{}", app.logs.len()),
    Color::Cyan,
  );
  render_stat_card(
    f,
    top_chunks[1],
    "Logs/sec",
    &format!("{:.1}", app.performance_stats.logs_per_sec),
    Color::Green,
  );
  render_stat_card(
    f,
    top_chunks[2],
    "Errors/min",
    &format!("{:.1}", app.performance_stats.errors_per_minute),
    Color::Red,
  );
  render_stat_card(
    f,
    top_chunks[3],
    "Uptime",
    &format_duration(&app.uptime),
    Color::Yellow,
  );

  // Middle section - charts
  let mid_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
    .split(chunks[1]);

  render_performance_chart(f, mid_chunks[0], app);
  render_log_level_distribution(f, mid_chunks[1], app);

  // Recent logs
  render_recent_logs(f, chunks[2], app);
}

fn render_logs_view(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(3), Constraint::Min(0)])
    .split(area);

  // Filter info
  let filter_info = format!(
    "Filters: Level={:?} | Component={} | Message={} | Total: {} logs",
    app
      .log_level_filter
      .as_ref()
      .map_or("All".to_string(), |l| l.as_str().to_string()),
    if app.component_filter.is_empty() {
      "All"
    } else {
      &app.component_filter
    },
    if app.log_filter.is_empty() {
      "All"
    } else {
      &app.log_filter
    },
    app.filtered_logs.len()
  );

  let filter_block = Paragraph::new(filter_info)
    .block(Block::default().borders(Borders::ALL).title("Log Filters"))
    .style(Style::default().fg(Color::Gray));

  f.render_widget(filter_block, chunks[0]);

  // Logs table
  let log_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Min(0), Constraint::Length(3)])
    .split(chunks[1]);

  render_detailed_logs(f, log_chunks[0], app);

  // Scrollbar
  let scrollbar = Scrollbar::default()
    .orientation(ScrollbarOrientation::VerticalRight)
    .begin_symbol(Some("↑"))
    .end_symbol(Some("↓"));

  f.render_stateful_widget(
    scrollbar,
    log_chunks[1].inner(Margin {
      vertical: 1,
      horizontal: 0,
    }),
    &mut app.log_scroll_state.clone(),
  );
}

fn render_metrics_view(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(10), Constraint::Min(0)])
    .split(area);

  // System metrics gauges
  let gauge_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(25); 4])
    .split(chunks[0]);

  if let Some(metrics) = app.metrics_history.back() {
    render_gauge(
      f,
      gauge_chunks[0],
      "CPU Usage",
      metrics.cpu_usage,
      Color::Red,
    );
    render_gauge(
      f,
      gauge_chunks[1],
      "Memory",
      metrics.memory_usage,
      Color::Blue,
    );
    render_gauge(
      f,
      gauge_chunks[2],
      "Disk I/O",
      (metrics.disk_io_read as f64 / 1000000.0).min(100.0),
      Color::Yellow,
    );
    render_gauge(
      f,
      gauge_chunks[3],
      "Network",
      (metrics.network_rx as f64 / 10000000.0).min(100.0),
      Color::Green,
    );
  }

  // Detailed metrics table
  render_metrics_table(f, chunks[1], app);
}

fn render_alerts_view(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(6), Constraint::Min(0)])
    .split(area);

  // Alert summary
  let active_alerts = app.alerts.iter().filter(|a| a.triggered).count();
  let total_alerts = app.alerts.len();

  let alert_summary = format!(
    "Active Alerts: {} / {} | Last Update: {} seconds ago",
    active_alerts,
    total_alerts,
    app.last_update.elapsed().as_secs()
  );

  let summary_block = Paragraph::new(alert_summary)
    .block(Block::default().borders(Borders::ALL).title("Alert Status"))
    .style(Style::default().fg(if active_alerts > 0 {
      Color::Red
    } else {
      Color::Green
    }));

  f.render_widget(summary_block, chunks[0]);

  // Alerts table
  render_alerts_table(f, chunks[1], app);
}

fn render_performance_view(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Length(8),
      Constraint::Length(12),
      Constraint::Min(0),
    ])
    .split(area);

  // Performance stats cards
  let perf_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(25); 4])
    .split(chunks[0]);

  render_stat_card(
    f,
    perf_chunks[0],
    "Avg Process Time",
    &format!("{:.1}μs", app.performance_stats.avg_processing_time_us),
    Color::Cyan,
  );
  render_stat_card(
    f,
    perf_chunks[1],
    "Memory Alloc",
    &format!("{:.1}MB", app.performance_stats.memory_allocated_mb),
    Color::Blue,
  );
  render_stat_card(
    f,
    perf_chunks[2],
    "Cache Hit Rate",
    &format!("{:.1}%", app.performance_stats.cache_hit_ratio * 100.0),
    Color::Green,
  );
  render_stat_card(
    f,
    perf_chunks[3],
    "Queue Depth",
    &format!("{}", app.performance_stats.queue_depth),
    Color::Yellow,
  );

  // Performance charts
  let chart_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(chunks[1]);

  render_throughput_chart(f, chart_chunks[0], app);
  render_latency_sparkline(f, chart_chunks[1], app);

  // Detailed performance table
  render_performance_table(f, chunks[2], app);
}

fn render_network_view(f: &mut Frame<'_>, area: Rect, app: &App) {
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(10), Constraint::Min(0)])
    .split(area);

  // Network stats
  let net_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
    .split(chunks[0]);

  render_network_stats(f, net_chunks[0], app);
  render_connection_info(f, net_chunks[1], app);

  // Network activity table
  render_network_activity(f, chunks[1], app);
}

fn render_stat_card(f: &mut Frame<'_>, area: Rect, title: &str, value: &str, color: Color) {
  let block = Block::default()
    .title(title)
    .borders(Borders::ALL)
    .border_style(Style::default().fg(color));

  let text = Text::from(vec![
    Line::from(""),
    Line::from(Span::styled(
      value,
      Style::default().fg(color).add_modifier(Modifier::BOLD),
    )),
  ]);

  let paragraph = Paragraph::new(text)
    .block(block)
    .alignment(Alignment::Center);

  f.render_widget(paragraph, area);
}

fn render_performance_chart(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Logs/sec Over Time")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  if app.performance_history.is_empty() {
    let empty_text = Paragraph::new("No data available")
      .block(block)
      .alignment(Alignment::Center);
    f.render_widget(empty_text, area);
    return;
  }

  let data: Vec<(f64, f64)> = app.performance_history.iter().cloned().collect();
  let min_time = data.first().map(|(t, _)| *t).unwrap_or(0.0);
  let max_time = data.last().map(|(t, _)| *t).unwrap_or(1.0);
  let max_value = data.iter().map(|(_, v)| *v).fold(0.0, f64::max);

  let datasets = vec![Dataset::default()
    .name("Logs/sec")
    .marker(symbols::Marker::Braille)
    .style(Style::default().fg(Color::Green))
    .graph_type(GraphType::Line)
    .data(&data)];

  let chart = Chart::new(datasets)
    .block(block)
    .x_axis(
      Axis::default()
        .title("Time (s)")
        .style(Style::default().fg(Color::Gray))
        .bounds([min_time, max_time])
        .labels(vec![
          Span::styled(format!("{:.0}", min_time), Style::default().fg(Color::Gray)),
          Span::styled(format!("{:.0}", max_time), Style::default().fg(Color::Gray)),
        ]),
    )
    .y_axis(
      Axis::default()
        .title("Rate")
        .style(Style::default().fg(Color::Gray))
        .bounds([0.0, max_value])
        .labels(vec![
          Span::styled("0", Style::default().fg(Color::Gray)),
          Span::styled(
            format!("{:.0}", max_value),
            Style::default().fg(Color::Gray),
          ),
        ]),
    );

  f.render_widget(chart, area);
}

fn render_log_level_distribution(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Log Level Distribution")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let data: Vec<(&str, u64)> = vec![
    ("TRACE", *app.log_stats.get(&LogLevel::Trace).unwrap_or(&0)),
    ("DEBUG", *app.log_stats.get(&LogLevel::Debug).unwrap_or(&0)),
    ("INFO", *app.log_stats.get(&LogLevel::Info).unwrap_or(&0)),
    ("WARN", *app.log_stats.get(&LogLevel::Warn).unwrap_or(&0)),
    ("ERROR", *app.log_stats.get(&LogLevel::Error).unwrap_or(&0)),
    ("FATAL", *app.log_stats.get(&LogLevel::Fatal).unwrap_or(&0)),
  ];

  let barchart = BarChart::default()
    .block(block)
    .data(&data)
    .bar_width(6)
    .bar_style(Style::default().fg(Color::Green))
    .value_style(Style::default().fg(Color::White));

  f.render_widget(barchart, area);
}

fn render_recent_logs(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Recent Logs")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let items: Vec<ListItem> = app
    .filtered_logs
    .iter()
    .take(20)
    .map(|log| {
      let content = Line::from(vec![
        Span::styled(
          format!("{} ", log.timestamp.split(' ').nth(1).unwrap_or("--:--:--")),
          Style::default().fg(Color::Gray),
        ),
        Span::styled(
          format!("{:<5} ", log.level.as_str()),
          Style::default()
            .fg(log.level.color())
            .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
          format!("{:<10} ", log.component),
          Style::default().fg(Color::Cyan),
        ),
        Span::styled(
          if log.message.len() > 50 {
            format!("{}...", &log.message[..47])
          } else {
            log.message.clone()
          },
          Style::default().fg(Color::White),
        ),
      ]);
      ListItem::new(content)
    })
    .collect();

  let logs_list = List::new(items).block(block);
  f.render_widget(logs_list, area);
}

fn render_detailed_logs(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title(format!(
      "Detailed Logs ({} entries)",
      app.filtered_logs.len()
    ))
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let header = Row::new(vec![
    Cell::from("Time"),
    Cell::from("Level"),
    Cell::from("Component"),
    Cell::from("Thread"),
    Cell::from("Message"),
    Cell::from("Duration"),
  ])
  .style(Style::default().fg(Color::Yellow))
  .height(1);

  let rows: Vec<Row> = app
    .filtered_logs
    .iter()
    .enumerate()
    .map(|(i, log)| {
      let style = if Some(i) == app.log_list_state.selected() {
        Style::default().bg(Color::DarkGray)
      } else {
        Style::default()
      };

      Row::new(vec![
        Cell::from(log.timestamp.split(' ').nth(1).unwrap_or("--:--:--")),
        Cell::from(log.level.as_str()).style(Style::default().fg(log.level.color())),
        Cell::from(log.component.as_str()),
        Cell::from(format!("{}", log.thread_id)),
        Cell::from(if log.message.len() > 40 {
          format!("{}...", &log.message[..37])
        } else {
          log.message.clone()
        }),
        Cell::from(
          log
            .duration_ms
            .map_or("-".to_string(), |d| format!("{}ms", d)),
        ),
      ])
      .style(style)
      .height(1)
    })
    .collect();

  let table = Table::new(
    rows,
    [
      Constraint::Length(8),
      Constraint::Length(6),
      Constraint::Length(12),
      Constraint::Length(6),
      Constraint::Min(20),
      Constraint::Length(8),
    ],
  )
  .header(header)
  .block(block)
  .highlight_style(Style::default().add_modifier(Modifier::BOLD));

  f.render_widget(table, area);
}

fn render_gauge(f: &mut Frame<'_>, area: Rect, title: &str, value: f64, color: Color) {
  let gauge = Gauge::default()
    .block(Block::default().title(title).borders(Borders::ALL))
    .gauge_style(Style::default().fg(color))
    .percent(value as u16)
    .label(format!("{:.1}%", value));

  f.render_widget(gauge, area);
}

fn render_metrics_table(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("System Metrics")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  if let Some(metrics) = app.metrics_history.back() {
    let rows = vec![
      Row::new(vec![
        Cell::from("CPU Usage"),
        Cell::from(format!("{:.1}%", metrics.cpu_usage)),
      ]),
      Row::new(vec![
        Cell::from("Memory Usage"),
        Cell::from(format!("{:.1}%", metrics.memory_usage)),
      ]),
      Row::new(vec![
        Cell::from("Disk Read"),
        Cell::from(format!("{} KB/s", metrics.disk_io_read / 1024)),
      ]),
      Row::new(vec![
        Cell::from("Disk Write"),
        Cell::from(format!("{} KB/s", metrics.disk_io_write / 1024)),
      ]),
      Row::new(vec![
        Cell::from("Network RX"),
        Cell::from(format!("{} KB/s", metrics.network_rx / 1024)),
      ]),
      Row::new(vec![
        Cell::from("Network TX"),
        Cell::from(format!("{} KB/s", metrics.network_tx / 1024)),
      ]),
      Row::new(vec![
        Cell::from("Active Threads"),
        Cell::from(format!("{}", metrics.threads_active)),
      ]),
      Row::new(vec![
        Cell::from("Open Connections"),
        Cell::from(format!("{}", metrics.connections_open)),
      ]),
    ];

    let header = Row::new(vec!["Metric", "Value"])
      .style(Style::default().fg(Color::Yellow))
      .height(1);

    let table = Table::new(
      rows,
      [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .header(header)
    .block(block);

    f.render_widget(table, area);
  } else {
    let empty_text = Paragraph::new("No metrics data available")
      .block(block)
      .alignment(Alignment::Center);
    f.render_widget(empty_text, area);
  }
}

fn render_alerts_table(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Alert Rules")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let header = Row::new(vec!["Alert", "Condition", "Status", "Last Triggered"])
    .style(Style::default().fg(Color::Yellow))
    .height(1);

  let rows: Vec<Row> = app
    .alerts
    .iter()
    .map(|alert| {
      let status = if alert.triggered { "ACTIVE" } else { "OK" };
      let status_color = if alert.triggered {
        Color::Red
      } else {
        Color::Green
      };

      let last_triggered = alert
        .last_triggered
        .map(|t| format!("{}s ago", t.elapsed().as_secs()))
        .unwrap_or_else(|| "Never".to_string());

      Row::new(vec![
        Cell::from(alert.name.as_str()),
        Cell::from(alert.condition.as_str()),
        Cell::from(status).style(Style::default().fg(status_color)),
        Cell::from(last_triggered),
      ])
      .height(1)
    })
    .collect();

  let table = Table::new(
    rows,
    [
      Constraint::Length(20),
      Constraint::Min(25),
      Constraint::Length(8),
      Constraint::Length(15),
    ],
  )
  .header(header)
  .block(block);

  f.render_widget(table, area);
}

fn render_throughput_chart(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Throughput History")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Green));

  if app.performance_history.len() < 2 {
    let empty_text = Paragraph::new("Collecting data...")
      .block(block)
      .alignment(Alignment::Center);
    f.render_widget(empty_text, area);
    return;
  }

  let data: Vec<u64> = app
    .performance_history
    .iter()
    .map(|(_, v)| *v as u64)
    .collect();

  let sparkline = Sparkline::default()
    .block(block)
    .data(&data)
    .style(Style::default().fg(Color::Green));

  f.render_widget(sparkline, area);
}

fn render_latency_sparkline(f: &mut Frame<'_>, area: Rect, _app: &App) {
  let block = Block::default()
    .title("Processing Latency")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Yellow));

  // Simulate latency data
  let mut latency_data = Vec::new();
  for i in 0..50 {
    let base_latency = 100.0 + (i as f64 * 2.0);
    let noise = (rand::random_f64() - 0.5) * 50.0;
    latency_data.push((base_latency + noise) as u64);
  }

  let sparkline = Sparkline::default()
    .block(block)
    .data(&latency_data)
    .style(Style::default().fg(Color::Yellow));

  f.render_widget(sparkline, area);
}

fn render_performance_table(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Performance Metrics")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let rows = vec![
    Row::new(vec![
      Cell::from("Logs Processed"),
      Cell::from(format!("{}", app.performance_stats.logs_processed)),
    ]),
    Row::new(vec![
      Cell::from("Current Rate"),
      Cell::from(format!(
        "{:.1} logs/sec",
        app.performance_stats.logs_per_sec
      )),
    ]),
    Row::new(vec![
      Cell::from("Avg Process Time"),
      Cell::from(format!(
        "{:.1} μs",
        app.performance_stats.avg_processing_time_us
      )),
    ]),
    Row::new(vec![
      Cell::from("Error Rate"),
      Cell::from(format!(
        "{:.1} errors/min",
        app.performance_stats.errors_per_minute
      )),
    ]),
    Row::new(vec![
      Cell::from("Memory Allocated"),
      Cell::from(format!(
        "{:.1} MB",
        app.performance_stats.memory_allocated_mb
      )),
    ]),
    Row::new(vec![
      Cell::from("GC Collections"),
      Cell::from(format!("{}", app.performance_stats.gc_collections)),
    ]),
    Row::new(vec![
      Cell::from("Cache Hit Ratio"),
      Cell::from(format!(
        "{:.1}%",
        app.performance_stats.cache_hit_ratio * 100.0
      )),
    ]),
    Row::new(vec![
      Cell::from("Queue Depth"),
      Cell::from(format!("{}", app.performance_stats.queue_depth)),
    ]),
  ];

  let header = Row::new(vec!["Metric", "Value"])
    .style(Style::default().fg(Color::Yellow))
    .height(1);

  let table = Table::new(
    rows,
    [Constraint::Percentage(50), Constraint::Percentage(50)],
  )
  .header(header)
  .block(block);

  f.render_widget(table, area);
}

fn render_network_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Network Statistics")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  if let Some(metrics) = app.metrics_history.back() {
    let text = Text::from(vec![
      Line::from(vec![
        Span::styled("RX: ", Style::default().fg(Color::White)),
        Span::styled(
          format!("{:.1} MB/s", metrics.network_rx as f64 / 1048576.0),
          Style::default().fg(Color::Green),
        ),
      ]),
      Line::from(vec![
        Span::styled("TX: ", Style::default().fg(Color::White)),
        Span::styled(
          format!("{:.1} MB/s", metrics.network_tx as f64 / 1048576.0),
          Style::default().fg(Color::Yellow),
        ),
      ]),
      Line::from(vec![
        Span::styled("Connections: ", Style::default().fg(Color::White)),
        Span::styled(
          format!("{}", metrics.connections_open),
          Style::default().fg(Color::Cyan),
        ),
      ]),
      Line::from(vec![
        Span::styled("Active Threads: ", Style::default().fg(Color::White)),
        Span::styled(
          format!("{}", metrics.threads_active),
          Style::default().fg(Color::Magenta),
        ),
      ]),
    ]);

    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
  }
}

fn render_connection_info(f: &mut Frame<'_>, area: Rect, _app: &App) {
  let block = Block::default()
    .title("Connection Info")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let text = Text::from(vec![
    Line::from(vec![
      Span::styled("Protocol: ", Style::default().fg(Color::White)),
      Span::styled("HTTP/2", Style::default().fg(Color::Green)),
    ]),
    Line::from(vec![
      Span::styled("Port: ", Style::default().fg(Color::White)),
      Span::styled("8080", Style::default().fg(Color::Cyan)),
    ]),
    Line::from(vec![
      Span::styled("SSL: ", Style::default().fg(Color::White)),
      Span::styled("Enabled", Style::default().fg(Color::Green)),
    ]),
    Line::from(vec![
      Span::styled("Keep-Alive: ", Style::default().fg(Color::White)),
      Span::styled("30s", Style::default().fg(Color::Yellow)),
    ]),
  ]);

  let paragraph = Paragraph::new(text).block(block);
  f.render_widget(paragraph, area);
}

fn render_network_activity(f: &mut Frame<'_>, area: Rect, _app: &App) {
  let block = Block::default()
    .title("Network Activity")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let header = Row::new(vec!["Remote IP", "Port", "Status", "Bytes", "Duration"])
    .style(Style::default().fg(Color::Yellow))
    .height(1);

  let sample_connections = vec![
    Row::new(vec![
      "192.168.1.100",
      "443",
      "ESTABLISHED",
      "1.2MB",
      "00:05:23",
    ]),
    Row::new(vec!["10.0.0.50", "80", "TIME_WAIT", "256KB", "00:00:30"]),
    Row::new(vec![
      "172.16.0.10",
      "8080",
      "ESTABLISHED",
      "5.1MB",
      "00:12:45",
    ]),
    Row::new(vec![
      "203.0.113.15",
      "443",
      "CLOSE_WAIT",
      "512KB",
      "00:01:15",
    ]),
    Row::new(vec![
      "198.51.100.25",
      "80",
      "ESTABLISHED",
      "2.8MB",
      "00:08:32",
    ]),
  ];

  let table = Table::new(
    sample_connections,
    [
      Constraint::Length(15),
      Constraint::Length(6),
      Constraint::Length(12),
      Constraint::Length(8),
      Constraint::Length(10),
    ],
  )
  .header(header)
  .block(block);

  f.render_widget(table, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Percentage((100 - percent_y) / 2),
      Constraint::Percentage(percent_y),
      Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

  Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage((100 - percent_x) / 2),
      Constraint::Percentage(percent_x),
      Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn format_duration(duration: &Duration) -> String {
  let total_secs = duration.as_secs();
  let hours = total_secs / 3600;
  let minutes = (total_secs % 3600) / 60;
  let seconds = total_secs % 60;

  if hours > 0 {
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
  } else {
    format!("{:02}:{:02}", minutes, seconds)
  }
}
