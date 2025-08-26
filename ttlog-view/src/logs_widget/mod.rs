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

use crate::widget::Widget;

#[derive(Debug, Clone)]
pub struct LogEntry {
  pub timestamp: String,
  pub level: String,
  pub message: String,
}

pub struct LogsWidget {
  pub id: u8,
  pub title: &'static str,
  pub logs: Vec<LogEntry>,
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
  pub fn new() -> Self {
    Self {
      id: 1,
      title: "~ System Logs ~",
      logs: vec![
        LogEntry {
          timestamp: "2025-08-25 20:01:00".into(),
          level: "INFO".into(),
          message: "Application startup sequence initiated. Loading configuration files from /etc/app/config and validating environment variables.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:05".into(),
          level: "DEBUG".into(),
          message: "Configuration loaded successfully: { database_url: postgres://user@localhost/db, redis_url: redis://127.0.0.1 }".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:10".into(),
          level: "INFO".into(),
          message: "Database connection established to host=localhost port=5432 dbname=main with connection pool size=15.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:15".into(),
          level: "WARN".into(),
          message: "Query execution exceeded expected threshold: SELECT * FROM users WHERE last_login IS NULL took 4521ms.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:20".into(),
          level: "INFO".into(),
          message: "Background scheduler started. Registered 4 recurring jobs: cleanup_temp_files, refresh_cache, sync_users, send_metrics.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:25".into(),
          level: "ERROR".into(),
          message: "Failed to fetch data from external API https://api.example.com/v1/items (HTTP 503 Service Unavailable). Retrying in 30s.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:30".into(),
          level: "INFO".into(),
          message: "Retrying API request... attempt=2 endpoint=https://api.example.com/v1/items params={limit=100, offset=0}.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:35".into(),
          level: "DEBUG".into(),
          message: "Cache lookup for key=session:43a7c12b-dfd2-4ad8-b51d returned hit. Expiration=3600s RemainingTTL=1782s.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:40".into(),
          level: "INFO".into(),
          message: "User login succeeded: user_id=42 username=alice ip=192.168.0.14 agent='Mozilla/5.0 (X11; Linux x86_64)'.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:45".into(),
          level: "INFO".into(),
          message: "Graceful shutdown requested. Flushing 37 pending log events, closing 12 DB connections, and terminating worker threads.".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:50".into(),
          level: "TRACE".into(),
          message: "Memory allocation: heap_size=256MB stack_size=2MB gc_cycles=42 allocations_per_second=1247".into(),
        },
        LogEntry {
          timestamp: "2025-08-25 20:01:55".into(),
          level: "FATAL".into(),
          message: "Critical system failure: Out of memory. Cannot allocate 512MB for buffer. System unstable.".into(),
        },
      ],
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

  fn filtered_and_sorted(&self) -> Vec<(usize, LogEntry)> {
    let mut data: Vec<(usize, LogEntry)> = self.logs.iter().cloned().enumerate().collect();

    // 1. Apply level filter
    if let Some(level_filter) = &self.level_filter {
      let filter = level_filter.to_uppercase();
      data.retain(|(_, e)| e.level == filter);
    }

    // 2. Apply search filter
    if !self.search_query.is_empty() {
      let query = self.search_query.to_lowercase();
      data.retain(|(_, e)| {
        e.timestamp.to_lowercase().contains(&query)
          || e.level.to_lowercase().contains(&query)
          || e.message.to_lowercase().contains(&query)
      });
    }

    // 3. Sort
    match self.sort_by {
      SortBy::Time => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.timestamp.cmp(&b.1.timestamp));
        } else {
          data.sort_by(|a, b| b.1.timestamp.cmp(&a.1.timestamp));
        }
      },
      SortBy::Level => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.level.cmp(&b.1.level));
        } else {
          data.sort_by(|a, b| b.1.level.cmp(&a.1.level));
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

  fn get_level_color(level: &str) -> Color {
    match level {
      "FATAL" => Color::Red,
      "ERROR" => Color::Magenta,
      "WARN" => Color::Yellow,
      "INFO" => Color::Green,
      "DEBUG" => Color::Cyan,
      "TRACE" => Color::Gray,
      _ => Color::White,
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
      format!(" {} ", indicators.join(" "))
    }
  }

  fn build_title_line(&self, focused: bool) -> Line<'_> {
    let title = format!(" {} ", self.title);

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
        let level_color = Self::get_level_color(&log.level);
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
          cells.push(Cell::from(log.timestamp.clone()).style(Style::default().fg(Color::Gray)));
        }

        // Level (if enabled)
        if self.show_levels {
          let level_text = if is_bookmarked {
            format!("🔖{}", log.level)
          } else {
            log.level.clone()
          };
          cells.push(
            Cell::from(level_text).style(
              Style::default()
                .fg(level_color)
                .add_modifier(Modifier::BOLD),
            ),
          );
        }

        // Message
        let message = if self.wrap_lines && log.message.len() > 50 {
          format!("{}...", &log.message[..47])
        } else {
          log.message.clone()
        };
        cells.push(Cell::from(message).style(Style::default().fg(Color::White)));

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
      constraints.push(ratatui::layout::Constraint::Length(10));
    }
    constraints.push(ratatui::layout::Constraint::Min(10));

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
    {
      let s = format!("Message{}", self.get_sort_indicator(SortBy::Message));
      header_cells.push(Cell::from(s));
    }

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
