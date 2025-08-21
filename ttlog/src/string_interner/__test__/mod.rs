#[cfg(test)]
mod __test__ {

  use crate::string_interner::StringInterner;
  use std::sync::Arc;
  use std::thread;

  #[test]
  fn test_string_interner_new() {
    let interner = StringInterner::new();

    // Verify initial state
    assert_eq!(interner.targets.read().unwrap().len(), 0);
    assert_eq!(interner.messages.read().unwrap().len(), 0);
    assert_eq!(interner.kvs.read().unwrap().len(), 0);
  }

  #[test]
  fn test_intern_target() {
    let interner = StringInterner::new();

    // Intern first target
    let id1 = interner.intern_target("module1");
    assert_eq!(id1, 0);

    // Intern same target again - should return same ID
    let id2 = interner.intern_target("module1");
    assert_eq!(id2, 0);

    // Intern different target - should get new ID
    let id3 = interner.intern_target("module2");
    assert_eq!(id3, 1);

    // Verify storage
    assert_eq!(interner.targets.read().unwrap().len(), 2);
  }

  #[test]
  fn test_intern_message() {
    let interner = StringInterner::new();

    // Intern first message
    let id1 = interner.intern_message("Hello World");
    assert_eq!(id1, 0);

    // Intern same message again - should return same ID
    let id2 = interner.intern_message("Hello World");
    assert_eq!(id2, 0);

    // Intern different message - should get new ID
    let id3 = interner.intern_message("Goodbye World");
    assert_eq!(id3, 1);

    // Verify storage
    assert_eq!(interner.messages.read().unwrap().len(), 2);
  }

  #[test]
  fn test_intern_field() {
    let interner = StringInterner::new();

    // Intern first field
    let id1 = interner.intern_field("key1");
    assert_eq!(id1, 0);

    // Intern same field again - should return same ID
    let id2 = interner.intern_field("key1");
    assert_eq!(id2, 0);

    // Intern different field - should get new ID
    let id3 = interner.intern_field("key2");
    assert_eq!(id3, 1);

    // Verify storage
    assert_eq!(interner.kvs.read().unwrap().len(), 2);
  }

  #[test]
  fn test_get_target() {
    let interner = StringInterner::new();

    // Get non-existent target
    assert!(interner.get_target(0).is_none());

    // Intern and retrieve target
    let id = interner.intern_target("test_module");
    let retrieved = interner.get_target(id).unwrap();
    assert_eq!(retrieved.as_ref(), "test_module");

    // Get non-existent target with high ID
    assert!(interner.get_target(999).is_none());
  }

  #[test]
  fn test_get_message() {
    let interner = StringInterner::new();

    // Get non-existent message
    assert!(interner.get_message(0).is_none());

    // Intern and retrieve message
    let id = interner.intern_message("test message");
    let retrieved = interner.get_message(id).unwrap();
    assert_eq!(retrieved.as_ref(), "test message");

    // Get non-existent message with high ID
    assert!(interner.get_message(999).is_none());
  }

  #[test]
  fn test_get_field() {
    let interner = StringInterner::new();

    // Get non-existent field
    assert!(interner.get_kv(0).is_none());

    // Intern and retrieve field
    let id = interner.intern_field("test_field");
    let retrieved = interner.get_kv(id).unwrap();
    assert_eq!(retrieved.as_ref(), "test_field");

    // Get non-existent field with high ID
    assert!(interner.get_kv(999).is_none());
  }

  #[test]
  fn test_concurrent_interning() {
    let interner = Arc::new(StringInterner::new());
    let mut handles = vec![];

    // Spawn multiple threads that intern the same strings
    for i in 0..10 {
      let interner_clone = Arc::clone(&interner);
      let handle = thread::spawn(move || {
        let target_id = interner_clone.intern_target("common_target");
        let message_id = interner_clone.intern_message("common_message");
        let field_id = interner_clone.intern_field("common_field");

        // Also intern unique strings
        let unique_target = format!("target_{}", i);
        let unique_message = format!("message_{}", i);
        let unique_field = format!("field_{}", i);

        interner_clone.intern_target(&unique_target);
        interner_clone.intern_message(&unique_message);
        interner_clone.intern_field(&unique_field);

        (target_id, message_id, field_id)
      });
      handles.push(handle);
    }

    // Wait for all threads and collect results
    let mut results = vec![];
    for handle in handles {
      results.push(handle.join().unwrap());
    }

    // All threads should get the same ID for common strings
    let first_result = results[0];
    for result in &results {
      assert_eq!(result.0, first_result.0); // target_id
      assert_eq!(result.1, first_result.1); // message_id
      assert_eq!(result.2, first_result.2); // field_id
    }

    // Verify final counts (1 common + 10 unique for each type)
    assert_eq!(interner.targets.read().unwrap().len(), 11);
    assert_eq!(interner.messages.read().unwrap().len(), 11);
    assert_eq!(interner.kvs.read().unwrap().len(), 11);
  }

  #[test]
  fn test_empty_string_interning() {
    let interner = StringInterner::new();

    // Intern empty strings
    let target_id = interner.intern_target("");
    let message_id = interner.intern_message("");
    let field_id = interner.intern_field("");

    // Verify they can be retrieved
    assert_eq!(interner.get_target(target_id).unwrap().as_ref(), "");
    assert_eq!(interner.get_message(message_id).unwrap().as_ref(), "");
    assert_eq!(interner.get_kv(field_id).unwrap().as_ref(), "");
  }

  #[test]
  fn test_large_string_interning() {
    let interner = StringInterner::new();

    // Create a large string
    let large_string = "x".repeat(10000);

    // Intern large strings
    let target_id = interner.intern_target(&large_string);
    let message_id = interner.intern_message(&large_string);
    let field_id = interner.intern_field(&large_string);

    // Verify they can be retrieved correctly
    assert_eq!(
      interner.get_target(target_id).unwrap().as_ref(),
      large_string
    );
    assert_eq!(
      interner.get_message(message_id).unwrap().as_ref(),
      large_string
    );
    assert_eq!(interner.get_kv(field_id).unwrap().as_ref(), large_string);
  }

  #[test]
  fn test_unicode_string_interning() {
    let interner = StringInterner::new();

    // Test various Unicode strings
    let unicode_strings = vec![
      "Hello 世界",
      "🦀 Rust",
      "Здравствуй мир",
      "مرحبا بالعالم",
      "🎉🎊✨",
    ];

    for unicode_str in &unicode_strings {
      let target_id = interner.intern_target(unicode_str);
      let message_id = interner.intern_message(unicode_str);
      let field_id = interner.intern_field(unicode_str);

      assert_eq!(
        interner.get_target(target_id).unwrap().as_ref(),
        *unicode_str
      );
      assert_eq!(
        interner.get_message(message_id).unwrap().as_ref(),
        *unicode_str
      );
      assert_eq!(interner.get_kv(field_id).unwrap().as_ref(), *unicode_str);
    }
  }
}
