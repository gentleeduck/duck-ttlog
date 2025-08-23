use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
  backend::{Backend, CrosstermBackend},
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{BarChart, Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
  Frame, Terminal,
};
use std::{
  error::Error,
  io,
  time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone)]
struct LogEntry {
  timestamp: String,
  level: LogLevel,
  component: String,
  message: String,
}

#[derive(Debug, Clone)]
enum LogLevel {
  Info,
  Debug,
  Trace,
  Error,
  Warn,
}

impl LogLevel {
  fn color(&self) -> Color {
    match self {
      LogLevel::Info => Color::Cyan,
      LogLevel::Debug => Color::Green,
      LogLevel::Trace => Color::Magenta,
      LogLevel::Error => Color::Red,
      LogLevel::Warn => Color::Yellow,
    }
  }

  fn as_str(&self) -> &str {
    match self {
      LogLevel::Info => "INFO",
      LogLevel::Debug => "DEBUG",
      LogLevel::Trace => "TRACE",
      LogLevel::Error => "ERROR",
      LogLevel::Warn => "WARN",
    }
  }
}

struct Stats {
  logs_processed: u64,
  logs_per_sec: f64,
  snapshots: u32,
  uptime: Duration,
  interner_size_mb: f32,
  strings_interned: u32,
  snapshot_size_avg_mb: f32,
  flush_interval_sec: u32,
  interner_hit_ratio: f32,
  snapshot_flush_rate: f32,
}

struct App {
  stats: Stats,
  logs: Vec<LogEntry>,
  show_help: bool,
  selected_log: usize,
  last_update: Instant,
}

impl App {
  fn new() -> App {
    App {
      stats: Stats {
        logs_processed: 72340,
        logs_per_sec: 18.2,
        snapshots: 12,
        uptime: Duration::from_secs(754), // 12:34
        interner_size_mb: 12.3,
        strings_interned: 8421,
        snapshot_size_avg_mb: 1.4,
        flush_interval_sec: 2,
        interner_hit_ratio: 0.92,
        snapshot_flush_rate: 0.5,
      },
      logs: generate_demo_logs(),
      show_help: false,
      selected_log: 0,
      last_update: Instant::now(),
    }
  }

  fn update(&mut self) {
    if self.last_update.elapsed() > Duration::from_millis(1000) {
      // Simulate real-time updates
      self.stats.logs_processed += (self.stats.logs_per_sec as u64).saturating_sub(5);
      self.stats.logs_per_sec += (rand::random_f64() - 0.5) * 2.0;
      self.stats.logs_per_sec = self.stats.logs_per_sec.max(0.0).min(50.0);

      // Add new log entry occasionally
      if rand::random_f64() < 0.3 {
        self.logs.insert(0, generate_random_log());
        if self.logs.len() > 20 {
          self.logs.pop();
        }
      }

      self.last_update = Instant::now();
    }
  }

  fn next_log(&mut self) {
    if !self.logs.is_empty() {
      self.selected_log = (self.selected_log + 1) % self.logs.len();
    }
  }

  fn previous_log(&mut self) {
    if !self.logs.is_empty() {
      self.selected_log = if self.selected_log == 0 {
        self.logs.len() - 1
      } else {
        self.selected_log - 1
      };
    }
  }

  fn toggle_help(&mut self) {
    self.show_help = !self.show_help;
  }
}

fn generate_demo_logs() -> Vec<LogEntry> {
  vec![
    LogEntry {
      timestamp: "2025-08-22 16:03:21".to_string(),
      level: LogLevel::Info,
      component: "Auth".to_string(),
      message: "successful".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:24".to_string(),
      level: LogLevel::Info,
      component: "Auth".to_string(),
      message: "user session".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:38".to_string(),
      level: LogLevel::Error,
      component: "Auth".to_string(),
      message: "session expir".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:38".to_string(),
      level: LogLevel::Debug,
      component: "Auth".to_string(),
      message: "connection".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:38".to_string(),
      level: LogLevel::Trace,
      component: "Auth".to_string(),
      message: "cache miss".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:39".to_string(),
      level: LogLevel::Debug,
      component: "Auth".to_string(),
      message: "enabled".to_string(),
    },
    LogEntry {
      timestamp: "2025-08-22 16:03:39".to_string(),
      level: LogLevel::Trace,
      component: "".to_string(),
      message: "debug mode".to_string(),
    },
  ]
}

