mod __test__;

use serde::{Deserialize, Serialize};

/// Defines the severity or importance level of an event.
///
/// This enum can be used to categorize events based on their significance.
/// The levels are commonly ordered from the most detailed to the most severe:
/// `Trace < Debug < Info < Warn < Error`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Level {
  /// Very detailed information, mostly useful for debugging
  Trace,
  /// Debug-level information, used for development or troubleshooting
  Debug,
  /// General informational messages, typically useful in production
  Info,
  /// Warning messages that indicate potential issues
  Warn,
  /// Error messages that indicate a failure or critical problem
  Error,
}

/// Represents an event with a timestamp, severity level, message, and target.
///
/// `Event` is a versatile structure that can be serialized and deserialized,
/// making it suitable for storage, transmission, and analysis in systems
/// where event tracking or monitoring is required.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
  /// The timestamp of the event, in milliseconds since the Unix epoch (January 1, 1970).
  ///
  /// This field allows events to be ordered chronologically or correlated
  /// with other events in a system.
  pub timestamp: u64,

  /// The severity level of the event as a string (e.g., `"INFO"`, `"WARN"`).
  ///
  /// This field indicates the importance of the event. While a `Level` enum
  /// exists, storing it as a string allows for compatibility with external
  /// systems or logging frameworks.
  pub level: String,

  /// The textual content or description of the event.
  ///
  /// This field should contain meaningful information describing what
  /// happened, why, or any other contextual data useful for analysis.
  pub message: String,

  /// The source or target of the event.
  ///
  /// This can represent a module name, component identifier, or system
  /// element associated with the event. Helps in filtering and routing events.
  pub target: String,
}

impl Event {
  /// Creates a new `Event` with the provided timestamp, level, message, and target.
  ///
  /// # Arguments
  /// * `timestamp` - Milliseconds since the Unix epoch representing the event time.
  /// * `level` - String indicating the severity of the event.
  /// * `message` - Description or content of the event.
  /// * `target` - Source or target system/component of the event.
  ///
  /// # Returns
  /// A new `Event` instance populated with the provided values.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::Event;
  /// let event = Event::new(
  ///     1_692_105_600_000u64,
  ///     "INFO".to_string(),
  ///     "User logged in".to_string(),
  ///     "auth_module".to_string(),
  /// );
  /// assert_eq!(event.level, "INFO");
  /// ```
  pub fn new(timestamp: u64, level: String, message: String, target: String) -> Self {
    Self {
      timestamp,
      level,
      message,
      target,
    }
  }

  /// Serializes the `Event` into a JSON string.
  ///
  /// Useful for storing, sending, or logging events in a standard format.
  ///
  /// # Panics
  /// This function will panic if serialization fails. In production systems,
  /// ensure the data can be serialized or handle errors using a custom wrapper.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::Event;
  /// let event = Event::new(123, "INFO".to_string(), "Hello".to_string(), "main".to_string());
  /// let json = event.serialize();
  /// assert!(json.contains("\"timestamp\":123"));
  /// ```
  pub fn serialize(&self) -> String {
    serde_json::to_string(self).expect("Failed to serialize")
  }

  /// Deserializes a JSON string into an `Event`.
  ///
  /// # Arguments
  /// * `json` - JSON string representing an `Event`.
  ///
  /// # Panics
  /// This function will panic if deserialization fails due to invalid JSON
  /// or mismatched structure.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::Event;
  /// let json = r#"{"timestamp":123,"level":"INFO","message":"Hello","target":"main"}"#.to_string();
  /// let event = Event::deserialize(json);
  /// assert_eq!(event.level, "INFO");
  /// ```
  pub fn deserialize(json: String) -> Self {
    serde_json::from_str::<Self>(&json).expect("Failed to deserialize")
  }
}

impl Default for Event {
  /// Returns a default `Event` with zero timestamp and empty strings.
  ///
  /// Useful for creating placeholder events or initializing structures
  /// before populating them with real data.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::Event;
  /// let default_event = Event::default();
  /// assert_eq!(default_event.timestamp, 0);
  /// assert_eq!(default_event.level, "");
  /// ```
  fn default() -> Self {
    Self {
      timestamp: 0,
      level: "".to_string(),
      message: String::new(),
      target: String::new(),
    }
  }
}

impl std::fmt::Display for Event {
  /// Formats the event as a JSON string.
  ///
  /// Allows `Event` instances to be printed directly using `println!` or
  /// included in other formatted strings.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::Event;
  /// let event = Event::default();
  /// let s = format!("{}", event);
  /// assert!(s.contains("\"timestamp\":0"));
  /// ```
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.serialize())
  }
}
