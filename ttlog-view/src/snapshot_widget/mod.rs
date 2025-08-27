use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  widgets::{Block, BorderType, Borders, Cell, Row, Table},
  Frame,
};

use crate::{snapshot_read::SnapshotFile, widget::Widget};

pub struct SnapshotWidget {
  pub id: u8,
  pub title: &'static str,
  pub snapshots: Vec<SnapshotFile>,
  pub area: Option<Rect>,
  pub focused: bool,
  pub selected: usize,
  pub show_events: bool, // whether we are inside the popover
}

impl SnapshotWidget {
  pub fn new() -> Self {
    Self {
      id: 5,
      title: "~ Snapshots ~",
      snapshots: Vec::new(),
      area: None,
      focused: false,
      selected: 0,
      show_events: false,
    }
  }
}

impl Widget for SnapshotWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);

    if self.show_events {
      self.render_event_popover(f, area);
    } else {
      self.render_snapshot_table(f, area);
    }
  }

  fn on_key(&mut self, key: KeyEvent) {
    if !self.focused {
      return;
    }

    match key.code {
      KeyCode::Up => {
        if self.selected > 0 {
          self.selected -= 1;
        }
      },
      KeyCode::Down => {
        if self.selected + 1 < self.snapshots.len() {
          self.selected += 1;
        }
      },
      KeyCode::Enter => {
        self.show_events = true; // open popover
      },
      KeyCode::Esc => {
        self.show_events = false; // close popover
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: MouseEvent) {}
}

impl SnapshotWidget {
  fn render_snapshot_table(&self, f: &mut Frame<'_>, area: Rect) {
    let block = Block::default()
      .title(self.title)
      .border_type(BorderType::Rounded)
      .borders(Borders::ALL)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    let rows: Vec<Row> = self
      .snapshots
      .iter()
      .enumerate()
      .map(|(i, snap)| {
        Row::new(vec![
          Cell::from(snap.name.clone()),
          Cell::from(snap.path.clone()),
          Cell::from(snap.create_at.clone()),
          Cell::from(format!("{}", snap.data.events.len())),
        ])
        .style(if i == self.selected {
          Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
        } else {
          Style::default()
        })
      })
      .collect();

    let table = Table::new(
      rows,
      vec![
        Constraint::Length(30), // name
        Constraint::Length(50), // path
        Constraint::Length(20), // created_at
        Constraint::Length(10), // event count
      ],
    )
    .block(block)
    .column_spacing(2);

    f.render_widget(table, area);
  }

  fn render_event_popover(&self, f: &mut Frame<'_>, area: Rect) {
    let snapshot = &self.snapshots[self.selected];
    let block = Block::default()
      .title(format!("Events in {}", snapshot.name))
      .border_type(BorderType::Double)
      .borders(Borders::ALL)
      .border_style(Style::default().fg(Color::Magenta));

    let rows: Vec<Row> = snapshot
      .data
      .events
      .iter()
      .map(|event| {
        Row::new(vec![
          Cell::from(event.message.clone()),
          Cell::from(event.target.clone()),
          Cell::from(format!("{}:{}", event.file, event.position.0)),
        ])
      })
      .collect();

    let table = Table::new(
      rows,
      vec![
        Constraint::Percentage(50),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
      ],
    )
    .block(block)
    .column_spacing(2);

    // Make the popover smaller (centered)
    let popup_area = centered_rect(80, 60, area);

    f.render_widget(table, popup_area);
  }
}

/// Utility: center a rect inside another
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
