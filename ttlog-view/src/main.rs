mod snapshot_read;
mod utils;

use crate::snapshot_read::SnapShot;
use crate::utils::{generate_ascii_art, print_snapshots};
use colored::*;
use inquire::Select;
use std::error::Error;
use std::fs;

fn main() -> Result<(), Box<dyn Error>> {
  // Clear screen
  print!("\x1B[2J\x1B[1;1H");

  // ASCII banner
  let banner = generate_ascii_art("TTLOG")?;
  println!("{}", banner.bright_yellow().bold());

  loop {
    // Main menu
    let choice = Select::new(
      "Main Menu - Select an action:",
      vec!["Show All Files", "Preview All Files", "Exit"],
    )
    .prompt()?;

    match choice {
      "Show All Files" => show_all_files_menu()?,
      "Preview All Files" => preview_all_files()?,
      "Exit" => {
        println!("{}", "Goodbye!".red().bold());
        break;
      },
      _ => unreachable!(),
    }
  }

  Ok(())
}

fn show_all_files_menu() -> Result<(), Box<dyn Error>> {
  let files = snapshot_read::read_snapshots()?;
  if files.is_empty() {
    println!("{}", "No log files found.".red());
    return Ok(());
  }

  loop {
    let mut options: Vec<String> = files
      .iter()
      .map(|f| f.name.clone()) // Convert PathBuf to String
      .collect();

    options.push("Back".to_string());

    let choice = Select::new("Select a file to manage:", options).prompt()?;

    if choice == "Back" {
      break;
    }

    file_action_menu(&choice, &files)?;
  }

  Ok(())
}

fn file_action_menu(file: &str, snapshots: &Vec<SnapShot>) -> Result<(), Box<dyn Error>> {
  loop {
    match Select::new(
      &format!("File: {} - Choose an action:", file),
      vec!["Preview", "Delete", "Back"],
    )
    .prompt()?
    {
      "Preview" => preview_file(file, snapshots)?,
      "Delete" => {
        fs::remove_file(format!("/tmp/{}.bin", file))?;
        println!("{}", format!("Deleted file: {}", file).red());
        break; // exit after deletion
      },
      "Back" => break,
      _ => unreachable!(),
    }
  }
  Ok(())
}

fn preview_file(file: &str, snapshots: &Vec<SnapShot>) -> Result<(), Box<dyn Error>> {
  // Find snapshot by path
  if let Some(snapshot) = snapshots.iter().find(|s| s.name == file) {
    println!("=== Preview: {} ===", snapshot.name);

    // Instead of raw debug output, reuse your old rendering logic
    print_snapshots(&vec![snapshot.clone()]);
    // Or if you had a function like render_snapshot(snapshot), call that:
    // render_snapshot(snapshot);
  } else {
    println!("File '{}' not found in snapshots.", file);
  }

  Ok(())
}

fn preview_all_files() -> Result<(), Box<dyn Error>> {
  let snapshots = snapshot_read::read_snapshots()?;
  print_snapshots(&snapshots);
  Ok(())
}
