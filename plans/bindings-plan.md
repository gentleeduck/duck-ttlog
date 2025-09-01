# TTLog Unified Bindings Architecture
## Single Source of Truth Strategy

```
┌─────────────────────────────────────────────────────────────────┐
│                        TTLog Rust Core                         │
│                    (src/lib.rs - optimized)                    │
└─────────────────────────┬───────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────────┐
│                   Universal C ABI Layer                        │
│                      (src/c_api.rs)                            │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  • ttlog_init()           • ttlog_log_with_fields()    │    │
│  │  • ttlog_log()            • ttlog_request_snapshot()   │    │
│  │  • ttlog_set_level()      • ttlog_destroy()           │    │
│  │  • ttlog_is_enabled()     • ttlog_flush()             │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────┬───────────────────────────────┬─────────────┬─────────┘
          │                               │             │
┌─────────▼─────────┐            ┌────────▼──────┐   ┌──▼──────────────┐
│   Direct C FFI    │            │  NAPI-RS      │   │  JNI Bridge     │
│   Consumers       │            │  (Node.js)    │   │  (Java)         │
├───────────────────┤            └───────────────┘   └─────────────────┘
│ • Go (CGO)        │                    │                     │
│ • Python (ctypes) │                    │                     │
│ • C# (P/Invoke)   │                    │                     │
│ • Ruby (FFI)      │                    │                     │
│ • Pure C/C++      │                    │                     │
└───────────────────┘                    │                     │
          │                              │                     │
          └──────────────────────────────┼─────────────────────┘
                                         │
                    ┌────────────────────▼────────────────────┐
                    │        Generated Bindings              │
                    │   (Same C ABI, different wrappers)     │
                    └─────────────────────────────────────────┘
```

## Core Architecture

### 1. Single C ABI Layer (The Foundation)
**File**: `src/c_api.rs` - This is your only interface layer

```rust
// src/c_api.rs - The ONLY interface all languages use
use std::ffi::{CStr, CString, c_char, c_void, c_int, c_uint, c_ulong};
use std::ptr;
use std::sync::Once;

// Global instance management
static mut GLOBAL_LOGGER: Option<Box<crate::Trace>> = None;
static INIT_ONCE: Once = Once::new();

// Opaque handle (same for ALL languages)
#[repr(C)]
pub struct TTLogHandle {
    _private: [u8; 0],
}

// Universal error codes
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TTLogResult {
    Success = 0,
    InvalidHandle = -1,
    InvalidParameter = -2,
    BufferFull = -3,
    NotInitialized = -4,
    InternalError = -5,
}

// Core initialization (used by ALL bindings)
#[no_mangle]
pub extern "C" fn ttlog_init_global(
    capacity: c_ulong,
    channel_capacity: c_ulong
) -> TTLogResult {
    INIT_ONCE.call_once(|| {
        unsafe {
            GLOBAL_LOGGER = Some(Box::new(
                crate::Trace::init(capacity as usize, channel_capacity as usize)
            ));
        }
    });
    TTLogResult::Success
}

// Fast logging (optimized for all languages)
#[no_mangle]
pub extern "C" fn ttlog_log_fast(
    level: c_uint,
    target: *const c_char,
    message: *const c_char,
) -> TTLogResult {
    if target.is_null() || message.is_null() {
        return TTLogResult::InvalidParameter;
    }
    
    unsafe {
        if let Some(logger) = &GLOBAL_LOGGER {
            let target_str = match CStr::from_ptr(target).to_str() {
                Ok(s) => s,
                Err(_) => return TTLogResult::InvalidParameter,
            };
            
            let message_str = match CStr::from_ptr(message).to_str() {
                Ok(s) => s,
                Err(_) => return TTLogResult::InvalidParameter,
            };
            
            // Use your optimized Rust logging
            crate::log_at_level(
                crate::LogLevel::from_u32(level),
                target_str,
                message_str
            );
            
            TTLogResult::Success
        } else {
            TTLogResult::NotInitialized
        }
    }
}

// Field-based logging
#[no_mangle]
pub extern "C" fn ttlog_log_with_fields(
    level: c_uint,
    target: *const c_char,
    message: *const c_char,
    fields: *const CField,
    field_count: c_ulong,
) -> TTLogResult {
    // Implementation using your optimized field system
    TTLogResult::Success
}

// Memory management
#[no_mangle]
pub extern "C" fn ttlog_cleanup() {
    unsafe {
        GLOBAL_LOGGER = None;
    }
}

// The rest of your C API functions...
```

