//! # String Interner
//!
//! A high-performance string interning system optimized for multi-threaded logging applications.
//! This interner reduces memory usage by storing each unique string only once and provides
//! fast lookups using small integer IDs instead of string comparisons.
//!
//! ## Features
//!
//! - **Thread-local caching**: Fast, lock-free access for frequently used strings
//! - **Category-specific optimization**: Separate storage pools for targets, messages, and fields
//! - **Lock-free fast path**: Thread-local cache avoids locking on hot paths
//! - **Optimized hashing**: Custom FNV-1a implementation processing 8 bytes at a time
//! - **Memory efficient**: Uses `u16` IDs supporting up to 65,535 unique strings per category
//!
//! ## Usage
//!
//! ```rust
//! use ttlog::string_interner::StringInterner;
//! let interner = StringInterner::new();
//!
//! // Intern strings and get compact IDs
//! let target_id = interner.intern_target("database");
//! let message_id = interner.intern_message("Connection timeout");
//! let field_id = interner.intern_field("timestamp");
//!
//! // Retrieve original strings
//! let target = interner.get_target(target_id).unwrap();
//! let message = interner.get_message(message_id).unwrap();
//! assert_eq!(target.as_ref(), "database");
//! assert_eq!(message.as_ref(), "Connection timeout");
//! let _ = field_id; // silence unused variable in example
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Cache hit**: O(1) with no locks (thread-local lookup)
//! - **Cache miss**: O(1) average with read/write locks
//! - **Memory overhead**: ~24 bytes per unique string + 16 bytes per cache entry
//! - **Thread safety**: Full concurrent access with optimistic locking

mod __test__;

use std::{
  cell::UnsafeCell,
  collections::HashMap,
  sync::atomic::{AtomicU16, Ordering},
  sync::{Arc, RwLock},
};

/// Thread-local cache for recently accessed string IDs.
///
/// Uses round-robin eviction to maintain a small, fast cache of hash->ID mappings.
/// Each cache is sized based on expected usage patterns:
/// - Targets: 8 entries (typically few unique targets)
/// - Messages: 16 entries (more variety in log messages)
/// - Fields: 8 entries (limited set of field names)
#[derive(Debug)]
struct LocalCache {
  /// Cache for target strings (hash, id) pairs
  target_cache: [(u64, u16); 8],
  /// Cache for message strings - larger due to higher variety
  message_cache: [(u64, u16); 16],
  /// Cache for field name strings
  field_cache: [(u64, u16); 8],

  /// Round-robin eviction counters for each cache type
  target_counter: u8,
  message_counter: u8,
  field_counter: u8,
}

impl LocalCache {
  /// Creates a new empty cache with all entries initialized to (0, 0).
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

