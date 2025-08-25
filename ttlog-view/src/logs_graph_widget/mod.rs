use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Style},
  symbols,
  widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph},
  Frame,
};
use std::collections::HashMap;
use ttlog::{
  event::LogLevel,
  snapshot::ResolvedEvent,
};

use crate::widget::Widget;

// Local extension traits for external types
trait ResolvedEventExt {
  fn timestamp_millis(&self) -> u64;
  fn level(&self) -> LogLevel;
  fn thread_id(&self) -> u8;
}

// Backward compatibility with previous name used in main.rs
pub type LogsGraphWidget = LogLevelBarWidget;

impl ResolvedEventExt for ResolvedEvent {
  #[inline]
  fn timestamp_millis(&self) -> u64 {
    self.packed_meta >> 12
  }

  #[inline]
  fn level(&self) -> LogLevel {
    // Safe conversion via provided helper
    let raw = ((self.packed_meta >> 8) & 0xF) as u8;
    LogLevel::from_u8(&raw)
  }

  #[inline]
  fn thread_id(&self) -> u8 {
    (self.packed_meta & 0xFF) as u8
  }
}

trait LogLevelUiExt {
  fn color(self) -> Color;
}

impl LogLevelUiExt for LogLevel {
  fn color(self) -> Color {
    match self {
      LogLevel::TRACE => Color::Cyan,
      LogLevel::DEBUG => Color::Blue,
      LogLevel::INFO => Color::Green,
      LogLevel::WARN => Color::Yellow,
      LogLevel::ERROR => Color::Red,
      LogLevel::FATAL => Color::Magenta,
    }
  }
}

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

pub struct LogLevelBarWidget {
  id: u8,
  title: &'static str,
  pub events: Vec<ResolvedEvent>,
  pub time_range: TimeRange,
  pub show_percentages: bool,
  pub highlight_errors: bool,
  pub show_counts: bool,
}

impl LogLevelBarWidget {
  pub fn new() -> Self {
    Self {
      id: 1,
      title: "Log Level Distribution",
      events: Vec::new(),
      time_range: TimeRange::Last1Hour,
      show_percentages: true,
      highlight_errors: true,
      show_counts: true,
    }
  }

  pub fn with_events(mut self, events: Vec<ResolvedEvent>) -> Self {
    self.events = events;
    self
  }

  fn filter_by_time_range(&self) -> Vec<&ResolvedEvent> {
    let now = std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_secs();

    let cutoff = match self.time_range {
      TimeRange::Last5Min => now - 5 * 60,
      TimeRange::Last15Min => now - 15 * 60,
      TimeRange::Last1Hour => now - 60 * 60,
      TimeRange::Last6Hours => now - 6 * 60 * 60,
      TimeRange::Last24Hours => now - 24 * 60 * 60,
      TimeRange::Last7Days => now - 7 * 24 * 60 * 60,
      TimeRange::All => 0,
    };

    self
      .events
      .iter()
      // packed timestamp is in milliseconds
      .filter(|event| (event.timestamp_millis() / 1000) >= cutoff)
      .collect()
  }

  fn calculate_level_distribution(&self) -> HashMap<LogLevel, u64> {
    let events = self.filter_by_time_range();
    let mut level_counts = HashMap::new();

    for event in events {
      *level_counts.entry(event.level()).or_insert(0u64) += 1;
    }

    level_counts
  }

  fn render_bar_chart(&self, f: &mut Frame<'_>, area: Rect, focused: bool) {
    let level_counts = self.calculate_level_distribution();
    let total_events: u64 = level_counts.values().sum();

    if total_events == 0 {
      let no_data = Paragraph::new("No log data available for selected time range")
        .style(Style::default().fg(Color::Gray))
        .alignment(Alignment::Center)
        .block(
          Block::default()
            .title(self.title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if focused { Color::Cyan } else { Color::Gray })),
        );
      f.render_widget(no_data, area);
      return;
    }

    // Define log levels in order of severity
    let levels = [
      LogLevel::FATAL,
      LogLevel::ERROR,
      LogLevel::WARN,
      LogLevel::INFO,
      LogLevel::DEBUG,
      LogLevel::TRACE,
    ];

