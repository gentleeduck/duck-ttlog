use std::{
  ops::Deref,
  sync::{self, Arc},
  thread,
  time::Duration,
};
use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{debug, error, fatal, info, trace, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut trace = Trace::init(2, 64, "test", Some("./tmp"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
  trace.set_level(ttlog::event::LogLevel::TRACE);

  // Simulate application startup
  info!("Application starting...");
  trace!("Loading configuration files");
  debug!(config_path = "./config/app.toml", "Configuration loaded");

  // Simulate database connection pool initialization
  info!(
    pool_size = 10,
    max_connections = 50,
    "Initializing database connection pool"
  );
  for i in 0..10 {
    debug!(
      connection_id = i,
      host = "localhost",
      port = 5432,
      "Establishing database connection"
    );
    thread::sleep(Duration::from_millis(5));
  }
  info!(active_connections = 10, "Database pool ready");

  // Simulate Redis cache connection
  info!(
    host = "redis.local",
    port = 6379,
    "Connecting to Redis cache"
  );
  debug!(latency_ms = 2, "Redis connection established");

  // Simulate user authentication flow (multiple users)
  let users = vec![
    ("alice", 1001, "192.168.1.100"),
    ("bob", 1002, "192.168.1.101"),
    ("charlie", 1003, "192.168.1.102"),
    ("diana", 1004, "192.168.1.103"),
    ("eve", 1005, "192.168.1.104"),
  ];

  for (username, user_id, ip) in &users {
    info!(
      user_id = *user_id,
      username = *username,
      ip_address = *ip,
      "User authentication attempt"
    );
    debug!(user_id = *user_id, "Validating credentials");
    trace!(user_id = *user_id, "Checking password hash");
    trace!(user_id = *user_id, "Verifying session token");
    let session_id = format!("sess_{}", user_id);
    info!(
      user_id = *user_id,
      username = *username,
      session_id = session_id,
      "User logged in successfully"
    );
  }

  // Simulate API requests
  let endpoints = vec![
    ("/api/users", "GET", 200, 45),
    ("/api/posts", "GET", 200, 120),
    ("/api/users/1001", "GET", 200, 32),
    ("/api/posts", "POST", 201, 85),
    ("/api/comments", "GET", 200, 67),
    ("/api/users/1002/profile", "PUT", 200, 95),
    ("/api/search", "GET", 200, 340),
    ("/api/analytics", "GET", 200, 520),
  ];

  for cycle in 0..50 {
    for (endpoint, method, status, duration_ms) in &endpoints {
      let request_id = format!("req_{}", cycle * 100 + duration_ms);
      trace!(
        request_id = request_id,
        method = *method,
        path = *endpoint,
        "Incoming HTTP request"
      );

      debug!(
        request_id = request_id,
        endpoint = *endpoint,
        method = *method,
        "Processing request"
      );

      // Simulate database queries
      if endpoint.contains("users") || endpoint.contains("posts") {
        let db_duration = *duration_ms / 2;
        trace!(
          request_id = request_id,
          query = "SELECT * FROM users WHERE id = $1",
          duration_ms = db_duration,
          "Database query executed"
        );
      }

      // Simulate cache operations
      if cycle % 3 == 0 {
        let cache_key = format!("cache:{}:{}", endpoint, cycle);
        debug!(
          request_id = request_id,
          key = cache_key,
          "Cache miss, fetching from database"
        );
      } else {
        let cache_key = format!("cache:{}:{}", endpoint, cycle);
        trace!(request_id = request_id, key = cache_key, "Cache hit");
      }

      info!(
        request_id = request_id,
        method = *method,
        path = *endpoint,
        status_code = *status,
        duration_ms = *duration_ms,
        "HTTP request completed"
      );
    }
  }

  // Simulate background jobs
  info!("Starting background job processor");
  for job_id in 0..30 {
    debug!(
      job_id = job_id,
      job_type = "email_notification",
      "Processing background job"
    );
    trace!(
      job_id = job_id,
      recipient = "user@example.com",
      "Sending email"
    );
    let job_duration = 150 + job_id * 5;
    info!(
      job_id = job_id,
      duration_ms = job_duration,
      status = "completed",
      "Background job finished"
    );
  }

  // Simulate some warnings
  for i in 0..15 {
    let conn_id = i % 10;
    let retry_cnt = i / 10;
    warn!(
      connection_id = conn_id,
      retry_count = retry_cnt,
      "Database connection slow, retrying"
    );
  }

  // Simulate memory and performance metrics
  for _ in 0..20 {
    debug!(
      memory_used_mb = 512,
      memory_total_mb = 2048,
      cpu_usage_percent = 45.5,
      "System metrics collected"
    );
  }

  // Simulate errors
  for error_id in 0..10 {
    error!(
      error_id = error_id,
      error_code = "DB_TIMEOUT",
      message = "Database query timeout",
      query_time_ms = 5000,
      "Database operation failed"
    );
  }

  error!(
    service = "payment_processor",
    transaction_id = "txn_12345",
    amount = 99.99,
    "Payment processing failed - insufficient funds"
  );

  error!(
    service = "external_api",
    endpoint = "https://api.external.com/data",
    status_code = 503,
    "External API unavailable"
  );

  // Simulate critical errors
  for i in 0..3 {
    fatal!(
      thread_id = i,
      error = "NullPointerException",
      stack_trace = "at com.example.Service.process(Service.java:42)",
      "Critical error in worker thread"
    );
  }

  // Simulate graceful shutdown
  info!("Received shutdown signal");
  debug!("Closing database connections");
  for i in 0..10 {
    trace!(connection_id = i, "Closing database connection");
  }
  debug!("Flushing cache to disk");
  info!(
    active_users = 5,
    pending_jobs = 3,
    "Waiting for active operations to complete"
  );
  info!("Application shutdown complete");

  trace.shutdown();
  Ok(())
}
