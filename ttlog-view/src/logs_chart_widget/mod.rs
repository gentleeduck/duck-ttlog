mod render;

use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Style},
  text::{Line, Span, Text},
  widgets::{Block, BorderType, Borders, Paragraph},
  Frame,
};
use ttlog::event::LogLevel;

use crate::logs::ResolvedLog;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub enum TimeRange {
  Last5Min,
  Last15Min,
  Last1Hour,
  Last6Hours,
  Last24Hours,
  Last7Days,
  All,
}

pub struct LogsChartWidget<'a> {
  pub id: u8,
  pub title: &'static str,
  pub logs: &'a Vec<ResolvedLog>,
  pub time_range: TimeRange,

  // toggles
  pub show_total: bool,
  pub show_fatal: bool,
  pub show_errors: bool,
  pub show_warnings: bool,
  pub show_info: bool,
  pub show_debug: bool,
  pub show_trace: bool,
  pub focused: bool,

  // Performance optimization
  pub cached_counts: Option<(u64, u64, u64, u64, u64, u64, u64)>,
  pub last_calculation_time: Option<Instant>,
  pub is_loading: bool,
  pub has_data: bool,
  pub processing_heavy_operation: bool,
}

impl<'a> LogsChartWidget<'a> {
  pub fn new(logs: &'a Vec<ResolvedLog>) -> Self {
    let has_data = !logs.is_empty();
    Self {
      id: 2,
      title: "~ Logs (bars) ~",
      logs,
      time_range: TimeRange::All,
      show_total: false, // default off
      show_fatal: true,
      show_errors: true,
      show_warnings: true,
      show_info: true,
      show_debug: true,
      show_trace: true,
      focused: false,
      cached_counts: None,
      last_calculation_time: None,
      is_loading: false,
      has_data,
      processing_heavy_operation: false,
    }
  }

  /// Aggregate counts across the filtered events (cached version).
  /// Returns (total, fatal, errors, warnings, info, debug, trace).
  fn aggregate_counts(&mut self) -> (u64, u64, u64, u64, u64, u64, u64) {
    // Return cached result if available
    if let Some(cached) = self.cached_counts {
      return cached;
    }

    // Set loading state for heavy operations
    if self.logs.len() > 1000 {
      self.processing_heavy_operation = true;
    }

    let start_time = Instant::now();

    let mut total = 0u64;
    let mut fatal = 0u64;
    let mut errors = 0u64;
    let mut warns = 0u64;
    let mut info = 0u64;
    let mut debug = 0u64;
    let mut trace = 0u64;

    for ev in self.logs {
      total += 1;
      match ev.level {
        LogLevel::FATAL => fatal += 1,
        LogLevel::ERROR => errors += 1,
        LogLevel::WARN => warns += 1,
        LogLevel::INFO => info += 1,
        LogLevel::DEBUG => debug += 1,
        LogLevel::TRACE => trace += 1,
      }
    }

    let result = (total, fatal, errors, warns, info, debug, trace);

    // Cache the result
    self.cached_counts = Some(result);
    self.last_calculation_time = Some(start_time);
    self.processing_heavy_operation = false;

    result
  }

  pub fn clear_cache(&mut self) {
    self.cached_counts = None;
    self.last_calculation_time = None;
  }

  /// Method to update logs after new data comes in
  pub fn update_logs(&mut self, new_logs: &'a Vec<ResolvedLog>) {
    self.logs = new_logs;
    self.has_data = !new_logs.is_empty();
    self.is_loading = false;
    self.clear_cache(); // Clear cache when logs change
  }

  pub fn is_processing(&self) -> bool {
    self.processing_heavy_operation || self.is_loading
  }

  pub fn get_status_text(&self) -> String {
    if self.is_loading {
      "Loading chart data...".to_string()
    } else if self.processing_heavy_operation {
      "Processing...".to_string()
    } else if !self.has_data {
      "No data available".to_string()
    } else {
      format!("{} logs", self.logs.len())
    }
  }

