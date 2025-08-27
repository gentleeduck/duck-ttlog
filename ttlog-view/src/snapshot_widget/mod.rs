use std::process::Command;

use crossterm::event::KeyCode;
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
  Frame,
};
use ttlog::snapshot::ResolvedEvent;

use crate::{
  snapshot_read::{self, SnapshotFile},
  widget::Widget,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
  Name,
  Created,
  Events,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
  Asc,
  Desc,
}

pub struct SnapshotWidget {
  pub id: u8,
  pub title: &'static str,
  pub snapshots: Vec<SnapshotFile>,
  pub area: Option<Rect>,
  pub focused: bool,

  // Main table state
  pub selected: usize,
  pub table_state: TableState,

  // Popover (events) state
  pub show_events: bool,
  pub pop_selected: usize,
  pub pop_table_state: TableState,

  // UI features
  pub show_help: bool,
  pub search_mode: bool,
  pub search_query: String,
  pub sort_by: SortBy,
  pub sort_order: SortOrder,
  pub marked: Vec<String>,   // snapshot names
  pub bookmarks: Vec<usize>, // original indexes (for quick jump)
}

impl SnapshotWidget {
  pub fn new() -> Self {
    let snapshots: Vec<SnapshotFile> = snapshot_read::read_snapshots().unwrap_or_default();
    let mut t = TableState::default();
    t.select(Some(0));
    let mut p = TableState::default();
    p.select(Some(0));
    Self {
      id: 5,
      title: "~ Snapshots ~",
      snapshots,
      area: None,
      focused: false,
      selected: 0,
      table_state: t,
      show_events: false,
      pop_selected: 0,
      pop_table_state: p,
      show_help: false,
      search_mode: false,
      search_query: String::new(),
      sort_by: SortBy::Created,
      sort_order: SortOrder::Desc,
      marked: Vec::new(),
      bookmarks: Vec::new(),
    }
  }

  // ---------- Utilities ----------
  fn get_sort_indicator(&self, col: SortBy) -> &str {
    if self.sort_by == col {
      match self.sort_order {
        SortOrder::Asc => "↑",
        SortOrder::Desc => "↓",
      }
    } else {
      ""
    }
  }

  fn status_indicators(&self) -> String {
    let mut v = Vec::new();
    if self.search_mode {
      v.push("🔍");
    }
    if !self.bookmarks.is_empty() {
      v.push("🔖");
    }
    if !self.marked.is_empty() {
      v.push("★");
    }
    if v.is_empty() {
      String::new()
    } else {
      format!("~ {} ~", v.join(" "))
    }
  }

