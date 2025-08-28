use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState,
  },
  Frame,
};
use smallvec::SmallVec;
use ttlog::{event::LogLevel, snapshot::ResolvedEvent};

use crate::widget::Widget;

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

#[derive(Debug, Clone, PartialEq)]
pub enum ViewState {
  Normal,
  Search,
  Help,
  LogDetail,
}

pub struct LogsWidget {
  // Core data
  pub id: u8,
  pub title: &'static str,
  pub logs: Vec<ResolvedEvent>,

  // State
  pub view_state: ViewState,
  pub focused: bool,
  pub paused: bool,

  // Selection and navigation
  pub selected_row: usize,
  pub scroll_offset: u16,

  // Search and filtering
  pub search_query: String,
  pub level_filter: Option<String>,
  pub sort_by: SortBy,
  pub sort_order: SortOrder,

  // View options
  pub show_timestamps: bool,
  pub show_levels: bool,
  pub show_line_numbers: bool,
  pub wrap_lines: bool,
  pub follow_tail: bool,
  pub auto_scroll: bool,

  // Bookmarks
  pub bookmarks: Vec<usize>,

  // UI state
  pub area: Option<Rect>,
  pub table_state: TableState,
}

impl LogsWidget {
  pub fn new() -> Self {
    let mut widget = Self {
      id: 1,
      title: "~ System Logs ~──",
      logs: Self::generate_sample_logs(),
      view_state: ViewState::Normal,
      focused: false,
      paused: false,
      selected_row: 0,
      scroll_offset: 0,
      search_query: String::new(),
      level_filter: None,
      sort_by: SortBy::Time,
      sort_order: SortOrder::Descending,
      show_timestamps: true,
      show_levels: true,
      show_line_numbers: false,
      wrap_lines: false,
      follow_tail: false,
      auto_scroll: true,
      bookmarks: Vec::new(),
      area: None,
      table_state: TableState::default(),
    };

    widget.table_state.select(Some(0));
    widget
  }

  // Log event utility functions
  fn ev_timestamp_millis(ev: &ResolvedEvent) -> u64 {
    ev.packed_meta >> 12
  }

  fn ev_level(ev: &ResolvedEvent) -> LogLevel {
    unsafe { std::mem::transmute(((ev.packed_meta >> 8) & 0xF) as u8) }
  }

  fn ev_thread_id(ev: &ResolvedEvent) -> u8 {
    (ev.packed_meta & 0xFF) as u8
  }

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

  fn level_color(level: LogLevel) -> Color {
    match level {
      LogLevel::FATAL => Color::Red,
      LogLevel::ERROR => Color::Magenta,
      LogLevel::WARN => Color::Yellow,
      LogLevel::INFO => Color::Green,
      LogLevel::DEBUG => Color::Cyan,
      LogLevel::TRACE => Color::Gray,
    }
  }

