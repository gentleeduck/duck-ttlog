use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::KeyCode;
use ratatui::{
  layout::{Alignment, Rect},
  style::Modifier,
  style::{Color, Style},
  text::{Line, Span},
  widgets::{Block, BorderType, Borders, Clear, Paragraph},
  widgets::{Cell, Row, Table},
  Frame,
};
use smallvec::SmallVec;
use ttlog::{event::LogLevel, snapshot::ResolvedEvent};

use crate::widget::Widget;

pub struct LogsWidget {
  pub id: u8,
  pub title: &'static str,
  pub logs: Vec<ResolvedEvent>,
  pub search_query: String,
  pub sort_by: SortBy,
  pub sort_order: SortOrder,
  pub selected: usize,
  pub search_mode: bool,
  pub auto_scroll: bool,
  pub show_timestamps: bool,
  pub show_levels: bool,
  pub level_filter: Option<String>,
  pub follow_tail: bool,
  pub wrap_lines: bool,
  pub show_line_numbers: bool,
  pub bookmark_indices: Vec<usize>,
  pub show_help: bool,
  pub paused: bool,
  pub focused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
  Time,
  Level,
  Message,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
  Ascending,
  Descending,
}

impl LogsWidget {
  #[inline]
  fn ev_timestamp_millis(ev: &ResolvedEvent) -> u64 {
    ev.packed_meta >> 12
  }

  #[inline]
  fn ev_level(ev: &ResolvedEvent) -> LogLevel {
    // SAFETY: levels are encoded from a valid LogLevel repr u8
    unsafe { std::mem::transmute(((ev.packed_meta >> 8) & 0xF) as u8) }
  }

  #[inline]
  fn ev_thread_id(ev: &ResolvedEvent) -> u8 {
    (ev.packed_meta & 0xFF) as u8
  }

