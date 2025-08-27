use crossterm::event::{KeyEvent, MouseEvent};

use ratatui::{
  layout::Rect,
  style::{Color, Style},
  symbols,
  text::Span,
  widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType},
  Frame,
};

use crate::widget::Widget;

pub struct EventsGraphWidget {
  pub id: u8,
  pub title: &'static str,
  pub events_per_sec: usize,
  pub snapshots: Vec<(f64, f64)>, // (time, events/sec)
  pub area: Option<Rect>,
  pub focused: bool,
  pub tick: f64,
}

impl EventsGraphWidget {
  pub fn new() -> Self {
    Self {
      id: 3,
      title: "~ Events Graph ~",
      events_per_sec: 0,
      snapshots: vec![],
      area: None,
      focused: false,
      tick: 0.0,
    }
  }

  /// call this in your app tick loop
  pub fn on_tick(&mut self, events_per_sec: usize) {
    // update with externally provided events/sec
    self.events_per_sec = events_per_sec;

    // push snapshot (tick, events/sec)
    self.snapshots.push((self.tick, self.events_per_sec as f64));
    self.tick += 1.0;

    // keep ~120 points (2 minutes window)
    if self.snapshots.len() > 120 {
      self.snapshots.remove(0);
    }
  }
}

impl Widget for EventsGraphWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);

    let block = Block::default()
      .title(self.title)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    // X axis bounds and labels (time window)
    let (x_min, x_max) =
      if let (Some(first), Some(last)) = (self.snapshots.first(), self.snapshots.last()) {
        (first.0, last.0.max(60.0)) // min 60s window
      } else {
        (0.0, 60.0)
      };

    let mid_x = (x_min + x_max) / 2.0;
    let x_labels = vec![
      Span::raw(format!("{:.0}s", x_min)),
      Span::raw(format!("{:.0}s", mid_x)),
      Span::raw(format!("{:.0}s", x_max)),
    ];

    // Y axis bounds and labels (scale to current max)
    let y_max = self
      .snapshots
      .iter()
      .map(|(_, v)| *v)
      .fold(1000000.0, f64::max) // at least 50
      .ceil();

    // Clamp all data points to y_max
    let clamped_data: Vec<(f64, f64)> = self
      .snapshots
      .iter()
      .map(|(x, y)| (*x, y.min(y_max)))
      .collect();

    let y_labels = vec![
      Span::raw("0"),
      Span::raw(format!("{:.0}", y_max / 2.0)),
      Span::raw(format!("{:.0}", y_max)),
    ];

    let dataset = Dataset::default()
      .marker(symbols::Marker::Braille)
      .style(Style::default().fg(Color::Yellow))
      .graph_type(GraphType::Line)
      .data(&clamped_data);

    let chart = Chart::new(vec![dataset])
      .block(block)
      .x_axis(
        Axis::default()
          .title("Time (s)")
          .style(Style::default().fg(Color::Gray))
          .labels(x_labels)
          .bounds([x_min, x_max]),
      )
      .y_axis(
        Axis::default()
          .title("Events/sec")
          .style(Style::default().fg(Color::Gray))
          .labels(y_labels)
          .bounds([0.0, y_max]),
      );

    f.render_widget(chart, area);
  }

  fn on_key(&mut self, _key: KeyEvent) {
    // optional: controls here
  }

  fn on_mouse(&mut self, _me: MouseEvent) {}
}
