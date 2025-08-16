# RFC: Lock-Free Ring Buffer for High-Performance Telemetry

## Summary
Replace the current `VecDeque`-based ring buffer with a lock-free implementation capable of >1M ops/sec with multiple producers and a single consumer.

## Motivation
Current ring buffer limitations:
- Uses `VecDeque` which requires locking for thread safety
- Poor cache locality due to dynamic allocation
- Cannot handle multiple producers efficiently  
- Blocks writers when buffer is full

Target performance:
- **1M+ events/second** sustained throughput
- **<100ns** hot path latency (current: ~500ns)
- **Multiple producers** without coordination
- **Non-blocking writers** with configurable back-pressure

## Detailed Design

### Architecture Overview

```rust
pub struct LockFreeRingBuffer<T> {
    // Pre-allocated circular buffer
    buffer: Vec<AtomicPtr<T>>,
    capacity: usize,
    
    // Atomic counters for lock-free coordination
    write_index: AtomicUsize,
    read_index: AtomicUsize,
    
    // Memory reclamation using epoch-based RCU
    epoch: epoch::Collector,
    
    // Back-pressure policy
    policy: BackPressurePolicy,
}

#[derive(Clone)]
pub enum BackPressurePolicy {
    Block,              // Block writers when full
    DropOldest,         // Overwrite oldest entries (current behavior)  
    DropNewest,         // Drop new entries when full
    SpillToDisk(PathBuf), // Write to disk overflow buffer
}
```

### Core Operations

#### Lock-Free Push (Multiple Producers)
```rust
impl<T> LockFreeRingBuffer<T> {
    pub fn push(&self, item: T) -> Result<(), PushError<T>> {
        let guard = self.epoch.pin();
        
        loop {
            let write_pos = self.write_index.load(Ordering::Acquire);
            let read_pos = self.read_index.load(Ordering::Acquire);
            
            // Check if buffer is full
            let next_write = (write_pos + 1) % self.capacity;
            if next_write == read_pos {
                return self.handle_full_buffer(item);
            }
            
            // Try to claim this slot
            let expected = ptr::null_mut();
            let item_ptr = Box::into_raw(Box::new(item));
            
            let slot = &self.buffer[write_pos % self.capacity];
            match slot.compare_exchange_weak(
                expected,
                item_ptr,
                Ordering::Release,
                Ordering::Relaxed
            ) {
                Ok(_) => {
                    // Successfully claimed slot, advance write pointer
                    let _ = self.write_index.compare_exchange_weak(
                        write_pos,
                        next_write,
                        Ordering::Release,
                        Ordering::Relaxed
                    );
                    return Ok(());
                }
                Err(_) => {
                    // Slot was claimed by another producer, retry
                    Box::from_raw(item_ptr); // Clean up
                    std::hint::spin_loop();
                    continue;
                }
            }
        }
    }
}
```

#### Batch Consumer (Single Consumer)
```rust
impl<T> LockFreeRingBuffer<T> {
    pub fn take_batch(&self, max_items: usize) -> Vec<T> {
        let mut batch = Vec::with_capacity(max_items);
        let guard = self.epoch.pin();
        
        let mut read_pos = self.read_index.load(Ordering::Acquire);
        let write_pos = self.write_index.load(Ordering::Acquire);
        
        while batch.len() < max_items && read_pos != write_pos {
            let slot = &self.buffer[read_pos % self.capacity];
            let item_ptr = slot.load(Ordering::Acquire);
            
            if !item_ptr.is_null() {
                // Atomically take ownership
                if let Ok(_) = slot.compare_exchange(
                    item_ptr,
                    ptr::null_mut(),
                    Ordering::Release,
                    Ordering::Relaxed
                ) {
                    unsafe {
                        batch.push(*Box::from_raw(item_ptr));
                    }
                    
                    read_pos = (read_pos + 1) % self.capacity;
                }
            } else {
                break; // No more items
            }
        }
        
        self.read_index.store(read_pos, Ordering::Release);
        batch
    }
}
```