  fn build_title(&self) -> Line<'_> {
    Line::from(vec![
      Span::styled(
        format!(" {}", self.title),
        Style::default()
          .fg(if self.focused {
            Color::Cyan
          } else {
            Color::White
          })
          .add_modifier(Modifier::BOLD),
      ),
      Span::raw(" "),
      Span::styled(self.status_indicators(), Style::default().fg(Color::Yellow)),
    ])
  }

  // Return filtered + sorted vector of (original_index, &SnapshotFile)
  fn filtered_sorted(&self) -> Vec<(usize, &SnapshotFile)> {
    let mut data: Vec<(usize, &SnapshotFile)> = self.snapshots.iter().enumerate().collect();

    // search
    if !self.search_query.is_empty() {
      let q = self.search_query.to_lowercase();
      data.retain(|(_, s)| {
        s.name.to_lowercase().contains(&q)
          || s.path.to_lowercase().contains(&q)
          || s.create_at.to_lowercase().contains(&q)
      });
    }

    // sort
    match self.sort_by {
      SortBy::Name => {
        if self.sort_order == SortOrder::Asc {
          data.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        } else {
          data.sort_by(|a, b| b.1.name.cmp(&a.1.name));
        }
      },
      SortBy::Created => {
        if self.sort_order == SortOrder::Asc {
          data.sort_by(|a, b| a.1.create_at.cmp(&b.1.create_at));
        } else {
          data.sort_by(|a, b| b.1.create_at.cmp(&a.1.create_at));
        }
      },
      SortBy::Events => {
        if self.sort_order == SortOrder::Asc {
          data.sort_by(|a, b| a.1.data.events.len().cmp(&b.1.data.events.len()));
        } else {
          data.sort_by(|a, b| b.1.data.events.len().cmp(&a.1.data.events.len()));
        }
      },
    }

    data
  }

  // Center helper (same as yours)
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

  // Attempt to open path with system opener (optional). You can remove if undesired.
  #[allow(dead_code)]
  fn try_open_path(path: &str) {
    #[cfg(target_os = "linux")]
    {
      let _ = Command::new("xdg-open").arg(path).spawn();
    }
    #[cfg(target_os = "macos")]
    {
      let _ = Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "windows")]
    {
      let _ = Command::new("cmd").arg("/C").arg("start").arg(path).spawn();
    }
  }

  // ---------- Renderers ----------
  fn render_snapshot_table(&self, f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
      .title(self.build_title())
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    if self.snapshots.is_empty() {
      let paragraph = Paragraph::new(Text::from("There is no snapshot yet!"))
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(paragraph, area);
      return;
    }

    let data = self.filtered_sorted();

    // header with sort indicators
    let header = Row::new(vec![
      Cell::from(format!("Name {}", self.get_sort_indicator(SortBy::Name))),
      Cell::from(format!("Path")),
      Cell::from(format!(
        "Created {}",
        self.get_sort_indicator(SortBy::Created)
      )),
      Cell::from(format!(
        "#Events {}",
        self.get_sort_indicator(SortBy::Events)
      )),
    ])
    .style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = data
      .iter()
      .enumerate()
      .map(|(i, (orig_idx, s))| {
        let mut style = Style::default();
        // If this is the currently selected row in the current filtered list
        if i == self.selected {
          style = style
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        }
        // Marked snapshots get green highlight (non-selected)
        if self.marked.contains(&s.name) {
          style = style.fg(Color::Green).add_modifier(Modifier::BOLD);
        }
        // Bookmarked snapshots get a small star prefix in the name cell
        let name_cell = if self.bookmarks.contains(orig_idx) {
          Cell::from(format!("🔖 {}", s.name.clone()))
        } else {
          Cell::from(s.name.clone())
        };

        Row::new(vec![
          name_cell,
          Cell::from(s.path.clone()),
          Cell::from(s.create_at.clone()),
          Cell::from(format!("{}", s.data.events.len())),
        ])
        .style(style)
      })
      .collect();

    let table = Table::new(
      rows,
      vec![
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(10),
      ],
    )
    .header(header)
    .block(block)
    .column_spacing(2)
    .highlight_style(
      Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(Color::Black)
        .bg(if self.focused {
          Color::Cyan
        } else {
          Color::DarkGray
        }),
    );

    // Maintain selection state (use table_state to position highlight in UI)
    let mut ts = self.table_state.clone();
    // If selected index may be out-of-bounds for filtered list, clamp
    let sel = if data.is_empty() {
      None
    } else {
      Some(self.selected.min(data.len().saturating_sub(1)))
    };
    ts.select(sel);

    f.render_stateful_widget(table, area, &mut ts);
  }

  fn render_event_popover(&self, f: &mut Frame<'_>, area: Rect) {
    let data = self.filtered_sorted();
    if data.is_empty() {
      // No snapshot selected in filtered view
      let popup_area = Self::centered_rect(60, 30, area);
      f.render_widget(Clear, popup_area);
      let block = Block::default()
        .title(" No snapshot ")
        .borders(Borders::ALL);
      let p = Paragraph::new("No snapshot selected")
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(p, popup_area);
      return;
    }

    let (_orig_idx, snapshot) = data.get(self.selected).unwrap();
    let events: &Vec<ResolvedEvent> = &snapshot.data.events;

    let popup_area = Self::centered_rect(92, 78, area);
    f.render_widget(Clear, popup_area);

    let block = Block::default()
      .title(format!(" Events in {} ", snapshot.name))
      .title_alignment(Alignment::Center)
      .border_type(BorderType::Double)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Magenta));

    if events.is_empty() {
      let paragraph = Paragraph::new("No events found")
        .alignment(Alignment::Center)
        .block(block);
      f.render_widget(paragraph, popup_area);
      return;
    }

    // header for events table
    let header = Row::new(vec!["Message", "Target", "File:Line"]).style(
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    );

    let rows: Vec<Row> = events
      .iter()
      .map(|ev| {
        Row::new(vec![
          Cell::from(ev.message.clone()).style(Style::default().fg(Color::White)),
          Cell::from(ev.target.clone()).style(Style::default().fg(Color::Gray)),
          Cell::from(format!("{}:{}", ev.file, ev.position.0))
            .style(Style::default().fg(Color::DarkGray)),
        ])
      })
      .collect();

    let table = Table::new(
      rows,
      vec![
        Constraint::Percentage(60),
        Constraint::Percentage(25),
        Constraint::Percentage(15),
      ],
    )
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
    let sel = Some(self.pop_selected.min(events.len().saturating_sub(1)));
    ps.select(sel);

    f.render_stateful_widget(table, popup_area, &mut ps);
  }

  fn render_help_popup(&self, f: &mut Frame<'_>, area: Rect) {
    let popup_area = Self::centered_rect(60, 50, area);
    f.render_widget(Clear, popup_area);

    let help_text = Text::from(
      "Snapshots - Help\n\n\
      Navigation:\n  ↑/↓  move\n  PgUp/PgDn  page\n  Home/End  first/last\n\n\
      Actions:\n  Enter  open events popover\n  m      mark/unmark snapshot\n  b      bookmark/unbookmark (for quick jump)\n  s      cycle sort (Name/Created/#Events)\n  r      reverse sort order\n  /      search (type, Enter to apply, Esc to cancel)\n  ?      help\n  Esc    close popover/help/search\n\n\
      Popover actions:\n  ↑/↓  navigate events\n  o    open event file with system opener (optional)\n  Esc  close popover\n",
    );

    let block = Block::default()
      .title(" Help ")
      .title_alignment(Alignment::Center)
      .border_type(BorderType::Double)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Green));

    let paragraph = Paragraph::new(help_text)
      .alignment(Alignment::Left)
      .block(block);
    f.render_widget(paragraph, popup_area);
  }
}

