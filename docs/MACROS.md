# Scribe Macros - Convenient Logging API

This document describes the convenient logging macros implemented in Scribe for Rust applications.

## Overview

Scribe provides ergonomic logging macros that support:
- **Automatic tag detection** from thread-local storage or call stack backtrace
- **Format string support** for dynamic message construction
- **All log levels**: Verbose, Debug, Info, Warn, Error
- **Explicit tagging** when needed
- **Thread-local tag planting** for context-aware logging

## Files Created

### Core Implementation
- **`src/macros.rs`** - Macro definitions for all logging levels
- **`src/tag.rs`** - Tag management and thread-local storage (enhanced with `plant()` and `uproot()`)
- **`src/tree.rs`** - Forest and Tree API for hierarchical logging
- **`src/backtrace.rs`** - Automatic tag detection from call stack
- **`src/lib.rs`** - Updated with `log_with_auto_tag()` helper function

### Examples and Tests
- **`examples/macro_usage.rs`** - Comprehensive usage examples
- **`tests/macro_integration_tests.rs`** - Integration tests for all macro features

## API Reference

### Simple Logging Macros (Automatic Tag Detection)

```rust
use scribe::{scribe_v, scribe_d, scribe_i, scribe_w, scribe_e};

// Simple messages
scribe_v!("Verbose message");
scribe_d!("Debug message");
scribe_i!("Info message");
scribe_w!("Warning message");
scribe_e!("Error message");

// With format strings
scribe_i!("User {} logged in", username);
scribe_d!("Processing {} items", count);
scribe_w!("Memory usage: {}MB", memory);
scribe_e!("Failed: {}", error);
```

**Automatic Tag Detection:**
1. First checks thread-local storage (set via `tag().plant()`)
2. Falls back to backtrace-based detection (calling module/function)

### Tagged Logging Macros (Explicit Tags)

```rust
use scribe::{scribe_tag_v, scribe_tag_d, scribe_tag_i, scribe_tag_w, scribe_tag_e};

// With explicit tags
scribe_tag_v!("network", "Packet received");
scribe_tag_d!("database", "Query executed");
scribe_tag_i!("auth", "User authenticated");
scribe_tag_w!("cache", "Cache miss");
scribe_tag_e!("io", "File not found");

// With format strings
scribe_tag_i!("http", "Request to {} completed in {}ms", endpoint, duration);
scribe_tag_d!("sql", "SELECT * FROM users WHERE id = {}", user_id);
```

### Thread-Local Tag Planting

```rust
use scribe::{tag, scribe_i, scribe_d};

// Plant a tag for the current thread
tag("session_123").plant();

// All subsequent logs in this thread use the planted tag
scribe_i!("Session started");
scribe_d!("Loading preferences");
scribe_i!("Session ready");

// Clear the planted tag
tag::uproot();
```

**Use Cases:**
- Request/session tracking
- Module-level context
- Function scope tagging
- Thread-specific operations

## Usage Examples

### Example 1: Basic Logging

```rust
use scribe::{scribe_i, scribe_e};

fn main() {
    scribe_i!("Application started");
    
    match connect_to_database() {
        Ok(_) => scribe_i!("Database connected"),
        Err(e) => scribe_e!("Database connection failed: {}", e),
    }
}
```

### Example 2: Request Processing with Context

```rust
use scribe::{tag, scribe_i, scribe_d, scribe_w};

fn process_request(request_id: &str) {
    // Plant tag for entire request context
    tag(request_id).plant();
    
    scribe_i!("Processing request");
    scribe_d!("Validating input");
    
    if validate_input() {
        scribe_d!("Input valid");
        handle_request();
    } else {
        scribe_w!("Invalid input received");
    }
    
    scribe_i!("Request completed");
    
    // Cleanup
    tag::uproot();
}
```

### Example 3: Mixed Tag Usage

```rust
use scribe::{tag, scribe_i, scribe_tag_d, scribe_tag_e};

fn handle_payment() {
    // Plant general context
    tag("payment_flow").plant();
    
    scribe_i!("Starting payment processing");
    
    // Explicit tags override planted tag
    scribe_tag_d!("database", "Fetching account details");
    scribe_tag_d!("api", "Calling payment gateway");
    
    // Back to planted tag
    scribe_i!("Payment completed");
    
    tag::uproot();
}
```

