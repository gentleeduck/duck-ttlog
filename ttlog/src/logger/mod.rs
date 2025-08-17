use crate::event::{LogEvent, LogLevel};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU8, Ordering};

pub trait Logger: Send + Sync {
  fn emit(&self, event: LogEvent);
  fn flush(&self);
  fn set_level(&self, level: LogLevel);
}

/// Wrapper to store in AtomicPtr
struct LoggerHolder(&'static dyn Logger);

/// Atomic pointer to allow swapping the logger safely
static LOGGER_PTR: AtomicPtr<LoggerHolder> = AtomicPtr::new(ptr::null_mut());

/// Cached reference for hot path
static mut LOGGER_CACHE: Option<&'static dyn Logger> = None;

/// Global log level
static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

/// Set the global logger
pub fn set_logger(logger: &'static dyn Logger) {
  let new_holder = Box::into_raw(Box::new(LoggerHolder(logger)));

  // Swap atomically and drop old holder
  let old = LOGGER_PTR.swap(new_holder, Ordering::Release);
  if !old.is_null() {
    unsafe {
      drop(Box::from_raw(old));
    }
  }

  // Update cache for fast-path emission
  unsafe {
    LOGGER_CACHE = Some(logger);
  }
}

/// Fast-path emission (zero-cost after logger is set)
#[inline]
pub fn emit_fast(event: LogEvent) {
  unsafe {
    if let Some(logger) = LOGGER_CACHE {
      logger.emit(event);
    }
  }
}

/// Check if a level is enabled
#[inline]
pub fn is_enabled(level: LogLevel, _target: &str) -> bool {
  let current_level = LOG_LEVEL.load(Ordering::Relaxed);
  (level as u8) >= current_level
}

/// Set the global log level
pub fn set_level(level: LogLevel) {
  LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

/// High-resolution timestamp
#[inline]
pub fn now_nanos() -> u64 {
  use std::time::{SystemTime, UNIX_EPOCH};
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos() as u64
}
