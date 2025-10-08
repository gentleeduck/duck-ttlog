use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
  layout::{Constraint, Direction, Layout, Rect},
  style::{Color, Modifier, Style},
  text::{Line, Span},
  widgets::{Block, BorderType, Borders, Paragraph},
  Frame,
};

use crate::{logs::LogsInfo, widget::Widget};

pub struct SystemInfoWidget<'a> {
  pub id: u8,
  pub title: &'static str,
  pub area: Option<Rect>,
  pub focused: bool,
  pub logs_info: &'a LogsInfo,
}

impl<'a> SystemInfoWidget<'a> {
  pub fn new(logs_info: &'a LogsInfo) -> Self {
    Self {
      id: 4,
      title: "~ System Info ~",
      area: None,
      focused: false,
      logs_info,
    }
  }

  fn create_column_sections(&self) -> (Vec<Line<'a>>, Vec<Line<'a>>, Vec<Line<'a>>) {
    let mut col1 = Vec::<Line<'a>>::new();
    let mut col2 = Vec::<Line<'a>>::new();
    let mut col3 = Vec::<Line<'a>>::new();

    // Column 1: Directory and Overall Stats
    col1.push(Line::from(vec![Span::styled(
      "Directory: ",
      Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD),
    )]));
    col1.push(Line::from(Span::raw(self.logs_info.directory_path.clone())));
    col1.push(Line::from(""));

    col1.push(Line::from(vec![Span::styled(
      "Overall Statistics",
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    )]));
    col1.push(Line::from(vec![
      Span::styled("Files:  ", Style::default().fg(Color::Green)),
      Span::raw(format!("{}", self.logs_info.total_files)),
    ]));
    col1.push(Line::from(vec![
      Span::styled("Size:   ", Style::default().fg(Color::Green)),
      Span::styled(
        &self.logs_info.total_size_formatted,
        Style::default().fg(Color::Magenta),
      ),
    ]));

    // Column 2: Binary Files
    col2.push(Line::from(vec![Span::styled(
      "Binary Files (.bin)",
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    )]));
    col2.push(Line::from(vec![
      Span::styled("Count:    ", Style::default().fg(Color::Green)),
      Span::raw(format!("{}", self.logs_info.bin_files.count)),
    ]));
    col2.push(Line::from(vec![
      Span::styled("Size:     ", Style::default().fg(Color::Green)),
      Span::styled(
        &self.logs_info.bin_files.total_size_formatted,
        Style::default().fg(Color::Magenta),
      ),
    ]));

    if self.logs_info.bin_files.count > 0 {
      if let Some(min_formatted) = &self.logs_info.bin_files.min_size_formatted {
        col2.push(Line::from(vec![
          Span::styled("Min:      ", Style::default().fg(Color::Green)),
          Span::styled(min_formatted, Style::default().fg(Color::Blue)),
        ]));
      }

      if let Some(max_formatted) = &self.logs_info.bin_files.max_size_formatted {
        col2.push(Line::from(vec![
          Span::styled("Max:      ", Style::default().fg(Color::Green)),
          Span::styled(max_formatted, Style::default().fg(Color::Red)),
        ]));
      }

      col2.push(Line::from(vec![
        Span::styled("Avg:      ", Style::default().fg(Color::Green)),
        Span::styled(
          &self.logs_info.bin_files.avg_size_formatted,
          Style::default().fg(Color::Cyan),
        ),
      ]));
    }

    // Column 3: Log Files
    col3.push(Line::from(vec![Span::styled(
      "Log Files (.log)",
      Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD),
    )]));
    col3.push(Line::from(vec![
      Span::styled("Count:    ", Style::default().fg(Color::Green)),
      Span::raw(format!("{}", self.logs_info.log_files.count)),
    ]));
    col3.push(Line::from(vec![
      Span::styled("Size:     ", Style::default().fg(Color::Green)),
      Span::styled(
        &self.logs_info.log_files.total_size_formatted,
        Style::default().fg(Color::Magenta),
      ),
    ]));

    // List log files if any (in column 3)
    if !self.logs_info.log_files.files.is_empty() {
      col3.push(Line::from(""));
      for file in &self.logs_info.log_files.files {
        col3.push(Line::from(vec![
          Span::styled("• ", Style::default().fg(Color::DarkGray)),
          Span::styled(&file.name, Style::default().fg(Color::White)),
        ]));
        col3.push(Line::from(vec![
          Span::styled("  ", Style::default()),
          Span::styled(&file.size_formatted, Style::default().fg(Color::Cyan)),
        ]));
      }
    }

    (col1, col2, col3)
  }
}

impl<'a> Widget for SystemInfoWidget<'a> {
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

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Create three-column layout
    let columns = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(34),
      ])
      .split(inner_area);

    // Get the column content
    let (col1_lines, col2_lines, col3_lines) = self.create_column_sections();

    // Render each column
    let col1_paragraph = Paragraph::new(col1_lines).style(Style::default().fg(Color::White));
    f.render_widget(col1_paragraph, columns[0]);

    let col2_paragraph = Paragraph::new(col2_lines).style(Style::default().fg(Color::White));
    f.render_widget(col2_paragraph, columns[1]);

    let col3_paragraph = Paragraph::new(col3_lines).style(Style::default().fg(Color::White));
    f.render_widget(col3_paragraph, columns[2]);
  }

  fn on_key(&mut self, key: KeyEvent) {
    if !self.focused {
      return;
    }
    match key.code {
      _ => {},
    }
  }

  fn on_mouse(&mut self, _me: MouseEvent) {}
}
