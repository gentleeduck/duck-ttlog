// use std::error::Error;
//
// use colored::*;
// use figlet_rs::FIGfont;
//
// use crate::snapshot_read::SnapshotFile;
//
// use chrono::{Datelike, NaiveDateTime, Timelike};
//
// pub fn format_timestamp(ts_str: &str) -> String {
//   // Parse from string with format, avoiding deprecated functions
//   let parsed = NaiveDateTime::parse_and_remainder(ts_str, "%Y%m%d%H%M%S")
//     .map(|(dt, _)| dt)
//     .unwrap_or_else(|_| NaiveDateTime::UNIX_EPOCH);
//
//   format!(
//     "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
//     parsed.year(),
//     parsed.month(),
//     parsed.day(),
//     parsed.hour(),
//     parsed.minute(),
//     parsed.second()
//   )
// }
//
// pub fn print_snapshots(snapshots: &[SnapshotFile]) {
//   if snapshots.is_empty() {
//     println!("{}", "No snapshots to display.".red());
//     return;
//   }
//
//   for snap in snapshots {
//     print_snapshot(snap);
//   }
// }
// fn print_snapshot(snap: &SnapshotFile) {
//   // Calculate max width for borders
//   let mut all_lines: Vec<String> = Vec::new();
//   all_lines.push(format!(
//     "📦 {} ({})",
//     snap.name,
//     format_timestamp(&snap.create_at)
//   ));
//   all_lines.push(format!("📄 {}", snap.path));
//   for event in &snap.data.events {
//     all_lines.push(format!(
//       "{} [{}] {}",
//       icon_for_level(&event.level),
//       event.level,
//       event.message
//     ));
//   }
//
//   let max_width = all_lines
//     .iter()
//     .map(|l| strip_ansi_codes(l).chars().count())
//     .max()
//     .unwrap_or(0);
//
//   // Top border
//   println!(
//     "{}",
//     format!("╔{}╗", "═".repeat(max_width + 3)).bright_black()
//   );
//
//   // Header
//   let header = format!(
//     "📦 {} ({})",
//     snap.name.bright_white().bold(),
//     format_timestamp(&snap.create_at).dimmed()
//   );
//   println!("{}", bordered_line(&header, max_width));
//
//   let path_line = format!("📄 {}", snap.path.dimmed());
//   println!("{}", bordered_line(&path_line, max_width));
//
//   // Separator
//   println!(
//     "{}",
//     format!("╠{}╣", "═".repeat(max_width + 3)).bright_black()
//   );
//
//   // Events
//   for event in &snap.data.events {
//     let icon = icon_for_level(&event.level);
//     let level_colored = match event.level.as_str() {
//       "INFO" => event.level.bright_blue().bold(),
//       "WARN" => event.level.bright_yellow().bold(),
//       "ERROR" => event.level.bright_red().bold(),
//       _ => event.level.bright_white().bold(),
//     };
//     let line = format!("{} [{}] {}", icon, level_colored, event.message);
//     println!("{}", bordered_line(&line, max_width));
//   }
//
//   // Bottom border
//   println!(
//     "{}",
//     format!("╚{}╝", "═".repeat(max_width + 3)).bright_black()
//   );
//   println!();
// }
//
// fn bordered_line(content: &str, max_width: usize) -> String {
//   let stripped_len = strip_ansi_codes(content).chars().count() - 1;
//   format!(
//     "{} {}{} {}",
//     "║".bright_black(),
//     content,
//     " ".repeat(max_width - stripped_len),
//     "║".bright_black()
//   )
// }
//
// fn icon_for_level(level: &str) -> colored::ColoredString {
//   match level {
//     "INFO" => "ℹ️".bright_blue(),
//     "WARN" => "⚠️".bright_yellow(),
//     "ERROR" => "❌".bright_red(),
//     _ => "•".bright_white(),
//   }
// }
//
// /// Remove ANSI color codes for correct length measurement
// fn strip_ansi_codes(s: &str) -> String {
//   let re = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap();
//   re.replace_all(s, "").to_string()
// }
//
// pub fn generate_ascii_art(text: &str) -> Result<String, Box<dyn Error>> {
//   // Load the ANSI Shadow font file (must be in your project folder or give absolute path)
//   let font = FIGfont::from_file("fonts/ANSI Shadow.flf")?;
//
//   let figure = font
//     .convert(text)
//     .ok_or("Failed to convert text to ASCII art")?;
//
//   Ok(figure.to_string())
// }
