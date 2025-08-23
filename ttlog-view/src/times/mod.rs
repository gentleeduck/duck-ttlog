use chrono::Duration;

pub fn format_duration(duration: Duration) -> (i64, i64, i64, i64) {
  let total_seconds = duration.num_seconds();

  let days = total_seconds / 86_400; // 24 * 60 * 60
  let hours = (total_seconds % 86_400) / 3600;
  let minutes = (total_seconds % 3600) / 60;
  let seconds = total_seconds % 60;

  (days, hours, minutes, seconds)
}
