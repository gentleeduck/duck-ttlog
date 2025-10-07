mod render;

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

use crate::{logs_widget::LogsWidget, snapshots::SnapshotFile, utils::Utils};
use ttlog::{
  event::{LogEvent, LogLevel},
  snapshot::ResolvedEvent,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
  Name,
  Path,
  CreateTime,
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
  SnapshotDetail,
  EventDetail,
}

pub struct SnapshotWidget<'a> {
  // Core data
  pub id: u8,
  pub title: &'static str,
  pub snapshots: &'a Vec<SnapshotFile>,

  // State
  pub view_state: ViewState,
  pub focused: bool,
  pub paused: bool,

  // Selection and navigation
  pub selected_row: usize,
  pub scroll_offset: u16,

  // Events table state
  pub events_selected_row: usize,
  pub events_scroll_offset: u16,

  // Search and filtering
  pub search_query: String,
  pub sort_by: SortBy,
  pub sort_order: SortOrder,

  // View options
  pub show_timestamps: bool,
  pub show_line_numbers: bool,
  pub wrap_lines: bool,
  pub follow_tail: bool,
  pub auto_scroll: bool,

  // Bookmarks
  pub bookmarks: Vec<usize>,

  // UI state
  pub area: Option<Rect>,
  pub table_state: TableState,
  pub events_table_state: TableState,
  pub events_widget: Option<LogsWidget<'a>>,
}

impl<'a> SnapshotWidget<'a> {
  pub fn new(snapshots: &'a Vec<SnapshotFile>) -> Self {
    let mut widget = Self {
      id: 5,
      title: "~ System Snapshots ~──",
      snapshots,
      view_state: ViewState::Normal,
      focused: false,
      paused: false,
      selected_row: 0,
      scroll_offset: 0,
      events_selected_row: 0,
      events_scroll_offset: 0,
      search_query: String::new(),
      sort_by: SortBy::CreateTime,
      sort_order: SortOrder::Descending,
      show_timestamps: true,
      show_line_numbers: false,
      wrap_lines: false,
      follow_tail: false,
      auto_scroll: true,
      bookmarks: Vec::new(),
      area: None,
      table_state: TableState::default(),
      events_table_state: TableState::default(),
      events_widget: None,
    };

    widget.table_state.select(Some(0));
    widget.events_table_state.select(Some(0));
    widget
  }

  // Get events from current snapshot
  fn get_current_snapshot_events(&self) -> Option<&[ResolvedEvent]> {
    let snapshots = self.filtered_and_sorted_snapshots();
    snapshots
      .get(self.selected_row)
      .map(|(_, s)| s.data.events.as_slice())
  }

  // Get selected event details
  fn get_selected_event(&self) -> Option<&ResolvedEvent> {
    self
      .get_current_snapshot_events()
      .and_then(|events| events.get(self.events_selected_row))
  }

  // Events navigation
  fn move_events_cursor_up(&mut self) {
    if self.events_selected_row > 0 {
      self.events_selected_row -= 1;
      self
        .events_table_state
        .select(Some(self.events_selected_row));
    }
  }

  fn move_events_cursor_down(&mut self) {
    let events_count = self.get_current_snapshot_events().map_or(0, |e| e.len());
    if events_count > 0 && self.events_selected_row + 1 < events_count {
      self.events_selected_row += 1;
      self
        .events_table_state
        .select(Some(self.events_selected_row));
    }
  }

  fn events_page_up(&mut self) {
    self.events_selected_row = self.events_selected_row.saturating_sub(10);
    self
      .events_table_state
      .select(Some(self.events_selected_row));
  }

  fn events_page_down(&mut self) {
    let events_count = self.get_current_snapshot_events().map_or(0, |e| e.len());
    if events_count > 0 {
      self.events_selected_row = (self.events_selected_row + 10).min(events_count - 1);
      self
        .events_table_state
        .select(Some(self.events_selected_row));
    }
  }

  fn events_go_to_top(&mut self) {
    self.events_selected_row = 0;
    self.events_table_state.select(Some(0));
  }

  fn events_go_to_bottom(&mut self) {
    let events_count = self.get_current_snapshot_events().map_or(0, |e| e.len());
    if events_count > 0 {
      self.events_selected_row = events_count - 1;
      self
        .events_table_state
        .select(Some(self.events_selected_row));
    }
  }

  // Event detail scrolling
  fn scroll_event_detail_up(&mut self) {
    self.events_scroll_offset = self.events_scroll_offset.saturating_sub(1);
  }

