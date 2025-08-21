// src/main.rs
#![allow(clippy::needless_return)]
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, RwLock};

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{error, info, warn},
};
use uuid::Uuid;

/// Shared ID registry that can be accessed by all services.
/// This is a local, in-memory simulation. In production this
/// would be a Redis/etcd-backed service, or some other central store.
#[derive(Debug, Clone)]
pub struct SharedIdRegistry {
  messages: Arc<RwLock<HashMap<String, u16>>>,
  targets: Arc<RwLock<HashMap<String, u16>>>,
  kvs: Arc<RwLock<HashMap<String, u16>>>,
  reverse_messages: Arc<RwLock<HashMap<u16, String>>>,
  reverse_targets: Arc<RwLock<HashMap<u16, String>>>,
  reverse_kvs: Arc<RwLock<HashMap<u16, String>>>,
  next_message_id: Arc<AtomicU16>,
  next_target_id: Arc<AtomicU16>,
  next_kv_id: Arc<AtomicU16>,
}

impl SharedIdRegistry {
  pub fn new() -> Self {
    Self {
      messages: Arc::new(RwLock::new(HashMap::new())),
      targets: Arc::new(RwLock::new(HashMap::new())),
      kvs: Arc::new(RwLock::new(HashMap::new())),
      reverse_messages: Arc::new(RwLock::new(HashMap::new())),
      reverse_targets: Arc::new(RwLock::new(HashMap::new())),
      reverse_kvs: Arc::new(RwLock::new(HashMap::new())),
      next_message_id: Arc::new(AtomicU16::new(1)),
      next_target_id: Arc::new(AtomicU16::new(1)),
      next_kv_id: Arc::new(AtomicU16::new(1)),
    }
  }

  /// Get existing message id or create a new one.
  /// This is atomic with respect to the counter, but if two threads
  /// race to insert the same string you may get duplicate counters
  /// if you don't coordinate (this demo assumes single-process registry).
  pub fn get_or_create_message_id(&self, message: &str) -> u16 {
    {
      let messages = self.messages.read().unwrap();
      if let Some(&id) = messages.get(message) {
        return id;
      }
    }

    let id = self.next_message_id.fetch_add(1, Ordering::SeqCst);

    {
      let mut messages = self.messages.write().unwrap();
      let mut reverse = self.reverse_messages.write().unwrap();
      messages.insert(message.to_string(), id);
      reverse.insert(id, message.to_string());
    }

    println!("Registry: Created message ID {} for '{}'", id, message);
    id
  }

  pub fn get_or_create_target_id(&self, target: &str) -> u16 {
    {
      let targets = self.targets.read().unwrap();
      if let Some(&id) = targets.get(target) {
        return id;
      }
    }

    let id = self.next_target_id.fetch_add(1, Ordering::SeqCst);

    {
      let mut targets = self.targets.write().unwrap();
      let mut reverse = self.reverse_targets.write().unwrap();
      targets.insert(target.to_string(), id);
      reverse.insert(id, target.to_string());
    }

    println!("Registry: Created target ID {} for '{}'", id, target);
    id
  }

  pub fn get_or_create_kv_id(&self, kv: &str) -> u16 {
    {
      let kvs = self.kvs.read().unwrap();
      if let Some(&id) = kvs.get(kv) {
        return id;
      }
    }

    let id = self.next_kv_id.fetch_add(1, Ordering::SeqCst);

    {
      let mut kvs = self.kvs.write().unwrap();
      let mut reverse = self.reverse_kvs.write().unwrap();
      kvs.insert(kv.to_string(), id);
      reverse.insert(id, kv.to_string());
    }

    println!("Registry: Created KV ID {} for '{}'", id, kv);
    id
  }

  // Reverse lookup helpers
  pub fn get_message_by_id(&self, id: u16) -> Option<String> {
    let reverse = self.reverse_messages.read().unwrap();
    reverse.get(&id).cloned()
  }

  pub fn get_target_by_id(&self, id: u16) -> Option<String> {
    let reverse = self.reverse_targets.read().unwrap();
    reverse.get(&id).cloned()
  }