  fn format_time_range_label(&self) -> &'static str {
    match self.time_range {
      TimeRange::Last5Min => "Last 5m",
      TimeRange::Last15Min => "Last 15m",
      TimeRange::Last1Hour => "Last 1h",
      TimeRange::Last6Hours => "Last 6h",
      TimeRange::Last24Hours => "Last 24h",
      TimeRange::Last7Days => "Last 7d",
      TimeRange::All => "All",
    }
  }

  /// Build visible bars: (label, value, style)
  fn build_bars(&mut self) -> Vec<(&'static str, u64, Style)> {
    let (total, fatal, errors, warns, info, debug, trace) = self.aggregate_counts();

    let mut bars = Vec::new();
    if self.show_total {
      bars.push(("Total", total, Style::default().fg(Color::Blue)));
    }
    if self.show_fatal {
      bars.push((
        "Fatal",
        fatal,
        Style::default()
          .fg(Color::Red)
          .add_modifier(ratatui::style::Modifier::BOLD),
      ));
    }
    if self.show_errors {
      bars.push((
        "Errors",
        errors,
        Style::default()
          .fg(Color::Magenta)
          .add_modifier(ratatui::style::Modifier::BOLD),
      ));
    }
    if self.show_warnings {
      bars.push(("Warns", warns, Style::default().fg(Color::Yellow)));
    }
    if self.show_info {
      bars.push(("Info", info, Style::default().fg(Color::Green)));
    }
    if self.show_debug {
      bars.push(("Debug", debug, Style::default().fg(Color::Cyan)));
    }
    if self.show_trace {
      bars.push(("Trace", trace, Style::default().fg(Color::Gray)));
    }

    if bars.is_empty() {
      bars.push(("No data", 0, Style::default().fg(Color::Gray)));
    }

    bars
  }

  /// Draw chart left, separator, right panel (40%), top overlays for total & range,
  /// and bottom shortcuts in right panel with separator + padding.
  fn render_colored_bars(&mut self, f: &mut Frame<'_>, area: Rect) {
    let bars = self.build_bars();
    let n = bars.len() as usize;
    let (total, _, _, _, _, _, _) = self.aggregate_counts();
    let max_value = bars.iter().map(|(_, v, _)| *v).max().unwrap_or(1).max(1);

    // outer block
    let outer_block = Block::default()
      .title(self.title)
      .borders(Borders::ALL)
      .border_type(BorderType::Rounded)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    f.render_widget(outer_block, area);

    // inner area
    let inner = Rect {
      x: area.x + 1,
      y: area.y + 1,
      width: area.width.saturating_sub(2),
      height: area.height.saturating_sub(2),
    };
    if inner.width == 0 || inner.height == 0 {
      return;
    }

    // layout split
    let gap_between = 1u16;
    let right_w = ((inner.width as f32) * 0.4).round() as u16;
    let right_w = right_w.max(20).min(inner.width.saturating_sub(10));
    let chart_w = inner.width.saturating_sub(right_w + gap_between);

    let chart_area = Rect {
      x: inner.x,
      y: inner.y,
      width: chart_w,
      height: inner.height,
    };
    let sep_area = Rect {
      x: inner.x + chart_w,
      y: inner.y,
      width: gap_between,
      height: inner.height,
    };
    let right_panel = Rect {
      x: inner.x + chart_w + gap_between,
      y: inner.y,
      width: right_w,
      height: inner.height,
    };

    // bar widths
    let gap = 1u16;
    let desired_bar_w = 8u16;
    let needed = n as u16 * desired_bar_w + (n as u16 - 1) * gap;
    let bar_w = if chart_area.width >= needed {
      desired_bar_w
    } else {
      (chart_area.width.saturating_sub((n as u16 - 1) * gap) / (n as u16)).max(1)
    };

    // build chart lines
    let chart_h_total = chart_area.height as usize;
    if chart_h_total == 0 {
      return;
    }
    let usable_h_f = (chart_h_total as f64) * 0.8;
    let mut lines: Vec<Line> = Vec::with_capacity(chart_h_total);
    for row in 0..chart_h_total {
      let mut spans = Vec::with_capacity(n * 2);
      for (_, value, style) in &bars {
        let bar_h = ((*value as f64 / max_value as f64) * usable_h_f).round() as usize;
        let dist_from_bottom = chart_h_total - row;
        let filled = dist_from_bottom <= bar_h;
        let chunk = if filled { "█" } else { " " };
        spans.push(Span::styled(chunk.repeat(bar_w as usize), *style));
        spans.push(Span::raw(" ".repeat(gap as usize)));
      }
      lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(Text::from(lines)), chart_area);

    // vertical separator full height
    let mut sep_lines = Vec::with_capacity(sep_area.height as usize);
    for _ in 0..sep_area.height {
      sep_lines.push(Line::from(Span::styled(
        "│",
        Style::default().fg(Color::White),
      )));
    }
    f.render_widget(Paragraph::new(Text::from(sep_lines)), sep_area);

    // split right side vertically (stats top / shortcuts bottom)
    let [right_top, right_bottom] = ratatui::layout::Layout::default()
      .direction(ratatui::layout::Direction::Vertical)
      .constraints([
        ratatui::layout::Constraint::Percentage(50),
        ratatui::layout::Constraint::Percentage(50),
      ])
      .split(right_panel)[..]
      .try_into()
      .unwrap();

    // padding
    let pad_x = 2;
    let right_inner_top = Rect {
      x: right_top.x + pad_x,
      y: right_top.y + 1,
      width: right_top.width.saturating_sub(pad_x * 2),
      height: right_top.height.saturating_sub(1),
    };
    let right_inner_bottom = Rect {
      x: right_bottom.x + pad_x,
      y: right_bottom.y + 1,
      width: right_bottom.width.saturating_sub(pad_x * 2),
      height: right_bottom.height.saturating_sub(1),
    };

    // draw horizontal separator between top/bottom
    let mut hsep_lines = Vec::new();
    hsep_lines.push(Line::from(vec![Span::styled(
      "─".repeat(right_panel.width as usize),
      Style::default().fg(Color::White),
    )]));
    f.render_widget(
      Paragraph::new(Text::from(hsep_lines)),
      Rect {
        x: right_panel.x,
        y: right_bottom.y,
        width: right_panel.width,
        height: 1,
      },
    );

    // === right top: stats
    let mut right_lines: Vec<Line> = Vec::new();
    for (label, value, style) in &bars {
      let bullet = Span::styled("■ ", *style);
      let lbl = Span::styled(
        format!("{:<7}", label),
        Style::default().add_modifier(ratatui::style::Modifier::BOLD),
      );
      let cnt = Span::styled(format!("{}", value), Style::default().fg(Color::White));
      right_lines.push(Line::from(vec![bullet, lbl, cnt]));
    }
    f.render_widget(Paragraph::new(Text::from(right_lines)), right_inner_top);

    // === right bottom: shortcuts
    let dark = Style::default().fg(Color::DarkGray);
    let help_lines: Vec<Line> = vec![
      Line::from(vec![
        Span::styled("t", Style::default().fg(Color::White)),
        Span::styled(" : cycle time range", dark),
      ]),
      Line::from(vec![
        Span::styled("v", Style::default().fg(Color::Blue)),
        Span::styled(" : toggle total", dark),
      ]),
      Line::from(vec![
        Span::styled("f", Style::default().fg(Color::Red)),
        Span::styled(" : toggle fatal", dark),
      ]),
      Line::from(vec![
        Span::styled("e", Style::default().fg(Color::Magenta)),
        Span::styled(" : toggle errors", dark),
      ]),
      Line::from(vec![
        Span::styled("w", Style::default().fg(Color::Yellow)),
        Span::styled(" : toggle warnings", dark),
      ]),
      Line::from(vec![
        Span::styled("i", Style::default().fg(Color::Green)),
        Span::styled(" : toggle info", dark),
      ]),
      Line::from(vec![
        Span::styled("d", Style::default().fg(Color::Cyan)),
        Span::styled(" : toggle debug", dark),
      ]),
      Line::from(vec![
        Span::styled("r", Style::default().fg(Color::Gray)),
        Span::styled(" : toggle trace", dark),
      ]),
    ];
    f.render_widget(Paragraph::new(Text::from(help_lines)), right_inner_bottom);

    // === top overlays
    // range (~60%)
    let range_text = Paragraph::new(Text::from(Line::from(vec![Span::styled(
      format!("~ Range: {} ~", self.format_time_range_label()),
      Style::default()
        .fg(Color::LightBlue)
        .add_modifier(ratatui::style::Modifier::BOLD),
    )])))
    .alignment(Alignment::Center);
    let range_rect = Rect {
      x: area.x + (area.width * 4 / 10),
      y: area.y,
      width: (area.width / 3).min(20),
      height: 1,
    };
    f.render_widget(range_text, range_rect);

    // total (right aligned)
    let total_text = Paragraph::new(Text::from(Line::from(vec![Span::styled(
      format!("~ Total events: {} ~", total),
      Style::default()
        .fg(Color::LightBlue)
        .add_modifier(ratatui::style::Modifier::BOLD),
    )])))
    .alignment(Alignment::Right);
    let total_rect = Rect {
      x: area.x + (area.width / 2),
      y: area.y,
      width: area.width / 2 - 1,
      height: 1,
    };
    f.render_widget(total_text, total_rect);
  }
}