### Example 4: Concurrent Logging

```rust
use scribe::{tag, scribe_i};
use std::thread;

fn main() {
    let handles: Vec<_> = (0..5)
        .map(|i| {
            thread::spawn(move || {
                // Each thread gets its own tag
                tag(&format!("worker_{}", i)).plant();
                
                scribe_i!("Worker started");
                do_work();
                scribe_i!("Worker finished");
                
                tag::uproot();
            })
        })
        .collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

### Example 5: Error Handling

```rust
use scribe::{scribe_d, scribe_w, scribe_e};

fn process_file(path: &str) -> Result<(), std::io::Error> {
    scribe_d!("Opening file: {}", path);
    
    let file = match std::fs::File::open(path) {
        Ok(f) => {
            scribe_d!("File opened successfully");
            f
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            scribe_w!("File not found: {}", path);
            return Err(e);
        }
        Err(e) => {
            scribe_e!("Failed to open file: {}", e);
            return Err(e);
        }
    };
    
    // Process file...
    Ok(())
}
```

## Implementation Details

### Macro Definitions

All macros follow the same pattern:

```rust
#[macro_export]
macro_rules! scribe_i {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Info, &format!($($arg)*))
    };
}
```

Tagged macros directly call the tree API:

```rust
#[macro_export]
macro_rules! scribe_tag_i {
    ($tag:expr, $($arg:tt)*) => {
        $crate::tag::tag($tag).i(&format!($($arg)*))
    };
}
```

### Helper Function

```rust
pub fn log_with_auto_tag(level: LogLevel, message: &str) {
    let tag = tag::get_thread_tag()
        .or_else(|| backtrace::get_calling_class());
    
    tree::forest().log(level, tag.as_deref(), message);
}
```

**Tag Resolution Order:**
1. Thread-local tag (if planted via `tag().plant()`)
2. Backtrace-based detection (calling function/module)
3. None (if backtrace detection fails)

### Tag Management

```rust
// TaggedLogger with plant support
impl TaggedLogger {
    pub fn plant(&self) {
        set_thread_tag(Some(self.tag.clone()));
    }
}

// Global uproot function
pub fn uproot() {
    set_thread_tag(None);
}
```

## Testing

Run all macro tests:

```bash
# Unit tests in macros.rs
cargo test --lib macros

# Integration tests
cargo test --test macro_integration_tests

# Run example
cargo run --example macro_usage
```

## Performance Considerations

1. **Zero-cost for simple messages**: Static strings are efficiently compiled
2. **Format overhead**: `format!()` allocates, use wisely in hot paths
3. **Tag storage**: Thread-local storage is very fast
4. **Backtrace detection**: Has overhead, prefer planted tags in critical sections

## Best Practices

### ✅ DO

- Use planted tags for request/session context
- Use explicit tags for cross-cutting concerns (database, network)
- Clear planted tags with `uproot()` when done
- Use appropriate log levels
- Format strings for dynamic data

### ❌ DON'T

- Don't forget to `uproot()` planted tags
- Don't over-use verbose/debug in production
- Don't log sensitive data (passwords, tokens)
- Don't rely on backtrace detection in hot paths

## Integration with Existing Code

The macros integrate seamlessly with existing Scribe infrastructure:

- **Tree/Forest API**: All logs go through the global forest
- **Double buffering**: Automatic buffer management
- **Metrics**: Logging operations are tracked
- **FFI compatibility**: Can coexist with C/C++ FFI calls

## Future Enhancements

Potential improvements:
- Compile-time log level filtering
- Structured logging support (key-value pairs)
- Async logging macros
- Scoped tags (RAII-style)
- Custom formatters

## Summary

This implementation provides:
- ✅ 10 convenient logging macros (5 levels × 2 variants)
- ✅ Automatic tag detection from backtrace
- ✅ Thread-local tag planting for context
- ✅ Full format string support
- ✅ Integration with Tree/Forest API
- ✅ Comprehensive examples and tests
- ✅ Thread-safe and performant

The macros make Scribe logging ergonomic for Rust developers while maintaining compatibility with the existing FFI and high-performance architecture.
