use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Style},
  text::{Line, Span, Text},
  widgets::{Block, Borders, Paragraph},
  Frame,
};
use smallvec::SmallVec;
use ttlog::{event::LogLevel, snapshot::ResolvedEvent};

use crate::widget::Widget;

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

pub struct LogsGraphWidget {
  id: u8,
  title: &'static str,
  pub events: Vec<ResolvedEvent>,
  pub time_range: TimeRange,

  // minimal filter toggles (Total removed as a bar)
  pub show_fatal: bool,
  pub show_errors: bool,
  pub show_warnings: bool,
  pub show_info: bool,
  pub show_debug: bool,
  pub show_trace: bool,
}

impl LogsGraphWidget {
  pub fn new() -> Self {
    let events = || {
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
            // synthetic packed_meta: timestamp in high bits, level in bits [8..12]
            packed_meta: ((i as u64) << 12) | ((level as u64) << 8),
            message: format!(
              "[{}] Simulated log message {}",
              match level {
                LogLevel::FATAL => "FATAL",
                LogLevel::ERROR => "ERROR",
                LogLevel::WARN => "WARN",
                LogLevel::INFO => "INFO",
                LogLevel::DEBUG => "DEBUG",
                LogLevel::TRACE => "TRACE",
              },
              i
            ),
            target: format!(
              "{}::module{}",
              match level {
                LogLevel::FATAL => "main",
                LogLevel::ERROR => "service",
                LogLevel::WARN => "controller",
                LogLevel::INFO => "api",
                LogLevel::DEBUG => "worker",
                LogLevel::TRACE => "internal",
              },
              i % 3
            ),
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
    };

    Self {
      id: 2,
      title: "~ Logs (bars) ~",
      events: events(),
      time_range: TimeRange::All,
      show_fatal: true,
      show_errors: true,
      show_warnings: true,
      show_info: true,
      show_debug: true,
      show_trace: true,
    }
  }

  pub fn with_events(mut self, events: Vec<ResolvedEvent>) -> Self {
    self.events = events;
    self
  }

  #[inline]
  fn event_ts_secs(e: &ResolvedEvent) -> u64 {
    // same extraction as original: packed_meta >> 12 is ms since epoch
    (e.packed_meta >> 12) / 1000
  }

  #[inline]
  fn event_level(e: &ResolvedEvent) -> LogLevel {
    let raw = ((e.packed_meta >> 8) & 0xF) as u8;
    LogLevel::from_u8(&raw)
  }

  fn cutoff_secs(&self) -> u64 {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs();

    match self.time_range {
      TimeRange::Last5Min => now - 5 * 60,
      TimeRange::Last15Min => now - 15 * 60,
      TimeRange::Last1Hour => now - 60 * 60,
      TimeRange::Last6Hours => now - 6 * 60 * 60,
      TimeRange::Last24Hours => now - 24 * 60 * 60,
      TimeRange::Last7Days => now - 7 * 24 * 60 * 60,
      TimeRange::All => 0,
    }
  }