fn generate_random_log() -> LogEntry {
  let levels = [
    LogLevel::Info,
    LogLevel::Debug,
    LogLevel::Trace,
    LogLevel::Error,
    LogLevel::Warn,
  ];
  let components = ["Auth", "DB", "Cache", "API", "Worker"];
  let messages = [
    "connection established",
    "query executed",
    "cache hit",
    "timeout occurred",
    "request processed",
  ];

  let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
  let timestamp = format!(
    "2025-08-23 {:02}:{:02}:{:02}",
    (now.as_secs() / 3600) % 24,
    (now.as_secs() / 60) % 60,
    now.as_secs() % 60
  );

  LogEntry {
    timestamp,
    level: levels[(rand::random::<u64>() as usize) % levels.len()].clone(),
    component: components[(rand::random::<u64>() as usize) % components.len()].to_string(),
    message: messages[(rand::random::<u64>() as usize) % messages.len()].to_string(),
  }
}

// Simple random number generator to avoid external dependencies
mod rand {
  use std::cell::Cell;
  use std::time::{SystemTime, UNIX_EPOCH};

  thread_local! {
      static SEED: Cell<u64> = Cell::new({
          SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos() as u64
      });
  }

  // Xorshift-based PRNG core returning a new u64 each call
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

  // Generate a uniform f64 in [0, 1)
  pub fn random_f64() -> f64 {
    // Divide by (MAX as f64 + 1.0) to keep result in [0,1)
    (next_u64() as f64) / ((u64::MAX as f64) + 1.0)
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
            KeyCode::Down | KeyCode::Char('j') => app.next_log(),
            KeyCode::Up | KeyCode::Char('k') => app.previous_log(),
            KeyCode::Esc => app.show_help = false,
            _ => {},
          }
        }
      }
    }
  }
}

fn ui(f: &mut Frame<'_>, app: &App) {
  // Main layout
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(10), Constraint::Min(0)].as_ref())
    .split(f.area());

  // Top section layout
  let top_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    .split(chunks[0]);

  // Bottom section layout
  let bottom_chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
    .split(chunks[1]);

  // TTLog Stats (top-left)
  render_stats(f, top_chunks[0], app);

  // Memory & Interning (top-right)
  render_memory_stats(f, top_chunks[1], app);

  // Logs (bottom-left)
  render_logs(f, bottom_chunks[0], app);

  // Log Levels and Event Sizes (bottom-right)
  let right_chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
    .split(bottom_chunks[1]);

  render_log_levels(f, right_chunks[0], app);
  render_event_sizes(f, right_chunks[1], app);

  // Additional stats at bottom
  render_additional_stats(f, app);

  // Help modal
  if app.show_help {
    render_help(f);
  }
}

fn render_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("TTLog Stats")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let uptime_mins = app.stats.uptime.as_secs() / 60;
  let uptime_secs = app.stats.uptime.as_secs() % 60;

  let text = Text::from(vec![
    Line::from(vec![
      Span::styled("Logs processed: ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8}", app.stats.logs_processed),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Logs/sec:       ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8.1}", app.stats.logs_per_sec),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Snapshots:      ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8}", app.stats.snapshots),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Uptime:         ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8}:{:02}", uptime_mins, uptime_secs),
        Style::default().fg(Color::Green),
      ),
    ]),
  ]);

  let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

  f.render_widget(paragraph, area);
}

fn render_memory_stats(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Memory & Interning")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let text = Text::from(vec![
    Line::from(vec![
      Span::styled("Interner size:     ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8.1} MB", app.stats.interner_size_mb),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Strings interned:  ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8}", app.stats.strings_interned),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Snapshot size avg: ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8.1} MB", app.stats.snapshot_size_avg_mb),
        Style::default().fg(Color::Green),
      ),
    ]),
    Line::from(vec![
      Span::styled("Flush interval:    ", Style::default().fg(Color::White)),
      Span::styled(
        format!("{:>8}s", app.stats.flush_interval_sec),
        Style::default().fg(Color::Green),
      ),
    ]),
  ]);

  let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

  f.render_widget(paragraph, area);
}

fn render_logs(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Logs")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let items: Vec<ListItem> = app
    .logs
    .iter()
    .enumerate()
    .map(|(i, log)| {
      let style = if i == app.selected_log {
        Style::default().bg(Color::DarkGray)
      } else {
        Style::default()
      };

      let content = Line::from(vec![
        Span::styled(
          format!("[{}] ", log.timestamp),
          Style::default().fg(Color::Gray),
        ),
        Span::styled(
          format!("{:<5} ", log.level.as_str()),
          Style::default()
            .fg(log.level.color())
            .add_modifier(Modifier::BOLD),
        ),
        Span::styled(&log.component, Style::default().fg(Color::Cyan)),
        Span::styled(
          format!(" {}", log.message),
          Style::default().fg(Color::White),
        ),
      ]);

      ListItem::new(content).style(style)
    })
    .collect();

  let logs_list = List::new(items).block(block);
  f.render_widget(logs_list, area);

  // Render additional stats at bottom of logs area
  let stats_area = Rect {
    x: area.x + 1,
    y: area.y + area.height - 3,
    width: area.width - 2,
    height: 2,
  };

  let stats_text = Text::from(vec![
    Line::from(vec![
      Span::styled(
        format!("Logs/sec: {:.1}k", app.stats.logs_per_sec),
        Style::default().fg(Color::White),
      ),
      Span::styled(
        format!(
          "    Snapshot flush rate: {:.1}/s",
          app.stats.snapshot_flush_rate
        ),
        Style::default().fg(Color::White),
      ),
    ]),
    Line::from(vec![Span::styled(
      format!(
        "Interner hit ratio: {:.0}%",
        app.stats.interner_hit_ratio * 100.0
      ),
      Style::default().fg(Color::White),
    )]),
  ]);

  let stats_paragraph = Paragraph::new(stats_text);
  f.render_widget(Clear, stats_area);
  f.render_widget(stats_paragraph, stats_area);
}

