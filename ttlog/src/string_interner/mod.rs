mod __test__;

use std::{
  collections::HashMap,
  sync::{Arc, RwLock},
};

#[derive(Debug)]
pub struct StringInterner {
  // Separate storage for different string types for better cache locality
  pub targets: RwLock<Vec<Arc<str>>>, // Common targets (module names)
  pub messages: RwLock<Vec<Arc<str>>>, // Log messages
  pub fields: RwLock<Vec<Arc<str>>>,  // Field keys/values

  // Hash maps for fast lookup
  pub target_lookup: RwLock<HashMap<u64, u16>>,
  pub message_lookup: RwLock<HashMap<u64, u16>>,
  pub field_lookup: RwLock<HashMap<u64, u16>>,
}

impl StringInterner {
  pub fn new() -> Self {
    Self {
      targets: RwLock::new(Vec::with_capacity(256)), // Most apps have <256 targets
      messages: RwLock::new(Vec::with_capacity(4096)), // More messages
      fields: RwLock::new(Vec::with_capacity(512)),  // Moderate field variety
      target_lookup: RwLock::new(HashMap::with_capacity(256)),
      message_lookup: RwLock::new(HashMap::with_capacity(4096)),
      field_lookup: RwLock::new(HashMap::with_capacity(512)),
    }
  }

  #[inline]
  pub fn intern_target(&self, string: &str) -> u16 {
    self.intern_string(string, &self.targets, &self.target_lookup)
  }

  #[inline]
  pub fn intern_message(&self, string: &str) -> u16 {
    self.intern_string(string, &self.messages, &self.message_lookup)
  }

  #[inline]
  pub fn intern_field(&self, string: &str) -> u16 {
    self.intern_string(string, &self.fields, &self.field_lookup)
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
      self.targets.read().unwrap().len(),
      self.messages.read().unwrap().len(),
      self.fields.read().unwrap().len(),
    )
  }

  #[inline]
  pub fn intern_string(
    &self,
    string: &str,
    storage: &RwLock<Vec<Arc<str>>>,
    lookup: &RwLock<HashMap<u64, u16>>,
  ) -> u16 {
    let hash = self.fast_hash(string);

    // Fast path: read lock only
    if let Ok(lookup_guard) = lookup.read() {
      if let Some(&id) = lookup_guard.get(&hash) {
        return id;
      }
    }

    // Slow path: write lock
    let mut lookup_guard = lookup.write().unwrap();

    // Double-check after acquiring write lock
    if let Some(&id) = lookup_guard.get(&hash) {
      return id;
    }

    let mut storage_guard = storage.write().unwrap();
    let id = storage_guard.len() as u16;
    println!("_______________________{:?}", hash);

    if id == u16::MAX {
      return 0;
    }

    storage_guard.push(Arc::from(string));
    lookup_guard.insert(hash, id);
    println!("{:?} {:?} {:?}", hash, id, string);
    id
  }

  /// Computes a fast, non-cryptographic hash of a string using the FNV-1a algorithm.
  ///
  /// # Algorithm
  /// This is the 64-bit FNV-1a hash:
  /// 1. Start with an **offset basis**: `0xcbf29ce484222325`.
  /// 2. For each byte in the string:
  ///    - XOR the current hash with the byte.
  ///    - Multiply the result by the **FNV prime** `0x100000001b3`.
  ///
  /// # Characteristics
  /// - **Fast:** Optimized for short strings, avoids allocations.
  /// - **Non-cryptographic:** Not suitable for security-sensitive applications.
  /// - **Deterministic:** Same string always produces the same hash.
  /// - **Low collision probability** for small datasets (suitable for interning strings).
  ///
  /// # Returns
  /// A `u64` hash value representing the input string.
  #[inline]
  fn fast_hash(&self, s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64; // FNV-1a offset basis
    for byte in s.bytes() {
      hash ^= byte as u64;
      hash = hash.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    hash
  }
}