// ---------- Widget trait impl ----------
impl Widget for SnapshotWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);

    if self.show_help {
      self.render_help_popup(f, area);
    } else if self.show_events {
      self.render_event_popover(f, area);
    } else {
      self.render_snapshot_table(f, area);
    }
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    if !self.focused {
      return;
    }

    // Help
    if self.show_help {
      match key.code {
        KeyCode::Esc | KeyCode::Char('?') => self.show_help = false,
        _ => {},
      }
      return;
    }

    // Popover mode
    if self.show_events {
      match key.code {
        KeyCode::Esc => {
          self.show_events = false;
        },
        KeyCode::Up => {
          if self.pop_selected > 0 {
            self.pop_selected -= 1;
          }
        },
        KeyCode::Down => {
          // need event count for bounds; get from filtered snapshots
          let data = self.filtered_sorted();
          if let Some((_, snap)) = data.get(self.selected) {
            if self.pop_selected + 1 < snap.data.events.len() {
              self.pop_selected += 1;
            }
          }
        },
        KeyCode::PageUp => {
          self.pop_selected = self.pop_selected.saturating_sub(10);
        },
        KeyCode::PageDown => {
          let data = self.filtered_sorted();
          if let Some((_, snap)) = data.get(self.selected) {
            let len = snap.data.events.len();
            self.pop_selected = (self.pop_selected + 10).min(len.saturating_sub(1));
          }
        },
        KeyCode::Home => {
          self.pop_selected = 0;
        },
        KeyCode::End => {
          let data = self.filtered_sorted();
          if let Some((_, snap)) = data.get(self.selected) {
            let len = snap.data.events.len();
            if len > 0 {
              self.pop_selected = len - 1;
            }
          }
        },
        KeyCode::Char('o') => {
          // optional: open the file path of the selected event (if present).
          // This will attempt to run xdg-open/open/start. Remove if you don't want it.
          let data = self.filtered_sorted();
          if let Some((_, snap)) = data.get(self.selected) {
            if let Some(ev) = snap.data.events.get(self.pop_selected) {
              // Try open file (comment out if undesired)
              let _ = Self::try_open_path(&ev.file);
            }
          }
        },
        _ => {},
      }
      // keep popup state table selection in sync
      self.pop_table_state.select(Some(self.pop_selected));
      return;
    }

    // Search mode (typing)
    if self.search_mode {
      match key.code {
        KeyCode::Esc => {
          self.search_mode = false;
          self.search_query.clear();
        },
        KeyCode::Enter => {
          self.search_mode = false;
          // keep the query
          self.selected = 0;
        },
        KeyCode::Backspace => {
          self.search_query.pop();
        },
        KeyCode::Char(c) => {
          self.search_query.push(c);
        },
        _ => {},
      }
      return;
    }

    // Normal mode
    match key.code {
      KeyCode::Up => {
        if self.selected > 0 {
          self.selected -= 1;
        }
      },
      KeyCode::Down => {
        let filtered_len = self.filtered_sorted().len();
        if filtered_len > 0 {
          self.selected = (self.selected + 1).min(filtered_len - 1);
        }
      },
      KeyCode::PageUp => {
        self.selected = self.selected.saturating_sub(10);
      },
      KeyCode::PageDown => {
        let filtered_len = self.filtered_sorted().len();
        if filtered_len > 0 {
          self.selected = (self.selected + 10).min(filtered_len - 1);
        }
      },
      KeyCode::Home => {
        self.selected = 0;
      },
      KeyCode::End => {
        let filtered_len = self.filtered_sorted().len();
        if filtered_len > 0 {
          self.selected = filtered_len - 1;
        }
      },
      KeyCode::Enter => {
        if !self.filtered_sorted().is_empty() {
          self.show_events = true;
          self.pop_selected = 0;
          self.pop_table_state.select(Some(0));
        }
      },
      KeyCode::Char('?') => {
        self.show_help = true;
      },
      KeyCode::Char('/') => {
        self.search_mode = true;
        self.search_query.clear();
      },
      KeyCode::Char('s') => {
        self.sort_by = match self.sort_by {
          SortBy::Name => SortBy::Created,
          SortBy::Created => SortBy::Events,
          SortBy::Events => SortBy::Name,
        };
      },
      KeyCode::Char('r') => {
        self.sort_order = match self.sort_order {
          SortOrder::Asc => SortOrder::Desc,
          SortOrder::Desc => SortOrder::Asc,
        };
      },
      KeyCode::Char('m') => {
        if let Some(name) = {
          // Shorten immutable borrow: extract cloned name first
          self
            .filtered_sorted()
            .get(self.selected)
            .map(|(_, s)| s.name.clone())
        } {
          if self.marked.contains(&name) {
            self.marked.retain(|n| n != &name);
          } else {
            self.marked.push(name);
          }
        }
      },
      KeyCode::Char('b') => {
        if let Some(orig_idx) = {
          // Shorten immutable borrow: extract index first
          self.filtered_sorted().get(self.selected).map(|(i, _)| *i)
        } {
          if self.bookmarks.contains(&orig_idx) {
            self.bookmarks.retain(|&x| x != orig_idx);
          } else {
            self.bookmarks.push(orig_idx);
          }
        }
      },
      _ => {},
    }

    // keep table_state selection in sync after handling keys
    let filtered_len = self.filtered_sorted().len();
    let sel = if filtered_len == 0 {
      None
    } else {
      Some(self.selected.min(filtered_len - 1))
    };
    self.table_state.select(sel);
  }

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {}
}