fn render_log_levels(f: &mut Frame<'_>, area: Rect, app: &App) {
  let block = Block::default()
    .title("Log Levels")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  // Calculate log level distribution
  let mut counts = [0u64; 5]; // INFO, DEBUG, TRACE, ERROR, WARN
  for log in &app.logs {
    match log.level {
      LogLevel::Info => counts[0] += 1,
      LogLevel::Debug => counts[1] += 1,
      LogLevel::Trace => counts[2] += 1,
      LogLevel::Error => counts[3] += 1,
      LogLevel::Warn => counts[4] += 1,
    }
  }

  let max_count = *counts.iter().max().unwrap_or(&1);
  let bar_width = (area.width.saturating_sub(4)) / 5;

  let inner_area = block.inner(area);
  f.render_widget(block, area);

  // Render bars
  let colors = [
    Color::Cyan,
    Color::Green,
    Color::Magenta,
    Color::Red,
    Color::Yellow,
  ];
  let labels = ["INFO", "DEBUG", "TRACE", "ERROR", "WARN"];

  for (i, (&count, &color)) in counts.iter().zip(colors.iter()).enumerate() {
    let bar_height = if max_count > 0 {
      ((count as f64 / max_count as f64) * (inner_area.height.saturating_sub(2)) as f64) as u16
    } else {
      0
    };

    let bar_area = Rect {
      x: inner_area.x + (i as u16 * bar_width) + 1,
      y: inner_area.y + inner_area.height.saturating_sub(bar_height + 1),
      width: bar_width.saturating_sub(1),
      height: bar_height,
    };

    let bar = Block::default().style(Style::default().bg(color));
    f.render_widget(bar, bar_area);

    // Render label
    let label_area = Rect {
      x: inner_area.x + (i as u16 * bar_width) + 1,
      y: inner_area.y + inner_area.height.saturating_sub(1),
      width: bar_width.saturating_sub(1),
      height: 1,
    };

    let label = Paragraph::new(labels[i])
      .alignment(Alignment::Center)
      .style(Style::default().fg(Color::White));
    f.render_widget(label, label_area);
  }
}

fn render_event_sizes(f: &mut Frame<'_>, area: Rect, _app: &App) {
  let block = Block::default()
    .title("Event Sizes")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Cyan));

  let data = [("avg", 45u64), ("max", 32u64), ("min", 18u64)];

  let barchart = BarChart::default()
    .block(block)
    .data(&data)
    .bar_width(8)
    .bar_style(Style::default().fg(Color::Cyan))
    .value_style(Style::default().fg(Color::White));

  f.render_widget(barchart, area);
}

fn render_additional_stats(_f: &mut Frame<'_>, _app: &App) {
  // This could render additional stats at the bottom if needed
}

fn render_help(f: &mut Frame<'_>) {
  let area = centered_rect(50, 50, f.area());

  let block = Block::default()
    .title("Help")
    .borders(Borders::ALL)
    .border_style(Style::default().fg(Color::Yellow));

  let help_text = Text::from(vec![
    Line::from("Keyboard Shortcuts:"),
    Line::from(""),
    Line::from(vec![
      Span::styled(
        "q",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("       - Quit application"),
    ]),
    Line::from(vec![
      Span::styled(
        "h, F1",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("   - Toggle this help"),
    ]),
    Line::from(vec![
      Span::styled(
        "↑/k",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("     - Previous log entry"),
    ]),
    Line::from(vec![
      Span::styled(
        "↓/j",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("     - Next log entry"),
    ]),
    Line::from(vec![
      Span::styled(
        "Esc",
        Style::default()
          .fg(Color::Yellow)
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw("     - Close help"),
    ]),
    Line::from(""),
    Line::from("Press any key to close..."),
  ]);

  let paragraph = Paragraph::new(help_text)
    .block(block)
    .wrap(Wrap { trim: true });

  f.render_widget(Clear, area);
  f.render_widget(paragraph, area);
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
