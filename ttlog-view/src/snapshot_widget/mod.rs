use chrono::{DateTime, Utc};
use crossterm::event::KeyCode;
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::Modifier,
  style::{Color, Style},
  text::{Line, Span},
  widgets::{Block, BorderType, Borders, Clear, Paragraph},
  widgets::{Cell, Row, Table, TableState},
  Frame,
};

use crate::{
  logs_widget::LogsWidget,
  snapshot_read::{read_snapshots, SnapshotFile},
  widget::Widget,
};

pub struct SnapshotWidget {
  pub id: u8,
  pub title: &'static str,
  pub logs: Vec<SnapshotFile>,
  pub area: Option<Rect>,
  pub search_query: String,
  pub sort_by: SortBy,
  pub sort_order: SortOrder,
  pub selected: usize,
  pub search_mode: bool,
  pub auto_scroll: bool,
  pub show_help: bool,
  pub paused: bool,
  pub focused: bool,

  // Popover state
  pub show_log_popover: bool,
  pub pop_selected: usize,
  pub pop_table_state: TableState,
  pub evnets_widget: LogsWidget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
  Name,
  Path,
  CreatedAt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
  Ascending,
  Descending,
}

impl SnapshotWidget {
  pub fn new() -> Self {
    let snapshots: Vec<SnapshotFile> = read_snapshots().unwrap_or_default();

    let mut ts = TableState::default();
    ts.select(Some(0));

    Self {
      id: 5,
      title: "~ Snapshots ~──",
      logs: snapshots,
      area: None,
      search_query: String::new(),
      sort_by: SortBy::CreatedAt,
      sort_order: SortOrder::Descending,
      selected: 0,
      search_mode: false,
      auto_scroll: true,
      show_help: false,
      paused: false,
      focused: false,
      show_log_popover: false,
      pop_selected: 0,
      pop_table_state: ts,
      evnets_widget: LogsWidget::new(),
    }
  }

