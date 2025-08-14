use rand::{thread_rng, Rng};
use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};
use tokio;
use tracing::{debug, error, info, span, trace, warn, Instrument, Level};

// Assuming your library is named 'ttlog'
use ttlog::{event::Event, panic_hook::PanicHook, trace::Trace};

/// Complex distributed system simulation with multiple services
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize trace with large buffer for high-throughput scenario
  let trace_system = Trace::init(100);
  let buffer = trace_system.get_buffer();

  // Install panic hook for crash recovery
  PanicHook::install(buffer.clone());

  // Simulate a complex microservices architecture
  let _services: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

  // Service registry for inter-service communication
  let service_registry = Arc::new(Mutex::new(ServiceRegistry::new()));

  info!("🚀 Starting distributed system simulation");

  // Spawn multiple concurrent services
  let mut handles = vec![];

  // 1. API Gateway Service
  let gateway_buffer = buffer.clone();
  let gateway_registry = service_registry.clone();
  handles.push(tokio::spawn(async move {
    run_api_gateway(gateway_buffer, gateway_registry).await;
  }));

  // 2. User Management Service
  let user_buffer = buffer.clone();
  let user_registry = service_registry.clone();
  handles.push(tokio::spawn(async move {
    run_user_service(user_buffer, user_registry).await;
  }));

  // 3. Order Processing Service
  let order_buffer = buffer.clone();
  let order_registry = service_registry.clone();
  handles.push(tokio::spawn(async move {
    run_order_service(order_buffer, order_registry).await;
  }));

  // 4. Payment Processing Service (with deliberate failures)
  let payment_buffer = buffer.clone();
  let payment_registry = service_registry.clone();
  handles.push(tokio::spawn(async move {
    run_payment_service_with_failures(payment_buffer, payment_registry).await;
  }));

  // 5. Metrics Collector Service
  let metrics_buffer = buffer.clone();
  handles.push(tokio::spawn(async move {
    run_metrics_collector(metrics_buffer).await;
  }));

  // 6. Background Job Processor
  let job_buffer = buffer.clone();
  handles.push(tokio::spawn(async move {
    run_background_jobs(job_buffer).await;
  }));

  // 7. Database Connection Pool Manager
  let db_buffer = buffer.clone();
  handles.push(tokio::spawn(async move {
    run_database_manager(db_buffer).await;
  }));

  // Simulate high-load scenario for 30 seconds
  info!("Running high-load simulation for 30 seconds...");
  tokio::time::sleep(Duration::from_secs(30)).await;

  // Trigger manual snapshot before shutdown
  info!("Triggering pre-shutdown snapshot...");
  Trace::flush_snapshot(buffer.clone(), "shutdown");

  // Graceful shutdown simulation
  warn!("Initiating graceful shutdown...");
  for handle in handles {
    handle.abort();
  }

  // Final metrics report
  let final_buffer = buffer.lock().unwrap();
  let total_events = final_buffer.iter().count();
  info!(
    "System shutdown complete. Total events captured: {}",
    total_events
  );

  Ok(())
}

#[derive(Debug, Clone)]
struct ServiceRegistry {
  services: HashMap<String, ServiceInfo>,
}

#[derive(Debug, Clone)]
struct ServiceInfo {
  id: String,
  status: ServiceStatus,
  last_heartbeat: Instant,
  request_count: u64,
  error_count: u64,
}

#[derive(Debug, Clone)]
enum ServiceStatus {
  Healthy,
  Degraded,
  Unhealthy,
  Down,
}

impl ServiceRegistry {
  fn new() -> Self {
    Self {
      services: HashMap::new(),
    }
  }

  fn register_service(&mut self, id: String) {
    self.services.insert(
      id.clone(),
      ServiceInfo {
        id: id.clone(),
        status: ServiceStatus::Healthy,
        last_heartbeat: Instant::now(),
        request_count: 0,
        error_count: 0,
      },
    );
    info!(service_id = %id, "Service registered");
  }

  fn update_heartbeat(&mut self, service_id: &str) {
    if let Some(service) = self.services.get_mut(service_id) {
      service.last_heartbeat = Instant::now();
    }
  }
}

