use crossterm::event::MouseEvent;
use ratatui::{
  layout::{Alignment, Rect},
  style::{Color, Style},
  text::Text,
  widgets::{Block, BorderType, Borders, Paragraph, Table},
  Frame,
};

use crate::{
  logs_widget::{LogsWidget, ViewState},
  widget::Widget,
};

impl<'a> Widget for LogsWidget<'a> {
  fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
    self.area = Some(area);
    // Temporarily move out the table state to avoid borrowing conflicts
    let mut table_state = std::mem::take(&mut self.table_state);

    // Compute filtered count and update local table_state selection
    let filtered_count = self.filtered_and_sorted_logs().len();
    if self.follow_tail && filtered_count > 0 {
      table_state.select(Some(filtered_count - 1));
    } else if filtered_count > 0 {
      table_state.select(Some(self.selected_row.min(filtered_count - 1)));
    } else {
      table_state.select(None);
    }

    // Build the main block and table
    let title_line = self.build_title_line();
    let block = Block::default()
      .title(title_line)
      .borders(Borders::ALL)
      .border_type(BorderType::Rounded)
      .border_style(if self.focused {
        Style::default().fg(Color::Cyan)
      } else {
        Style::default().fg(Color::White)
      });

    // Build table components
    let constraints = self.build_table_constraints();
    let header = self.build_table_header();
    let rows = self.build_table_rows();

    let table = Table::new(rows, &constraints)
      .header(header)
      .block(block)
      .highlight_style(if self.focused {
        Style::default().fg(Color::Black).bg(Color::Cyan)
      } else {
        Style::default().fg(Color::Black).bg(Color::Blue)
      });

    // Render main table with the local table_state
    f.render_stateful_widget(table, area, &mut table_state);

    // Put the table state back
    self.table_state = table_state;

    // Render control line
    let control_line = self.build_control_line();
    let control_paragraph = Paragraph::new(Text::from(control_line)).alignment(Alignment::Right);
    let control_area = Rect {
      x: area.x + (area.width / 2).saturating_sub(10),
      y: area.y,
      width: area.width / 2 + 10,
      height: 1,
    };
    f.render_widget(control_paragraph, control_area);

    // Render popups
    match self.view_state {
      ViewState::LogDetail => {
        self.render_dim_overlay(f, area);
        self.render_log_detail_popup(f, area);
      },
      ViewState::Help => {
        self.render_dim_overlay(f, area);
        self.render_help_popup(f, area);
      },
      _ => {},
    }
  }

  fn on_key(&mut self, key: crossterm::event::KeyEvent) {
    if !self.focused {
      return;
    }

    match self.view_state {
      ViewState::LogDetail => self.handle_log_detail_keys(key),
      ViewState::Help => self.handle_help_keys(key),
      ViewState::Search => self.handle_search_keys(key),
      ViewState::Normal => self.handle_normal_keys(key),
    }
  }

  fn on_mouse(&mut self, _event: MouseEvent) {
    // Mouse handling can be implemented here if needed
    // For now, we'll leave it empty but store the area for future use
  }
}
