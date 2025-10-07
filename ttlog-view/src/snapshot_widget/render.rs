use crossterm::event::{KeyCode, MouseEvent};
use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Style},
  text::Text,
  widgets::{Block, BorderType, Borders, Paragraph, Table},
  Frame,
};

use crate::{
  snapshot_widget::{SnapshotWidget, ViewState},
  widget::Widget,
};

impl<'a> Widget for SnapshotWidget<'a> {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
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
      ViewState::EventDetail => {
        self.render_dim_overlay(f, area);
        self.render_snapshot_detail_popup(f, area);
        self.render_event_detail_popup(f, area);
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
      ViewState::EventDetail => self.handle_event_detail_keys(key),
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
impl<'a> SnapshotWidget<'a> {
  fn handle_event_detail_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      KeyCode::Esc => {
        self.view_state = ViewState::SnapshotDetail;
        self.events_scroll_offset = 0;
      },
      KeyCode::Up | KeyCode::Char('k') => self.scroll_event_detail_up(),
      KeyCode::Down | KeyCode::Char('j') => self.scroll_event_detail_down(),
      KeyCode::PageUp => self.scroll_event_detail_page_up(),
      KeyCode::PageDown => self.scroll_event_detail_page_down(),
      KeyCode::Home => self.scroll_event_detail_to_top(),
      KeyCode::End => self.scroll_event_detail_to_bottom(),
      _ => {},
    }
  }

  fn handle_snapshot_detail_keys(&mut self, key: crossterm::event::KeyEvent) {
    let events = self.get_current_snapshot_events();

    let has_events = events.map_or(false, |e| !e.is_empty());
    if has_events {
      // Handle events table navigation
      match key.code {
        KeyCode::Esc => {
          self.view_state = ViewState::Normal;
          self.scroll_offset = 0;
          self.events_selected_row = 0;
          self.events_table_state.select(Some(0));
        },
        KeyCode::Up | KeyCode::Char('k') => self.move_events_cursor_up(),
        KeyCode::Down | KeyCode::Char('j') => self.move_events_cursor_down(),
        KeyCode::PageUp => self.events_page_up(),
        KeyCode::PageDown => self.events_page_down(),
        KeyCode::Home => self.events_go_to_top(),
        KeyCode::End => self.events_go_to_bottom(),
        KeyCode::Enter => {
          if has_events {
            self.view_state = ViewState::EventDetail;
            self.events_scroll_offset = 0;
          }
        },
        _ => {},
      }
    } else {
      // Handle original snapshot detail scrolling
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
    if let Some(widget) = self.events_widget.as_mut() {
      widget.on_key(key);
    }

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
          self.events_selected_row = 0;
          self.events_table_state.select(Some(0));
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
