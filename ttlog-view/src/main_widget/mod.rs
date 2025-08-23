use crossterm::event::{KeyCode, KeyEvent, MouseEvent};

use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span, Text},
  widgets::{Block, BorderType, Borders, Paragraph, Tabs as T, Wrap},
  Frame,
};

use crate::widget::Widget;

pub struct MainWidget {
  pub id: usize,
  pub title: &'static str,
}

impl MainWidget {
  pub fn new() -> Self {
    Self { id: 0, title: "" }
  }

  pub fn render(&mut self, f: &mut Frame<'_>, b: &Block<'_>, area: Rect) {
    // Main block with rounded border

    f.render_widget(b, area);

    // Render stats inline on the top border
    let stats_text = Text::from(vec![Line::from(vec![
      Span::raw("~ "), // margin left
      Span::styled("Logs: 1234", Style::default().fg(Color::Green)),
      Span::raw("   "), // space between items
      Span::styled("Rate: 10/s", Style::default().fg(Color::Yellow)),
      Span::raw("   "),
      Span::styled("Uptime: 01:23:45", Style::default().fg(Color::Magenta)),
      Span::raw(" ~"),
    ])]);

    // Inline area: align with the top border inside the main block
    let inline_area = Rect {
      // i want it be align at the end or the line
      x: area.x + 1,
      y: area.y,
      width: area.width - 2,
      height: 1,
    };

    let paragraph = Paragraph::new(stats_text)
      .style(Style::default().bg(Color::Black))
      .alignment(Alignment::Right); // optional bg to match terminal
                                    //
    let title = Paragraph::new("~ TTLog Dashboard ~").style(Style::default().bg(Color::Black));

    f.render_widget(title, inline_area);
    f.render_widget(paragraph, inline_area);
  }
}