  pub fn get_kv_by_id(&self, id: u16) -> Option<String> {
    let reverse = self.reverse_kvs.read().unwrap();
    reverse.get(&id).cloned()
  }

  pub fn print_state(&self) {
    println!("=== Registry State ===");
    let messages = self.reverse_messages.read().unwrap();
    let targets = self.reverse_targets.read().unwrap();
    let kvs = self.reverse_kvs.read().unwrap();

    println!("Messages: {:?}", *messages);
    println!("Targets: {:?}", *targets);
    println!("KVs: {:?}", *kvs);
    println!("======================");
  }
}

// Global registry instance (single-process simulation).
static GLOBAL_ID_REGISTRY: std::sync::OnceLock<Arc<SharedIdRegistry>> = std::sync::OnceLock::new();

fn get_global_registry() -> Arc<SharedIdRegistry> {
  GLOBAL_ID_REGISTRY
    .get_or_init(|| Arc::new(SharedIdRegistry::new()))
    .clone()
}

// Simple demonstration service that uses ttlog with coordinated IDs
fn cart_service(user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
  let trace_id = Uuid::new_v4();

  // Pre-register strings in the shared registry so IDs are stable across services.
  let registry = get_global_registry();
  let _trace_id_kv = registry.get_or_create_kv_id("trace_id");
  let _user_id_kv = registry.get_or_create_kv_id("user_id");
  let _msg_id = registry.get_or_create_message_id("Cart service: received checkout request");
  let _target_id = registry.get_or_create_target_id("cart_service");

  // Use ttlog normally: it still has its own interner internally,
  // but we keep the shared registry for cross-service correlation and snapshots.
  let trace = Trace::init(4096, 64, "cart_service", Some("./tmp/"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/cart_ttlog.log")?));

  info!(
    trace_id = trace_id,
    user_id = user_id,
    "Cart service: received checkout request"
  );

  println!("Cart service logged with trace_id: {}", trace_id);

  // Simulate calling another service in the same process/demo.
  auth_service(trace_id, user_id)?;
  Ok(())
}

fn auth_service(trace_id: Uuid, user_id: i32) -> Result<(), Box<dyn std::error::Error>> {
  // Pre-register same IDs in our shared registry - they should match cart_service
  let registry = get_global_registry();
  let _trace_id_kv = registry.get_or_create_kv_id("trace_id"); // Same ID as cart service
  let _user_id_kv = registry.get_or_create_kv_id("user_id"); // Same ID as cart service
  let _msg_id = registry.get_or_create_message_id("Auth service: validating user");
  let _target_id = registry.get_or_create_target_id("auth_service");

  // Use ttlog normally
  let trace = Trace::init(4096, 64, "auth_service", Some("./tmp2/"));
  trace.add_listener(Arc::new(FileListener::new("./tmp2/auth_ttlog.log")?));

  info!(
    trace_id = trace_id,
    user_id = user_id,
    "Auth service: validating user"
  );

  if user_id == 42 {
    let _warn_msg = registry.get_or_create_message_id("User flagged for suspicious activity");
    warn!(
      trace_id = trace_id,
      user_id = user_id,
      "User flagged for suspicious activity"
    );
  } else {
    let _error_msg = registry.get_or_create_message_id("Unknown user login attempt");
    error!(
      trace_id = trace_id,
      user_id = user_id,
      "Unknown user login attempt"
    );
  }

  println!("Auth service logged with same trace_id: {}", trace_id);
  Ok(())
}

pub fn example_distributed_shared_ids() -> Result<(), Box<dyn std::error::Error>> {
  println!("TTLog Distributed System with Shared IDs Example");
  println!("=================================================");

  // Create directories used by FileListener (demo only)
  std::fs::create_dir_all("./tmp")?;
  std::fs::create_dir_all("./tmp2")?;

  // Simulate distributed services using the same ID registry
  cart_service(42)?;

  // Show the shared registry state
  println!("\nShared Registry State After Logging:");
  get_global_registry().print_state();

  println!("\nKey Benefits:");
  println!("- 'trace_id' has the same ID across both services");
  println!("- 'user_id' has the same ID across both services");
  println!("- Message templates share IDs when identical");
  println!("- Log correlation becomes trivial with shared IDs");

  Ok(())
}

// Demonstrate ID sharing concept without the full logging complexity
pub fn demonstrate_id_sharing() {
  println!("Demonstrating ID sharing across services:");
  println!("=========================================");

  let registry = get_global_registry();

  // Simulate what would happen in cart_service
  let trace_id_msg_id = registry.get_or_create_kv_id("trace_id");
  let user_id_kv_id = registry.get_or_create_kv_id("user_id");
  let cart_msg_id = registry.get_or_create_message_id("Cart service: received checkout request");
  let cart_target_id = registry.get_or_create_target_id("cart_service");

  println!("\nCart Service IDs:");
  println!("  trace_id kv: {}", trace_id_msg_id);
  println!("  user_id kv: {}", user_id_kv_id);
  println!(
    "  message: {} -> '{}'",
    cart_msg_id,
    registry.get_message_by_id(cart_msg_id).unwrap_or_default()
  );
  println!(
    "  target: {} -> '{}'",
    cart_target_id,
    registry
      .get_target_by_id(cart_target_id)
      .unwrap_or_default()
  );

  // Simulate what would happen in auth_service
  let same_trace_id = registry.get_or_create_kv_id("trace_id"); // Should be same ID
  let same_user_id = registry.get_or_create_kv_id("user_id"); // Should be same ID
  let auth_msg_id = registry.get_or_create_message_id("Auth service: validating user");
  let auth_target_id = registry.get_or_create_target_id("auth_service");

  println!("\nAuth Service IDs:");
  println!(
    "  trace_id kv: {} (same as cart: {})",
    same_trace_id,
    same_trace_id == trace_id_msg_id
  );
  println!(
    "  user_id kv: {} (same as cart: {})",
    same_user_id,
    same_user_id == user_id_kv_id
  );
  println!(
    "  message: {} -> '{}'",
    auth_msg_id,
    registry.get_message_by_id(auth_msg_id).unwrap_or_default()
  );
  println!(
    "  target: {} -> '{}'",
    auth_target_id,
    registry
      .get_target_by_id(auth_target_id)
      .unwrap_or_default()
  );

  println!("\nFinal Registry State:");
  registry.print_state();

  println!("\nConclusion:");
  println!("- Both services get ID {} for 'trace_id'", trace_id_msg_id);
  println!("- Both services get ID {} for 'user_id'", user_id_kv_id);
  println!("- This enables easy log correlation across distributed services!");
}

// Simple network sync simulation
pub fn sync_registry_across_network() -> Result<(), Box<dyn std::error::Error>> {
  println!("Simulating registry sync across network...");
  println!("==========================================");

  // Service 1 creates some entries
  let registry1 = Arc::new(SharedIdRegistry::new());
  let msg_id1 = registry1.get_or_create_message_id("User login attempt");
  let target_id1 = registry1.get_or_create_target_id("auth_service");
  let kv_id1 = registry1.get_or_create_kv_id("session_id");

  println!(
    "Service 1 - Message ID: {}, Target ID: {}, KV ID: {}",
    msg_id1, target_id1, kv_id1
  );

  // In real implementation, you'd serialize and send over network
  // For demo, we'll just show what happens without sync
  let registry2 = Arc::new(SharedIdRegistry::new());

  // Simulate receiving the same strings (should get same IDs if synced properly)
  let msg_id2 = registry2.get_or_create_message_id("User login attempt");
  let target_id2 = registry2.get_or_create_target_id("auth_service");
  let kv_id2 = registry2.get_or_create_kv_id("session_id");

  println!(
    "Service 2 - Message ID: {}, Target ID: {}, KV ID: {}",
    msg_id2, target_id2, kv_id2
  );

  // Without proper sync, these will be different
  let all_match = msg_id1 == msg_id2 && target_id1 == target_id2 && kv_id1 == kv_id2;
  println!(
    "All IDs match: {} (expected: false without sync)",
    all_match
  );

  println!("\nIn production, you would:");
  println!("- Use Redis/etcd for centralized ID storage");
  println!("- Implement atomic counter operations");
  println!("- Add conflict resolution for network partitions");
  println!("- Cache frequently used IDs locally");

  println!("Registry sync simulation complete!");
  Ok(())
}

pub fn production_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("Production-style ID coordination example:");
  println!("========================================");

  // Create a single shared registry (in production, this would be Redis/etcd)
  let shared_registry = get_global_registry();

  // Service A logs something
  println!("\n1. Service A starting...");
  let service_a_trace_id = shared_registry.get_or_create_kv_id("trace_id");
  let service_a_request_id = shared_registry.get_or_create_kv_id("request_id");
  let service_a_msg = shared_registry.get_or_create_message_id("Processing user request");

  println!(
    "   Service A - trace_id: {}, request_id: {}, message: {}",
    service_a_trace_id, service_a_request_id, service_a_msg
  );

  // Service B logs with same context
  println!("\n2. Service B receiving request...");
  let service_b_trace_id = shared_registry.get_or_create_kv_id("trace_id"); // Same!
  let service_b_request_id = shared_registry.get_or_create_kv_id("request_id"); // Same!
  let service_b_msg = shared_registry.get_or_create_message_id("Validating request");

  println!(
    "   Service B - trace_id: {}, request_id: {}, message: {}",
    service_b_trace_id, service_b_request_id, service_b_msg
  );

  // Service C joins the party
  println!("\n3. Service C processing...");
  let service_c_trace_id = shared_registry.get_or_create_kv_id("trace_id"); // Same!
  let service_c_request_id = shared_registry.get_or_create_kv_id("request_id"); // Same!
  let service_c_msg = shared_registry.get_or_create_message_id("Storing results");

  println!(
    "   Service C - trace_id: {}, request_id: {}, message: {}",
    service_c_trace_id, service_c_request_id, service_c_msg
  );

  println!("\n4. Final verification:");
  println!("   All services use trace_id: {} ✓", service_a_trace_id);
  println!("   All services use request_id: {} ✓", service_a_request_id);
  println!("   Each service has unique message IDs for different content ✓");

  shared_registry.print_state();

  println!("\nResult: Log correlation across all services is now trivial!");
  println!(
    "You can group all logs by trace_id={} or request_id={}",
    service_a_trace_id, service_a_request_id
  );

  Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Run small demo
  example_distributed_shared_ids()?;

  // Show the simple sync simulation
  sync_registry_across_network()?;

  // Demonstrate production example
  production_example()?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_shared_id_consistency() {
    let registry = Arc::new(SharedIdRegistry::new());

    // Both requests in same registry should get same ID for same string
    let id1 = registry.get_or_create_message_id("test message");
    let id2 = registry.get_or_create_message_id("test message");
    assert_eq!(id1, id2);

    let target_id1 = registry.get_or_create_target_id("test_target");
    let target_id2 = registry.get_or_create_target_id("test_target");
    assert_eq!(target_id1, target_id2);

    let kv_id1 = registry.get_or_create_kv_id("test_kv");
    let kv_id2 = registry.get_or_create_kv_id("test_kv");
    assert_eq!(kv_id1, kv_id2);
  }

  #[test]
  fn test_different_strings_get_different_ids() {
    let registry = Arc::new(SharedIdRegistry::new());

    let id1 = registry.get_or_create_message_id("message 1");
    let id2 = registry.get_or_create_message_id("message 2");
    assert_ne!(id1, id2);
  }

  #[test]
  fn test_reverse_lookup() {
    let registry = Arc::new(SharedIdRegistry::new());
    let original_message = "test reverse lookup";
    let id = registry.get_or_create_message_id(original_message);
    let retrieved = registry.get_message_by_id(id);
    assert_eq!(retrieved.as_deref(), Some(original_message));
  }
}