  fn format_timestamp(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let sub_ms = (ms % 1000) as u32;
    let dt: DateTime<Utc> = Utc
      .timestamp_opt(secs, sub_ms * 1_000_000)
      .single()
      .unwrap_or_else(|| Utc.timestamp_opt(0, 0).earliest().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
  }

  // Data processing
  fn filtered_and_sorted_logs(&self) -> Vec<(usize, &ResolvedEvent)> {
    let mut filtered: Vec<(usize, &ResolvedEvent)> = self
      .logs
      .iter()
      .enumerate()
      .filter(|(_, ev)| self.matches_filters(ev))
      .collect();

    self.sort_logs(&mut filtered);
    filtered
  }

  fn matches_filters(&self, event: &ResolvedEvent) -> bool {
    // Level filter
    if let Some(ref level_filter) = self.level_filter {
      let event_level = Self::level_name(Self::ev_level(event));
      if event_level != level_filter.as_str() {
        return false;
      }
    }

    // Search filter
    if !self.search_query.is_empty() {
      let query = self.search_query.to_lowercase();
      let timestamp = Self::format_timestamp(Self::ev_timestamp_millis(event)).to_lowercase();
      let level = Self::level_name(Self::ev_level(event)).to_lowercase();

      return timestamp.contains(&query)
        || level.contains(&query)
        || event.message.to_lowercase().contains(&query)
        || event.target.to_lowercase().contains(&query)
        || event.file.to_lowercase().contains(&query);
    }

    true
  }

  fn sort_logs(&self, logs: &mut Vec<(usize, &ResolvedEvent)>) {
    match self.sort_by {
      SortBy::Time => {
        logs.sort_by(|a, b| {
          let cmp = Self::ev_timestamp_millis(a.1).cmp(&Self::ev_timestamp_millis(b.1));
          if self.sort_order == SortOrder::Descending {
            cmp.reverse()
          } else {
            cmp
          }
        });
      },
      SortBy::Level => {
        logs.sort_by(|a, b| {
          let cmp = (Self::ev_level(a.1) as u8).cmp(&(Self::ev_level(b.1) as u8));
          if self.sort_order == SortOrder::Descending {
            cmp.reverse()
          } else {
            cmp
          }
        });
      },
      SortBy::Message => {
        logs.sort_by(|a, b| {
          let cmp = a.1.message.cmp(&b.1.message);
          if self.sort_order == SortOrder::Descending {
            cmp.reverse()
          } else {
            cmp
          }
        });
      },
    }
  }

  // UI State management
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

  fn get_status_indicators(&self) -> Vec<&str> {
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
    if !self.bookmarks.is_empty() {
      indicators.push("🔖");
    }

    indicators
  }

  // Navigation
  fn move_cursor_up(&mut self) {
    if self.selected_row > 0 {
      self.selected_row -= 1;
    }
  }

  fn move_cursor_down(&mut self) {
    let filtered_count = self.filtered_and_sorted_logs().len();
    if filtered_count > 0 && self.selected_row + 1 < filtered_count {
      self.selected_row += 1;
    }
  }

  fn page_up(&mut self) {
    self.selected_row = self.selected_row.saturating_sub(10);
  }

  fn page_down(&mut self) {
    let filtered_count = self.filtered_and_sorted_logs().len();
    if filtered_count > 0 {
      self.selected_row = (self.selected_row + 10).min(filtered_count - 1);
    }
  }

  fn go_to_top(&mut self) {
    self.selected_row = 0;
  }

  fn go_to_bottom(&mut self) {
    let filtered_count = self.filtered_and_sorted_logs().len();
    if filtered_count > 0 {
      self.selected_row = filtered_count - 1;
    }
  }

  // Popup scrolling
  fn scroll_popup_up(&mut self) {
    self.scroll_offset = self.scroll_offset.saturating_sub(1);
  }

  fn scroll_popup_down(&mut self) {
    self.scroll_offset = self.scroll_offset.saturating_add(1);
  }

  fn scroll_popup_page_up(&mut self) {
    self.scroll_offset = self.scroll_offset.saturating_sub(10);
  }

  fn scroll_popup_page_down(&mut self) {
    self.scroll_offset = self.scroll_offset.saturating_add(10);
  }

  fn scroll_popup_to_top(&mut self) {
    self.scroll_offset = 0;
  }

  fn scroll_popup_to_bottom(&mut self) {
    if let Some(content_height) = self.get_popup_content_height() {
      self.scroll_offset = content_height.saturating_sub(1);
    }
  }

  fn get_popup_content_height(&self) -> Option<u16> {
    let logs = self.filtered_and_sorted_logs();
    if let Some((_, event)) = logs.get(self.selected_row) {
      let json_content = serde_json::to_string_pretty(event).ok()?;
      Some(json_content.lines().count() as u16)
    } else {
      None
    }
  }

  // Actions
  fn toggle_bookmark(&mut self) {
    let logs = self.filtered_and_sorted_logs();
    if let Some((original_idx, _)) = logs.get(self.selected_row) {
      if let Some(pos) = self.bookmarks.iter().position(|&x| x == *original_idx) {
        self.bookmarks.remove(pos);
      } else {
        self.bookmarks.push(*original_idx);
      }
    }
  }

  fn jump_to_next_bookmark(&mut self) {
    if self.bookmarks.is_empty() {
      return;
    }

    // Collect indices first to avoid holding a borrow of `self` while mutating selection
    let indices: Vec<usize> = self
      .filtered_and_sorted_logs()
      .into_iter()
      .map(|(original_idx, _)| original_idx)
      .collect();
    let mut found = false;

    // Look for next bookmark after current position
    for (i, original_idx) in indices.iter().enumerate() {
      if self.bookmarks.contains(original_idx) && i > self.selected_row {
        self.selected_row = i;
        found = true;
        break;
      }
    }

    // If not found, wrap to first bookmark
    if !found {
      for (i, original_idx) in indices.iter().enumerate() {
        if self.bookmarks.contains(original_idx) {
          self.selected_row = i;
          break;
        }
      }
    }
  }

  fn cycle_level_filter(&mut self) {
    self.level_filter = match &self.level_filter {
      None => Some("ERROR".to_string()),
      Some(level) => match level.as_str() {
        "FATAL" => Some("ERROR".to_string()),
        "ERROR" => Some("WARN".to_string()),
        "WARN" => Some("INFO".to_string()),
        "INFO" => Some("DEBUG".to_string()),
        "DEBUG" => Some("TRACE".to_string()),
        "TRACE" => Some("FATAL".to_string()),
        _ => None,
      },
    };
  }

  fn cycle_sort_column(&mut self) {
    self.sort_by = match self.sort_by {
      SortBy::Time => SortBy::Level,
      SortBy::Level => SortBy::Message,
      SortBy::Message => SortBy::Time,
    };
  }

  fn toggle_sort_order(&mut self) {
    self.sort_order = match self.sort_order {
      SortOrder::Ascending => SortOrder::Descending,
      SortOrder::Descending => SortOrder::Ascending,
    };
  }

  fn clear_all_filters(&mut self) {
    self.search_query.clear();
    self.level_filter = None;
    self.sort_by = SortBy::Time;
    self.sort_order = SortOrder::Descending;
  }

  // Rendering helpers
  fn build_title_line(&self) -> Line<'_> {
    let title = format!(" {}", self.title);
    let status_indicators = self.get_status_indicators();
    let status = if status_indicators.is_empty() {
      String::new()
    } else {
      format!(" ~ {} ~", status_indicators.join(" "))
    };

    Line::from(vec![
      Span::styled(
        title,
        Style::default()
          .fg(if self.focused {
            Color::Cyan
          } else {
            Color::White
          })
          .add_modifier(Modifier::BOLD),
      ),
      Span::styled(status, Style::default().fg(Color::Yellow)),
    ])
  }

