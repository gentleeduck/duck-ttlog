mod keydown;
mod render;

use crate::{logs::ResolvedLog, utils::Utils};

use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{
    Block, BorderType, Borders, Cell, Clear, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, TableState,
  },
  Frame,
};
use ttlog::event::LogLevel;

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

pub struct LogsWidget<'a> {
  // Core data
  pub id: u8,
  pub title: &'a str,
  pub logs: &'a Vec<ResolvedLog>,

  // State
  pub view_state: ViewState,
  pub focused: bool,
  pub paused: bool,
  pub is_loading: bool,
  pub has_data: bool,
  pub error_message: Option<String>,

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
  pub page_size: usize,
  pub processing_heavy_operation: bool,

  // Performance optimization - simple caching
  pub cached_filtered_logs: Option<Vec<(usize, ResolvedLog)>>,
  pub cache_key: String,
  pub page_size_limit: usize, // Limit rendering to improve performance

  // Virtualized rendering for millions of events
  pub virtual_scroll_offset: usize, // Start index of visible window
  pub virtual_window_size: usize,   // Number of visible rows
  pub total_filtered_count: usize,  // Total number of filtered items

  // Background and on-demand loading
  pub is_sample_data: bool,      // True if showing only sample data
  pub full_data_loading: bool,   // True if full data is being loaded in background
  pub load_more_requested: bool, // True if user requested more data
}

impl<'a> LogsWidget<'a> {
  pub fn new(logs: &'a Vec<ResolvedLog>) -> Self {
    let has_data = !logs.is_empty();

    // If logs are empty, start in loading state
    let is_loading = logs.is_empty();
    let mut widget = Self {
      id: 1,
      title: "~ System Logs ~──",
      logs,
      view_state: ViewState::Normal,
      focused: false,
      paused: false,
      is_loading,
      has_data,
      error_message: None,

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
      page_size: 20,
      processing_heavy_operation: false,
      cached_filtered_logs: None,
      cache_key: String::new(),
      page_size_limit: 100, // Limit to 100 visible logs for performance
      virtual_scroll_offset: 0,
      virtual_window_size: 20, // Only render 20 visible rows
      total_filtered_count: 0,
      is_sample_data: logs.len() <= 100, // Detect if we have sample data
      full_data_loading: false,
      load_more_requested: false,
    };

    widget.table_state.select(Some(0));
    widget
  }

  // Method to update logs after async loading
  pub fn update_logs(&mut self, new_logs: &'a Vec<ResolvedLog>) {
    self.logs = new_logs;
    self.has_data = !new_logs.is_empty();
    self.is_loading = false;
    self.error_message = None;
    self.clear_cache(); // Clear cache when logs change

    // Reset selection if we now have data
    if self.has_data {
      self.table_state.select(Some(0));
    }
  }

  // Method to set loading state
  pub fn set_loading_state(&mut self, loading: bool) {
    self.is_loading = loading;
    if loading {
      self.error_message = None;
    }
  }

  // Method to set error state
  pub fn set_error_state(&mut self, error: String) {
    self.is_loading = false;
    self.has_data = false;
    self.error_message = Some(error);
  }

  pub fn with_events(mut self, logs: &'a Vec<ResolvedLog>) -> Self {
    self.logs = logs;
    self.focused = true;
    self
  }

  fn ev_timestamp_millis(event: &ResolvedLog) -> u64 {
    // Parse the timestamp string to extract milliseconds
    // The timestamp is in format "YYYY-MM-DD HH:MM:SS.sss"
    if let Ok(dt) = chrono::DateTime::parse_from_str(&event.timestamp, "%Y-%m-%d %H:%M:%S%.3f %z") {
      dt.timestamp_millis() as u64
    } else if let Ok(dt) =
      chrono::NaiveDateTime::parse_from_str(&event.timestamp, "%Y-%m-%d %H:%M:%S%.3f")
    {
      dt.and_utc().timestamp_millis() as u64
    } else {
      // Fallback: try to parse as a simple timestamp or return 0
      0
    }
  }

  fn ev_level(event: &ResolvedLog) -> LogLevel {
    event.level
  }

  // Virtualized data processing - only return visible window
  fn filtered_and_sorted_logs(&self) -> Vec<(usize, &ResolvedLog)> {
    // For very large datasets, use virtualized approach
    if self.logs.len() > 10000 {
      return self.get_virtualized_logs();
    }

    // For large datasets, use cached approach
    if self.logs.len() > 1000 {
      return self.get_cached_filtered_logs();
    }

    // For smaller datasets, use direct approach (faster)
    self.compute_filtered_logs_direct()
  }

