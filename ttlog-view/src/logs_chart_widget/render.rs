use crossterm::event::KeyCode;
use ratatui::{layout::Rect, Frame};

use crate::{
  logs_chart_widget::{LogsChartWidget, TimeRange},
  widget::Widget,
};

impl<'a> Widget for LogsChartWidget<'a> {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    // draw chart + separator + right panel + overlays
    self.render_colored_bars(f, area);
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    if !self.focused {
      return;
    }

    match key.code {
      KeyCode::Char('t') => {
        self.time_range = match self.time_range {
          TimeRange::Last5Min => TimeRange::Last15Min,
          TimeRange::Last15Min => TimeRange::Last1Hour,
          TimeRange::Last1Hour => TimeRange::Last6Hours,
          TimeRange::Last6Hours => TimeRange::Last24Hours,
          TimeRange::Last24Hours => TimeRange::Last7Days,
          TimeRange::Last7Days => TimeRange::All,
          TimeRange::All => TimeRange::Last5Min,
        };
        self.clear_cache(); // Clear cache when time range changes
      },
      KeyCode::Char('v') => {
        // toggle total bar
        self.show_total = !self.show_total;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('f') => {
        self.show_fatal = !self.show_fatal;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('e') => {
        self.show_errors = !self.show_errors;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('w') => {
        self.show_warnings = !self.show_warnings;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('i') => {
        self.show_info = !self.show_info;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('d') => {
        self.show_debug = !self.show_debug;
        self.clear_cache(); // Clear cache when display options change
      },
      KeyCode::Char('r') => {
        self.show_trace = !self.show_trace;
        self.clear_cache(); // Clear cache when display options change
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {
    // no-op
  }
}