  fn build_control_line(&self) -> Line<'_> {
    let mut spans = Vec::new();

    // Search status
    let search_text = match self.view_state {
      ViewState::Search => format!("🔍 {}_", self.search_query),
      _ if !self.search_query.is_empty() => format!("🔍 {}", self.search_query),
      _ => "Search: None".to_string(),
    };

    // Sort info
    let sort_text = format!(
      "Sort: {}{}",
      match self.sort_by {
        SortBy::Time => "Time",
        SortBy::Level => "Level",
        SortBy::Message => "Message",
      },
      self.get_sort_indicator(self.sort_by)
    );

    // Filter info
    let filter_text = self
      .level_filter
      .as_ref()
      .map(|f| format!("Filter: {}", f))
      .unwrap_or_else(|| "Filter: All".to_string());

    spans.extend([
      Span::styled("~", Style::default().fg(Color::White)),
      Span::styled(
        format!(" {} ", search_text),
        Style::default().fg(if self.view_state == ViewState::Search {
          Color::Yellow
        } else {
          Color::Gray
        }),
      ),
      Span::styled("│", Style::default().fg(Color::DarkGray)),
      Span::styled(format!(" {} ", sort_text), Style::default().fg(Color::Cyan)),
      Span::styled("│", Style::default().fg(Color::DarkGray)),
      Span::styled(
        format!(" {} ", filter_text),
        Style::default().fg(Color::Green),
      ),
    ]);

