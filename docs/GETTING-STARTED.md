# Getting Started with Scribe

Scribe is a high-performance logging library for Rust that provides efficient file-based logging with automatic rotation, compression, and cleanup capabilities.

## Installation

Add Scribe to your `Cargo.toml`:

```toml
[dependencies]
scribe = "0.1.0"
```

Or use cargo add:

```bash
cargo add scribe
```

## Basic Usage

### Simple Console Logging

```rust
use scribe::{Logger, LogLevel};

fn main() {
    let logger = Logger::new();
    
    logger.log(LogLevel::Info, "Application started");
    logger.log(LogLevel::Warning, "This is a warning");
    logger.log(LogLevel::Error, "An error occurred");
}
```

### File-Based Logging

```rust
use scribe::{Logger, LoggerConfig, LogLevel};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LoggerConfig {
        log_dir: PathBuf::from("./logs"),
        max_file_size: 10 * 1024 * 1024, // 10 MB
        max_files: 5,
        buffer_size: 8192,
        enable_compression: true,
        ..Default::default()
    };
    
    let logger = Logger::with_config(config)?;
    
    logger.info("Application initialized");
    logger.warn("Low memory warning");
    logger.error("Database connection failed");
    
    Ok(())
}
```

### Structured Logging

```rust
use scribe::{Logger, LogLevel};
use serde_json::json;

fn main() {
    let logger = Logger::new();
    
    logger.log_structured(
        LogLevel::Info,
        "user_login",
        json!({
            "user_id": "12345",
            "ip_address": "192.168.1.100",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    );
}
```

## Configuration Options

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `log_dir` | `PathBuf` | `./logs` | Directory for log files |
| `max_file_size` | `usize` | `10 MB` | Maximum size per log file |
| `max_files` | `usize` | `5` | Maximum number of log files to keep |
| `buffer_size` | `usize` | `8192` | Write buffer size in bytes |
| `enable_compression` | `bool` | `false` | Enable gzip compression for rotated logs |
| `compression_level` | `u32` | `6` | Compression level (1-9) |
| `log_format` | `LogFormat` | `Text` | Log format (Text or JSON) |

## Best Practices

### 1. Use Appropriate Log Levels

```rust
// Debug: Detailed information for debugging
logger.debug("Processing item: {}", item_id);

// Info: General informational messages
logger.info("Server started on port 8080");

// Warning: Warning messages for potentially harmful situations
logger.warn("Retry attempt {} failed", attempt);

// Error: Error messages for error events
logger.error("Failed to connect to database: {}", error);
```

### 2. Configure Based on Environment

```rust
use scribe::{Logger, LoggerConfig};

fn create_logger() -> Logger {
    let config = if cfg!(debug_assertions) {
        // Development configuration
        LoggerConfig {
            max_file_size: 5 * 1024 * 1024,
            max_files: 3,
            enable_compression: false,
            ..Default::default()
        }
    } else {
        // Production configuration
        LoggerConfig {
            max_file_size: 50 * 1024 * 1024,
            max_files: 10,
            enable_compression: true,
            compression_level: 9,
            ..Default::default()
        }
    };
    
    Logger::with_config(config).expect("Failed to create logger")
}
```

### 3. Use Structured Logging for Analytics

```rust
use serde_json::json;

// Log structured data for easier parsing and analysis
logger.log_structured(
    LogLevel::Info,
    "api_request",
    json!({
        "method": "GET",
        "path": "/api/users",
        "status": 200,
        "duration_ms": 45,
        "user_agent": "Mozilla/5.0..."
    })
);
```

### 4. Handle Errors Gracefully

```rust
use scribe::{Logger, LoggerConfig};

fn main() {
    let logger = match Logger::with_config(LoggerConfig::default()) {
        Ok(logger) => logger,
        Err(e) => {
            eprintln!("Failed to initialize logger: {}", e);
            std::process::exit(1);
        }
    };
    
    logger.info("Logger initialized successfully");
}
```

## Common Questions

### Q: How do I change the log file name pattern?

A: Configure the `file_prefix` option:

```rust
let config = LoggerConfig {
    file_prefix: "myapp".to_string(),
    ..Default::default()
};
```

This will create files like `myapp-2024-01-15.log`.

### Q: Can I log to multiple files simultaneously?

A: Yes, create multiple logger instances with different configurations:

```rust
let access_logger = Logger::with_config(LoggerConfig {
    log_dir: PathBuf::from("./logs/access"),
    file_prefix: "access".to_string(),
    ..Default::default()
})?;

let error_logger = Logger::with_config(LoggerConfig {
    log_dir: PathBuf::from("./logs/errors"),
    file_prefix: "error".to_string(),
    ..Default::default()
})?;
```

### Q: How do I disable console output?

A: Set the `console_output` option to false:

```rust
let config = LoggerConfig {
    console_output: false,
    ..Default::default()
};
```

### Q: What happens when disk space is full?

A: Scribe will return an error when write operations fail. Implement error handling:

```rust
if let Err(e) = logger.info("Message") {
    eprintln!("Logging failed: {}", e);
    // Handle the error (e.g., alert monitoring system)
}
```

### Q: How do I flush logs immediately?

A: Call the `flush()` method:

```rust
logger.info("Critical operation completed");
logger.flush()?; // Ensure logs are written to disk immediately
```

## Next Steps

- Review the [Integration Guide](./INTEGRATION-GUIDE.md) for platform-specific setup
- Learn about [Performance Tuning](./PERFORMANCE-TUNING.md) to optimize for your use case
- Check [Troubleshooting](./TROUBLESHOOTING.md) if you encounter issues

## Example Projects

### Web Server Logging

```rust
use scribe::{Logger, LoggerConfig, LogLevel};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Arc::new(Logger::with_config(LoggerConfig {
        log_dir: PathBuf::from("./logs"),
        max_file_size: 50 * 1024 * 1024,
        enable_compression: true,
        ..Default::default()
    })?);
    
    // Share logger across threads
    let logger_clone = Arc::clone(&logger);
    std::thread::spawn(move || {
        logger_clone.info("Worker thread started");
    });
    
    logger.info("Server listening on 0.0.0.0:8080");
    
    Ok(())
}
```

### CLI Application Logging

```rust
use scribe::{Logger, LoggerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::with_config(LoggerConfig {
        console_output: true,
        log_dir: PathBuf::from("~/.myapp/logs"),
        max_file_size: 1024 * 1024, // 1 MB
        max_files: 3,
        ..Default::default()
    })?;
    
    logger.info("Processing files...");
    
    // Your application logic
    
    logger.info("Processing complete");
    Ok(())
}
```
