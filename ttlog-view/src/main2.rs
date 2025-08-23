mod main1;
use color_eyre::Result;
use crossterm::event::{self, Event};
use ratatui::widgets::Paragraph;
use ratatui::{DefaultTerminal, Frame};

fn main() -> Result<()> {
  color_eyre::install()?;
  let mut terminal = ratatui::init();
  let result = run(&mut terminal);
  ratatui::restore();
  result
}

fn run(terminal: &mut DefaultTerminal) -> Result<()> {
  loop {
    terminal.draw(render)?;
    if matches!(event::read()?, Event::Key(_)) {
      break Ok(());
    }
  }
}

fn render(frame: &mut Frame) {
  let text = Paragraph::new("FUCK YOU!");
  frame.render_widget(text, frame.area());
}