    if self.focused && self.view_state == ViewState::Normal {
      spans.extend([
        Span::styled("│", Style::default().fg(Color::DarkGray)),
        Span::styled(
          " [?] Help",
          Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::DIM),
        ),
      ]);
    }

    spans.push(Span::styled(" ~", Style::default().fg(Color::White)));
    Line::from(spans)
  }

  fn build_table_constraints(&self) -> Vec<Constraint> {
    let mut constraints = Vec::new();

    if self.show_line_numbers {
      constraints.push(Constraint::Length(6));
    }
    if self.show_timestamps {
      constraints.push(Constraint::Length(20));
    }
    if self.show_levels {
      constraints.push(Constraint::Length(7));
    }
    constraints.push(Constraint::Length(6)); // Thread ID
    constraints.push(Constraint::Min(20)); // Message
    constraints.push(Constraint::Length(15)); // Target

    constraints
  }

  fn build_table_header(&self) -> Row<'_> {
    let mut cells = Vec::new();

    if self.show_line_numbers {
      cells.push(Cell::from("#"));
    }
    if self.show_timestamps {
      cells.push(Cell::from(format!(
        "Time{}",
        self.get_sort_indicator(SortBy::Time)
      )));
    }
    if self.show_levels {
      cells.push(Cell::from(format!(
        "Level{}",
        self.get_sort_indicator(SortBy::Level)
      )));
    }
    cells.push(Cell::from("Thread"));
    cells.push(Cell::from(format!(
      "Message{}",
      self.get_sort_indicator(SortBy::Message)
    )));
    cells.push(Cell::from("Target"));

    Row::new(cells).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    )
  }

  fn build_table_rows(&self) -> Vec<Row<'_>> {
    let logs = self.filtered_and_sorted_logs();

    logs
      .iter()
      .map(|(original_idx, event)| {
        let level = Self::ev_level(event);
        let level_name = Self::level_name(level);
        let level_color = Self::level_color(level);
        let timestamp = Self::format_timestamp(Self::ev_timestamp_millis(event));
        let thread_id = Self::ev_thread_id(event);
        let is_bookmarked = self.bookmarks.contains(original_idx);

        let mut cells = Vec::new();

        // Line number
        if self.show_line_numbers {
          let line_text = if is_bookmarked {
            format!("🔖{}", original_idx + 1)
          } else {
            format!("{}", original_idx + 1)
          };
          cells.push(Cell::from(line_text).style(Style::default().fg(Color::DarkGray)));
        }

        // Timestamp
        if self.show_timestamps {
          cells.push(Cell::from(timestamp).style(Style::default().fg(Color::Gray)));
        }

        // Level
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

        // Thread ID
        cells
          .push(Cell::from(format!("{}", thread_id)).style(Style::default().fg(Color::DarkGray)));

        // Message with file:line prefix
        let (line, col) = event.position;
        let prefix = format!("{}:{}:{} ", event.file, line, col);
        let message = if self.wrap_lines && event.message.len() > 50 {
          format!("{}{}...", prefix, &event.message[..47])
        } else {
          format!("{}{}", prefix, event.message)
        };
        cells.push(Cell::from(message).style(Style::default().fg(Color::White)));

        // Target
        cells.push(Cell::from(event.target.clone()).style(Style::default().fg(Color::Gray)));

        Row::new(cells)
      })
      .collect()
  }

  fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
      ])
      .split(area);

    Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
      ])
      .split(popup_layout[1])[1]
  }

  fn render_dim_overlay(&self, f: &mut Frame<'_>, area: Rect) {
    let dim_block = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(dim_block, area);
  }

  fn render_log_detail_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let logs = self.filtered_and_sorted_logs();

    let popup_area = Self::centered_rect(80, 70, area);
    f.render_widget(Clear, popup_area);

    if let Some((_, event)) = logs.get(self.selected_row) {
      let json_content = serde_json::to_string_pretty(event)
        .unwrap_or_else(|_| "Failed to serialize log".to_string());
      let total_lines = json_content.lines().count() as u16;

      let block = Block::default()
        .title(" Log Detail ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green));

      let paragraph = Paragraph::new(Text::from(json_content))
        .block(block)
        .scroll((self.scroll_offset, 0))
        .alignment(Alignment::Left);

      f.render_widget(paragraph, popup_area);

      // Render scrollbar
      let mut scrollbar_state =
        ScrollbarState::new(total_lines as usize).position(self.scroll_offset as usize);
      let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(Color::Green));
      f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
    } else {
      let block = Block::default()
        .title(" No Log Selected ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
      let paragraph = Paragraph::new("No log available")
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(paragraph, popup_area);
    }
  }

  fn render_help_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let help_text = [
      "┌────────────────────────────── LOG VIEWER HELP ────────────────────────────┐",
      "│   Navigation:                 │     View Options:                         │",
      "│    ↑↓      Move cursor        │      t     Toggle timestamps              │",
      "│    PgUp/Dn Page up/down       │      l     Cycle level filter             │",
      "│    Home/End First/last        │      w     Toggle wrap lines              │",
      "│    Enter   View log detail    │      #     Toggle line numbers            │",
      "│                               │      f     Toggle follow tail             │",
      "│   Search & Filter:            │      Space Toggle pause                   │",
      "│    /       Start search       │                                           │",
      "│    n/N     Next/Prev result   │     Sorting:                              │",
      "│    c       Clear all filters  │      s     Cycle sort column              │",
      "│                               │      r     Reverse sort order             │",
      "│   Bookmarks:                  │                                           │",
      "│    b       Toggle bookmark    │     Other:                                │",
      "│    B       Jump to next       │      ?     Toggle this help               │",
      "│                               │      ESC   Close popups                   │",
      "└───────────────────────────────────────────────────────────────────────────┘",
    ];

    let help_height = help_text.len() as u16;
    let help_width = 77;

    let popup_area = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length((area.height.saturating_sub(help_height)) / 2),
        Constraint::Length(help_height),
        Constraint::Min(0),
      ])
      .split(area)[1];

    let popup_area = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Length((area.width.saturating_sub(help_width)) / 2),
        Constraint::Length(help_width),
        Constraint::Min(0),
      ])
      .split(popup_area)[1];

    f.render_widget(Clear, popup_area);

    let help_paragraph = Paragraph::new(help_text.join("\n"))
      .style(Style::default().fg(Color::White).bg(Color::Black))
      .alignment(Alignment::Left);

    f.render_widget(help_paragraph, popup_area);
  }

  // Sample data generator
  fn generate_sample_logs() -> Vec<ResolvedEvent> {
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
          packed_meta: ((i as u64) << 12) | ((level as u64) << 8) | (i as u64),
          message: format!("[{}] Simulated log message {}", Self::level_name(level), i),
          target: match level {
            LogLevel::FATAL => "main",
            LogLevel::ERROR => "service",
            LogLevel::WARN => "controller",
            LogLevel::INFO => "api",
            LogLevel::DEBUG => "worker",
            LogLevel::TRACE => "internal",
          }
          .to_string(),
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
  }
}

