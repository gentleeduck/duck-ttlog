use ratatui::style::Color;
use ttlog::event::LogLevel;

use chrono::{DateTime, Datelike, NaiveDateTime, TimeZone, Timelike, Utc};

pub struct Utils;

impl Utils {
  pub fn level_name(level: LogLevel) -> &'static str {
    match level {
      LogLevel::FATAL => "FATAL",
      LogLevel::ERROR => "ERROR",
      LogLevel::WARN => "WARN",
      LogLevel::INFO => "INFO",
      LogLevel::DEBUG => "DEBUG",
      LogLevel::TRACE => "TRACE",
    }
  }

  pub fn level_color(level: LogLevel) -> Color {
    match level {
      LogLevel::FATAL => Color::Red,
      LogLevel::ERROR => Color::Magenta,
      LogLevel::WARN => Color::Yellow,
      LogLevel::INFO => Color::Green,
      LogLevel::DEBUG => Color::Cyan,
      LogLevel::TRACE => Color::Gray,
    }
  }

  pub fn format_timestamp(ms: u64) -> String {
    let secs = (ms / 1000) as i64;
    let sub_ms = (ms % 1000) as u32;
    let dt: DateTime<Utc> = Utc
      .timestamp_opt(secs, sub_ms * 1_000_000)
      .single()
      .unwrap_or_else(|| Utc.timestamp_opt(0, 0).earliest().unwrap());
    dt.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
  }

  pub fn format_timestamp_from_string(ts_str: &str) -> String {
    let parsed = NaiveDateTime::parse_from_str(ts_str, "%Y%m%d%H%M%S")
      .map(|dt| dt)
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

  pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
      format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
      format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
      format!("{:.2} KB", size as f64 / KB as f64)
    } else {
      format!("{} bytes", size)
    }
  }
}

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
//     let (icon, level, pid) = LogEvent::unpack_meta(event.packed_meta);
//     let level = LogLevel::from_u8_to_str(&level);
//     all_lines.push(format!(
//       "{} [{}] {}",
//       icon_for_level(&level),
//       level,
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
//     let (ts, level, pid) = LogEvent::unpack_meta(event.packed_meta);
//     let level = LogLevel::from_u8_to_str(&level);
//     let icon = icon_for_level(&level);
//
//     let level_colored = match level {
//       "TRACE" => level.bright_magenta().bold(),
//       "DEBUG" => level.bright_cyan().bold(),
//       "INFO" => level.bright_blue().bold(),
//       "WARN" => level.bright_yellow().bold(),
//       "ERROR" => level.bright_red().bold(),
//       _ => level.bright_white().bold(),
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
//     "TRACE" => "🔎".bright_magenta(),
//     "INFO" => "ℹ️".bright_blue(),
//     "DEBUG" => "🔍".bright_cyan(),
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