  /// Looks up a target string ID by hash.
  ///
  /// Returns `Some(id)` if found in cache, `None` otherwise.
  /// Time complexity: O(n) where n is cache size (8), but very fast in practice.
  fn get_target(&self, hash: u64) -> Option<u16> {
    self
      .target_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  /// Stores a target string hash->ID mapping in the cache.
  ///
  /// Uses round-robin eviction: overwrites the oldest entry when cache is full.
  fn put_target(&mut self, hash: u64, id: u16) {
    let idx = self.target_counter as usize % 8;
    self.target_cache[idx] = (hash, id);
    self.target_counter = self.target_counter.wrapping_add(1);
  }

  /// Looks up a message string ID by hash.
  fn get_message(&self, hash: u64) -> Option<u16> {
    self
      .message_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  /// Stores a message string hash->ID mapping in the cache.
  fn put_message(&mut self, hash: u64, id: u16) {
    let idx = self.message_counter as usize % 16;
    self.message_cache[idx] = (hash, id);
    self.message_counter = self.message_counter.wrapping_add(1);
  }

  /// Looks up a field string ID by hash.
  fn get_field(&self, hash: u64) -> Option<u16> {
    self
      .field_cache
      .iter()
      .find(|(h, _)| *h == hash)
      .map(|(_, id)| *id)
  }

  /// Stores a field string hash->ID mapping in the cache.
  fn put_field(&mut self, hash: u64, id: u16) {
    let idx = self.field_counter as usize % 8;
    self.field_cache[idx] = (hash, id);
    self.field_counter = self.field_counter.wrapping_add(1);
  }
}

/// Thread-local storage for the cache.
///
/// Each thread gets its own cache instance to avoid contention.
/// Uses `UnsafeCell` for interior mutability - safe because each thread
/// accesses only its own cache.
thread_local! {
    static LOCAL_CACHE: UnsafeCell<LocalCache> = UnsafeCell::new(LocalCache::new());
}

/// High-performance string interner with thread-local caching.
///
/// The interner maintains separate storage pools for different string categories,
/// each optimized for typical usage patterns in logging systems:
///
/// - **Targets**: Log destinations, routing keys (256 capacity)
/// - **Messages**: Log message content (4096 capacity)  
/// - **Fields**: Structured field names (512 capacity)
///
/// ## Thread Safety
///
/// The interner is fully thread-safe using a combination of:
/// - Thread-local caches for lock-free fast paths
/// - RwLocks for shared data structures
/// - Atomic counters for statistics
///
/// ## Memory Layout
///
/// Each category uses:
/// - `Vec<Arc<str>>`: String storage (indexed by ID)
/// - `HashMap<u64, u16>`: Hash->ID lookup table
/// - `AtomicU16`: Count of interned strings
///
/// ## Performance Notes
///
/// - Cache hits require no synchronization (fastest path)
/// - Cache misses use read locks first, write locks only for new strings
/// - Custom hash function processes 8 bytes at a time for speed
/// - `#[cold]` attribute on slow path helps branch prediction
#[derive(Debug)]
pub struct StringInterner {
  // Separate storage vectors for each string category
  targets: RwLock<Vec<Arc<str>>>,
  messages: RwLock<Vec<Arc<str>>>,
  fields: RwLock<Vec<Arc<str>>>,

  // Hash-based lookup tables for existing strings
  target_lookup: RwLock<HashMap<u64, u16>>,
  message_lookup: RwLock<HashMap<u64, u16>>,
  field_lookup: RwLock<HashMap<u64, u16>>,

  // Lock-free counters for statistics and capacity checks
  target_count: AtomicU16,
  message_count: AtomicU16,
  field_count: AtomicU16,
}

impl StringInterner {
  /// Creates a new string interner with pre-allocated capacity.
  ///
  /// Capacities are sized for typical logging workloads:
  /// - 256 targets (log destinations, routing keys)
  /// - 4096 messages (high variety log content)
  /// - 512 fields (structured logging field names)
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

  /// Interns a target string and returns its unique ID.
  ///
  /// Target strings are typically log destinations, routing keys, or other
  /// infrastructure identifiers with low cardinality.
  ///
  /// ## Performance
  /// - **Cache hit**: ~1-2ns (no locks, linear search of 8 entries)
  /// - **Cache miss**: ~100-500ns (requires locking and hash table lookup)
  ///
  /// ## Examples
  /// ```rust
  /// use ttlog::string_interner::StringInterner;
  /// let interner = StringInterner::new();
  /// let id1 = interner.intern_target("database");
  /// let id2 = interner.intern_target("database"); // Same ID returned
  /// assert_eq!(id1, id2);
  /// ```
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

  /// Interns a message string and returns its unique ID.
  ///
  /// Message strings are typically log message content with high variety.
  /// The message cache is larger (16 entries) to accommodate this variety.
  ///
  /// ## Examples
  /// ```rust
  /// use ttlog::string_interner::StringInterner;
  /// let interner = StringInterner::new();
  /// let id = interner.intern_message("Connection timeout occurred");
  /// ```
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

  /// Interns a field name string and returns its unique ID.
  ///
  /// Field strings are typically structured logging field names like
  /// "timestamp", "level", "module", etc. with moderate cardinality.
  ///
  /// ## Examples
  /// ```rust
  /// use ttlog::string_interner::StringInterner;
  /// let interner = StringInterner::new();
  /// let id = interner.intern_field("timestamp");
  /// ```
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

  /// Slow path for string interning when cache misses occur.
  ///
  /// This method is marked `#[cold]` to optimize the common cache-hit case.
  /// It implements a double-checked locking pattern:
  /// 1. Try read lock first (multiple threads can read concurrently)
  /// 2. If not found, acquire write lock
  /// 3. Double-check after write lock (another thread may have inserted)
  /// 4. Insert new string if still not found
  ///
  /// ## Error Handling
  /// Returns ID 0 if the maximum number of strings (65,535) is reached.
  /// This is a degenerate case that should rarely occur in practice.
  #[cold]
  fn intern_string_slow(
    &self,
    string: &str,
    storage: &RwLock<Vec<Arc<str>>>,
    lookup: &RwLock<HashMap<u64, u16>>,
    counter: &AtomicU16,
  ) -> u16 {
    let hash = self.fast_hash(string);

    // Try read lock first - allows concurrent reads
    if let Ok(lookup_guard) = lookup.read() {
      if let Some(&id) = lookup_guard.get(&hash) {
        return id;
      }
    }

    // Need write lock for insertion
    let mut lookup_guard = lookup.write().unwrap();

    // Double-check after acquiring write lock (race condition protection)
    if let Some(&id) = lookup_guard.get(&hash) {
      return id;
    }

    let mut storage_guard = storage.write().unwrap();
    let id = storage_guard.len() as u16;

    // Handle overflow case (extremely rare)
    if id == u16::MAX {
      return 0;
    }

    // Insert new string
    storage_guard.push(Arc::from(string));
    lookup_guard.insert(hash, id);
    counter.store(id + 1, Ordering::Relaxed);

    id
  }

  /// Retrieves the original target string for a given ID.
  ///
  /// Returns `None` if the ID is invalid or out of bounds.
  ///
  /// ## Examples
  /// ```rust
  /// use ttlog::string_interner::StringInterner;
  /// let interner = StringInterner::new();
  /// let id = interner.intern_target("database");
  /// let string = interner.get_target(id).unwrap();
  /// assert_eq!(string.as_ref(), "database");
  /// ```
  pub fn get_target(&self, id: u16) -> Option<Arc<str>> {
    self.targets.read().unwrap().get(id as usize).cloned()
  }

  /// Retrieves the original message string for a given ID.
  ///
  /// Returns `None` if the ID is invalid or out of bounds.
  pub fn get_message(&self, id: u16) -> Option<Arc<str>> {
    self.messages.read().unwrap().get(id as usize).cloned()
  }

  /// Retrieves the original field string for a given ID.
  ///
  /// Returns `None` if the ID is invalid or out of bounds.
  pub fn get_field(&self, id: u16) -> Option<Arc<str>> {
    self.fields.read().unwrap().get(id as usize).cloned()
  }

  /// Returns statistics about the number of interned strings in each category.
  ///
  /// Returns a tuple of (target_count, message_count, field_count).
  /// These counts are eventually consistent due to lock-free atomic access.
  ///
  /// ## Examples
  /// ```rust
  /// use ttlog::string_interner::StringInterner;
  /// let interner = StringInterner::new();
  /// let (_targets, _messages, _fields) = interner.stats();
  /// ```
  pub fn stats(&self) -> (usize, usize, usize) {
    (
      self.target_count.load(Ordering::Relaxed) as usize,
      self.message_count.load(Ordering::Relaxed) as usize,
      self.field_count.load(Ordering::Relaxed) as usize,
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