### Memory Management Strategy

#### Epoch-Based Reclamation
- Use `crossbeam_epoch` for safe memory reclamation
- Defer deallocation until all readers finish
- Batch reclamation to reduce overhead

#### Cache-Friendly Layout
```rust
// Align buffer to cache line boundaries
#[repr(align(64))]
pub struct CacheAligned<T>(T);

// Separate read/write indices to avoid false sharing
#[repr(align(64))]
struct ProducerState {
    write_index: AtomicUsize,
    // Pad to full cache line
    _padding: [u8; 64 - std::mem::size_of::<AtomicUsize>()],
}

#[repr(align(64))]  
struct ConsumerState {
    read_index: AtomicUsize,
    _padding: [u8; 64 - std::mem::size_of::<AtomicUsize>()],
}
```

### Back-Pressure Handling

#### Configurable Policies
```rust
impl<T> LockFreeRingBuffer<T> {
    fn handle_full_buffer(&self, item: T) -> Result<(), PushError<T>> {
        match &self.policy {
            BackPressurePolicy::Block => {
                // Use futex-style parking for efficient blocking
                self.wait_for_space();
                self.push(item) // Retry
            }
            
            BackPressurePolicy::DropOldest => {
                // Advance read pointer to make space
                let read_pos = self.read_index.load(Ordering::Acquire);
                let new_read = (read_pos + 1) % self.capacity;
                self.read_index.store(new_read, Ordering::Release);
                self.push(item) // Retry
            }
            
            BackPressurePolicy::DropNewest => {
                Err(PushError::BufferFull(item))
            }
            
            BackPressurePolicy::SpillToDisk(path) => {
                self.spill_to_disk(item, path)?;
                Ok(())
            }
        }
    }
}
```

### Performance Optimizations

#### SIMD Batching
```rust
// Process multiple events in SIMD registers
pub fn push_batch(&self, items: &[T]) -> Result<usize, BatchPushError> {
    // Use SIMD to process metadata in parallel
    let timestamps: Vec<u64> = items.iter()
        .map(|item| item.timestamp())
        .collect();
    
    // Vectorized validation and processing
    self.validate_batch_simd(&timestamps)?;
    
    // Batch insert with fewer atomic operations
    let start_pos = self.reserve_batch_space(items.len())?;
    self.write_batch_unchecked(start_pos, items);
    
    Ok(items.len())
}
```

#### CPU Cache Optimization  
```rust
// Prefetch next cache lines during iteration
use std::intrinsics::prefetch_read_data;

impl<T> Iterator for RingBufferIter<'_, T> {
    fn next(&mut self) -> Option<&T> {
        if self.pos < self.end {
            // Prefetch next item
            if self.pos + 4 < self.end {
                unsafe {
                    prefetch_read_data(
                        self.buffer.get_unchecked(self.pos + 4) as *const _,
                        0 // temporal locality hint
                    );
                }
            }
            
            let item = unsafe { self.buffer.get_unchecked(self.pos) };
            self.pos += 1;
            Some(item)
        } else {
            None
        }
    }
}
```

## Benchmarks & Validation

### Performance Tests
```rust
#[cfg(test)]
mod benchmarks {
    use criterion::{criterion_group, criterion_main, Criterion};
    
    fn bench_single_producer_throughput(c: &mut Criterion) {
        let buffer = LockFreeRingBuffer::new(1024);
        
        c.bench_function("single_producer_1M_ops", |b| {
            b.iter(|| {
                for i in 0..1_000_000 {
                    buffer.push(Event::new(i, "INFO", "test", "bench")).unwrap();
                }
            })
        });
    }
    
    fn bench_multi_producer_contention(c: &mut Criterion) {
        let buffer = Arc::new(LockFreeRingBuffer::new(1024));
        
        c.bench_function("4_producers_contention", |b| {
            b.iter(|| {
                let handles: Vec<_> = (0..4).map(|thread_id| {
                    let buf = buffer.clone();
                    thread::spawn(move || {
                        for i in 0..250_000 {
                            buf.push(Event::new(i, "INFO", "test", &thread_id.to_string())).unwrap();
                        }
                    })
                }).collect();
                
                for handle in handles {
                    handle.join().unwrap();
                }
            })
        });
    }
}
```