/// API Gateway Service - Routes requests and handles load balancing
async fn run_api_gateway(
  _buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>,
  registry: Arc<Mutex<ServiceRegistry>>,
) {
  let service_id = "api-gateway";

  // Register service
  {
    let mut reg = registry.lock().unwrap();
    reg.register_service(service_id.to_string());
  }

  let span = span!(Level::INFO, "api_gateway_service", service = service_id);

  async move {
    info!("API Gateway starting up");

    let mut request_counter = 0u64;
    let mut last_metrics_report = Instant::now();

    loop {
      // Simulate incoming HTTP requests
      let request_type = match request_counter % 4 {
        0 => "GET /users",
        1 => "POST /orders",
        2 => "GET /health",
        _ => "POST /payments",
      };

      let request_id = format!("req_{}", request_counter);
      let processing_time = std::time::Duration::from_millis(thread_rng().gen_range(10..200));

      let request_span = span!(Level::DEBUG, "http_request",
          request_id = %request_id,
          method_path = request_type,
          processing_time_ms = processing_time.as_millis()
      );

      async {
        debug!("Processing HTTP request");

        // Simulate request processing
        tokio::time::sleep(processing_time).await;

        // Simulate occasional errors
        if thread_rng().r#gen::<f32>() < 0.05 {
          error!(error_type = "timeout", "Request processing failed");
        } else {
          info!(status_code = 200, "Request processed successfully");
        }

        request_counter += 1;

        // Update service registry heartbeat
        {
          let mut reg = registry.lock().unwrap();
          reg.update_heartbeat(service_id);
        }

        // Periodic metrics reporting
        if last_metrics_report.elapsed() > Duration::from_secs(5) {
          info!(
            total_requests = request_counter,
            avg_response_time_ms = processing_time.as_millis(),
            "API Gateway metrics report"
          );
          last_metrics_report = Instant::now();
        }
      }
      .instrument(request_span)
      .await;

      tokio::time::sleep(Duration::from_millis(50)).await;
    }
  }
  .instrument(span)
  .await;
}

/// User Management Service
async fn run_user_service(
  _buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>,
  registry: Arc<Mutex<ServiceRegistry>>,
) {
  let service_id = "user-service";

  {
    let mut reg = registry.lock().unwrap();
    reg.register_service(service_id.to_string());
  }

  let span = span!(Level::INFO, "user_service", service = service_id);

  async move {
    info!("User Management Service starting up");

    let mut _user_operations = 0u64;

    loop {
      let operation = match thread_rng().gen_range(0..4) {
        0 => "create_user",
        1 => "update_profile",
        2 => "authenticate",
        _ => "fetch_user_data",
      };

      let user_id = thread_rng().gen_range(1000..9999);
      let operation_span = span!(
        Level::DEBUG,
        "user_operation",
        operation = operation,
        user_id = user_id
      );

      async {
        trace!("Starting user operation");

        // Simulate database operations
        match operation {
          "create_user" => {
            debug!("Creating new user account");
            tokio::time::sleep(Duration::from_millis(100)).await;
            info!(user_id = user_id, "User account created successfully");
          },
          "authenticate" => {
            debug!("Authenticating user");
            tokio::time::sleep(Duration::from_millis(50)).await;
            if thread_rng().r#gen::<f32>() < 0.02 {
              warn!(
                user_id = user_id,
                "Authentication failed - invalid credentials"
              );
            } else {
              info!(user_id = user_id, "User authenticated successfully");
            }
          },
          "update_profile" => {
            debug!("Updating user profile");
            tokio::time::sleep(Duration::from_millis(75)).await;
            info!(user_id = user_id, "Profile updated");
          },
          _ => {
            trace!("Fetching user data from cache");
            tokio::time::sleep(Duration::from_millis(25)).await;
            info!(cache_hit = true, "User data retrieved");
          },
        }

        _user_operations += 1;
      }
      .instrument(operation_span)
      .await;

      // Heartbeat
      {
        let mut reg = registry.lock().unwrap();
        reg.update_heartbeat(service_id);
      }

      tokio::time::sleep(Duration::from_millis(100)).await;
    }
  }
  .instrument(span)
  .await;
}

