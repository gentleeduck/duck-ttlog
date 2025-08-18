use std::{
  cell::UnsafeCell,
  collections::HashMap,
  sync::atomic::{AtomicU16, Ordering},
  sync::{Arc, RwLock},
};

// Fast thread-local cache to avoid locking on hot paths
#[derive(Debug)]
struct LocalCache {
  // Small LRU cache for recently used strings
  target_cache: [(u64, u16); 8],   // hash -> id pairs
  message_cache: [(u64, u16); 16], // messages are more varied
  field_cache: [(u64, u16); 8],

  // Round-robin eviction counters
  target_counter: u8,
  message_counter: u8,
  field_counter: u8,
}

impl LocalCache {
  fn new() -> Self {
    Self {
      target_cache: [(0, 0); 8],
      message_cache: [(0, 0); 16],
      field_cache: [(0, 0); 8],
      target_counter: 0,
      message_counter: 0,
      field_counter: 0,
    }
  }

  fn get_target(&self, hash: u64) -> Option<u16> {
    self
      .target_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  fn put_target(&mut self, hash: u64, id: u16) {
    let idx = self.target_counter as usize % 8;
    self.target_cache[idx] = (hash, id);
    self.target_counter = self.target_counter.wrapping_add(1);
  }

  fn get_message(&self, hash: u64) -> Option<u16> {
    self
      .message_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  fn put_message(&mut self, hash: u64, id: u16) {
    let idx = self.message_counter as usize % 16;
    self.message_cache[idx] = (hash, id);
    self.message_counter = self.message_counter.wrapping_add(1);
  }

  fn get_field(&self, hash: u64) -> Option<u16> {
    self
      .field_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  fn put_field(&mut self, hash: u64, id: u16) {
    let idx = self.field_counter as usize % 8;
    self.field_cache[idx] = (hash, id);
    self.field_counter = self.field_counter.wrapping_add(1);
  }
}

thread_local! {
    static LOCAL_CACHE: UnsafeCell<LocalCache> = UnsafeCell::new(LocalCache::new());
}

#[derive(Debug)]
pub struct StringInterner {
  // Separate storage for different string types
  targets: RwLock<Vec<Arc<str>>>,
  messages: RwLock<Vec<Arc<str>>>,
  fields: RwLock<Vec<Arc<str>>>,

  // Optimized hash maps with better hasher
  target_lookup: RwLock<HashMap<u64, u16>>,
  message_lookup: RwLock<HashMap<u64, u16>>,
  field_lookup: RwLock<HashMap<u64, u16>>,

  // Atomic counters for lock-free fast path when possible
  target_count: AtomicU16,
  message_count: AtomicU16,
  field_count: AtomicU16,
}

impl StringInterner {
  pub fn new() -> Self {
    Self {
      targets: RwLock::new(Vec::with_capacity(256)),
      messages: RwLock::new(Vec::with_capacity(4096)),
      fields: RwLock::new(Vec::with_capacity(512)),
      target_lookup: RwLock::new(HashMap::with_capacity(256)),
      message_lookup: RwLock::new(HashMap::with_capacity(4096)),
      field_lookup: RwLock::new(HashMap::with_capacity(512)),
      target_count: AtomicU16::new(0),
      message_count: AtomicU16::new(0),
      field_count: AtomicU16::new(0),
    }
  }

  #[inline]
  pub fn intern_target(&self, string: &str) -> u16 {
    let hash = self.fast_hash(string);

    // Fast path: check thread-local cache first
    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_target(hash) {
          return id;
        }
      }

      // Cache miss - use slower interning with locks
      let id = self.intern_string_slow(
        string,
        &self.targets,
        &self.target_lookup,
        &self.target_count,
      );

      unsafe {
        (*cache_ptr).put_target(hash, id);
      }

      id
    })
  }

  #[inline]
  pub fn intern_message(&self, string: &str) -> u16 {
    let hash = self.fast_hash(string);

    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_message(hash) {
          return id;
        }
      }

      let id = self.intern_string_slow(
        string,
        &self.messages,
        &self.message_lookup,
        &self.message_count,
      );

      unsafe {
        (*cache_ptr).put_message(hash, id);
      }

      id
    })
  }

  #[inline]
  pub fn intern_field(&self, string: &str) -> u16 {
    let hash = self.fast_hash(string);

    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_field(hash) {
          return id;
        }
      }

      let id = self.intern_string_slow(string, &self.fields, &self.field_lookup, &self.field_count);

      unsafe {
        (*cache_ptr).put_field(hash, id);
      }

      id
    })
  }

  // Slow path with locking - only called on cache misses
  #[cold]
  fn intern_string_slow(
    &self,
    string: &str,
    storage: &RwLock<Vec<Arc<str>>>,
    lookup: &RwLock<HashMap<u64, u16>>,
    counter: &AtomicU16,
  ) -> u16 {
    let hash = self.fast_hash(string);

    // Try read lock first
    if let Ok(lookup_guard) = lookup.read() {
      if let Some(&id) = lookup_guard.get(&hash) {
        return id;
      }
    }

    // Need write lock
    let mut lookup_guard = lookup.write().unwrap();

    // Double-check after acquiring write lock
    if let Some(&id) = lookup_guard.get(&hash) {
      return id;
    }

    let mut storage_guard = storage.write().unwrap();
    let id = storage_guard.len() as u16;

    if id == u16::MAX {
      return 0;
    }

    storage_guard.push(Arc::from(string));
    lookup_guard.insert(hash, id);
    counter.store(id + 1, Ordering::Relaxed);

    id
  }

  pub fn get_target(&self, id: u16) -> Option<Arc<str>> {
    self.targets.read().unwrap().get(id as usize).cloned()
  }

  pub fn get_message(&self, id: u16) -> Option<Arc<str>> {
    self.messages.read().unwrap().get(id as usize).cloned()
  }

  pub fn get_field(&self, id: u16) -> Option<Arc<str>> {
    self.fields.read().unwrap().get(id as usize).cloned()
  }

  pub fn stats(&self) -> (usize, usize, usize) {
    (
      self.target_count.load(Ordering::Relaxed) as usize,
      self.message_count.load(Ordering::Relaxed) as usize,
      self.field_count.load(Ordering::Relaxed) as usize,
    )
  }

  /// Optimized FNV-1a hash - slightly faster than the previous version
  #[inline]
  fn fast_hash(&self, s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    let bytes = s.as_bytes();

    // Process 8 bytes at a time when possible
    let chunks = bytes.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
      // Process 8 bytes as u64 (unsafe but fast)
      let chunk_u64 = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const u64) };
      hash ^= chunk_u64;
      hash = hash.wrapping_mul(0x100000001b3);
    }

    // Process remaining bytes
    for &byte in remainder {
      hash ^= byte as u64;
      hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
  }
}
