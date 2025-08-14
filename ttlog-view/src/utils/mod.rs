use std::error::Error;

use colored::*;
use figlet_rs::FIGfont;

use crate::snapshot_read::SnapShot;

use chrono::{Datelike, NaiveDateTime, Timelike};

pub fn format_timestamp(ts_str: &str) -> String {
  // Parse from string with format, avoiding deprecated functions
  let parsed = NaiveDateTime::parse_and_remainder(ts_str, "%Y%m%d%H%M%S")
    .map(|(dt, _)| dt)
    .unwrap_or_else(|_| NaiveDateTime::UNIX_EPOCH);

  format!(
    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
    parsed.year(),
    parsed.month(),
    parsed.day(),
    parsed.hour(),
    parsed.minute(),
    parsed.second()
  )
}

pub fn print_snapshots(snapshots: &[SnapShot]) {
  for snap in snapshots {
    println!(
      "{} {} {}",
      "📦".bright_yellow(),
      snap.name.bright_white().bold(),
      format!("({})", format_timestamp(&snap.create_at)).dimmed()
    );
    println!("    {} {}", "📄".bright_cyan(), snap.path.dimmed());

    for event in &snap.data {
      let icon = match event.level.as_str() {
        "INFO" => "ℹ️".bright_blue(),
        "WARN" => "⚠️".bright_yellow(),
        "ERROR" => "❌".bright_red(),
        _ => "•".bright_white(),
      };

      println!(
        "    {} [{}] {}",
        icon,
        event.level.color(match event.level.as_str() {
          "INFO" => "blue",
          "WARN" => "yellow",
          "ERROR" => "red",
          _ => "white",
        }),
        event.message
      );
    }
    println!();
  }
}

pub fn generate_ascii_art(text: &str) -> Result<String, Box<dyn Error>> {
  // Load the ANSI Shadow font file (must be in your project folder or give absolute path)
  let font = FIGfont::from_file("fonts/ANSI Shadow.flf")?;

  let figure = font
    .convert(text)
    .ok_or("Failed to convert text to ASCII art")?;

  Ok(figure.to_string())
}
