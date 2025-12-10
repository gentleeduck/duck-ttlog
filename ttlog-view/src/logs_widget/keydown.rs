use crossterm::event::KeyCode;

use crate::logs_widget::{LogsWidget, ViewState};

impl<'a> LogsWidget<'a> {
  pub fn handle_log_detail_keys(&mut self, key: crossterm::event::KeyEvent) {
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

  pub fn handle_help_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
        self.view_state = ViewState::Normal;
      },
      _ => {},
    }
  }

  pub fn handle_search_keys(&mut self, key: crossterm::event::KeyEvent) {
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

  pub fn handle_normal_keys(&mut self, key: crossterm::event::KeyEvent) {
    match key.code {
      // Navigation - enhanced for virtual scrolling
      KeyCode::Up | KeyCode::Char('k') => self.move_cursor_up(),
      KeyCode::Down | KeyCode::Char('j') => self.move_cursor_down(),
      KeyCode::PageUp => self.page_up(),
      KeyCode::PageDown => self.page_down(),
      KeyCode::Home => self.go_to_top(),
      KeyCode::End => self.go_to_bottom(),

      // Virtual scrolling controls for large datasets
      KeyCode::Char('J') => self.virtual_scroll_down(5), // Fast scroll down
      KeyCode::Char('K') => self.virtual_scroll_up(5),   // Fast scroll up
      KeyCode::Char('G') => self.go_to_bottom(),         // Go to end
      KeyCode::Char('g') => self.go_to_top(),            // Go to beginning

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
      KeyCode::Char('L') => {
        // Load full dataset if currently showing sample data
        if self.is_sample_data {
          self.request_full_data_load();
        }
      },
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

      // Load more data
      KeyCode::Char('m') => {
        // Alternative key for loading more data
        if self.is_sample_data {
          self.request_full_data_load();
        }
      },

      _ => {},
    }
  }
}