  fn scroll_event_detail_down(&mut self) {
    self.events_scroll_offset = self.events_scroll_offset.saturating_add(1);
  }

  fn scroll_event_detail_page_up(&mut self) {
    self.events_scroll_offset = self.events_scroll_offset.saturating_sub(10);
  }

  fn scroll_event_detail_page_down(&mut self) {
    self.events_scroll_offset = self.events_scroll_offset.saturating_add(10);
  }

  fn scroll_event_detail_to_top(&mut self) {
    self.events_scroll_offset = 0;
  }

  fn scroll_event_detail_to_bottom(&mut self) {
    if let Some(content_height) = self.get_event_detail_content_height() {
      self.events_scroll_offset = content_height.saturating_sub(1);
    }
  }

  fn get_event_detail_content_height(&self) -> Option<u16> {
    if let Some(event) = self.get_selected_event() {
      // Convert to JSON string
      let json_content = serde_json::to_string(event).ok()?;

      // Now you can use resolved fields safely
      let pretty_json = serde_json::to_string_pretty(&json_content).ok()?;
      Some(pretty_json.lines().count() as u16)
    } else {
      None
    }
  }

  // Data processing
  fn filtered_and_sorted_snapshots(&self) -> Vec<(usize, &SnapshotFile)> {
    let mut filtered: Vec<(usize, &SnapshotFile)> = self
      .snapshots
      .iter()
      .enumerate()
      .filter(|(_, snapshot)| self.matches_filters(snapshot))
      .collect();

    self.sort_snapshots(&mut filtered);
    filtered
  }

  fn matches_filters(&self, snapshot: &SnapshotFile) -> bool {
    // Search filter
    if !self.search_query.is_empty() {
      let query = self.search_query.to_lowercase();
      return snapshot.name.to_lowercase().contains(&query)
        || snapshot.path.to_lowercase().contains(&query)
        || snapshot.create_at.to_lowercase().contains(&query);
    }

    true
  }

