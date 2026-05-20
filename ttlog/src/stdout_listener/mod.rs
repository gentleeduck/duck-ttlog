use chrono::{DateTime, TimeZone, Utc};
use std::io::{self, Write};

use crate::event::LogEvent;
use crate::listener::LogListener;
use crate::string_interner::StringInterner;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";

pub struct StdoutListener {
  buffer: std::sync::Mutex<String>,
}

impl StdoutListener {
  pub fn new() -> Self {
    Self {
      buffer: std::sync::Mutex::new(String::with_capacity(256)),
    }
  }
}

impl Default for StdoutListener {
  fn default() -> Self {
    Self::new()
  }
}

impl LogListener for StdoutListener {
  fn handle(&self, event: &LogEvent, interner: &StringInterner) {
    if let Ok(mut buf) = self.buffer.try_lock() {
      buf.clear();

      let target: String = {
        let this = interner.get_target(event.target_id).map(|t| t.to_string());
        match this {
          Some(x) => x,
          None => "".to_string(),
        }
      };

      let message: String = match event.message_id {
        Some(id) => {
          let this = interner.get_message(id.get()).map(|arc| arc.to_string());
          match this {
            Some(x) => x,
            None => "".to_string(),
          }
        },
        None => "".to_string(),
      };

      let kv: String = match event.kv_id {
        Some(kv_id) => match interner.get_kv(kv_id.get()) {
          Some(kv_data) => match std::str::from_utf8(kv_data.as_slice()) {
            Ok(s) => s.to_string(),
            Err(_) => "".to_string(),
          },
          None => "".to_string(),
        },
        None => "".to_string(),
      };

      let (line, col) = event.position;

      let ts_ms = event.timestamps();
      let level = event.level();
      let thread_id = event.thread_id();

      let datetime: DateTime<Utc> =
        DateTime::from_timestamp((ts_ms / 1000) as i64, ((ts_ms % 1000) * 1_000_000) as u32)
          .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());

      // Strip terminal control characters from every attacker-influenced
      // field so a crafted log payload cannot inject ANSI escapes (SEC-004).
      let target = sanitize_for_terminal(&target);
      let message = sanitize_for_terminal(&message);
      let kv = sanitize_for_terminal(&kv);

      let level_colored = color_level(level.as_str());
      let target_colored = format!("{}{}{}", MAGENTA, target, RESET);
      let msg_colored = format!("{}{}{}", WHITE, message, RESET);
      let kv_colored = format!("{}{}{}", BLUE, kv, RESET);

      use std::fmt::Write;
      let _ = writeln!(
        buf,
        "{time_color}[{time}]{reset} {level} {thread_color}t{tid}{reset} {target}:{line}:{col} {msg} {kv}",
        time_color = GREEN,
        reset = RESET,
        level = level_colored,
        thread_color = CYAN,
        tid = thread_id,
        time = datetime.format("%H:%M:%S%.3f"),
        target = target_colored,
        line = line,
        col = col,
        msg = msg_colored,
        kv = kv_colored
      );

      let _ = io::stdout().write_all(buf.as_bytes());
    }
  }
}

/// Strips terminal control characters from untrusted log data before it is
/// written to stdout interleaved with the listener's own ANSI color codes.
///
/// An attacker-supplied log message containing escape sequences (e.g.
/// `\x1b[2J\x1b[H`) could clear the operator's terminal or forge log lines.
/// We drop every control code point below `0x20` (except `\t`) and `\x7f`
/// (DEL); normal printable Unicode is left intact.
///
/// We additionally drop the C1 control range `U+0080..=U+009F` — some
/// terminals in 8-bit mode treat these as escape introducers (`U+009B` CSI,
/// `U+009D` OSC) — and the Unicode line/paragraph separators `U+2028` /
/// `U+2029`, which separator-aware log viewers may render as new lines and
/// so allow forged log entries (SEC-019).
///
/// We further drop Unicode bidirectional formatting characters — the
/// overrides/embeddings `U+202A..=U+202E` and the isolates `U+2066..=U+2069`
/// — and the byte-order mark `U+FEFF` (SEC-023). BIDI controls let an
/// attacker visually reorder a log line so the rendered text differs from the
/// logical bytes (log-line spoofing in BIDI-aware viewers); the BOM can be
/// abused as an invisible zero-width character.
fn sanitize_for_terminal(s: &str) -> String {
  s.chars()
    .filter(|c| {
      let cp = *c as u32;
      *c == '\t'
        || (cp >= 0x20
          && cp != 0x7f
          && !(0x80..=0x9f).contains(&cp)
          && *c != '\u{2028}'
          && *c != '\u{2029}'
          && !('\u{202A}'..='\u{202E}').contains(c)
          && !('\u{2066}'..='\u{2069}').contains(c)
          && *c != '\u{FEFF}')
    })
    .collect()
}