/// Order Processing Service
async fn run_order_service(
  _buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>,
  registry: Arc<Mutex<ServiceRegistry>>,
) {
  let service_id = "order-service";

  {
    let mut reg = registry.lock().unwrap();
    reg.register_service(service_id.to_string());
  }

  let span = span!(Level::INFO, "order_service", service = service_id);

  async move {
    info!("Order Processing Service starting up");

    loop {
      let order_id = thread_rng().gen_range(100000..999999);
      let order_value = thread_rng().r#gen::<f32>() * 1000.0;
      let item_count = thread_rng().gen_range(1..10);

      let order_span = span!(
        Level::INFO,
        "process_order",
        order_id = order_id,
        order_value = order_value,
        item_count = item_count
      );

      async {
        info!("Processing new order");

        // Simulate order validation
        debug!("Validating order items");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Simulate inventory check
        debug!("Checking inventory availability");
        tokio::time::sleep(Duration::from_millis(100)).await;

        if thread_rng().r#gen::<f32>() < 0.03 {
          error!(reason = "insufficient_inventory", "Order processing failed");
        } else {
          // Simulate payment processing call
          debug!("Initiating payment processing");
          tokio::time::sleep(Duration::from_millis(200)).await;

          // Simulate fulfillment
          debug!("Order sent to fulfillment");
          tokio::time::sleep(Duration::from_millis(150)).await;

          info!(processing_time_ms = 500, "Order processed successfully");
        }
      }
      .instrument(order_span)
      .await;

      // Heartbeat
      {
        let mut reg = registry.lock().unwrap();
        reg.update_heartbeat(service_id);
      }

      tokio::time::sleep(Duration::from_millis(200)).await;
    }
  }
  .instrument(span)
  .await;
}

/// Payment Service with Deliberate Failures (for testing error handling)
async fn run_payment_service_with_failures(
  _buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>,
  registry: Arc<Mutex<ServiceRegistry>>,
) {
  let service_id = "payment-service";

  {
    let mut reg = registry.lock().unwrap();
    reg.register_service(service_id.to_string());
  }

  let span = span!(Level::INFO, "payment_service", service = service_id);

  async move {
    info!("Payment Processing Service starting up");

    let mut transaction_count = 0u64;
    let mut error_count = 0u64;

    loop {
      let transaction_id = format!("txn_{}", thread_rng().gen_range(10000000..99999999));
      let amount = thread_rng().r#gen::<f32>() * 500.0;
      let payment_method = match thread_rng().gen_range(0..3) {
        0 => "credit_card",
        1 => "paypal",
        _ => "bank_transfer",
      };

      let payment_span = span!(Level::INFO, "process_payment",
          transaction_id = %transaction_id,
          amount = amount,
          payment_method = payment_method
      );

      async {
        debug!("Starting payment processing");

        // Simulate various failure scenarios
        let failure_rate = 0.08; // 8% failure rate
        let random = thread_rng().r#gen::<f32>();

        if random < failure_rate {
          let _error_type = match thread_rng().gen_range(0..4) {
            0 => {
              error!(error_code = "CARD_DECLINED", "Payment declined by issuer");
              "card_declined"
            },
            1 => {
              error!(error_code = "INSUFFICIENT_FUNDS", "Insufficient funds");
              "insufficient_funds"
            },
            2 => {
              error!(error_code = "NETWORK_TIMEOUT", "Payment gateway timeout");
              "network_timeout"
            },
            _ => {
              // Simulate a panic scenario
              if thread_rng().r#gen::<f32>() < 0.001 {
                // Very rare
                panic!("Critical payment processor failure!");
              }
              error!(error_code = "UNKNOWN_ERROR", "Unknown payment error");
              "unknown_error"
            },
          };

          error_count += 1;
          warn!(
            error_rate = (error_count as f64 / transaction_count as f64) * 100.0,
            "Payment error rate elevated"
          );
        } else {
          // Simulate successful payment processing
          debug!("Validating payment details");
          tokio::time::sleep(Duration::from_millis(100)).await;

          debug!("Processing with payment gateway");
          tokio::time::sleep(Duration::from_millis(300)).await;

          debug!("Updating transaction records");
          tokio::time::sleep(Duration::from_millis(50)).await;

          info!(processing_time_ms = 450, "Payment processed successfully");
        }

        transaction_count += 1;

        // Log metrics every 100 transactions
        if transaction_count % 100 == 0 {
          info!(
            total_transactions = transaction_count,
            total_errors = error_count,
            error_rate_percent = (error_count as f64 / transaction_count as f64) * 100.0,
            "Payment service metrics"
          );
        }
      }
      .instrument(payment_span)
      .await;

      // Heartbeat
      {
        let mut reg = registry.lock().unwrap();
        reg.update_heartbeat(service_id);
      }

      tokio::time::sleep(Duration::from_millis(150)).await;
    }
  }
  .instrument(span)
  .await;
}

