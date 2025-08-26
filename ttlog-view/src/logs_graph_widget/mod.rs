use crossterm::event::KeyCode;
use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Style},
  text::{Line, Span, Text},
  widgets::{Block, BorderType, Borders, Paragraph},
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
  pub id: u8,
  pub title: &'static str,
  pub events: Vec<ResolvedEvent>,
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
      show_total: false, // default off
      show_fatal: true,
      show_errors: true,
      show_warnings: true,
      show_info: true,
      show_debug: true,
      show_trace: true,
      focused: false,
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

  /// Build visible bars: (label, value, style)
  fn build_bars(&self) -> Vec<(&'static str, u64, Style)> {
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
  fn render_colored_bars(&self, f: &mut Frame<'_>, area: Rect) {
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

impl Widget for LogsGraphWidget {
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
      },
      KeyCode::Char('v') => {
        // toggle total bar
        self.show_total = !self.show_total;
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