  // Virtualized rendering - only compute and return visible rows
  fn get_virtualized_logs(&self) -> Vec<(usize, &ResolvedLog)> {
    // Get total count efficiently without materializing all results
    let total_count = self.get_total_filtered_count_fast();

    // Calculate visible window bounds
    let start_idx = self.virtual_scroll_offset;
    let end_idx = (start_idx + self.virtual_window_size).min(total_count);

    if start_idx >= total_count {
      return Vec::new();
    }

    // Pre-compute search query once
    let search_query_lower = if !self.search_query.is_empty() {
      Some(self.search_query.to_lowercase())
    } else {
      None
    };

    // Stream through logs and only collect the visible window
    let mut visible_logs = Vec::with_capacity(self.virtual_window_size);
    let mut current_idx = 0;
    let mut collected = 0;

    for (original_idx, log) in self.logs.iter().enumerate() {
      if self.matches_filters_optimized(log, &search_query_lower) {
        if current_idx >= start_idx && collected < self.virtual_window_size {
          visible_logs.push((original_idx, log));
          collected += 1;

          if collected >= self.virtual_window_size {
            break; // We have enough visible rows
          }
        }
        current_idx += 1;

        if current_idx >= end_idx {
          break; // We've passed the visible window
        }
      }
    }

    // Sort only the visible rows (much faster)
    self.sort_logs(&mut visible_logs);
    visible_logs
  }

  // Fast count without materializing results
  fn get_total_filtered_count_fast(&self) -> usize {
    if let Some(ref cached) = self.cached_filtered_logs {
      return cached.len();
    }

    // Pre-compute search query once
    let search_query_lower = if !self.search_query.is_empty() {
      Some(self.search_query.to_lowercase())
    } else {
      None
    };

    // Count matching logs without collecting them
    self
      .logs
      .iter()
      .filter(|log| self.matches_filters_optimized(log, &search_query_lower))
      .count()
  }

  fn get_cached_filtered_logs(&self) -> Vec<(usize, &ResolvedLog)> {
    let current_key = self.get_cache_key();

    // Check if cache is valid
    if let Some(ref cached) = self.cached_filtered_logs {
      if self.cache_key == current_key {
        // Return references from cached data, limited for performance
        return cached
          .iter()
          .take(self.page_size_limit)
          .map(|(idx, log)| (*idx, log))
          .collect();
      }
    }

    // Cache miss - fall back to direct computation but limited
    self
      .compute_filtered_logs_direct()
      .into_iter()
      .take(self.page_size_limit)
      .collect()
  }

  fn compute_filtered_logs_direct(&self) -> Vec<(usize, &ResolvedLog)> {
    // Pre-compute lowercase search query once for performance
    let search_query_lower = if !self.search_query.is_empty() {
      Some(self.search_query.to_lowercase())
    } else {
      None
    };

    let mut filtered: Vec<(usize, &ResolvedLog)> = self
      .logs
      .iter()
      .enumerate()
      .filter(|(_, log)| self.matches_filters_optimized(log, &search_query_lower))
      .collect();

    self.sort_logs(&mut filtered);
    filtered
  }

  fn get_cache_key(&self) -> String {
    format!(
      "{}|{}|{}|{}",
      self.level_filter.as_deref().unwrap_or(""),
      self.search_query,
      format!("{:?}", self.sort_by),
      format!("{:?}", self.sort_order)
    )
  }

  fn matches_filters_optimized(
    &self,
    event: &ResolvedLog,
    search_query_lower: &Option<String>,
  ) -> bool {
    // Level filter (most selective first for performance)
    if let Some(ref level_filter) = self.level_filter {
      let event_level = Utils::level_name(event.level);
      if event_level != level_filter.as_str() {
        return false;
      }
    }

    // Search filter (optimized - query already lowercased)
    if let Some(ref query) = search_query_lower {
      // Check most common fields first, avoid repeated to_lowercase() calls
      if event.message.to_lowercase().contains(query) {
        return true;
      }
      if event.level.as_str().to_lowercase().contains(query) {
        return true;
      }
      if event.target.to_lowercase().contains(query) {
        return true;
      }
      if event.timestamp.contains(query) {
        // timestamp usually doesn't need lowercasing
        return true;
      }
      if event.file.to_lowercase().contains(query) {
        return true;
      }

      return false;
    }

    true
  }