  #[inline]
  fn level_name(level: LogLevel) -> &'static str {
    match level {
      LogLevel::FATAL => "FATAL",
      LogLevel::ERROR => "ERROR",
      LogLevel::WARN => "WARN",
      LogLevel::INFO => "INFO",
      LogLevel::DEBUG => "DEBUG",
      LogLevel::TRACE => "TRACE",
    }
  }

  fn fmt_timestamp(ms: u64) -> String {
    // Interpret as UNIX epoch millis in UTC
    let secs = (ms / 1000) as i64;
    let sub_ms = (ms % 1000) as u32;
    let dt: DateTime<Utc> = Utc
      .timestamp_opt(secs, sub_ms * 1_000_000)
      .single()
      .unwrap_or_else(|| Utc.timestamp_opt(0, 0).earliest().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
  }

  pub fn new() -> Self {
    let events = || {
      let log_levels = [
        LogLevel::ERROR,
        LogLevel::WARN,
        LogLevel::INFO,
        LogLevel::DEBUG,
        LogLevel::TRACE,
        LogLevel::FATAL,
      ];

      (0..50)
        .map(|i| {
          let level = log_levels[i % log_levels.len()];
          ResolvedEvent {
            // synthetic packed_meta: timestamp in high bits, level in bits [8..12]
            packed_meta: ((i as u64) << 12) | ((level as u64) << 8) | (i as u64),
            message: format!(
              "[{}] Simulated log message {}",
              match level {
                LogLevel::FATAL => "FATAL",
                LogLevel::ERROR => "ERROR",
                LogLevel::WARN => "WARN",
                LogLevel::INFO => "INFO",
                LogLevel::DEBUG => "DEBUG",
                LogLevel::TRACE => "TRACE",
              },
              i
            ),
            target: format!(
              "{}::module{}",
              match level {
                LogLevel::FATAL => "main",
                LogLevel::ERROR => "service",
                LogLevel::WARN => "controller",
                LogLevel::INFO => "api",
                LogLevel::DEBUG => "worker",
                LogLevel::TRACE => "internal",
              },
              i % 3
            ),
            kv: Some(SmallVec::from_vec(vec![
              (i % 255) as u8,
              (i * 2 % 255) as u8,
              (i * 3 % 255) as u8,
            ])),
            file: format!(
              "{}.rs",
              match level {
                LogLevel::FATAL => "fatales",
                LogLevel::ERROR => "errors",
                LogLevel::WARN => "warnings",
                LogLevel::INFO => "infos",
                LogLevel::DEBUG => "debugs",
                LogLevel::TRACE => "traces",
              }
            ),
            position: (i as u32, (i * 7 % 100) as u32),
          }
        })
        .collect()
    };

    Self {
      id: 1,
      title: "~ System Logs ~──",
      logs: events(),
      search_query: String::new(),
      sort_by: SortBy::Time,
      sort_order: SortOrder::Descending,
      selected: 0,
      search_mode: false,
      auto_scroll: true,
      show_timestamps: true,
      show_levels: true,
      level_filter: None,
      follow_tail: false,
      wrap_lines: false,
      show_line_numbers: false,
      bookmark_indices: Vec::new(),
      show_help: false,
      paused: false,
      focused: false,
    }
  }

  fn filtered_and_sorted(&self) -> Vec<(usize, ResolvedEvent)> {
    let mut data: Vec<(usize, ResolvedEvent)> = self.logs.iter().cloned().enumerate().collect();

    // 1. Apply level filter
    if let Some(level_filter) = &self.level_filter {
      let filter = level_filter.to_uppercase();
      data.retain(|(_, e)| Self::level_name(Self::ev_level(e)) == filter);
    }

    // 2. Apply search filter
    if !self.search_query.is_empty() {
      let query = self.search_query.to_lowercase();
      data.retain(|(_, e)| {
        let ts = Self::fmt_timestamp(Self::ev_timestamp_millis(e)).to_lowercase();
        let lvl = Self::level_name(Self::ev_level(e)).to_lowercase();
        ts.contains(&query)
          || lvl.contains(&query)
          || e.message.to_lowercase().contains(&query)
          || e.target.to_lowercase().contains(&query)
          || e.file.to_lowercase().contains(&query)
      });
    }

    // 3. Sort
    match self.sort_by {
      SortBy::Time => {
        if self.sort_order == SortOrder::Ascending {
          data
            .sort_by(|a, b| Self::ev_timestamp_millis(&a.1).cmp(&Self::ev_timestamp_millis(&b.1)));
        } else {
          data
            .sort_by(|a, b| Self::ev_timestamp_millis(&b.1).cmp(&Self::ev_timestamp_millis(&a.1)));
        }
      },
      SortBy::Level => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| (Self::ev_level(&a.1) as u8).cmp(&(Self::ev_level(&b.1) as u8)));
        } else {
          data.sort_by(|a, b| (Self::ev_level(&b.1) as u8).cmp(&(Self::ev_level(&a.1) as u8)));
        }
      },
      SortBy::Message => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.message.cmp(&b.1.message));
        } else {
          data.sort_by(|a, b| b.1.message.cmp(&a.1.message));
        }
      },
    }

    data
  }

  fn get_sort_indicator(&self, column: SortBy) -> &str {
    if self.sort_by == column {
      match self.sort_order {
        SortOrder::Ascending => "↑",
        SortOrder::Descending => "↓",
      }
    } else {
      ""
    }
  }

  fn get_level_color(level: LogLevel) -> Color {
    match level {
      LogLevel::FATAL => Color::Red,
      LogLevel::ERROR => Color::Magenta,
      LogLevel::WARN => Color::Yellow,
      LogLevel::INFO => Color::Green,
      LogLevel::DEBUG => Color::Cyan,
      LogLevel::TRACE => Color::Gray,
    }
  }

  fn get_status_indicators(&self) -> String {
    let mut indicators = Vec::new();

    if self.paused {
      indicators.push("⏸");
    }
    if self.auto_scroll {
      indicators.push("📜");
    }
    if self.follow_tail {
      indicators.push("👁");
    }
    if self.wrap_lines {
      indicators.push("↩");
    }
    if self.show_line_numbers {
      indicators.push("#");
    }
    if !self.bookmark_indices.is_empty() {
      indicators.push("🔖");
    }

    if indicators.is_empty() {
      String::new()
    } else {
      format!("~ {} ~", indicators.join(" "))
    }
  }

  fn build_title_line(&self, focused: bool) -> Line<'_> {
    let title = format!(" {}", self.title);

    let status = self.get_status_indicators();

    Line::from(vec![
      Span::styled(
        title,
        Style::default()
          .fg(if focused { Color::Cyan } else { Color::White })
          .add_modifier(Modifier::BOLD),
      ),
      Span::styled(status, Style::default().fg(Color::Yellow)),
    ])
  }

  fn build_control_line(&self, focused: bool) -> Line<'_> {
    let mut spans = Vec::new();

    // Search status
    let search_status = if self.search_mode {
      format!("🔍 {}_", self.search_query)
    } else if !self.search_query.is_empty() {
      format!("🔍 {}", self.search_query)
    } else {
      "Search: None".to_string()
    };

    // Sort info
    let sort_info = format!(
      "Sort: {}{}",
      match self.sort_by {
        SortBy::Time => "Time",
        SortBy::Level => "Level",
        SortBy::Message => "Message",
      },
      self.get_sort_indicator(self.sort_by)
    );

    // Level filter
    let filter_info = if let Some(level) = &self.level_filter {
      format!("Filter: {}", level)
    } else {
      "Filter: All".to_string()
    };

    spans.push(Span::styled("~", Style::default().fg(Color::White)));
    spans.push(Span::styled(
      format!(" {} ", search_status),
      Style::default().fg(if self.search_mode {
        Color::Yellow
      } else {
        Color::Gray
      }),
    ));
    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
      format!(" {} ", sort_info),
      Style::default().fg(Color::Cyan),
    ));
    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
      format!(" {} ", filter_info),
      Style::default().fg(Color::Green),
    ));

    // Help shortcuts when focused
    if focused && !self.show_help {
      spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
      spans.push(Span::styled(
        " [?] Help",
        Style::default()
          .fg(Color::White)
          .add_modifier(Modifier::DIM),
      ));
      spans.push(Span::styled(" ~", Style::default().fg(Color::White)));
    } else {
      spans.push(Span::styled("~", Style::default().fg(Color::White)));
    }

    Line::from(spans)
  }

  fn render_help_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let help_text = vec![
      "┌────────────────────────────── LOG VIEWER HELP ────────────────────────────┐",
      "│   Navigation:                                                             │",
      "│    ↑↓   Move cursor         │     View Options:                           │",
      "│    PgUp/Dn Page up/dn       │      t     Timestamps                       │",
      "│    Home/End First/last      │      w     Wrap lines                       │",
      "│    g/G   Go to line         │      #     Line numbers                     │",
      "│                             │      f     Follow tail Space Pause/Resume   │",
      "│   Search & Filter:          │                                             │",
      "│    /     Search             │     Bookmarks:                              │",
      "│    n/N   Next/Prev          │      b     Bookmark                         │",
      "│    l     Level filter       │      B     View marks                       │",
      "│    c     Clear all          │                                             │",
      "│                             │      ESC   Exit help                        │",
      "│   Sorting:                  │                                             │",
      "│    s     Sort column        │                                             │",
      "│    r     Reverse                                                          │",
      "└───────────────────────────────────────────────────────────────────────────┘",
    ];

    let popup_area = ratatui::layout::Layout::default()
      .direction(ratatui::layout::Direction::Vertical)
      .constraints([
        ratatui::layout::Constraint::Length(
          (area.height.saturating_sub(help_text.len() as u16)) / 2,
        ),
        ratatui::layout::Constraint::Length(help_text.len() as u16),
        ratatui::layout::Constraint::Min(0),
      ])
      .split(area)[1];

    let popup_area = ratatui::layout::Layout::default()
      .direction(ratatui::layout::Direction::Horizontal)
      .constraints([
        ratatui::layout::Constraint::Length((area.width.saturating_sub(71)) / 2),
        ratatui::layout::Constraint::Length(77),
        ratatui::layout::Constraint::Min(0),
      ])
      .split(popup_area)[1];

    f.render_widget(Clear, popup_area);

    let help_paragraph = Paragraph::new(help_text.join("\n"))
      .style(Style::default().fg(Color::White).bg(Color::Black))
      .alignment(Alignment::Left);

    f.render_widget(help_paragraph, popup_area);
  }
}