impl Widget for LogsWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);
    // Temporarily move out the table state to avoid borrowing conflicts
    let mut table_state = std::mem::take(&mut self.table_state);

    // Compute filtered count and update local table_state selection
    let filtered_count = self.filtered_and_sorted_logs().len();
    if self.follow_tail && filtered_count > 0 {
      table_state.select(Some(filtered_count - 1));
    } else if filtered_count > 0 {
      table_state.select(Some(self.selected_row.min(filtered_count - 1)));
    } else {
      table_state.select(None);
    }

    // Build the main block and table
    let title_line = self.build_title_line();
    let block = Block::default()
      .title(title_line)
      .borders(Borders::ALL)
      .border_type(BorderType::Rounded)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    // Build table components
    let constraints = self.build_table_constraints();
    let header = self.build_table_header();
    let rows = self.build_table_rows();

    let table = Table::new(rows, &constraints)
      .header(header)
      .block(block)
      .highlight_style(if self.focused {
        Style::default().fg(Color::Black).bg(Color::Cyan)
      } else {
        Style::default().fg(Color::Black).bg(Color::Blue)
      });

    // Render main table with the local table_state
    f.render_stateful_widget(table, area, &mut table_state);

    // Put the table state back
    self.table_state = table_state;

    // Render control line
    let control_line = self.build_control_line();
    let control_paragraph = Paragraph::new(Text::from(control_line)).alignment(Alignment::Right);
    let control_area = Rect {
      x: area.x + (area.width / 2).saturating_sub(10),
      y: area.y,
      width: area.width / 2 + 10,
      height: 1,
    };
    f.render_widget(control_paragraph, control_area);

    // Render popups
    match self.view_state {
      ViewState::LogDetail => {
        self.render_dim_overlay(f, area);
        self.render_log_detail_popup(f, area);
      },
      ViewState::Help => {
        self.render_dim_overlay(f, area);
        self.render_help_popup(f, area);
      },
      _ => {},
    }
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    if !self.focused {
      return;
    }

    match self.view_state {
      ViewState::LogDetail => self.handle_log_detail_keys(key),
      ViewState::Help => self.handle_help_keys(key),
      ViewState::Search => self.handle_search_keys(key),
      ViewState::Normal => self.handle_normal_keys(key),
    }
  }

  fn on_mouse(&mut self, _event: MouseEvent) {
    // Mouse handling can be implemented here if needed
    // For now, we'll leave it empty but store the area for future use
  }
}