  // Keep original method for backward compatibility
  fn matches_filters(&self, event: &ResolvedLog) -> bool {
    let search_query_lower = if !self.search_query.is_empty() {
      Some(self.search_query.to_lowercase())
    } else {
      None
    };
    self.matches_filters_optimized(event, &search_query_lower)
  }

  fn sort_logs(&self, logs: &mut Vec<(usize, &ResolvedLog)>) {
    match self.sort_by {
      SortBy::Time => {
        logs.sort_by(|(_, a), (_, b)| {
          let a_time = Self::ev_timestamp_millis(a);
          let b_time = Self::ev_timestamp_millis(b);
          match self.sort_order {
            SortOrder::Ascending => a_time.cmp(&b_time),
            SortOrder::Descending => b_time.cmp(&a_time),
          }
        });
      },
      SortBy::Level => {
        logs.sort_by(|(_, a), (_, b)| {
          let a_level = Self::ev_level(a) as u8;
          let b_level = Self::ev_level(b) as u8;
          match self.sort_order {
            SortOrder::Ascending => a_level.cmp(&b_level),
            SortOrder::Descending => b_level.cmp(&a_level),
          }
        });
      },
      SortBy::Message => {
        logs.sort_by(|(_, a), (_, b)| match self.sort_order {
          SortOrder::Ascending => a.message.cmp(&b.message),
          SortOrder::Descending => b.message.cmp(&a.message),
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
  pub fn move_cursor_up(&mut self) {
    if self.logs.len() > 10000 {
      // Virtual scrolling mode
      if self.selected_row > 0 {
        self.selected_row -= 1;
      } else if self.virtual_scroll_offset > 0 {
        self.scroll_up(1);
        // Keep selection at top when scrolling up
      }
    } else {
      // Normal mode
      if self.selected_row > 0 {
        self.selected_row -= 1;
      }
    }
  }

  pub fn move_cursor_down(&mut self) {
    if self.logs.len() > 10000 {
      // Virtual scrolling mode
      let visible_logs = self.filtered_and_sorted_logs();
      if self.selected_row < visible_logs.len().saturating_sub(1) {
        self.selected_row += 1;
      } else {
        // Try to scroll down to show more logs
        let total_count = self.get_total_filtered_count();
        if self.virtual_scroll_offset + self.virtual_window_size < total_count {
          self.scroll_down(1);
          // Keep selection at bottom when scrolling down
        }
      }
    } else {
      // Normal mode
      let filtered_count = self.filtered_and_sorted_logs().len();
      if filtered_count > 0 && self.selected_row < filtered_count - 1 {
        self.selected_row += 1;
      }
    }
  }

  // Virtual scrolling methods for fast navigation
  pub fn virtual_scroll_up(&mut self, lines: usize) {
    if self.logs.len() > 10000 {
      self.scroll_up(lines);
    } else {
      // Fallback to regular navigation for smaller datasets
      for _ in 0..lines {
        self.move_cursor_up();
      }
    }
  }

  pub fn virtual_scroll_down(&mut self, lines: usize) {
    if self.logs.len() > 10000 {
      self.scroll_down(lines);
    } else {
      // Fallback to regular navigation for smaller datasets
      for _ in 0..lines {
        self.move_cursor_down();
      }
    }
  }

  // Removed duplicate methods - using virtualized versions below

  pub fn go_to_top(&mut self) {
    if self.logs.len() > 10000 {
      self.scroll_to_top();
    } else {
      self.selected_row = 0;
    }
  }

  pub fn go_to_bottom(&mut self) {
    if self.logs.len() > 10000 {
      self.scroll_to_bottom();
    } else {
      let filtered_count = self.filtered_and_sorted_logs().len();
      if filtered_count > 0 {
        self.selected_row = filtered_count - 1;
      }
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

  pub fn cycle_level_filter(&mut self) {
    let new_filter = match &self.level_filter {
      None => Some("ERROR".to_string()),
      Some(level) => match level.as_str() {
        "ERROR" => Some("WARN".to_string()),
        "WARN" => Some("INFO".to_string()),
        "INFO" => Some("DEBUG".to_string()),
        "DEBUG" => Some("TRACE".to_string()),
        "TRACE" => Some("FATAL".to_string()),
        "FATAL" => None,
        _ => None,
      },
    };
    self.level_filter = new_filter;
  }

  pub fn cycle_sort_column(&mut self) {
    self.sort_by = match self.sort_by {
      SortBy::Time => SortBy::Level,
      SortBy::Level => SortBy::Message,
      SortBy::Message => SortBy::Time,
    };
  }

  pub fn toggle_sort_order(&mut self) {
    self.sort_order = match self.sort_order {
      SortOrder::Ascending => SortOrder::Descending,
      SortOrder::Descending => SortOrder::Ascending,
    };
  }

  pub fn clear_all_filters(&mut self) {
    self.level_filter = None;
    self.search_query.clear();

    // Update cache for large datasets
    if self.logs.len() > 1000 {
      self.update_cache();
    }
  }

  pub fn clear_cache(&mut self) {
    // Clear cache when filters change
    self.cached_filtered_logs = None;
    self.cache_key.clear();
    self.processing_heavy_operation = false;

    // Reset virtual scroll when filters change
    self.virtual_scroll_offset = 0;
    self.selected_row = 0;
  }

  // Update cache asynchronously for better performance
  pub fn update_cache(&mut self) {
    if self.logs.len() <= 1000 {
      return; // No need to cache small datasets
    }

    let current_key = self.get_cache_key();
    if self.cache_key == current_key {
      return; // Cache is already up to date
    }

    // Set processing flag for heavy operations
    self.processing_heavy_operation = true;

    // Pre-compute lowercase search query once
    let search_query_lower = if !self.search_query.is_empty() {
      Some(self.search_query.to_lowercase())
    } else {
      None
    };

    // Filter and clone logs for caching
    let mut filtered: Vec<(usize, ResolvedLog)> = self
      .logs
      .iter()
      .enumerate()
      .filter(|(_, log)| self.matches_filters_optimized(log, &search_query_lower))
      .map(|(idx, log)| (idx, log.clone()))
      .collect();

    // Sort the cached data
    self.sort_cached_logs(&mut filtered);

    // Update cache
    self.cached_filtered_logs = Some(filtered);
    self.cache_key = current_key;
    self.processing_heavy_operation = false;
  }

  fn sort_cached_logs(&self, logs: &mut Vec<(usize, ResolvedLog)>) {
    match self.sort_by {
      SortBy::Time => {
        logs.sort_by(|(_, a), (_, b)| {
          let a_time = Self::ev_timestamp_millis(a);
          let b_time = Self::ev_timestamp_millis(b);
          match self.sort_order {
            SortOrder::Ascending => a_time.cmp(&b_time),
            SortOrder::Descending => b_time.cmp(&a_time),
          }
        });
      },
      SortBy::Level => {
        logs.sort_by(|(_, a), (_, b)| {
          let a_level = Self::ev_level(a) as u8;
          let b_level = Self::ev_level(b) as u8;
          match self.sort_order {
            SortOrder::Ascending => a_level.cmp(&b_level),
            SortOrder::Descending => b_level.cmp(&a_level),
          }
        });
      },
      SortBy::Message => {
        logs.sort_by(|(_, a), (_, b)| match self.sort_order {
          SortOrder::Ascending => a.message.cmp(&b.message),
          SortOrder::Descending => b.message.cmp(&a.message),
        });
      },
    }
  }

  // Performance monitoring methods
  pub fn get_total_filtered_count(&self) -> usize {
    // Use fast count for virtualized rendering
    if self.logs.len() > 10000 {
      return self.get_total_filtered_count_fast();
    }

    if let Some(ref cached) = self.cached_filtered_logs {
      cached.len()
    } else {
      // Fallback - count without caching (slower but accurate)
      self
        .logs
        .iter()
        .filter(|log| self.matches_filters(log))
        .count()
    }
  }

  // Virtual scrolling controls
  pub fn scroll_up(&mut self, lines: usize) {
    if self.virtual_scroll_offset >= lines {
      self.virtual_scroll_offset -= lines;
    } else {
      self.virtual_scroll_offset = 0;
    }
    self.update_selection_for_virtual_scroll();
  }

  pub fn scroll_down(&mut self, lines: usize) {
    let total_count = self.get_total_filtered_count();
    let max_offset = if total_count > self.virtual_window_size {
      total_count - self.virtual_window_size
    } else {
      0
    };

    self.virtual_scroll_offset = (self.virtual_scroll_offset + lines).min(max_offset);
    self.update_selection_for_virtual_scroll();
  }

  pub fn page_up(&mut self) {
    if self.logs.len() > 10000 {
      self.scroll_up(self.virtual_window_size);
    } else {
      // Traditional page navigation for smaller datasets
      self.selected_row = self.selected_row.saturating_sub(10);
    }
  }

  pub fn page_down(&mut self) {
    if self.logs.len() > 10000 {
      self.scroll_down(self.virtual_window_size);
    } else {
      // Traditional page navigation for smaller datasets
      let filtered_count = self.filtered_and_sorted_logs().len();
      if filtered_count > 0 {
        self.selected_row = (self.selected_row + 10).min(filtered_count - 1);
      }
    }
  }

  pub fn scroll_to_top(&mut self) {
    self.virtual_scroll_offset = 0;
    self.selected_row = 0;
  }

  pub fn scroll_to_bottom(&mut self) {
    let total_count = self.get_total_filtered_count();
    if total_count > self.virtual_window_size {
      self.virtual_scroll_offset = total_count - self.virtual_window_size;
    } else {
      self.virtual_scroll_offset = 0;
    }
    self.selected_row = self.virtual_window_size.saturating_sub(1);
  }

  fn update_selection_for_virtual_scroll(&mut self) {
    // Keep selection within visible window
    if self.selected_row >= self.virtual_window_size {
      self.selected_row = self.virtual_window_size.saturating_sub(1);
    }
  }

  // Get virtual scroll info for UI display
  pub fn get_virtual_scroll_info(&self) -> (usize, usize, usize) {
    let total_count = self.get_total_filtered_count();
    let visible_start = self.virtual_scroll_offset + 1; // 1-indexed for display
    let visible_end = (self.virtual_scroll_offset + self.virtual_window_size).min(total_count);
    (visible_start, visible_end, total_count)
  }

  pub fn set_page_size_limit(&mut self, limit: usize) {
    self.page_size_limit = limit.max(10); // Ensure at least 10
    self.clear_cache(); // Clear cache when limit changes
  }

  pub fn set_search_query(&mut self, query: String) {
    if self.search_query != query {
      self.search_query = query;

      // Update cache for large datasets
      if self.logs.len() > 1000 {
        self.update_cache();
      }
    }
  }

  pub fn is_processing(&self) -> bool {
    self.processing_heavy_operation || self.is_loading
  }

  pub fn get_status_text(&mut self) -> String {
    if self.is_loading {
      "Loading logs...".to_string()
    } else if self.processing_heavy_operation {
      "Processing...".to_string()
    } else if self.full_data_loading {
      "Loading full dataset in background...".to_string()
    } else if !self.has_data {
      "No data available".to_string()
    } else if let Some(ref error) = self.error_message {
      format!("Error: {}", error)
    } else {
      let filtered_count = self.get_filtered_count();
      let total_count = self.logs.len();
      let status = if filtered_count != total_count {
        format!("{} of {} logs", filtered_count, total_count)
      } else {
        format!("{} logs", total_count)
      };

      if self.is_sample_data {
        format!("{} (sample - press 'L' to load all)", status)
      } else {
        status
      }
    }
  }

  // Request loading of full dataset
  pub fn request_full_data_load(&mut self) {
    if self.is_sample_data && !self.full_data_loading {
      self.load_more_requested = true;
      self.full_data_loading = true;
    }
  }

  // Check if full data load was requested
  pub fn should_load_full_data(&self) -> bool {
    self.load_more_requested && !self.is_loading
  }

  // Signal that full data should be loaded (simpler approach)
  pub fn mark_full_data_needed(&mut self) {
    self.is_sample_data = false;
    self.full_data_loading = false;
    self.load_more_requested = false;
    self.clear_cache(); // Clear cache to refresh with new data
  }

  pub fn get_filtered_count(&self) -> usize {
    self.filtered_and_sorted_logs().len()
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
        let level_name = Utils::level_name(event.level);
        let level_color = Utils::level_color(event.level);
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
          cells.push(Cell::from(event.timestamp.clone()).style(Style::default().fg(Color::Gray)));
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
        cells.push(
          Cell::from(format!("{}", event.thread_id)).style(Style::default().fg(Color::DarkGray)),
        );

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
}