impl Widget for LogsWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    let title_line = self.build_title_line(self.focused);
    let control_line = self.build_control_line(self.focused);

    // Create title with two lines by chaining .title()
    let block = Block::default()
      .title(title_line)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    // Apply filter + sort
    let data = self.filtered_and_sorted();

    // Build rows with enhanced styling
    let rows: Vec<Row> = data
      .iter()
      .map(|(original_idx, log)| {
        let level = Self::ev_level(log);
        let level_name = Self::level_name(level);
        let level_color = Self::get_level_color(level);
        let ts_str = Self::fmt_timestamp(Self::ev_timestamp_millis(log));
        let th = Self::ev_thread_id(log);
        let is_bookmarked = self.bookmark_indices.contains(original_idx);

        let mut cells = Vec::new();

        // Line number (if enabled)
        if self.show_line_numbers {
          let line_num = if is_bookmarked {
            format!("🔖{}", original_idx + 1)
          } else {
            format!("{}", original_idx + 1)
          };
          cells.push(Cell::from(line_num).style(Style::default().fg(Color::DarkGray)));
        }

        // Timestamp (if enabled)
        if self.show_timestamps {
          cells.push(Cell::from(ts_str).style(Style::default().fg(Color::Gray)));
        }

        // Level (if enabled)
        if self.show_levels {
          let level_text = if is_bookmarked {
            format!("🔖{}", level_name)
          } else {
            level_name.to_string()
          };
          cells.push(
            Cell::from(level_text).style(
              Style::default()
                .fg(level_color)
                .add_modifier(Modifier::BOLD),
            ),
          );
        }

        // Thread id column
        cells.push(Cell::from(format!("{}", th)).style(Style::default().fg(Color::DarkGray)));

        // Message
        let mut message = if self.wrap_lines && log.message.len() > 50 {
          format!("{}...", &log.message[..47])
        } else {
          log.message.clone()
        };
        // Prefix file:line
        let (line, col) = (log.position.0, log.position.1);
        let prefix = format!("{}:{}:{} ", log.file, line, col);
        message = format!("{}{}", prefix, message);
        cells.push(Cell::from(message).style(Style::default().fg(Color::White)));

        // Target column
        cells.push(Cell::from(log.target.clone()).style(Style::default().fg(Color::Gray)));

        Row::new(cells)
      })
      .collect();

    // Build dynamic constraints based on enabled columns
    let mut constraints = Vec::new();

    if self.show_line_numbers {
      constraints.push(ratatui::layout::Constraint::Length(6));
    }
    if self.show_timestamps {
      constraints.push(ratatui::layout::Constraint::Length(20));
    }
    if self.show_levels {
      constraints.push(ratatui::layout::Constraint::Length(4));
    }
    // Thread id
    constraints.push(ratatui::layout::Constraint::Length(6));
    // Message (with file:line prefix)
    constraints.push(ratatui::layout::Constraint::Min(20));
    // Target
    constraints.push(ratatui::layout::Constraint::Length(18));

    // Build dynamic header (own the strings to avoid temporary borrow issues)
    let mut header_cells: Vec<Cell> = Vec::new();
    if self.show_line_numbers {
      header_cells.push(Cell::from("#"));
    }
    if self.show_timestamps {
      let s = format!("Time{}", self.get_sort_indicator(SortBy::Time));
      header_cells.push(Cell::from(s));
    }
    if self.show_levels {
      let s = format!("Level{}", self.get_sort_indicator(SortBy::Level));
      header_cells.push(Cell::from(s));
    }
    header_cells.push(Cell::from("Thread"));
    {
      let s = format!("Message{}", self.get_sort_indicator(SortBy::Message));
      header_cells.push(Cell::from(s));
    }
    header_cells.push(Cell::from("Target"));

    let header = Row::new(header_cells).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let table = Table::new(rows, &constraints)
      .header(header)
      .block(block)
      .highlight_style(if self.focused {
        Style::default()
          .add_modifier(Modifier::REVERSED)
          .fg(Color::Black)
          .bg(Color::Cyan)
      } else {
        Style::default()
          .add_modifier(Modifier::REVERSED)
          .fg(Color::Black)
          .bg(Color::DarkGray)
      });

    // Auto-scroll to bottom if following tail
    let mut table_state = ratatui::widgets::TableState::default();
    if self.follow_tail && !data.is_empty() {
      table_state = table_state.with_selected(Some(data.len() - 1));
    } else {
      table_state =
        table_state.with_selected(Some(self.selected.min(data.len().saturating_sub(1))));
    }

    f.render_stateful_widget(table, area, &mut table_state);

    // Render help popup if enabled
    if self.show_help {
      self.render_help_popup(f, area);
    }

    let total_text =
      Paragraph::new(ratatui::text::Text::from(control_line)).alignment(Alignment::Right);
    let total_rect = Rect {
      x: area.x + (area.width / 2) - 10,
      y: area.y,
      width: area.width / 2 + 9,
      height: 1,
    };
    f.render_widget(total_text, total_rect);
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    if !self.focused {
      return;
    }

    // Handle help popup
    if self.show_help {
      match key.code {
        KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => {
          self.show_help = false;
        },
        _ => {},
      }
      return;
    }

    // Handle search mode
    if self.search_mode {
      match key.code {
        KeyCode::Char(c) => {
          self.search_query.push(c);
        },
        KeyCode::Backspace => {
          self.search_query.pop();
        },
        KeyCode::Enter | KeyCode::Esc => {
          self.search_mode = false;
        },
        _ => {},
      }
      return;
    }

    // Normal mode key handling
    match key.code {
      // Help
      KeyCode::Char('?') => {
        self.show_help = true;
      },

      // Search
      KeyCode::Char('/') => {
        self.search_mode = true;
      },
      KeyCode::Char('n') => {
        // Find next search result (simplified)
        if !self.search_query.is_empty() {
          let filtered = self.filtered_and_sorted();
          if self.selected + 1 < filtered.len() {
            self.selected += 1;
          }
        }
      },
      KeyCode::Char('N') => {
        // Find previous search result
        if !self.search_query.is_empty() && self.selected > 0 {
          self.selected -= 1;
        }
      },

      // Level filter
      KeyCode::Char('l') => {
        self.level_filter = match &self.level_filter {
          None => Some("ERROR".to_string()),
          Some(level) => match level.as_str() {
            "ERROR" => Some("WARN".to_string()),
            "WARN" => Some("INFO".to_string()),
            "INFO" => Some("DEBUG".to_string()),
            "DEBUG" => Some("TRACE".to_string()),
            _ => None,
          },
        };
      },

      // Clear all filters and sorting
      KeyCode::Char('c') => {
        self.search_query.clear();
        self.level_filter = None;
        self.sort_by = SortBy::Time;
        self.sort_order = SortOrder::Descending;
      },

      // Sorting
      KeyCode::Char('s') => {
        self.sort_by = match self.sort_by {
          SortBy::Time => SortBy::Level,
          SortBy::Level => SortBy::Message,
          SortBy::Message => SortBy::Time,
        };
      },
      KeyCode::Char('r') => {
        self.sort_order = match self.sort_order {
          SortOrder::Ascending => SortOrder::Descending,
          SortOrder::Descending => SortOrder::Ascending,
        };
      },

      // View toggles
      KeyCode::Char('t') => {
        self.show_timestamps = !self.show_timestamps;
      },
      KeyCode::Char('w') => {
        self.wrap_lines = !self.wrap_lines;
      },
      KeyCode::Char('#') => {
        self.show_line_numbers = !self.show_line_numbers;
      },
      KeyCode::Char('f') => {
        self.follow_tail = !self.follow_tail;
      },
      KeyCode::Char(' ') => {
        self.paused = !self.paused;
      },

      // Bookmarks
      KeyCode::Char('b') => {
        let filtered = self.filtered_and_sorted();
        if let Some((original_idx, _)) = filtered.get(self.selected) {
          if self.bookmark_indices.contains(original_idx) {
            self.bookmark_indices.retain(|&x| x != *original_idx);
          } else {
            self.bookmark_indices.push(*original_idx);
          }
        }
      },
      KeyCode::Char('B') => {
        // Jump to next bookmark
        if !self.bookmark_indices.is_empty() {
          let filtered = self.filtered_and_sorted();
          for (i, (original_idx, _)) in filtered.iter().enumerate() {
            if self.bookmark_indices.contains(original_idx) && i > self.selected {
              self.selected = i;
              return;
            }
          }
          // If no bookmark found after current, wrap to first
          for (i, (original_idx, _)) in filtered.iter().enumerate() {
            if self.bookmark_indices.contains(original_idx) {
              self.selected = i;
              break;
            }
          }
        }
      },

      // Navigation
      KeyCode::Down => {
        let filtered_count = self.filtered_and_sorted().len();
        if filtered_count > 0 && !self.follow_tail {
          self.selected = (self.selected + 1).min(filtered_count - 1);
        }
      },
      KeyCode::Up => {
        if self.selected > 0 && !self.follow_tail {
          self.selected -= 1;
        }
      },
      KeyCode::Home | KeyCode::Char('g') => {
        self.selected = 0;
        self.follow_tail = false;
      },
      KeyCode::End | KeyCode::Char('G') => {
        let filtered_count = self.filtered_and_sorted().len();
        if filtered_count > 0 {
          self.selected = filtered_count - 1;
        }
      },
      KeyCode::PageUp => {
        self.selected = self.selected.saturating_sub(10);
        self.follow_tail = false;
      },
      KeyCode::PageDown => {
        let filtered_count = self.filtered_and_sorted().len();
        if filtered_count > 0 {
          self.selected = (self.selected + 10).min(filtered_count - 1);
        }
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {}
}