### 2. Language-Specific Thin Wrappers

Each language gets a **thin wrapper** that calls the same C functions but provides idiomatic APIs:

#### NAPI-RS (Node.js) - Zero Duplication
```rust
// src/nodejs.rs - Calls C API, no duplication
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi]
pub fn init_logger(capacity: u32, channel_capacity: u32) -> Result<()> {
    let result = unsafe {
        crate::c_api::ttlog_init_global(capacity as c_ulong, channel_capacity as c_ulong)
    };
    
    match result {
        crate::c_api::TTLogResult::Success => Ok(()),
        _ => Err(Error::from_reason("Failed to initialize logger")),
    }
}

#[napi]
pub fn info(message: String, target: Option<String>) -> Result<()> {
    let target = target.unwrap_or_default();
    let target_cstr = std::ffi::CString::new(target)?;
    let message_cstr = std::ffi::CString::new(message)?;
    
    let result = unsafe {
        crate::c_api::ttlog_log_fast(
            2, // Info level
            target_cstr.as_ptr(),
            message_cstr.as_ptr(),
        )
    };
    
    match result {
        crate::c_api::TTLogResult::Success => Ok(()),
        _ => Err(Error::from_reason("Logging failed")),
    }
}

// Just thin wrappers around C API - no logic duplication!
```

#### PyO3 (Python) - Zero Duplication
```rust
// src/python.rs - Also calls same C API
use pyo3::prelude::*;

#[pyfunction]
fn init_logger(capacity: usize, channel_capacity: usize) -> PyResult<()> {
    let result = unsafe {
        crate::c_api::ttlog_init_global(capacity as c_ulong, channel_capacity as c_ulong)
    };
    
    match result {
        crate::c_api::TTLogResult::Success => Ok(()),
        _ => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Init failed")),
    }
}

#[pyfunction]
fn info(message: &str, target: Option<&str>) -> PyResult<()> {
    let target = target.unwrap_or("");
    let target_cstr = std::ffi::CString::new(target)?;
    let message_cstr = std::ffi::CString::new(message)?;
    
    // Same C API call as Node.js!
    let result = unsafe {
        crate::c_api::ttlog_log_fast(2, target_cstr.as_ptr(), message_cstr.as_ptr())
    };
    
    match result {
        crate::c_api::TTLogResult::Success => Ok(()),
        _ => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Log failed")),
    }
}

#[pymodule]
fn ttlog(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init_logger, m)?)?;
    m.add_function(wrap_pyfunction!(info, m)?)?;
    Ok(())
}
```

#### Pure FFI Languages (Go, C#, Ruby, etc.)
These languages call the C API directly - **no Rust wrapper code at all!**

```go
// go/ttlog.go - Direct C API calls
package ttlog

/*
#cgo LDFLAGS: -L../target/release -lttlog
#include "../include/ttlog.h"
*/
import "C"

func InitLogger(capacity, channelCapacity int) error {
    result := C.ttlog_init_global(C.ulong(capacity), C.ulong(channelCapacity))
    if result != C.TTLogResult_Success {
        return fmt.Errorf("init failed: %d", int(result))
    }
    return nil
}

func Info(message, target string) error {
    cmsg := C.CString(message)
    defer C.free(unsafe.Pointer(cmsg))
    ctarget := C.CString(target)
    defer C.free(unsafe.Pointer(ctarget))
    
    // Same C function that Node.js and Python call!
    result := C.ttlog_log_fast(2, ctarget, cmsg)
    if result != C.TTLogResult_Success {
        return fmt.Errorf("log failed: %d", int(result))
    }
    return nil
}
```