### Correctness Validation
```rust
#[cfg(test)]
mod stress_tests {
    // Property-based testing with proptest
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn no_data_loss(ops in prop::collection::vec(op_strategy(), 1..10000)) {
            let buffer = LockFreeRingBuffer::new(1024);
            let mut expected_items = Vec::new();
            
            for op in ops {
                match op {
                    Op::Push(item) => {
                        if buffer.push(item.clone()).is_ok() {
                            expected_items.push(item);
                        }
                    }
                    Op::TakeBatch(n) => {
                        let batch = buffer.take_batch(n);
                        for item in batch {
                            let pos = expected_items.iter().position(|x| x == &item);
                            prop_assert!(pos.is_some(), "Unexpected item: {:?}", item);
                            expected_items.remove(pos.unwrap());
                        }
                    }
                }
            }
        }
    }
}
```

## Migration Strategy

### Phase 1: Drop-in Replacement
```rust
// Current API compatibility
impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        // Internally use LockFreeRingBuffer
        Self {
            inner: LockFreeRingBuffer::new(capacity, BackPressurePolicy::DropOldest),
            _phantom: PhantomData,
        }
    }
    
    pub fn push(&mut self, item: T) {
        // Single-threaded compatibility mode
        self.inner.push(item).unwrap_or_else(|_| {
            // Handle back-pressure according to current behavior
        });
    }
    
    pub fn take_snapshot(&mut self) -> Vec<T> {
        self.inner.take_batch(usize::MAX)
    }
}
```

### Phase 2: Enhanced API
```rust
// New multi-threaded API
impl<T> LockFreeRingBuffer<T> {
    pub fn new_shared(capacity: usize, policy: BackPressurePolicy) -> Arc<Self>;
    pub fn clone_producer(&self) -> ProducerHandle<T>;
    pub fn clone_consumer(&self) -> ConsumerHandle<T>; 
    
    // Async support
    pub async fn push_async(&self, item: T) -> Result<(), PushError<T>>;
    pub async fn take_batch_async(&self, max_items: usize) -> Vec<T>;
}
```

## Future Extensions

### NUMA Awareness
- Detect NUMA topology
- Place buffers on same NUMA node as producers  
- Use per-NUMA-node sub-buffers for better locality

### Persistent Storage Integration
- WAL (Write-Ahead Log) for durability
- Memory-mapped files for zero-copy persistence
- Compression aware ring buffer layout

### Telemetry & Monitoring
- Built-in metrics (throughput, latency, drops)
- Integration with `ttlog`'s own telemetry system
- Adaptive sizing based on load patterns

## Success Criteria

### Performance Targets
- [ ] **Throughput**: 1M+ ops/sec single producer
- [ ] **Latency**: p99 < 100ns (current: ~500ns)  
- [ ] **Multi-producer**: 500K+ ops/sec with 4 producers
- [ ] **Memory**: <10MB overhead for 1M capacity buffer

### Correctness Goals
- [ ] Zero data loss under normal conditions
- [ ] No memory leaks under stress testing
- [ ] ABA problem resistance via epoch-based RCU
- [ ] No deadlocks or livelocks in any scenario

### API Compatibility
- [ ] Drop-in replacement for existing `RingBuffer`
- [ ] Graceful degradation when back-pressure hits
- [ ] Clear error handling for all failure modes
- [ ] Extensive documentation and examples

---

**Implementation Timeline**: 4-6 weeks
**Dependencies**: `crossbeam-epoch`, `criterion` (benchmarking)
**Breaking Changes**: None (initially)