  /// Return filtered + sorted list of `(idx, SnapshotFile)` pairs
  fn filtered_and_sorted(&self) -> Vec<(usize, SnapshotFile)> {
    let mut data: Vec<(usize, SnapshotFile)> = self.logs.clone().into_iter().enumerate().collect();

    // Search filter: check name, path, create_at, or debug of data
    if !self.search_query.is_empty() {
      let q = self.search_query.to_lowercase();
      data.retain(|(_, s)| {
        let data_debug = format!("{:?}", &s.data).to_lowercase(); // may require Debug on SnapShot
        s.name.to_lowercase().contains(&q)
          || s.path.to_lowercase().contains(&q)
          || s.create_at.to_lowercase().contains(&q)
          || data_debug.contains(&q)
      });
    }

    // Sorting
    match self.sort_by {
      SortBy::Name => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        } else {
          data.sort_by(|a, b| b.1.name.cmp(&a.1.name));
        }
      },
      SortBy::Path => {
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.path.cmp(&b.1.path));
        } else {
          data.sort_by(|a, b| b.1.path.cmp(&a.1.path));
        }
      },
      SortBy::CreatedAt => {
        // using lexicographic order on the create_at string; replace with parsed datetime if desired
        if self.sort_order == SortOrder::Ascending {
          data.sort_by(|a, b| a.1.create_at.cmp(&b.1.create_at));
        } else {
          data.sort_by(|a, b| b.1.create_at.cmp(&a.1.create_at));
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

  fn get_status_indicators(&self) -> String {
    let mut indicators = Vec::new();
    if self.paused {
      indicators.push("⏸");
    }
    if self.auto_scroll {
      indicators.push("📜");
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

  fn build_control_line(&self) -> Line<'_> {
    let mut spans = Vec::new();
    let search_status = if self.search_mode {
      format!("🔍 {}_", self.search_query)
    } else if !self.search_query.is_empty() {
      format!("🔍 {}", self.search_query)
    } else {
      "Search: None".to_string()
    };

    let sort_info = format!(
      "Sort: {}{}",
      match self.sort_by {
        SortBy::Name => "Name",
        SortBy::Path => "Path",
        SortBy::CreatedAt => "CreatedAt",
      },
      self.get_sort_indicator(self.sort_by)
    );

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
    spans.push(Span::styled(" ~", Style::default().fg(Color::White)));

    Line::from(spans)
  }

  // Center helper (same approach used in your snapshot file)
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

  /// Paint a dim background over `area` so the popup feels modal.
  fn render_dim_overlay(&self, f: &mut Frame<'_>, area: Rect) {
    let dim = Block::default().style(Style::default().bg(Color::Black));
    f.render_widget(dim, area);
  }

  fn format_timestamp(ts: &str) -> String {
    // Try parsing as RFC3339 (ISO 8601, e.g. 2025-08-28T14:53:21Z)
    if let Ok(dt) = DateTime::parse_from_rfc3339(ts) {
      return dt
        .with_timezone(&Utc)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    }

    // fallback: show as-is
    ts.to_string()
  }

  /// Render a popover showing details of the selected snapshot.
  fn render_log_popover(&self, f: &mut Frame<'_>, area: Rect) {
    let data = self.filtered_and_sorted();
    if data.is_empty() {
      let popup_area = Self::centered_rect(50, 20, area);
      f.render_widget(Clear, popup_area);
      let block = Block::default()
        .title(" No snapshot ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
      let p = Paragraph::new("No snapshot selected")
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(p, popup_area);
      return;
    }

    let (_orig_idx, snap) = data.get(self.selected).unwrap();

    // Big popover
    let popup_area = Self::centered_rect(70, 50, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
      .title(format!(" Snapshot Detail "))
      .title_alignment(Alignment::Center)
      .border_type(BorderType::Double)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Green));

    // Prepare content rows: Name, Path, CreatedAt, Data (truncated)
    let header = Row::new(vec!["Field", "Value"]).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let mut rows: Vec<Row> = Vec::new();
    rows.push(Row::new(vec![
      Cell::from("Name"),
      Cell::from(snap.name.clone()),
    ]));
    rows.push(Row::new(vec![
      Cell::from("Path"),
      Cell::from(snap.path.clone()),
    ]));
    rows.push(Row::new(vec![
      Cell::from("Created At"),
      Cell::from(SnapshotWidget::format_timestamp(&snap.create_at.clone())),
    ]));

    // Data debug/truncated
    let data_text = format!("{:?}", &snap.data);
    let data_trunc = if data_text.len() > 300 {
      format!("{}...", &data_text[..297])
    } else {
      data_text
    };
    rows.push(Row::new(vec![Cell::from("Data"), Cell::from(data_trunc)]));

    let table = Table::new(rows, vec![Constraint::Length(14), Constraint::Min(20)])
      .header(header)
      .block(block)
      .column_spacing(2)
      .highlight_style(
        Style::default()
          .add_modifier(Modifier::REVERSED)
          .fg(Color::Black)
          .bg(Color::Cyan),
      );

    let mut ps = self.pop_table_state.clone();
    let sel = Some(self.pop_selected.min(3)); // 4 rows (0..3)
    ps.select(sel);

    f.render_stateful_widget(table, popup_area, &mut ps);
  }

  fn render_help_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let help_text = vec![
      "┌────────────────────────────── SNAPSHOT VIEWER HELP ───────────────────────┐",
      "│   Navigation:                                                             │",
      "│    ↑↓   Move cursor         │     View Options:                           │",
      "│    PgUp/Dn Page up/dn       │      /     Search                            │",
      "│    Home/End First/last      │      s     Sort column                       │",
      "│   Search & Filter:          │                                             │",
      "│    /     Search             │      ESC   Exit help                        │",
      "└───────────────────────────────────────────────────────────────────────────┘",
    ];

    let popup_area = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length((area.height.saturating_sub(help_text.len() as u16)) / 2),
        Constraint::Length(help_text.len() as u16),
        Constraint::Min(0),
      ])
      .split(area)[1];

    let popup_area = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Length((area.width.saturating_sub(71)) / 2),
        Constraint::Length(77),
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

impl Widget for SnapshotWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);

    let title_line = self.build_title_line(self.focused);

    let block = Block::default()
      .title(title_line)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    let data = self.filtered_and_sorted();

    if data.is_empty() {
      let empty_msg = Paragraph::new("No snapshots available")
        .alignment(Alignment::Center)
        .block(block.clone());
      f.render_widget(empty_msg, area);
      return;
    }

    let rows: Vec<Row> = data
      .iter()
      .map(|(_, snap)| {
        let mut cells = Vec::new();

        let name_display = if snap.name.len() > 30 {
          format!("...{}", &snap.name[snap.name.len() - 28..])
        } else {
          snap.name.clone()
        };

        cells.push(Cell::from(name_display).style(Style::default().fg(Color::White)));

        let path_display = if snap.path.len() > 45 {
          format!("...{}", &snap.path[snap.path.len() - 27..])
        } else {
          snap.path.clone()
        };
        cells.push(Cell::from(path_display).style(Style::default().fg(Color::Gray)));

        cells.push(
          Cell::from(SnapshotWidget::format_timestamp(&snap.create_at.clone()))
            .style(Style::default().fg(Color::DarkGray)),
        );
        Row::new(cells)
      })
      .collect();

    let constraints = vec![
      Constraint::Length(31),
      Constraint::Length(50),
      Constraint::Length(16),
    ];

    let header_cells = vec![
      Cell::from(format!("Name{}", self.get_sort_indicator(SortBy::Name))),
      Cell::from(format!("Path{}", self.get_sort_indicator(SortBy::Path))),
      Cell::from(format!(
        "Created{}",
        self.get_sort_indicator(SortBy::CreatedAt)
      )),
    ];

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

    let mut table_state = TableState::default();
    table_state = table_state.with_selected(Some(self.selected.min(data.len().saturating_sub(1))));

    f.render_stateful_widget(table, area, &mut table_state);

    if self.show_log_popover {
      self.render_dim_overlay(f, area);
      self.render_log_popover(f, area);
    } else if self.show_help {
      self.render_dim_overlay(f, area);
      self.render_help_popup(f, area);
    }

    let total_text = Paragraph::new(ratatui::text::Text::from(self.build_control_line()))
      .alignment(Alignment::Right);
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

    // Popover navigation
    if self.show_log_popover {
      match key.code {
        KeyCode::Esc => {
          self.show_log_popover = false;
        },
        KeyCode::Up => {
          if self.pop_selected > 0 {
            self.pop_selected -= 1;
          }
        },
        KeyCode::Down => {
          self.pop_selected = (self.pop_selected + 1).min(3);
        },
        KeyCode::PageUp => {
          self.pop_selected = self.pop_selected.saturating_sub(3);
        },
        KeyCode::PageDown => {
          self.pop_selected = (self.pop_selected + 3).min(3);
        },
        KeyCode::Home => {
          self.pop_selected = 0;
        },
        KeyCode::End => {
          self.pop_selected = 3;
        },
        _ => {},
      }
      self.pop_table_state.select(Some(self.pop_selected));
      return;
    }

    // Help popup
    if self.show_help {
      match key.code {
        KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Esc => {
          self.show_help = false;
        },
        _ => {},
      }
      return;
    }

    // Search mode
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
      KeyCode::Char('?') => {
        self.show_help = true;
      },
      KeyCode::Enter => {
        if !self.filtered_and_sorted().is_empty() {
          self.show_log_popover = true;
          self.pop_selected = 0;
          self.pop_table_state.select(Some(0));
        }
      },
      KeyCode::Char('/') => {
        self.search_mode = true;
      },
      KeyCode::Char('n') => {
        if !self.search_query.is_empty() {
          let filtered = self.filtered_and_sorted();
          if self.selected + 1 < filtered.len() {
            self.selected += 1;
          }
        }
      },
      KeyCode::Char('N') => {
        if !self.search_query.is_empty() && self.selected > 0 {
          self.selected -= 1;
        }
      },
      KeyCode::Char('c') => {
        self.search_query.clear();
        self.sort_by = SortBy::CreatedAt;
        self.sort_order = SortOrder::Descending;
      },
      // Cycle sort column
      KeyCode::Char('s') => {
        self.sort_by = match self.sort_by {
          SortBy::Name => SortBy::Path,
          SortBy::Path => SortBy::CreatedAt,
          SortBy::CreatedAt => SortBy::Name,
        };
      },
      // Reverse sort order
      KeyCode::Char('r') => {
        self.sort_order = match self.sort_order {
          SortOrder::Ascending => SortOrder::Descending,
          SortOrder::Descending => SortOrder::Ascending,
        };
      },

      // Pause toggle
      KeyCode::Char(' ') => {
        self.paused = !self.paused;
      },

      // Navigation
      KeyCode::Down => {
        let filtered_count = self.filtered_and_sorted().len();
        if filtered_count > 0 {
          self.selected = (self.selected + 1).min(filtered_count - 1);
        }
      },
      KeyCode::Up => {
        if self.selected > 0 {
          self.selected -= 1;
        }
      },
      KeyCode::Home | KeyCode::Char('g') => {
        self.selected = 0;
      },
      KeyCode::End | KeyCode::Char('G') => {
        let filtered_count = self.filtered_and_sorted().len();
        if filtered_count > 0 {
          self.selected = filtered_count - 1;
        }
      },
      KeyCode::PageUp => {
        self.selected = self.selected.saturating_sub(10);
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

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {
    // left as-is
  }
}
