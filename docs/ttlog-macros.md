# `ttlog-macros` Crate

The `ttlog-macros` crate provides the primary user-facing API for logging. These procedural macros are designed to be ergonomic and extremely fast, offloading as much work as possible to compile time.

## Available Macros

The crate provides a macro for each standard log level:

- `trace!`
- `debug!`
- `info!`
- `warn!`
- `error!`
- `fatal!`

## Usage

The macros are flexible and can be called in several ways.

### 1. Message Only

This is the simplest form, equivalent to a standard `println!`.

```rust
use ttlog_macros::info;

info!("User successfully authenticated.");
```

### 2. Message with Structured Data (Key-Value Pairs)

You can add structured context to your logs using `key = value` syntax. The message string is the last string literal in the argument list.

```rust
use ttlog_macros::warn;

let user_id = 123;
let attempt_count = 3;
warn!(user_id = user_id, attempts = attempt_count, "Failed login attempt");
```

### 3. Structured Data Only

Sometimes, the key-value pairs are self-explanatory, and a message is not needed.

```rust
use ttlog_macros::debug;

let db_latency_ms = 55;
let query = "SELECT * FROM users";
debug!(latency = db_latency_ms, query = query);
```

### 4. Event-Only Logging

You can also log an event without a message or any specific data. This is useful for tracking occurrences.

```rust
use ttlog_macros::trace;

trace!("Entering critical section");
// ... some code ...
trace!(); // Log that we've entered the section
```

## How It Works: Performance Optimizations

The macros are more than just syntactic sugar. They perform significant optimizations at compile time to reduce runtime overhead to a minimum.

1.  **Static String Interning**: The macros parse the log arguments at compile time. The message string (e.g., `"User logged in"`) and key names (e.g., `"user_id"`) are known. The macro generates code that interns these strings only once during the entire application's lifecycle using `std::sync::OnceLock`. Subsequent calls to the same log statement are simple integer lookups.

2.  **Static Metadata**: Information like the file path, module path, and line number are captured at compile time using `file!()`, `module_path!()`, and `line!()`. This avoids the need to gather this information at runtime.

3.  **Efficient Value Serialization**: When you provide key-value pairs, the values are serialized into a compact binary format. The macros generate specialized code to handle primitive integer types (`i64`, `u64`) and floating-point types (`f32`, `f64`) particularly efficiently, converting them to strings without going through the standard `serde` machinery.

4.  **Direct Logger Interaction**: The expanded macro code calls directly into the `ttlog` crate's `GLOBAL_LOGGER`, bypassing layers of abstraction that might be present in other logging libraries.

The end result is that a call to `info!` in your code expands to a highly optimized block of code that is tailored to the specific arguments you provided.
