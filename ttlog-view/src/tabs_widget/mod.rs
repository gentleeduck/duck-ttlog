use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

use ratatui::{
  layout::Rect,
  style::{Color, Modifier, Style},
  text::Line,
  widgets::{Block, BorderType, Borders, Tabs as T},
  Frame,
};

#[derive(Debug, Clone)]
pub enum ViewMode {
  Overview,
  Logs,
  Metrics,
  Alerts,
  Performance,
  Network,
}

impl ViewMode {
  fn as_str(&self) -> &str {
    match self {
      ViewMode::Overview => "Overview",
      ViewMode::Logs => "Logs",
      ViewMode::Metrics => "Metrics",
      ViewMode::Alerts => "Alerts",
      ViewMode::Performance => "Performance",
      ViewMode::Network => "Network",
    }
  }
}

use crate::widget::Widget;
pub struct ListWidget {
  pub id: usize,
  pub title: &'static str,
  pub items: [ViewMode; 2],
  pub selected: usize,
  pub area: Option<Rect>, // keep track of where we were rendered
}

impl ListWidget {
  pub fn new() -> Self {
    Self {
      id: 0,
      title: "~ Menu ~",
      items: [ViewMode::Overview, ViewMode::Logs],
      selected: 0,
      area: None,
    }
  }
}

impl Widget for ListWidget {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect, focused: bool) {
    let tab_titles: Vec<Line> = self
      .items
      .iter()
      .map(|t| Line::from(format!(" {} ", t.as_str())))
      .collect();

    // base style for unfocused
    let base_style = Style::default().fg(Color::DarkGray);

    // highlight style for selection (changes if focused)
    let highlight_style = if focused {
      Style::default()
        .fg(Color::Black)
        .bg(Color::Blue)
        .add_modifier(Modifier::BOLD)
    } else {
      Style::default().fg(Color::Gray).bg(Color::Reset)
    };

    // border style also changes depending on focus
    let block = Block::default()
      .borders(Borders::ALL)
      .title(self.title)
      .border_type(BorderType::Rounded)
      .border_style(if focused {
        Style::default().fg(Color::LightBlue)
      } else {
        Style::default().fg(Color::DarkGray)
      });

    let tabs = T::new(tab_titles)
      .block(block)
      .select(self.selected) // now controlled by on_key
      .style(base_style)
      .highlight_style(highlight_style);

    // self.width = tabs;
    f.render_widget(tabs, area);
  }

  fn on_key(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Tab => {
        self.selected = (self.selected + 1) % self.items.len();
      },
      KeyCode::BackTab => {
        self.selected = (self.selected + self.items.len() - 1) % self.items.len();
      },
      _ => {},
    }
  }

  fn on_mouse(&mut self, me: MouseEvent) {
    use crossterm::event::MouseEventKind;

    if let Some(area) = self.area {
      match me.kind {
        MouseEventKind::Down(_) => {
          // Tabs are drawn on one row (area.top())
          if me.row == area.top() {
            let mut x = area.left() + 1; // +1 for left border
            for (i, item) in self.items.iter().enumerate() {
              let tab_width = item.as_str().len() as u16 + 2; // <-- FIX HERE

              if me.column >= x && me.column < x + tab_width {
                self.selected = i;
                break;
              }

              x += tab_width;
            }
          }
        },
        _ => {},
      }
    }
  }
}
