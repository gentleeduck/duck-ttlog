// #[cfg(test)]
// mod tests {
//   use std::sync::Arc;
//
//   use crate::{event_builder::EventBuilder, string_interner::StringInterner};
//
//   fn create_test_interner() -> Arc<StringInterner> {
//     Arc::new(StringInterner::new())
//   }
//
//   #[test]
//   fn test_event_builder_new() {
//     let interner = create_test_interner();
//     let builder = EventBuilder::new(interner.clone());
//
//     assert!(Arc::ptr_eq(&builder.interner, &interner));
//     assert_eq!(builder.event_pool.capacity(), 16);
//     assert_eq!(builder.pool_index, 0);
//     // Thread ID should be deterministic for this thread
//     assert!(builder.thread_id > 0);
//   }
//
//   #[test]
//   fn test_get_pooled_event() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // First call should create a new event
//     let event1 = builder.get_pooled_event();
//     assert_eq!(builder.event_pool.len(), 1);
//     assert_eq!(builder.pool_index, 1);
//
//     // Modify the event
//     event1.field_count = 5;
//
//     // Get another event
//     let event2 = builder.get_pooled_event();
//     assert_eq!(builder.event_pool.len(), 2);
//     assert_eq!(builder.pool_index, 2);
//
//     // Events should be different instances
//     assert_ne!(event1 as *const _, event2 as *const _);
//   }
//
//   #[test]
//   fn test_pooled_event_reset() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let event = builder.get_pooled_event();
//     event.field_count = 3;
//     event.packed_meta = 12345;
//
//     // Get the same event again (after cycling through pool)
//     for _ in 0..16 {
//       builder.get_pooled_event();
//     }
//
//     let reused_event = builder.get_pooled_event();
//     // Event should be reset
//     assert_eq!(reused_event.field_count, 0);
//     assert_eq!(reused_event.packed_meta, 0);
//   }
//
//   #[test]
//   fn test_build_fast() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let timestamp = 1234567890;
//     let level = LogLevel::Info;
//     let target = "test::module";
//     let message = "Test message";
//
//     let event = builder.build_fast(timestamp, level, target, message);
//
//     // Verify packed metadata contains our data
//     let (unpacked_timestamp, unpacked_level, unpacked_thread_id) =
//       LogEvent::unpack_meta(event.packed_meta);
//
//     assert_eq!(unpacked_timestamp, timestamp);
//     assert_eq!(unpacked_level, level);
//     assert_eq!(unpacked_thread_id, builder.thread_id);
//
//     // Verify string interning worked
//     assert!(event.target_id > 0);
//     assert!(event.message_id > 0);
//
//     // Should have no fields
//     assert_eq!(event.field_count, 0);
//   }
//
//   #[test]
//   fn test_build_with_fields() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let fields = vec![
//       ("key1".to_string(), FieldValue::String("value1".to_string())),
//       ("key2".to_string(), FieldValue::Integer(42)),
//       ("key3".to_string(), FieldValue::Boolean(true)),
//     ];
//
//     let event = builder.build_with_fields(
//       1234567890,
//       LogLevel::Debug,
//       "test::target",
//       "Test with fields",
//       &fields,
//     );
//
//     assert_eq!(event.field_count, 3);
//
//     // Verify fields were added correctly
//     for (i, (expected_key, expected_value)) in fields.iter().enumerate() {
//       let field = &event.fields[i];
//       assert!(field.key_id > 0); // Key should be interned
//       assert_eq!(field.value, *expected_value);
//     }
//   }
//
//   #[test]
//   fn test_build_with_fields_truncation() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Create 5 fields, but only 3 should be used
//     let fields = vec![
//       ("key1".to_string(), FieldValue::String("value1".to_string())),
//       ("key2".to_string(), FieldValue::Integer(42)),
//       ("key3".to_string(), FieldValue::Boolean(true)),
//       (
//         "key4".to_string(),
//         FieldValue::String("ignored".to_string()),
//       ),
//       ("key5".to_string(), FieldValue::Integer(999)),
//     ];
//
//     let event = builder.build_with_fields(
//       1234567890,
//       LogLevel::Warn,
//       "test::target",
//       "Test truncation",
//       &fields,
//     );
//
//     // Should only have 3 fields (silently truncated)
//     assert_eq!(event.field_count, 3);
//   }
//
//   #[test]
//   fn test_build_with_empty_fields() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let fields = vec![];
//
//     let event = builder.build_with_fields(
//       1234567890,
//       LogLevel::Error,
//       "test::target",
//       "No fields",
//       &fields,
//     );
//
//     assert_eq!(event.field_count, 0);
//   }
//
//   #[test]
//   fn test_thread_id_stability() {
//     let interner = create_test_interner();
//     let builder1 = EventBuilder::new(interner.clone());
//     let builder2 = EventBuilder::new(interner);
//
//     // Same thread should have same thread ID
//     assert_eq!(builder1.thread_id, builder2.thread_id);
//   }
//
//   #[test]
//   fn test_thread_id_different_threads() {
//     let interner = create_test_interner();
//     let interner_clone = interner.clone();
//
//     let main_thread_id = {
//       let builder = EventBuilder::new(interner);
//       builder.thread_id
//     };
//
//     let other_thread_id = thread::spawn(move || {
//       let builder = EventBuilder::new(interner_clone);
//       builder.thread_id
//     })
//     .join()
//     .unwrap();
//
//     // Different threads should have different IDs (with high probability)
//     // Note: There's a small chance they could be the same due to hash collisions
//     // but it's extremely unlikely with u8 range
//     assert_ne!(main_thread_id, other_thread_id);
//   }
//
//   #[test]
//   fn test_message_visitor_string() {
//     let mut visitor = MessageVisitor::default();
//     let field = tracing::field::Field::new("message", tracing::field::Kind::STR);
//
//     visitor.record_str(&field, "Hello, world!");
//
//     assert_eq!(visitor.message, Some("Hello, world!".to_string()));
//   }
//
//   #[test]
//   fn test_message_visitor_debug() {
//     let mut visitor = MessageVisitor::default();
//     let field = tracing::field::Field::new("message", tracing::field::Kind::DEBUG);
//
//     visitor.record_debug(&field, &42);
//
//     assert_eq!(visitor.message, Some("42".to_string()));
//   }
//
//   #[test]
//   fn test_message_visitor_non_message_field() {
//     let mut visitor = MessageVisitor::default();
//     let field = tracing::field::Field::new("other_field", tracing::field::Kind::STR);
//
//     visitor.record_str(&field, "This should be ignored");
//
//     assert_eq!(visitor.message, None);
//   }
//
//   #[test]
//   fn test_message_visitor_debug_priority() {
//     let mut visitor = MessageVisitor::default();
//     let field = tracing::field::Field::new("message", tracing::field::Kind::DEBUG);
//
//     // Set string first
//     visitor.record_str(&field, "String value");
//
//     // Debug should not overwrite existing string
//     visitor.record_debug(&field, &42);
//
//     assert_eq!(visitor.message, Some("String value".to_string()));
//   }
//
//   #[test]
//   fn test_build_event_stack() {
//     let interner = create_test_interner();
//
//     let event = build_event_stack(
//       &interner,
//       1234567890,
//       LogLevel::Info,
//       "stack::target",
//       "Stack message",
//     );
//
//     let (timestamp, level, _thread_id) = LogEvent::unpack_meta(event.packed_meta);
//     assert_eq!(timestamp, 1234567890);
//     assert_eq!(level, LogLevel::Info);
//     assert!(event.target_id > 0);
//     assert!(event.message_id > 0);
//     assert_eq!(event.field_count, 0);
//   }
//
//   #[test]
//   fn test_build_event_fast_function() {
//     use tracing::{info, Level, Metadata};
//
//     let interner = create_test_interner();
//
//     // Create a mock tracing event
//     let metadata = Metadata::new(
//       "test_event",
//       "test::target",
//       Level::INFO,
//       Some(file!()),
//       Some(line!()),
//       Some("test::module"),
//       tracing::field::FieldSet::new(
//         &["message"],
//         tracing::callsite::Identifier(&metadata as *const _ as usize),
//       ),
//       tracing::metadata::Kind::EVENT,
//     );
//
//     // Note: Creating a real tracing::Event is complex due to private fields
//     // In practice, this would be tested with actual tracing macros
//     // For now, we'll test the thread-local behavior indirectly
//
//     // Test that multiple calls to the same thread use the same builder
//     let _event1 = build_event_stack(&interner, 1000, LogLevel::Info, "target1", "msg1");
//     let _event2 = build_event_stack(&interner, 2000, LogLevel::Warn, "target2", "msg2");
//
//     // Both should succeed without panics
//     assert!(true);
//   }
//
//   #[test]
//   fn test_string_interning_deduplication() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let event1 = builder.build_fast(1000, LogLevel::Info, "same::target", "same message");
//     let event2 = builder.build_fast(2000, LogLevel::Debug, "same::target", "same message");
//
//     // Same strings should have same interned IDs
//     assert_eq!(event1.target_id, event2.target_id);
//     assert_eq!(event1.message_id, event2.message_id);
//   }
//
//   #[test]
//   fn test_field_value_types() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let fields = vec![
//       (
//         "str_field".to_string(),
//         FieldValue::String("test".to_string()),
//       ),
//       ("int_field".to_string(), FieldValue::Integer(42)),
//       ("bool_field".to_string(), FieldValue::Boolean(false)),
//     ];
//
//     let event = builder.build_with_fields(
//       1000,
//       LogLevel::Trace,
//       "test::types",
//       "Field types test",
//       &fields,
//     );
//
//     assert_eq!(event.field_count, 3);
//
//     match &event.fields[0].value {
//       FieldValue::String(s) => assert_eq!(s, "test"),
//       _ => panic!("Expected string field"),
//     }
//
//     match &event.fields[1].value {
//       FieldValue::Integer(i) => assert_eq!(*i, 42),
//       _ => panic!("Expected integer field"),
//     }
//
//     match &event.fields[2].value {
//       FieldValue::Boolean(b) => assert_eq!(*b, false),
//       _ => panic!("Expected boolean field"),
//     }
//   }
//
//   #[test]
//   fn test_concurrent_builders() {
//     let interner = create_test_interner();
//     let handles: Vec<_> = (0..4)
//       .map(|i| {
//         let interner_clone = interner.clone();
//         thread::spawn(move || {
//           let mut builder = EventBuilder::new(interner_clone);
//           let event = builder.build_fast(
//             1000 + i as u64,
//             LogLevel::Info,
//             &format!("thread::{}", i),
//             &format!("Message from thread {}", i),
//           );
//           (builder.thread_id, event.target_id, event.message_id)
//         })
//       })
//       .collect();
//
//     let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
//
//     // All threads should complete successfully
//     assert_eq!(results.len(), 4);
//
//     // Each thread should have a unique thread ID (with high probability)
//     let thread_ids: Vec<_> = results.iter().map(|(tid, _, _)| *tid).collect();
//     let unique_thread_ids: std::collections::HashSet<_> = thread_ids.iter().collect();
//     assert!(unique_thread_ids.len() >= 2); // At least some should be different
//   }
//
//   #[test]
//   fn test_pool_wraparound() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Fill the pool beyond capacity
//     for i in 0..20 {
//       let event = builder.get_pooled_event();
//       event.field_count = i as u8; // Mark the event
//     }
//
//     // Pool should have wrapped around
//     assert_eq!(builder.pool_index, 4); // 20 % 16 = 4
//     assert_eq!(builder.event_pool.len(), 16); // Capped at 16
//   }
//
//   #[test]
//   fn test_build_stack_vs_pooled() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner.clone());
//
//     let pooled_event = builder.get_pooled_event();
//     let stack_event = build_event_stack(
//       &interner,
//       1000,
//       LogLevel::Info,
//       "test::target",
//       "test message",
//     );
//
//     // They should be different instances
//     assert_ne!(pooled_event as *const _, &stack_event as *const _);
//
//     // But should have similar structure
//     assert_eq!(pooled_event.field_count, stack_event.field_count);
//   }
//
//   #[test]
//   fn test_log_levels() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let levels = vec![
//       LogLevel::Trace,
//       LogLevel::Debug,
//       LogLevel::Info,
//       LogLevel::Warn,
//       LogLevel::Error,
//     ];
//
//     for level in levels {
//       let event = builder.build_fast(1000, level, "test", "message");
//       let (_, unpacked_level, _) = LogEvent::unpack_meta(event.packed_meta);
//       assert_eq!(unpacked_level, level);
//     }
//   }
//
//   #[test]
//   fn test_large_field_sets() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Create many fields to test the 3-field limit
//     let mut fields = Vec::new();
//     for i in 0..10 {
//       fields.push((format!("key{}", i), FieldValue::Integer(i as i64)));
//     }
//
//     let event = builder.build_with_fields(
//       1000,
//       LogLevel::Debug,
//       "test::many_fields",
//       "Many fields test",
//       &fields,
//     );
//
//     // Should only have first 3 fields
//     assert_eq!(event.field_count, 3);
//
//     for i in 0..3 {
//       match &event.fields[i].value {
//         FieldValue::Integer(val) => assert_eq!(*val, i as i64),
//         _ => panic!("Expected integer field at index {}", i),
//       }
//     }
//   }
//
//   #[test]
//   fn test_empty_strings() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let event = builder.build_fast(1000, LogLevel::Info, "", "");
//
//     // Empty strings should still be interned
//     assert!(event.target_id > 0);
//     assert!(event.message_id > 0);
//   }
//
//   #[test]
//   fn test_message_visitor_default() {
//     let visitor = MessageVisitor::default();
//     assert_eq!(visitor.message, None);
//   }
//
//   #[test]
//   fn test_current_thread_id_consistency() {
//     // Should return the same value when called multiple times in same thread
//     let id1 = EventBuilder::current_thread_id_u64();
//     let id2 = EventBuilder::current_thread_id_u64();
//     assert_eq!(id1, id2);
//   }
//
//   #[test]
//   fn test_memory_usage() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Build many events to test memory behavior
//     for i in 0..100 {
//       let _event = builder.build_fast(
//         i,
//         LogLevel::Info,
//         &format!("target::{}", i % 5), // Some repetition for interning
//         &format!("Message {}", i),
//       );
//     }
//
//     // Pool should be stable at 16
//     assert!(builder.event_pool.len() <= 16);
//
//     // Interner should have reasonable number of entries
//     // (This would need access to interner internals to verify properly)
//   }
//
//   #[test]
//   fn test_metadata_packing_edge_cases() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Test with maximum timestamp value
//     let max_timestamp = u64::MAX;
//     let event = builder.build_fast(max_timestamp, LogLevel::Error, "test", "max timestamp");
//
//     let (unpacked_timestamp, unpacked_level, _) = LogEvent::unpack_meta(event.packed_meta);
//     // Depending on packing implementation, this might be truncated
//     // The exact behavior depends on LogEvent::pack_meta implementation
//     assert!(unpacked_timestamp <= max_timestamp);
//     assert_eq!(unpacked_level, LogLevel::Error);
//   }
//
//   #[test]
//   fn test_field_interning() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let fields1 = vec![(
//       "common_key".to_string(),
//       FieldValue::String("value1".to_string()),
//     )];
//
//     let fields2 = vec![(
//       "common_key".to_string(),
//       FieldValue::String("value2".to_string()),
//     )];
//
//     let event1 = builder.build_with_fields(1000, LogLevel::Info, "test", "msg1", &fields1);
//     let event2 = builder.build_with_fields(2000, LogLevel::Info, "test", "msg2", &fields2);
//
//     // Same field key should have same interned ID
//     assert_eq!(event1.fields[0].key_id, event2.fields[0].key_id);
//
//     // But different values
//     assert_ne!(event1.fields[0].value, event2.fields[0].value);
//   }
// }
//
// #[cfg(test)]
// mod integration_tests {
//   use super::*;
//
//   #[test]
//   fn test_real_world_usage_pattern() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Simulate a real logging scenario
//     let start_time = 1000;
//
//     // Application startup
//     let _startup_event = builder.build_fast(
//       start_time,
//       LogLevel::Info,
//       "app::startup",
//       "Application starting",
//     );
//
//     // Request processing with fields
//     let request_fields = vec![
//       (
//         "request_id".to_string(),
//         FieldValue::String("req-123".to_string()),
//       ),
//       ("method".to_string(), FieldValue::String("GET".to_string())),
//       ("status".to_string(), FieldValue::Integer(200)),
//     ];
//
//     let _request_event = builder.build_with_fields(
//       start_time + 100,
//       LogLevel::Info,
//       "app::http",
//       "Request processed",
//       &request_fields,
//     );
//
//     // Error handling
//     let error_fields = vec![
//       (
//         "error_code".to_string(),
//         FieldValue::String("E001".to_string()),
//       ),
//       ("retryable".to_string(), FieldValue::Boolean(true)),
//     ];
//
//     let _error_event = builder.build_with_fields(
//       start_time + 200,
//       LogLevel::Error,
//       "app::database",
//       "Connection failed",
//       &error_fields,
//     );
//
//     // All events should be created successfully
//     assert!(true);
//   }
//
//   #[test]
//   fn test_performance_characteristics() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     let start = std::time::Instant::now();
//
//     // Build many events quickly
//     for i in 0..1000 {
//       let _event = builder.build_fast(i, LogLevel::Info, "perf::test", "Performance test message");
//     }
//
//     let duration = start.elapsed();
//
//     // Should complete quickly (this is a rough performance check)
//     assert!(duration.as_millis() < 100);
//
//     // Pool should be managed efficiently
//     assert_eq!(builder.event_pool.len(), 16);
//   }
// }
//
// #[cfg(test)]
// mod benchmark_tests {
//   use super::*;
//   use std::time::Instant;
//
//   #[test]
//   fn bench_build_fast_vs_stack() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner.clone());
//
//     let iterations = 10000;
//
//     // Benchmark build_fast
//     let start = Instant::now();
//     for i in 0..iterations {
//       let _event = builder.build_fast(i, LogLevel::Info, "bench::target", "Benchmark message");
//     }
//     let build_fast_duration = start.elapsed();
//
//     // Benchmark build_event_stack
//     let start = Instant::now();
//     for i in 0..iterations {
//       let _event = build_event_stack(
//         &interner,
//         i,
//         LogLevel::Info,
//         "bench::target",
//         "Benchmark message",
//       );
//     }
//     let build_stack_duration = start.elapsed();
//
//     println!("build_fast: {:?}", build_fast_duration);
//     println!("build_stack: {:?}", build_stack_duration);
//
//     // Both should complete in reasonable time
//     assert!(build_fast_duration.as_millis() < 1000);
//     assert!(build_stack_duration.as_millis() < 1000);
//   }
// }
//
// // Property-based testing helpers
// #[cfg(test)]
// mod property_tests {
//   use super::*;
//
//   #[test]
//   fn test_thread_id_deterministic() {
//     // Thread ID should be deterministic for the same thread
//     let ids: Vec<_> = (0..10)
//       .map(|_| EventBuilder::current_thread_id_u64())
//       .collect();
//
//     // All IDs should be the same
//     for id in &ids[1..] {
//       assert_eq!(*id, ids[0]);
//     }
//   }
//
//   #[test]
//   fn test_pool_bounds() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Test pool behavior with various access patterns
//     for _ in 0..50 {
//       let _event = builder.get_pooled_event();
//     }
//
//     // Pool should never exceed capacity
//     assert!(builder.event_pool.len() <= 16);
//     assert!(builder.pool_index < 16);
//   }
//
//   #[test]
//   fn test_field_limits_respected() {
//     let interner = create_test_interner();
//     let mut builder = EventBuilder::new(interner);
//
//     // Test various field counts
//     for field_count in 0..=10 {
//       let fields: Vec<_> = (0..field_count)
//         .map(|i| (format!("key{}", i), FieldValue::I64(i as i64)))
//         .collect();
//
//       let event = builder.build_with_fields(1000, LogLevel::INFO, "test", "message", &fields);
//
//       // Should never exceed 3 fields
//       assert!(event.field_count <= 3);
//       assert_eq!(event.field_count, std::cmp::min(field_count, 3) as u8);
//     }
//   }
// }
