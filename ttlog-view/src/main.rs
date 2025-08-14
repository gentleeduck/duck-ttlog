use crate::utils::{generate_ascii_art, print_snapshots};

mod snapshot_read;
mod utils;

// fn main() {
//   print_snapshots(&snapshots);
// }

use colored::*;
use inquire::Select;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
  // Clear screen
  print!("\x1B[2J\x1B[1;1H");

  // ASCII banner
  let banner = generate_ascii_art("TTLOG")?;
  println!("{}", banner.bright_yellow().bold());

  // Menu items
  let options = vec!["View log file", "View snapshot", "Filter logs", "Exit"];

  // Prompt
  let choice = Select::new("Select an action:", options.clone()).prompt()?;

  let snapshots = snapshot_read::read_snapshots()?;

  // Match choice
  match choice {
    "View log files" => {
      println!("{}", "Opening log file...".cyan().bold());
      println!("{:?}", snapshots);
    },
    "View snapshot" => {
      println!("{}", "Opening snapshot viewer...".cyan().bold());
      print_snapshots(&snapshots);
    },
    "Filter logs" => {
      println!("{}", "Filtering logs...".cyan().bold());
      // Filtering logic here
    },
    "Exit" => {
      println!("{}", "Goodbye!".red().bold());
      std::process::exit(0);
    },
    _ => unreachable!(), // Should never happen
  }

  Ok(())
}