/// Metrics Collector Service
async fn run_metrics_collector(_buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>) {
  let span = span!(Level::INFO, "metrics_service");

  async move {
    info!("Metrics Collector Service starting up");

    loop {
      // Collect and log system metrics
      let cpu_usage = thread_rng().r#gen::<f32>() * 100.0;
      let memory_usage = thread_rng().r#gen::<f32>() * 100.0;
      let disk_usage = thread_rng().r#gen::<f32>() * 100.0;
      let active_connections = thread_rng().gen_range(100..1000);

      info!(
        cpu_usage_percent = cpu_usage,
        memory_usage_percent = memory_usage,
        disk_usage_percent = disk_usage,
        active_connections = active_connections,
        "System metrics collected"
      );

      // Simulate alert conditions
      if cpu_usage > 85.0 {
        warn!(
          threshold = 85.0,
          current = cpu_usage,
          "High CPU usage detected"
        );
      }

      if memory_usage > 90.0 {
        error!(
          threshold = 90.0,
          current = memory_usage,
          "Critical memory usage!"
        );
      }

      tokio::time::sleep(Duration::from_secs(10)).await;
    }
  }
  .instrument(span)
  .await;
}

/// Background Job Processor
async fn run_background_jobs(_buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>) {
  let span = span!(Level::INFO, "background_jobs");

  async move {
    info!("Background Job Processor starting up");

    let mut job_counter = 0u64;

    loop {
      let job_type = match thread_rng().gen_range(0..4) {
        0 => "email_notification",
        1 => "data_backup",
        2 => "cache_warming",
        _ => "log_rotation",
      };

      let job_id = format!("job_{}_{}", job_type, job_counter);

      let job_span = span!(Level::DEBUG, "background_job",
          job_id = %job_id,
          job_type = job_type
      );

      async {
        debug!("Starting background job");

        let execution_time = match job_type {
          "email_notification" => {
            debug!("Sending email notifications");
            Duration::from_millis(thread_rng().gen_range(500..2000))
          },
          "data_backup" => {
            debug!("Performing data backup");
            Duration::from_millis(thread_rng().gen_range(5000..15000))
          },
          "cache_warming" => {
            debug!("Warming application cache");
            Duration::from_millis(thread_rng().gen_range(1000..5000))
          },
          _ => {
            debug!("Rotating log files");
            Duration::from_millis(thread_rng().gen_range(2000..8000))
          },
        };

        tokio::time::sleep(execution_time).await;

        info!(
          execution_time_ms = execution_time.as_millis(),
          "Background job completed"
        );

        job_counter += 1;
      }
      .instrument(job_span)
      .await;

      tokio::time::sleep(Duration::from_secs(5)).await;
    }
  }
  .instrument(span)
  .await;
}

/// Database Connection Pool Manager
async fn run_database_manager(_buffer: Arc<Mutex<ttlog::buffer::RingBuffer<Event>>>) {
  let span = span!(Level::INFO, "database_manager");

  async move {
    info!("Database Manager starting up");

    let mut connection_count = 10u32;
    let max_connections = 50u32;
    let min_connections = 5u32;

    loop {
      // Simulate connection pool management
      let active_queries = thread_rng().gen_range(0..connection_count);
      let connection_utilization = (active_queries as f64 / connection_count as f64) * 100.0;

      debug!(
        total_connections = connection_count,
        active_queries = active_queries,
        utilization_percent = connection_utilization,
        "Database pool status"
      );

      // Simulate dynamic connection scaling
      if connection_utilization > 80.0 && connection_count < max_connections {
        connection_count += 1;
        info!(
          new_total = connection_count,
          "Scaling up database connections"
        );
      } else if connection_utilization < 20.0 && connection_count > min_connections {
        connection_count -= 1;
        info!(
          new_total = connection_count,
          "Scaling down database connections"
        );
      }

      // Simulate occasional connection issues
      if thread_rng().r#gen::<f32>() < 0.02 {
        warn!(
          failed_connections = 1,
          remaining_connections = connection_count - 1,
          "Database connection failure detected"
        );
      }

      // Simulate slow queries
      if thread_rng().r#gen::<f32>() < 0.05 {
        let query_time = thread_rng().gen_range(5000..30000);
        warn!(
          query_duration_ms = query_time,
          threshold_ms = 5000,
          "Slow query detected"
        );
      }

      tokio::time::sleep(Duration::from_secs(3)).await;
    }
  }
  .instrument(span)
  .await;
}