// Key handling implementation
impl LogsWidget {
  fn handle_log_detail_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      KeyCode::Esc => {
        self.view_state = ViewState::Normal;
        self.scroll_offset = 0;
      },
      KeyCode::Up | KeyCode::Char('k') => self.scroll_popup_up(),
      KeyCode::Down | KeyCode::Char('j') => self.scroll_popup_down(),
      KeyCode::PageUp => self.scroll_popup_page_up(),
      KeyCode::PageDown => self.scroll_popup_page_down(),
      KeyCode::Home => self.scroll_popup_to_top(),
      KeyCode::End => self.scroll_popup_to_bottom(),
      _ => {},
    }
  }

  fn handle_help_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
        self.view_state = ViewState::Normal;
      },
      _ => {},
    }
  }

  fn handle_search_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      KeyCode::Char(c) => {
        self.search_query.push(c);
      },
      KeyCode::Backspace => {
        self.search_query.pop();
      },
      KeyCode::Enter | KeyCode::Esc => {
        self.view_state = ViewState::Normal;
      },
      _ => {},
    }
  }

  fn handle_normal_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      // Navigation
      KeyCode::Up | KeyCode::Char('k') => self.move_cursor_up(),
      KeyCode::Down | KeyCode::Char('j') => self.move_cursor_down(),
      KeyCode::PageUp => self.page_up(),
      KeyCode::PageDown => self.page_down(),
      KeyCode::Home => self.go_to_top(),
      KeyCode::End => self.go_to_bottom(),

      // View log detail
      KeyCode::Enter => {
        if !self.filtered_and_sorted_logs().is_empty() {
          self.view_state = ViewState::LogDetail;
          self.scroll_offset = 0;
        }
      },

      // Search
      KeyCode::Char('/') => {
        self.view_state = ViewState::Search;
      },
      KeyCode::Char('n') => {
        if !self.search_query.is_empty() {
          self.move_cursor_down(); // Simple next implementation
        }
      },
      KeyCode::Char('N') => {
        if !self.search_query.is_empty() {
          self.move_cursor_up(); // Simple previous implementation
        }
      },

      // Filtering and sorting
      KeyCode::Char('l') => self.cycle_level_filter(),
      KeyCode::Char('s') => self.cycle_sort_column(),
      KeyCode::Char('r') => self.toggle_sort_order(),
      KeyCode::Char('c') => self.clear_all_filters(),

      // View options
      KeyCode::Char('t') => self.show_timestamps = !self.show_timestamps,
      KeyCode::Char('w') => self.wrap_lines = !self.wrap_lines,
      KeyCode::Char('#') => self.show_line_numbers = !self.show_line_numbers,
      KeyCode::Char('f') => self.follow_tail = !self.follow_tail,
      KeyCode::Char(' ') => self.paused = !self.paused,

      // Bookmarks
      KeyCode::Char('b') => self.toggle_bookmark(),
      KeyCode::Char('B') => self.jump_to_next_bookmark(),

      // Help
      KeyCode::Char('?') => {
        self.view_state = ViewState::Help;
      },

      _ => {},
    }
  }
}