    // Keep owned data vectors alive while referenced by datasets
    let mut series: Vec<(&'static str, Color, Vec<(f64, f64)>)> = Vec::new();
    for (i, &level) in levels.iter().enumerate() {
      let count = *level_counts.get(&level).unwrap_or(&0);
      if count == 0 {
        continue;
      }
      let data = vec![(i as f64, count as f64)];
      let color = if self.highlight_errors && matches!(level, LogLevel::ERROR | LogLevel::FATAL) {
        Color::Red
      } else {
        level.color()
      };
      series.push((level.as_str(), color, data));
    }

    let datasets: Vec<Dataset> = series
      .iter()
      .map(|(name, color, data)| {
        Dataset::default()
          .name(*name)
          .marker(symbols::Marker::Block)
          .style(Style::default().fg(*color))
          .graph_type(ratatui::widgets::GraphType::Line)
          .data(data)
      })
      .collect();

    let max_count = level_counts.values().max().unwrap_or(&1);

    // Create labels with counts and percentages if enabled
    let x_labels: Vec<String> = levels
      .iter()
      .map(|&level| {
        let count = *level_counts.get(&level).unwrap_or(&0);
        let mut label = level.as_str().to_string();

        if self.show_counts {
          label.push_str(&format!("\n({})", count));
        }

        if self.show_percentages && total_events > 0 {
          let percentage = (count as f64 / total_events as f64) * 100.0;
          label.push_str(&format!("\n{:.1}%", percentage));
        }

        label
      })
      .collect();

    let chart = Chart::new(datasets)
      .block(
        Block::default()
          .title(format!("{} ({} events)", self.title, total_events))
          .borders(Borders::ALL)
          .border_style(Style::default().fg(if focused { Color::Cyan } else { Color::Gray })),
      )
      .x_axis(
        Axis::default()
          .title("Log Levels")
          .style(Style::default().fg(Color::Gray))
          .bounds([0.0, (levels.len() - 1) as f64])
          .labels(x_labels),
      )
      .y_axis(
        Axis::default()
          .title("Count")
          .style(Style::default().fg(Color::Gray))
          .bounds([0.0, *max_count as f64 * 1.1])
          .labels(vec![
            "0".to_string(),
            format!("{}", max_count / 4),
            format!("{}", max_count / 2),
            format!("{}", (max_count * 3) / 4),
            format!("{}", max_count),
          ]),
      );

    f.render_widget(chart, area);
  }

  pub fn get_error_rate(&self) -> f64 {
    let level_counts = self.calculate_level_distribution();
    let total = level_counts.values().sum::<u64>();

    if total == 0 {
      return 0.0;
    }

    let error_count = level_counts.get(&LogLevel::ERROR).unwrap_or(&0)
      + level_counts.get(&LogLevel::FATAL).unwrap_or(&0);

    (error_count as f64 / total as f64) * 100.0
  }

  pub fn get_critical_alerts(&self) -> Vec<String> {
    let level_counts = self.calculate_level_distribution();
    let total = level_counts.values().sum::<u64>();
    let mut alerts = Vec::new();

    if total == 0 {
      return alerts;
    }

    let fatal_count = *level_counts.get(&LogLevel::FATAL).unwrap_or(&0);
    let error_count = *level_counts.get(&LogLevel::ERROR).unwrap_or(&0);
    let warn_count = *level_counts.get(&LogLevel::WARN).unwrap_or(&0);

    if fatal_count > 0 {
      alerts.push(format!("🚨 {} FATAL errors detected!", fatal_count));
    }

    let error_rate = ((error_count + fatal_count) as f64 / total as f64) * 100.0;
    if error_rate > 10.0 {
      alerts.push(format!("⚠️ High error rate: {:.1}%", error_rate));
    }

    if warn_count > total / 2 {
      alerts.push(format!("⚠️ High warning count: {}", warn_count));
    }

    alerts
  }
}

impl Widget for LogLevelBarWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect, focused: bool) {
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .constraints([Constraint::Min(0), Constraint::Length(3)])
      .split(area);

    // Render main chart
    self.render_bar_chart(f, chunks[0], focused);

    // Render status/alerts bar at bottom
    let alerts = self.get_critical_alerts();
    let status_text = if alerts.is_empty() {
      format!(
        "Error Rate: {:.2}% | Range: {:?}",
        self.get_error_rate(),
        self.time_range
      )
    } else {
      alerts.join(" | ")
    };

    let status_color = if alerts.is_empty() {
      Color::Green
    } else {
      Color::Yellow
    };

    let status_paragraph = Paragraph::new(status_text)
      .style(Style::default().fg(status_color))
      .alignment(Alignment::Center)
      .block(
        Block::default()
          .borders(Borders::TOP)
          .border_style(Style::default().fg(Color::DarkGray)),
      );

    f.render_widget(status_paragraph, chunks[1]);
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
          TimeRange::Last24Hours => TimeRange::All,
          TimeRange::Last7Days => TimeRange::All,
          TimeRange::All => TimeRange::Last5Min,
        };
      },
      KeyCode::Char('p') => {
        self.show_percentages = !self.show_percentages;
      },
      KeyCode::Char('c') => {
        self.show_counts = !self.show_counts;
      },
      KeyCode::Char('h') => {
        self.highlight_errors = !self.highlight_errors;
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: crossterm::event::MouseEvent) {}
}