  fn sort_snapshots(&self, snapshots: &mut Vec<(usize, &SnapshotFile)>) {
    match self.sort_by {
      SortBy::Name => {
        snapshots.sort_by(|a, b| {
          let cmp = a.1.name.cmp(&b.1.name);
          if self.sort_order == SortOrder::Descending {
            cmp.reverse()
          } else {
            cmp
          }
        });
      },
      SortBy::Path => {
        snapshots.sort_by(|a, b| {
          let cmp = a.1.path.cmp(&b.1.path);
          if self.sort_order == SortOrder::Descending {
            cmp.reverse()
          } else {
            cmp
          }
        });
      },
      SortBy::CreateTime => {
        snapshots.sort_by(|a, b| {
          let cmp = a.1.create_at.cmp(&b.1.create_at);
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
    let filtered_count = self.filtered_and_sorted_snapshots().len();
    if filtered_count > 0 && self.selected_row + 1 < filtered_count {
      self.selected_row += 1;
    }
  }

  fn page_up(&mut self) {
    self.selected_row = self.selected_row.saturating_sub(10);
  }

  fn page_down(&mut self) {
    let filtered_count = self.filtered_and_sorted_snapshots().len();
    if filtered_count > 0 {
      self.selected_row = (self.selected_row + 10).min(filtered_count - 1);
    }
  }

  fn go_to_top(&mut self) {
    self.selected_row = 0;
  }

  fn go_to_bottom(&mut self) {
    let filtered_count = self.filtered_and_sorted_snapshots().len();
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
    let snapshots = self.filtered_and_sorted_snapshots();
    if let Some((_, snapshot)) = snapshots.get(self.selected_row) {
      let json_content = serde_json::to_string_pretty(&snapshot.data).ok()?;
      Some(json_content.lines().count() as u16)
    } else {
      None
    }
  }

  // Actions
  fn toggle_bookmark(&mut self) {
    let snapshots = self.filtered_and_sorted_snapshots();
    if let Some((original_idx, _)) = snapshots.get(self.selected_row) {
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
      .filtered_and_sorted_snapshots()
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

  fn cycle_sort_column(&mut self) {
    self.sort_by = match self.sort_by {
      SortBy::Name => SortBy::Path,
      SortBy::Path => SortBy::CreateTime,
      SortBy::CreateTime => SortBy::Name,
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
    self.sort_by = SortBy::CreateTime;
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
        SortBy::Name => "Name",
        SortBy::Path => "Path",
        SortBy::CreateTime => "Time",
      },
      self.get_sort_indicator(self.sort_by)
    );

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
    constraints.push(Constraint::Min(20)); // Name
    constraints.push(Constraint::Min(30)); // Path
    if self.show_timestamps {
      constraints.push(Constraint::Length(20)); // Create Time
    }

    constraints
  }

  fn build_table_header(&self) -> Row<'_> {
    let mut cells = Vec::new();

    if self.show_line_numbers {
      cells.push(Cell::from("#"));
    }
    cells.push(Cell::from(format!(
      "Name{}",
      self.get_sort_indicator(SortBy::Name)
    )));
    cells.push(Cell::from(format!(
      "Path{}",
      self.get_sort_indicator(SortBy::Path)
    )));
    if self.show_timestamps {
      cells.push(Cell::from(format!(
        "Created{}",
        self.get_sort_indicator(SortBy::CreateTime)
      )));
    }

    Row::new(cells).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    )
  }

  fn build_table_rows(&self) -> Vec<Row<'_>> {
    let snapshots = self.filtered_and_sorted_snapshots();

    snapshots
      .iter()
      .map(|(original_idx, snapshot)| {
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

        // Name
        let name_text = if is_bookmarked {
          format!("🔖{}", snapshot.name)
        } else {
          snapshot.name.clone()
        };
        cells.push(Cell::from(name_text).style(Style::default().fg(Color::White)));

        // Path
        let path_text = if self.wrap_lines && snapshot.path.len() > 40 {
          format!("{}...", &snapshot.path[..37])
        } else {
          snapshot.path.clone()
        };
        cells.push(Cell::from(path_text).style(Style::default().fg(Color::Cyan)));

        // Create Time
        if self.show_timestamps {
          cells.push(
            Cell::from(Utils::format_timestamp_from_string(&snapshot.create_at))
              .style(Style::default().fg(Color::Gray)),
          );
        }

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

  fn render_snapshot_detail_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
    let popup_area = Self::centered_rect(95, 90, area);
    f.render_widget(Clear, popup_area);

    // Avoid holding borrows across calls
    let snapshot_info = {
      let snapshots = self.filtered_and_sorted_snapshots();
      snapshots
        .get(self.selected_row)
        .map(|(_, s)| (s.name.clone(), s.data.clone()))
    };

    if let Some((snapshot_name, snapshot_data)) = snapshot_info {
      if snapshot_data.events.is_empty() {
        let json_content = serde_json::to_string_pretty(&snapshot_data)
          .unwrap_or_else(|_| "Failed to serialize snapshot data".to_string());
        let total_lines = json_content.lines().count() as u16;

        let block = Block::default()
          .title(format!(" Snapshot Data: {} ", snapshot_name))
          .title_alignment(Alignment::Center)
          .borders(Borders::ALL)
          .border_type(BorderType::Rounded)
          .border_style(Style::default().fg(Color::Green));

        let paragraph = Paragraph::new(Text::from(json_content))
          .block(block)
          .scroll((self.scroll_offset, 0))
          .alignment(Alignment::Left);

        f.render_widget(paragraph, popup_area);

        let mut scrollbar_state =
          ScrollbarState::new(total_lines as usize).position(self.scroll_offset as usize);
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
          .thumb_style(Style::default().fg(Color::Green));
        f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
      } else {
        self.render_events_table(f, popup_area, &snapshot_name);
      }
    } else {
      let block = Block::default()
        .title(" No Snapshot Selected ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));
      let paragraph = Paragraph::new("No snapshot available")
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(paragraph, popup_area);
    }
  }

  fn render_events_table(&mut self, f: &mut Frame<'_>, area: Rect, snapshot_name: &str) {
    let events = self.get_current_snapshot_events().unwrap_or(&[]);

    let block = Block::default()
      .title(format!(
        " Events in Snapshot: {} [Enter to view details] ",
        snapshot_name
      ))
      .title_alignment(Alignment::Center)
      .borders(Borders::ALL)
      .border_type(BorderType::Rounded)
      .border_style(Style::default().fg(Color::Green));

    // Build events table
    let header = Row::new(vec![
      Cell::from("#"),
      Cell::from("Event Type"),
      Cell::from("Timestamp"),
      Cell::from("Summary"),
    ])
    .style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = events
      .iter()
      .enumerate()
      .map(|(i, event)| {
        let (timestamp, level, _) = LogEvent::unpack_meta(event.packed_meta);
        let mut summary = event.message.clone();
        if summary.len() > 50 {
          summary.truncate(47);
          summary.push_str("...");
        }

        Row::new(vec![
          Cell::from(format!("{}", i + 1)),
          Cell::from(LogLevel::from_u8(&level).as_str()),
          Cell::from(Utils::format_timestamp(timestamp)),
          Cell::from(summary),
        ])
      })
      .collect();

    let constraints = [
      Constraint::Length(4),  // #
      Constraint::Length(15), // Event Type
      Constraint::Length(20), // Timestamp
      Constraint::Min(20),    // Summary
    ];

    let mut events_table_state = std::mem::take(&mut self.events_table_state);
    events_table_state.select(Some(self.events_selected_row));

    let table = Table::new(rows, &constraints)
      .header(header)
      .block(block)
      .highlight_style(Style::default().fg(Color::Black).bg(Color::Yellow));

    f.render_stateful_widget(table, area, &mut events_table_state);
    self.events_table_state = events_table_state;

    // Show navigation hint
    let hint_area = Rect {
      x: area.x + 2,
      y: area.y + area.height - 2,
      width: area.width - 4,
      height: 1,
    };

    let hint = Paragraph::new("↑↓/jk: Navigate | Enter: View details | ESC: Back")
      .style(Style::default().fg(Color::Gray));
    f.render_widget(hint, hint_area);
  }

  fn render_event_detail_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
    let popup_area = Self::centered_rect(90, 80, area);
    f.render_widget(Clear, popup_area);

    if let Some(event) = self.get_selected_event() {
      // Convert to JSON string
      let json_content = serde_json::to_string(event).unwrap();

      // Deserialize it into a strongly typed struct
      let resolved: ResolvedEvent = serde_json::from_str(&json_content).unwrap();
      let (timestamp, level, thread_id) = LogEvent::unpack_meta(resolved.packed_meta);
      let level = LogLevel::from_u8(&level);
      let timestamp = Utils::format_timestamp(timestamp);

      let json = serde_json::json!({
        "level": level,
        "timestamp": timestamp,
        "thread_id": thread_id,
        "message": resolved.message,
        "target": resolved.target,
        "kv": resolved.kv,
        "file": resolved.file,
        "position": resolved.position,
      });

      let json_content = serde_json::to_string_pretty(&json).unwrap();

      let total_lines = json_content.lines().count() as u16;

      let block = Block::default()
        .title(format!(" Event Detail #{} ", self.events_selected_row + 1))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta));

      let paragraph = Paragraph::new(Text::from(json_content))
        .block(block)
        .scroll((self.events_scroll_offset, 0))
        .alignment(Alignment::Left);

      f.render_widget(paragraph, popup_area);

      // Render scrollbar
      let mut scrollbar_state =
        ScrollbarState::new(total_lines as usize).position(self.events_scroll_offset as usize);
      let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .thumb_style(Style::default().fg(Color::Magenta));
      f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);

      // Show navigation hint
      let hint_area = Rect {
        x: popup_area.x + 2,
        y: popup_area.y + popup_area.height - 2,
        width: popup_area.width - 4,
        height: 1,
      };

      let hint = Paragraph::new("↑↓/jk: Scroll | ESC: Back to events")
        .style(Style::default().fg(Color::Gray));
      f.render_widget(hint, hint_area);
    }
  }

  fn render_help_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let help_text = [
      "┌──────────────────────────── SNAPSHOT VIEWER HELP ─────────────────────────┐",
      "│   Navigation:                 │     View Options:                         │",
      "│    ↑↓      Move cursor        │      t     Toggle timestamps              │",
      "│    PgUp/Dn Page up/down       │      w     Toggle wrap lines              │",
      "│    Home/End First/last        │      #     Toggle line numbers            │",
      "│    Enter   View snapshot data │      f     Toggle follow tail             │",
      "│                               │      Space Toggle pause                   │",
      "│   Search & Filter:            │                                           │",
      "│    /       Start search       │     Sorting:                              │",
      "│    n/N     Next/Prev result   │      s     Cycle sort column              │",
      "│    c       Clear all filters  │      r     Reverse sort order             │",
      "│                               │                                           │",
      "│   Events Navigation:          │     Other:                                │",
      "│    In Events Table:           │      ?     Toggle this help               │",
      "│    ↑↓/jk   Navigate events    │      ESC   Close popups                   │",
      "│    Enter   View event detail  │                                           │",
      "│    In Event Detail:           │                                           │",
      "│    ↑↓/jk   Scroll content     │                                           │",
      "│                               │                                           │",
      "│   Bookmarks:                  │                                           │",
      "│    b       Toggle bookmark    │                                           │",
      "│    B       Jump to next       │                                           │",
      "│                               │                                           │",
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
}