  /// Aggregate counts across the filtered events.
  /// Returns (total, fatal, errors, warnings, info, debug, trace).
  fn aggregate_counts(&self) -> (u64, u64, u64, u64, u64, u64, u64) {
    let cutoff = self.cutoff_secs();

    let mut total = 0u64;
    let mut fatal = 0u64;
    let mut errors = 0u64;
    let mut warns = 0u64;
    let mut info = 0u64;
    let mut debug = 0u64;
    let mut trace = 0u64;

    for ev in &self.events {
      let ts = Self::event_ts_secs(ev);
      if ts < cutoff {
        continue;
      }
      total += 1;
      match Self::event_level(ev) {
        LogLevel::FATAL => fatal += 1,
        LogLevel::ERROR => errors += 1,
        LogLevel::WARN => warns += 1,
        LogLevel::INFO => info += 1,
        LogLevel::DEBUG => debug += 1,
        LogLevel::TRACE => trace += 1,
        _ => {},
      }
    }

    (total, fatal, errors, warns, info, debug, trace)
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

  /// Build visible bars: (label, value, style) — Total removed as a bar
  fn build_bars(&self) -> Vec<(&'static str, u64, Style)> {
    let (_total, fatal, errors, warns, info, debug, trace) = self.aggregate_counts();

    let mut bars = Vec::new();
    if self.show_fatal {
      bars.push((
        "Fatal",
        fatal,
        Style::default()
          .fg(Color::Magenta)
          .add_modifier(ratatui::style::Modifier::BOLD),
      ));
    }
    if self.show_errors {
      bars.push((
        "Errors",
        errors,
        Style::default()
          .fg(Color::Red)
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

    // at least one bar to avoid empty layout
    if bars.is_empty() {
      bars.push(("No data", 0, Style::default().fg(Color::Gray)));
    }

    bars
  }

  /// Helper: center / truncate string to width (ASCII-aware).
  fn center_pad(s: &str, width: usize) -> String {
    let mut s = s.to_string();
    if s.len() > width {
      s.truncate(width);
    }
    let pad = width.saturating_sub(s.len());
    let left = pad / 2;
    let right = pad - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
  }

  /// Render vertical bars manually so each bar can have its own color.
  /// Bars have a fixed nominal width of 8 characters (falls back if space is tight).
  /// Bars occupy at most 80% of chart height (top 20% is padding).
  /// Right 40% is used as a colored bullet panel listing counts; bottom text removed.
  fn render_colored_bars(&self, f: &mut Frame<'_>, area: Rect, focused: bool) {
    let bars = self.build_bars();
    let n = bars.len() as usize;
    let (total, _, _, _, _, _, _) = self.aggregate_counts();
    let max_value = bars.iter().map(|(_, v, _)| *v).max().unwrap_or(1);
    let max_value = if max_value == 0 { 1 } else { max_value };

    // draw outer block first (border + title)
    let outer_block = Block::default()
      .title(self.title)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(if focused { Color::Cyan } else { Color::Gray }));
    // render the outer block (border + title) around the whole area
    f.render_widget(outer_block.clone(), area);

    // leave space for border/title inside block
    let inner = Rect {
      x: area.x + 1,
      y: area.y + 1,
      width: area.width.saturating_sub(2),
      height: area.height.saturating_sub(2),
    };

    if inner.width == 0 || inner.height == 0 {
      return;
    }

    // carve right 40% for the bullet panel
    let gap_between = 1u16;
    let right_w = {
      let w = ((inner.width as f32) * 0.4f32).round() as u16;
      if w < 12 {
        12
      } else {
        w
      } // minimum reasonable width
    };
    let right_w = right_w.min(inner.width.saturating_sub(10)); // ensure chart has space

    let chart_w = inner.width.saturating_sub(right_w + gap_between);

    let chart_area = Rect {
      x: inner.x,
      y: inner.y,
      width: chart_w,
      height: inner.height,
    };

    let right_panel = Rect {
      x: inner.x + chart_w + gap_between,
      y: inner.y,
      width: right_w,
      height: inner.height,
    };

    // desired per-bar width and gap (works inside chart_area)
    let gap = 1u16;
    let desired_bar_w = 8u16; // requested fixed width
    let needed = n as u16 * desired_bar_w + (n as u16 - 1) * gap;
    let bar_w = if chart_area.width >= needed {
      desired_bar_w
    } else {
      let mut w = (chart_area.width.saturating_sub((n as u16 - 1) * gap)) / (n as u16);
      if w == 0 {
        w = 1;
      }
      w
    };

    // chart height in rows
    let chart_h_total = chart_area.height as usize;

    // if chart has no vertical space, bail
    if chart_h_total == 0 {
      return;
    }

    // usable portion for bars = 80% of chart_h_total
    let usable_h_f = (chart_h_total as f64) * 0.8f64;
    let usable_h = usable_h_f.round() as usize;
    let top_padding = chart_h_total.saturating_sub(usable_h);

    // build chart rows top->bottom
    let mut lines: Vec<Line> = Vec::with_capacity(chart_h_total);
    for row in 0..chart_h_total {
      let mut spans: Vec<Span> = Vec::with_capacity(n * 2);
      for (_, value, style) in &bars {
        // compute bar height in rows relative to usable_h
        let bar_h = ((*value as f64 / max_value as f64) * usable_h_f).round() as usize;
        // distance from bottom of chart area
        let dist_from_bottom = chart_h_total - row;
        // the filled area starts after top_padding and occupies bar_h rows
        // filled when dist_from_bottom <= bar_h
        let filled = dist_from_bottom <= bar_h;
        let s = if filled {
          let chunk = std::iter::repeat('█')
            .take(bar_w as usize)
            .collect::<String>();
          Span::styled(chunk, *style)
        } else {
          let chunk = std::iter::repeat(' ')
            .take(bar_w as usize)
            .collect::<String>();
          Span::raw(chunk)
        };
        spans.push(s);
        if gap > 0 {
          spans.push(Span::raw(" ".repeat(gap as usize)));
        }
      }
      lines.push(Line::from(spans));
    }

    // render the chart area (no block here; outer border already drawn)
    let chart_para = Paragraph::new(Text::from(lines.clone())).alignment(Alignment::Left);
    f.render_widget(chart_para, chart_area);

    // draw the total count in the top-right corner of the border (small overlay)
    let total_text = format!(" Total events: {} ", total);
    let total_text_len = total_text.len() as u16;
    if area.width > total_text_len + 2 {
      let total_rect = Rect {
        x: area.x + area.width.saturating_sub(total_text_len) - 1,
        y: area.y,
        width: total_text_len,
        height: 1,
      };
      let total_para = Paragraph::new(Line::from(vec![Span::styled(
        total_text,
        Style::default()
          .fg(Color::White)
          .add_modifier(ratatui::style::Modifier::BOLD),
      )]))
      .alignment(Alignment::Right);
      f.render_widget(total_para, total_rect);
    }

    // Build right panel: bullet list with color + label + count
    let mut right_lines: Vec<Line> = Vec::with_capacity(bars.len());
    for (label, value, style) in &bars {
      // bullet + space
      let bullet = Span::styled("● ", *style);
      // label text (bold-ish)
      let lbl = Span::styled(
        format!("{}: ", label),
        Style::default().add_modifier(ratatui::style::Modifier::BOLD),
      );
      // count in plain white
      let cnt = Span::styled(format!("{}", value), Style::default().fg(Color::White));
      right_lines.push(Line::from(vec![bullet, lbl, cnt]));
    }

    // If there is extra space in the right panel, center content vertically a bit
    let right_para = Paragraph::new(Text::from(right_lines)).alignment(Alignment::Left);
    f.render_widget(right_para, right_panel);
  }
}

impl Widget for LogsGraphWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect, focused: bool) {
    // draw bars left + right bullet panel + top-right total
    self.render_colored_bars(f, area, focused);
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    use crossterm::event::KeyCode;
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
      },
      KeyCode::Char('f') => {
        self.show_fatal = !self.show_fatal;
      },
      KeyCode::Char('e') => {
        self.show_errors = !self.show_errors;
      },
      KeyCode::Char('w') => {
        self.show_warnings = !self.show_warnings;
      },
      KeyCode::Char('i') => {
        self.show_info = !self.show_info;
      },
      KeyCode::Char('d') => {
        self.show_debug = !self.show_debug;
      },
      KeyCode::Char('r') => {
        self.show_trace = !self.show_trace;
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {
    // no-op
  }
}