fn color_level(level: &str) -> String {
  match level {
    "ERROR" => format!("{}[{}]{}", RED, level, RESET),
    "WARN" => format!("{}[{}]{}", YELLOW, level, RESET),
    "INFO" => format!("{}[{}]{}", GREEN, level, RESET),
    "DEBUG" => format!("{}[{}]{}", BLUE, level, RESET),
    "TRACE" => format!("{}[{}]{}", CYAN, level, RESET),
    "FATAL" => format!("{}[{}]{}", RED, level, RESET),
    _ => level.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::sanitize_for_terminal;

  #[test]
  fn strips_ansi_escape_injection() {
    // A crafted log message that clears the screen and forges a log line.
    let malicious = "\x1b[2J\x1b[Hfake [INFO] root: admin login";
    let cleaned = sanitize_for_terminal(malicious);
    assert!(
      !cleaned.contains('\x1b'),
      "sanitized output must not contain ESC: {:?}",
      cleaned
    );
    // The printable remainder survives intact.
    assert!(cleaned.contains("fake [INFO] root: admin login"));
  }

  #[test]
  fn strips_c1_controls_and_unicode_separators() {
    // NEL (U+0085), CSI (U+009B) are C1 controls; U+2028 is a line separator.
    let malicious = "before\u{0085}mid\u{009b}2Jafter\u{2028}forged [INFO] root";
    let cleaned = sanitize_for_terminal(malicious);
    assert!(
      !cleaned.contains('\u{0085}'),
      "NEL must be stripped: {:?}",
      cleaned
    );
    assert!(
      !cleaned.contains('\u{009b}'),
      "CSI must be stripped: {:?}",
      cleaned
    );
    assert!(
      !cleaned.contains('\u{2028}'),
      "line separator must be stripped: {:?}",
      cleaned
    );
    assert_eq!(cleaned, "beforemid2Jafterforged [INFO] root");
  }

  #[test]
  fn strips_bidi_overrides_and_bom() {
    // SEC-023: U+202E (RTL override) can visually reverse text; U+2066/2069
    // are isolates; U+FEFF is the BOM (invisible zero-width). A crafted log
    // payload uses these to spoof what the operator sees.
    let malicious = "user\u{202E}drowssap\u{202C} \u{2066}isolated\u{2069}\u{FEFF}admin login";
    let cleaned = sanitize_for_terminal(malicious);
    assert!(
      !cleaned.contains('\u{202E}'),
      "RTL override must be stripped: {:?}",
      cleaned
    );
    assert!(
      !cleaned.contains('\u{202C}'),
      "pop-directional-formatting must be stripped: {:?}",
      cleaned
    );
    assert!(
      !cleaned.contains('\u{2066}') && !cleaned.contains('\u{2069}'),
      "BIDI isolates must be stripped: {:?}",
      cleaned
    );
    assert!(
      !cleaned.contains('\u{FEFF}'),
      "BOM must be stripped: {:?}",
      cleaned
    );
    assert_eq!(cleaned, "userdrowssap isolatedadmin login");
  }

  #[test]
  fn keeps_tab_and_printable_unicode_drops_other_controls() {
    let input = "col1\tcol2\nnext\r\u{7f}\u{0}emoji-😀";
    let cleaned = sanitize_for_terminal(input);
    assert_eq!(cleaned, "col1\tcol2nextemoji-😀");
  }
}
