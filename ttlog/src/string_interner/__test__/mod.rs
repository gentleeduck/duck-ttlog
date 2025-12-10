#[cfg(test)]
mod __test__ {

  use smallvec::smallvec;
  use std::sync::Arc;
  use std::thread;

  use crate::string_interner::StringInterner;

  #[test]
  fn test_intern_target_and_get() {
    let interner = StringInterner::new();

    let id1 = interner.intern_target("module1");
    let id2 = interner.intern_target("module1");
    let id3 = interner.intern_target("module2");

    assert!(id1 > 0);
    assert_eq!(id1, id2);
    assert_ne!(id1, id3);
    assert_eq!(interner.get_target(id1).unwrap().as_ref(), "module1");
  }

  #[test]
  fn test_intern_message_and_get() {
    let interner = StringInterner::new();

    let id = interner.intern_message("Hello World");
    assert_eq!(interner.intern_message("Hello World"), id);
    assert_eq!(interner.get_message(id).unwrap().as_ref(), "Hello World");
  }

  #[test]
  fn test_intern_kv_and_get() {
    let interner = StringInterner::new();
    let data = smallvec![b'{', b'}'];

    let kv_id = interner.intern_kv(data.clone());
    assert_eq!(interner.intern_kv(data.clone()), kv_id);

    let stored = interner.get_kv(kv_id).unwrap();
    assert_eq!(stored.as_slice(), data.as_slice());
  }

  #[test]
  fn test_stats() {
    let interner = StringInterner::new();
    interner.intern_target("module");
    interner.intern_message("message");
    let data = smallvec![b'{', b'}'];
    interner.intern_kv(data);

    let (targets, messages, kvs) = interner.stats();
    assert!(targets >= 2); // includes dummy entry
    assert!(messages >= 2);
    assert!(kvs >= 2);
  }

  #[test]
  fn test_concurrent_interning() {
    let interner = Arc::new(StringInterner::new());
    let mut handles = vec![];

    for i in 0..8 {
      let interner_clone = interner.clone();
      handles.push(thread::spawn(move || {
        let t = interner_clone.intern_target("common_target");
        let m = interner_clone.intern_message("common_message");
        let kv = interner_clone.intern_kv(smallvec![b'k', b'v']);

        let unique_target = format!("target_{}", i);
        interner_clone.intern_target(&unique_target);

        (t, m, kv)
      }));
    }

    let mut results = vec![];
    for handle in handles {
      results.push(handle.join().unwrap());
    }

    let first = results[0];
    for r in &results {
      assert_eq!(r.0, first.0);
      assert_eq!(r.1, first.1);
      assert_eq!(r.2, first.2);
    }
  }

  #[test]
  fn test_unicode_strings() {
    let interner = StringInterner::new();
    let unicode = "こんにちは世界";

    let target_id = interner.intern_target(unicode);
    let message_id = interner.intern_message(unicode);

    assert_eq!(interner.get_target(target_id).unwrap().as_ref(), unicode);
    assert_eq!(interner.get_message(message_id).unwrap().as_ref(), unicode);
  }

  #[test]
  fn test_large_kv_payload() {
    let interner = StringInterner::new();
    let mut payload = smallvec![0u8; 1024];
    for (idx, byte) in payload.iter_mut().enumerate() {
      *byte = (idx % 255) as u8;
    }

    let id = interner.intern_kv(payload.clone());
    let retrieved = interner.get_kv(id).unwrap();
    assert_eq!(retrieved.len(), payload.len());
  }
}
