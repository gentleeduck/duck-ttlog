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

use crate::{logs_widget::LogsWidget, snapshot_read::SnapshotFile, widget::Widget};

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
}

pub struct SnapshotWidget {
  // Core data
  pub id: u8,
  pub title: &'static str,
  pub snapshots: Vec<SnapshotFile>,

  // State
  pub view_state: ViewState,
  pub focused: bool,
  pub paused: bool,

  // Selection and navigation
  pub selected_row: usize,
  pub scroll_offset: u16,

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
}

impl SnapshotWidget {
  pub fn new() -> Self {
    let mut widget = Self {
      id: 5,
      title: "~ System Snapshots ~──",
      snapshots: crate::snapshot_read::read_snapshots().unwrap_or_default(),
      view_state: ViewState::Normal,
      focused: false,
      paused: false,
      selected_row: 0,
      scroll_offset: 0,
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
    };

    widget.table_state.select(Some(0));
    widget
  }

  fn format_timestamp(timestamp_str: &str) -> String {
    // Assume the timestamp is already formatted, or parse and reformat if needed
    timestamp_str.to_string()
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
            Cell::from(Self::format_timestamp(&snapshot.create_at))
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

  fn render_snapshot_detail_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let snapshots = self.filtered_and_sorted_snapshots();

    let popup_area = Self::centered_rect(95, 90, area);
    f.render_widget(Clear, popup_area);

    if let Some((_, snapshot)) = snapshots.get(self.selected_row) {
      let mut logs = LogsWidget::new().with_events(vec![]);
      logs.render(f, popup_area);

      // let json_content = serde_json::to_string_pretty(&snapshot.data)
      //   .unwrap_or_else(|_| "Failed to serialize snapshot data".to_string());
      // let total_lines = json_content.lines().count() as u16;
      //
      // let block = Block::default()
      //   .title(format!(" Snapshot Data: {} ", snapshot.name))
      //   .title_alignment(Alignment::Center)
      //   .borders(Borders::ALL)
      //   .border_type(BorderType::Rounded)
      //   .border_style(Style::default().fg(Color::Green));
      //
      // let paragraph = Paragraph::new(Text::from(json_content))
      //   .block(block)
      //   .scroll((self.scroll_offset, 0))
      //   .alignment(Alignment::Left);
      //
      // f.render_widget(paragraph, popup_area);
      //
      // // Render scrollbar
      // let mut scrollbar_state =
      //   ScrollbarState::new(total_lines as usize).position(self.scroll_offset as usize);
      // let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
      //   .thumb_style(Style::default().fg(Color::Green));
      // f.render_stateful_widget(scrollbar, popup_area, &mut scrollbar_state);
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
      "│   Bookmarks:                  │     Other:                                │",
      "│    b       Toggle bookmark    │      ?     Toggle this help               │",
      "│    B       Jump to next       │      ESC   Close popups                   │",
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

  // Sample data generator removed: we now load real snapshots via `snapshot_read::read_snapshots()`.
}

impl Widget for SnapshotWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect, &mut events_widget: &mut LogsWidget) {
    self.area = Some(area);
    // Temporarily move out the table state to avoid borrowing conflicts
    let mut table_state = std::mem::take(&mut self.table_state);

    // Compute filtered count and update local table_state selection
    let filtered_count = self.filtered_and_sorted_snapshots().len();
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
      ViewState::SnapshotDetail => {
        self.render_dim_overlay(f, area);
        self.render_snapshot_detail_popup(f, area);
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
      ViewState::SnapshotDetail => self.handle_snapshot_detail_keys(key),
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
impl SnapshotWidget {
  fn handle_snapshot_detail_keys(&mut self, key: crossterm::event::KeyEvent) {
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

      // View snapshot detail
      KeyCode::Enter => {
        if !self.filtered_and_sorted_snapshots().is_empty() {
          self.view_state = ViewState::SnapshotDetail;
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

      // Sorting
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
