mod __test__;

use smallvec::SmallVec;
use std::{
  cell::UnsafeCell,
  collections::HashMap,
  sync::atomic::{AtomicU16, Ordering},
  sync::{Arc, RwLock},
};

/// SEC-009: the lookup maps key on a 64-bit FNV hash. FNV is not
/// collision-resistant, so an attacker-influenced string can be crafted to
/// collide with an already-interned one. Each hash bucket therefore holds a
/// *list* of candidate ids; the slow path verifies `storage[id] == input`
/// before reusing an id and falls through to insert a fresh entry on a true
/// collision. This makes the interner collision-safe at the cost of a short
/// linear scan over the (normally length-1) bucket.
type IdBucket = SmallVec<[u16; 2]>;

#[derive(Debug)]
struct LocalCache {
  target_cache: [(u64, u16); 64],
  message_cache: [(u64, u16); 64],
  file_cache: [(u64, u16); 64],
  kv_cache: [(u64, u16); 64],
  target_counter: u8,
  message_counter: u8,
  file_counter: u8,
  kv_counter: u8,
}

impl LocalCache {
  fn new() -> Self {
    Self {
      target_cache: [(0, 0); 64],
      message_cache: [(0, 0); 64],
      file_cache: [(0, 0); 64],
      kv_cache: [(0, 0); 64],
      target_counter: 0,
      message_counter: 0,
      file_counter: 0,
      kv_counter: 0,
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

  fn get_file(&self, hash: u64) -> Option<u16> {
    self
      .file_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  fn put_message(&mut self, hash: u64, id: u16) {
    let idx = self.message_counter as usize % 16;
    self.message_cache[idx] = (hash, id);
    self.message_counter = self.message_counter.wrapping_add(1);
  }

  fn put_file(&mut self, hash: u64, id: u16) {
    let idx = self.file_counter as usize % 8;
    self.file_cache[idx] = (hash, id);
    self.file_counter = self.file_counter.wrapping_add(1);
  }

  fn get_kv(&self, hash: u64) -> Option<u16> {
    self
      .kv_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  fn put_kv(&mut self, hash: u64, id: u16) {
    let idx = self.kv_counter as usize % 8;
    self.kv_cache[idx] = (hash, id);
    self.kv_counter = self.kv_counter.wrapping_add(1);
  }
}

thread_local! {
    static LOCAL_CACHE: UnsafeCell<LocalCache> = UnsafeCell::new(LocalCache::new());
}

#[derive(Debug)]
pub struct StringInterner {
  targets: RwLock<Vec<Arc<str>>>,
  messages: RwLock<Vec<Arc<str>>>,
  files: RwLock<Vec<Arc<str>>>,
  kvs: RwLock<Vec<Arc<smallvec::SmallVec<[u8; 128]>>>>,

  target_lookup: RwLock<HashMap<u64, IdBucket>>,
  message_lookup: RwLock<HashMap<u64, IdBucket>>,
  file_lookup: RwLock<HashMap<u64, IdBucket>>,
  kv_lookup: RwLock<HashMap<u64, IdBucket>>,

  target_count: AtomicU16,
  message_count: AtomicU16,
  file_count: AtomicU16,
  kv_count: AtomicU16,
}

impl StringInterner {
  pub fn new() -> Self {
    let mut targets = Vec::with_capacity(256);
    let mut messages = Vec::with_capacity(4096);
    let mut files = Vec::with_capacity(512);
    let mut kvs = Vec::with_capacity(512);

    // Fill slot 0 with dummy values
    targets.push(Arc::from(""));
    messages.push(Arc::from(""));
    files.push(Arc::from(""));
    kvs.push(Arc::from(smallvec::SmallVec::new()));

    Self {
      targets: RwLock::new(targets),
      messages: RwLock::new(messages),
      files: RwLock::new(files),
      kvs: RwLock::new(kvs),

      target_lookup: RwLock::new(HashMap::with_capacity(256)),
      message_lookup: RwLock::new(HashMap::with_capacity(4096)),
      file_lookup: RwLock::new(HashMap::with_capacity(512)),
      kv_lookup: RwLock::new(HashMap::with_capacity(512)),

      // start counters at 1, so the next real string gets id = 1
      target_count: AtomicU16::new(1),
      message_count: AtomicU16::new(1),
      file_count: AtomicU16::new(1),
      kv_count: AtomicU16::new(1),
    }
  }

  #[inline]
  pub fn intern_target(&self, string: &str) -> u16 {
    let hash = self.fast_hash(string);

    // Fast path: check thread-local cache first. SEC-009: the cache is also
    // keyed on the FNV hash, so a cache hit is verified against storage to
    // reject collisions before it is trusted.
    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_target(hash) {
          if self.matches_target(id, string) {
            return id;
          }
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
          if self.matches_message(id, string) {
            return id;
          }
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
  pub fn intern_file(&self, string: &str) -> u16 {
    let hash = self.fast_hash(string);

    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_file(hash) {
          if self.matches_file(id, string) {
            return id;
          }
        }
      }

      let id = self.intern_string_slow(string, &self.files, &self.file_lookup, &self.file_count);

      unsafe {
        (*cache_ptr).put_file(hash, id);
      }

      id
    })
  }

  #[inline]
  pub fn intern_kv(&self, buf: smallvec::SmallVec<[u8; 128]>) -> u16 {
    let hash = self.fast_hash_smallvec(&buf);

    LOCAL_CACHE.with(|cache| {
      let cache_ptr = cache.get();
      unsafe {
        if let Some(id) = (*cache_ptr).get_kv(hash) {
          if self.matches_kv(id, &buf) {
            return id;
          }
        }
      }

      let id = self.intern_string_slow_smallvec(buf, &self.kvs, &self.kv_lookup, &self.kv_count);

      unsafe {
        (*cache_ptr).put_kv(hash, id);
      }

      id
    })
  }

  #[cold]
  fn intern_string_slow_smallvec(
    &self,
    string: smallvec::SmallVec<[u8; 128]>,
    storage: &RwLock<Vec<Arc<smallvec::SmallVec<[u8; 128]>>>>,
    lookup: &RwLock<HashMap<u64, IdBucket>>,
    counter: &AtomicU16,
  ) -> u16 {
    let hash = self.fast_hash_smallvec(&string);

    // Try read lock first - allows concurrent reads. SEC-009: a hash hit is
    // only a *candidate* — verify the stored bytes actually equal `string`
    // before reusing the id, otherwise a colliding payload would alias.
    if let Ok(lookup_guard) = lookup.read() {
      if let Some(bucket) = lookup_guard.get(&hash) {
        let storage_guard = storage.read().unwrap();
        for &id in bucket {
          if let Some(existing) = storage_guard.get(id as usize) {
            if existing.as_slice() == string.as_slice() {
              return id;
            }
          }
        }
      }
    }

    // Need write lock for insertion
    let mut lookup_guard = lookup.write().unwrap();
    let mut storage_guard = storage.write().unwrap();

    // Double-check after acquiring write lock (race condition protection),
    // again verifying the actual bytes to reject FNV collisions.
    if let Some(bucket) = lookup_guard.get(&hash) {
      for &id in bucket {
        if let Some(existing) = storage_guard.get(id as usize) {
          if existing.as_slice() == string.as_slice() {
            return id;
          }
        }
      }
    }

    let id = storage_guard.len() as u16;

    // Handle overflow case (extremely rare)
    if id == u16::MAX {
      return 0;
    }

    // Insert new string. The bucket holds every id that shares this hash so a
    // genuine collision keeps a distinct id rather than overwriting.
    storage_guard.push(Arc::from(string));
    lookup_guard.entry(hash).or_default().push(id);
    counter.store(id + 1, Ordering::Relaxed);

    id
  }

  #[cold]
  fn intern_string_slow(
    &self,
    string: &str,
    storage: &RwLock<Vec<Arc<str>>>,
    lookup: &RwLock<HashMap<u64, IdBucket>>,
    counter: &AtomicU16,
  ) -> u16 {
    let hash = self.fast_hash(string);

    // Try read lock first - allows concurrent reads. SEC-009: a hash hit is
    // only a *candidate* — verify the stored string actually equals `string`
    // before reusing the id, otherwise a colliding input would alias.
    if let Ok(lookup_guard) = lookup.read() {
      if let Some(bucket) = lookup_guard.get(&hash) {
        let storage_guard = storage.read().unwrap();
        for &id in bucket {
          if let Some(existing) = storage_guard.get(id as usize) {
            if existing.as_ref() == string {
              return id;
            }
          }
        }
      }
    }

    // Need write lock for insertion
    let mut lookup_guard = lookup.write().unwrap();
    let mut storage_guard = storage.write().unwrap();

    // Double-check after acquiring write lock (race condition protection),
    // again verifying the actual string to reject FNV collisions.
    if let Some(bucket) = lookup_guard.get(&hash) {
      for &id in bucket {
        if let Some(existing) = storage_guard.get(id as usize) {
          if existing.as_ref() == string {
            return id;
          }
        }
      }
    }

    let id = storage_guard.len() as u16;

    // Handle overflow case (extremely rare)
    if id == u16::MAX {
      return 0;
    }

    // Insert new string. The bucket holds every id that shares this hash so a
    // genuine collision keeps a distinct id rather than overwriting.
    storage_guard.push(Arc::from(string));
    lookup_guard.entry(hash).or_default().push(id);
    counter.store(id + 1, Ordering::Relaxed);

    id
  }

  /// SEC-009 collision guards: confirm the id resolved from a hash lookup
  /// actually stores the queried value before it is trusted.
  #[inline]
  fn matches_target(&self, id: u16, string: &str) -> bool {
    self
      .targets
      .read()
      .unwrap()
      .get(id as usize)
      .is_some_and(|s| s.as_ref() == string)
  }

  #[inline]
  fn matches_message(&self, id: u16, string: &str) -> bool {
    self
      .messages
      .read()
      .unwrap()
      .get(id as usize)
      .is_some_and(|s| s.as_ref() == string)
  }

  #[inline]
  fn matches_file(&self, id: u16, string: &str) -> bool {
    self
      .files
      .read()
      .unwrap()
      .get(id as usize)
      .is_some_and(|s| s.as_ref() == string)
  }

  #[inline]
  fn matches_kv(&self, id: u16, buf: &smallvec::SmallVec<[u8; 128]>) -> bool {
    self
      .kvs
      .read()
      .unwrap()
      .get(id as usize)
      .is_some_and(|s| s.as_slice() == buf.as_slice())
  }

  pub fn get_file(&self, id: u16) -> Option<Arc<str>> {
    self.files.read().unwrap().get(id as usize).cloned()
  }

  pub fn get_target(&self, id: u16) -> Option<Arc<str>> {
    self.targets.read().unwrap().get(id as usize).cloned()
  }

  pub fn get_message(&self, id: u16) -> Option<Arc<str>> {
    self.messages.read().unwrap().get(id as usize).cloned()
  }

  pub fn get_kv(&self, id: u16) -> Option<Arc<smallvec::SmallVec<[u8; 128]>>> {
    self.kvs.read().unwrap().get(id as usize).cloned()
  }

  pub fn stats(&self) -> (usize, usize, usize) {
    (
      self.target_count.load(Ordering::Relaxed) as usize,
      self.message_count.load(Ordering::Relaxed) as usize,
      self.kv_count.load(Ordering::Relaxed) as usize,
    )
  }

  /// Optimized FNV-1a hash function.
  ///
  /// Processes input strings 8 bytes at a time for improved performance
  /// over byte-by-byte hashing. Uses unaligned reads to handle arbitrary
  /// string lengths efficiently.
  ///
  /// ## Algorithm
  /// - Starts with FNV-1a offset basis: 0xcbf29ce484222325
  /// - For each 8-byte chunk: hash ^= chunk; hash *= FNV_PRIME
  /// - For remaining bytes: hash ^= byte; hash *= FNV_PRIME
  ///
  /// ## Safety
  /// Uses `unsafe` for unaligned u64 reads, but this is safe because:
  /// - Input comes from valid string slices
  /// - `read_unaligned` handles arbitrary alignment
  /// - Chunk size is validated by `chunks_exact(8)`
  #[inline]
  fn fast_hash_smallvec(&self, s: &smallvec::SmallVec<[u8; 128]>) -> u64 {
    let mut hash = 0xcbf29ce484222325u64; // FNV offset basis
    let bytes: &[u8] = s.as_slice();

    // Process 8 bytes at a time
    let chunks = bytes.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
      // Convert 8-byte chunk into u64 (little endian for consistency)
      let chunk_u64 = u64::from_le_bytes(chunk.try_into().unwrap());
      hash ^= chunk_u64;
      hash = hash.wrapping_mul(0x100000001b3);
    }

    // Process remaining bytes (0–7)
    for &byte in remainder {
      hash ^= byte as u64;
      hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
  }

  #[inline]
  fn fast_hash(&self, s: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64; // FNV offset basis
    let bytes = s.as_bytes();

    // Process 8 bytes at a time for better performance
    let chunks = bytes.chunks_exact(8);
    let remainder = chunks.remainder();

    for chunk in chunks {
      // SAFETY: chunk is guaranteed to be exactly 8 bytes by chunks_exact(8)
      let chunk_u64 = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const u64) };
      hash ^= chunk_u64;
      hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }

    // Process remaining bytes (0-7 bytes)
    for &byte in remainder {
      hash ^= byte as u64;
      hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
  }
}

impl Default for StringInterner {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
impl StringInterner {
  /// Test-only: expose the FNV hash so collision tests can search for, or
  /// assert on, hash equality without depending on private internals.
  pub(crate) fn hash_str(&self, s: &str) -> u64 {
    self.fast_hash(s)
  }

  /// Test-only: directly seed a collision into the target lookup — interns
  /// `victim` normally, then forces `attacker` to share the *same* hash
  /// bucket, simulating an FNV collision. Returns `(victim_id, attacker_id)`.
  /// The interner must still hand back distinct ids and resolve each id to
  /// its own string.
  pub(crate) fn intern_target_colliding(&self, victim: &str, attacker: &str) -> (u16, u16) {
    // Intern `victim` normally first.
    self.intern_string_slow(
      victim,
      &self.targets,
      &self.target_lookup,
      &self.target_count,
    );

    // Force the collision: register `attacker` under `victim`'s hash bucket.
    let victim_hash = self.fast_hash(victim);
    {
      let mut lookup = self.target_lookup.write().unwrap();
      let mut storage = self.targets.write().unwrap();
      let attacker_id = storage.len() as u16;
      storage.push(Arc::from(attacker));
      lookup.entry(victim_hash).or_default().push(attacker_id);
      self.target_count.store(attacker_id + 1, Ordering::Relaxed);
    }

    // Look both up again *through the public hash path*: the bucket now holds
    // two ids under one hash, so the verify-against-storage logic must pick
    // the right one for each input.
    let resolved_victim = self.intern_string_slow(
      victim,
      &self.targets,
      &self.target_lookup,
      &self.target_count,
    );
    let resolved_attacker = self.intern_string_slow(
      attacker,
      &self.targets,
      &self.target_lookup,
      &self.target_count,
    );
    (resolved_victim, resolved_attacker)
  }
}