## Build Configuration Strategy

### Single Cargo.toml with Feature Flags
```toml
[package]
name = "ttlog"
version = "0.1.0"

[lib]
name = "ttlog"
crate-type = ["cdylib", "staticlib", "rlib"]

[features]
default = ["c-api"]
c-api = []                    # Always included - the foundation
nodejs = ["napi", "napi-derive", "c-api"]
python = ["pyo3", "c-api"]
java = ["jni", "c-api"]
wasm = ["wasm-bindgen", "c-api"]

# Language-specific dependencies only when needed
[dependencies]
# Core dependencies (always included)
crossbeam-channel = "0.5"
crossbeam-queue = "0.3"
# ... your existing deps

# Language bindings (optional)
napi = { version = "2.0", optional = true }
napi-derive = { version = "2.0", optional = true }
pyo3 = { version = "0.20", optional = true, features = ["extension-module"] }
jni = { version = "0.21", optional = true }
wasm-bindgen = { version = "0.2", optional = true }

[build-dependencies]
cbindgen = "0.26"
napi-build = { version = "2.0", optional = true }
```

### Smart Build Scripts
```rust
// build.rs - Generates what's needed
fn main() {
    // Always generate C header
    generate_c_header();
    
    // Feature-specific builds
    #[cfg(feature = "nodejs")]
    napi_build::setup();
    
    #[cfg(feature = "python")]
    setup_python_build();
    
    #[cfg(feature = "java")]
    setup_java_build();
}

fn generate_c_header() {
    cbindgen::Builder::new()
        .with_crate(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .with_config(cbindgen::Config::from_file("cbindgen.toml").unwrap())
        .generate()
        .unwrap()
        .write_to_file("include/ttlog.h");
}
```

## Repository Structure
```
ttlog/
├── src/
│   ├── lib.rs              # Your core Rust implementation
│   ├── c_api.rs           # Universal C interface (single source of truth)
│   ├── nodejs.rs          # Thin NAPI wrapper
│   ├── python.rs          # Thin PyO3 wrapper  
│   ├── java.rs            # Thin JNI wrapper
│   └── wasm.rs            # Thin WASM wrapper
├── include/
│   └── ttlog.h            # Generated C header (for FFI languages)
├── bindings/
│   ├── go/                # Pure Go FFI code
│   │   ├── ttlog.go
│   │   └── go.mod
│   ├── csharp/            # Pure C# P/Invoke code
│   │   ├── TTLog.cs
│   │   └── TTLog.csproj
│   ├── ruby/              # Pure Ruby FFI code
│   │   ├── lib/ttlog.rb
│   │   └── ttlog.gemspec
│   └── c/                 # Pure C wrapper
│       ├── ttlog_wrapper.c
│       └── Makefile
├── cbindgen.toml          # C header generation config
├── Cargo.toml             # Feature flags for different bindings
└── README.md
```

## Build Commands
```bash
# Core C library
cargo build --release --features="c-api"

# Node.js module  
cargo build --release --features="nodejs"
npm pack

# Python wheel
cargo build --release --features="python" 
maturin build --release

# Java library
cargo build --release --features="java"
mvn package

# WASM module
cargo build --release --target wasm32-unknown-unknown --features="wasm"
wasm-pack build

# Go module (uses C library built above)
cd bindings/go && go build

# C# package (uses C library built above)
cd bindings/csharp && dotnet pack
```

## Key Benefits of This Architecture

1. **Zero Code Duplication**: All logic is in Rust core + single C API
2. **Consistent Behavior**: All languages call the same optimized functions  
3. **Easy Maintenance**: Bug fixes/optimizations automatically benefit all languages
4. **Performance**: Direct C calls, no intermediate layers
5. **Type Safety**: Each language gets idiomatic error handling
6. **Simple Testing**: Test C API once, confidence in all bindings
7. **Easy Distribution**: Build matrix generates all artifacts from single source

This approach gives you maximum code reuse while letting you use the best binding technology for each language (NAPI-RS, PyO3, etc.) without any duplication!
