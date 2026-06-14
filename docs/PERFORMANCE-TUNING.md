# Performance Tuning Guide

This guide helps you optimize Scribe's performance for different workloads and scenarios.

## Key Performance Factors

### 1. Buffer Size
### 2. Compression Level
### 3. File Rotation Strategy
### 4. I/O Patterns
### 5. Thread Contention

## Buffer Size Optimization

The buffer size determines how much data is accumulated before writing to disk. Larger buffers reduce I/O operations but increase memory usage and potential data loss on crashes.

### Choosing Buffer Size

```rust
use scribe::{Logger, LoggerConfig};

// Low-frequency logging (< 10 logs/second)
let config_low_frequency = LoggerConfig {
    buffer_size: 4096, // 4 KB
    ..Default::default()
};

// Medium-frequency logging (10-100 logs/second)
let config_medium_frequency = LoggerConfig {
    buffer_size: 8192, // 8 KB (default)
    ..Default::default()
};

// High-frequency logging (> 100 logs/second)
let config_high_frequency = LoggerConfig {
    buffer_size: 32768, // 32 KB
    ..Default::default()
};

// Very high-frequency logging (> 1000 logs/second)
let config_very_high_frequency = LoggerConfig {
    buffer_size: 65536, // 64 KB
    ..Default::default()
};
```

### Buffer Size Impact

| Buffer Size | Throughput | Memory | Latency | Data Loss Risk |
|-------------|------------|--------|---------|----------------|
| 4 KB | Low | Minimal | Low | Minimal |
| 8 KB | Medium | Low | Medium | Low |
| 32 KB | High | Medium | Higher | Medium |
| 64 KB | Very High | High | Highest | Higher |

### Benchmarking Buffer Sizes

```rust
use scribe::{Logger, LoggerConfig};
use std::time::Instant;

fn benchmark_buffer_size(buffer_size: usize, num_logs: usize) -> std::time::Duration {
    let config = LoggerConfig {
        buffer_size,
        console_output: false,
        ..Default::default()
    };
    
    let logger = Logger::with_config(config).unwrap();
    let start = Instant::now();
    
    for i in 0..num_logs {
        logger.info(&format!("Log message {}", i));
    }
    
    logger.flush().unwrap();
    start.elapsed()
}

fn main() {
    let num_logs = 10000;
    
    for buffer_size in [4096, 8192, 16384, 32768, 65536] {
        let duration = benchmark_buffer_size(buffer_size, num_logs);
        println!(
            "Buffer size: {} bytes, Time: {:?}, Throughput: {:.2} logs/sec",
            buffer_size,
            duration,
            num_logs as f64 / duration.as_secs_f64()
        );
    }
}
```

### Adaptive Buffer Management

```rust
use scribe::{Logger, LoggerConfig};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct AdaptiveLogger {
    logger: Logger,
    log_count: Arc<AtomicUsize>,
}

impl AdaptiveLogger {
    pub fn new() -> Self {
        let config = LoggerConfig {
            buffer_size: 8192, // Start with default
            ..Default::default()
        };
        
        Self {
            logger: Logger::with_config(config).unwrap(),
            log_count: Arc::new(AtomicUsize::new(0)),
        }
    }
    
    pub fn log(&self, message: &str) {
        self.logger.info(message);
        self.log_count.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn should_increase_buffer(&self) -> bool {
        // If logging more than 100 messages per second, consider larger buffer
        self.log_count.load(Ordering::Relaxed) > 100
    }
}
```

## Compression Level Selection

Compression reduces disk usage but increases CPU usage. Choose based on your CPU/disk tradeoff.

### Compression Levels

```rust
use scribe::{Logger, LoggerConfig};

// Fastest compression (least CPU, larger files)
let config_fast = LoggerConfig {
    enable_compression: true,
    compression_level: 1,
    ..Default::default()
};

// Balanced compression (recommended)
let config_balanced = LoggerConfig {
    enable_compression: true,
    compression_level: 6, // Default
    ..Default::default()
};

// Best compression (most CPU, smallest files)
let config_best = LoggerConfig {
    enable_compression: true,
    compression_level: 9,
    ..Default::default()
};
```

### Compression Performance Comparison

| Level | Speed | Compression Ratio | CPU Usage | Use Case |
|-------|-------|-------------------|-----------|----------|
| 1 | Fastest | ~2-3x | Low | High-volume logs, CPU-constrained |
| 3 | Fast | ~3-4x | Medium-Low | Balanced performance |
| 6 | Medium | ~4-5x | Medium | General purpose (default) |
| 9 | Slow | ~5-6x | High | Storage-constrained, archive logs |

### Compression Benchmarking

```rust
use scribe::{Logger, LoggerConfig};
use std::time::Instant;
use std::fs;

fn benchmark_compression(level: u32, num_logs: usize) {
    let log_dir = format!("./benchmark_logs_{}", level);
    let config = LoggerConfig {
        log_dir: PathBuf::from(&log_dir),
        enable_compression: true,
        compression_level: level,
        max_file_size: 1024 * 1024, // 1 MB to force rotation
        ..Default::default()
    };
    
    let logger = Logger::with_config(config).unwrap();
    let start = Instant::now();
    
    for i in 0..num_logs {
        logger.info(&format!("This is a test log message number {} with some additional text to make it more realistic", i));
    }
    
    let duration = start.elapsed();
    let total_size: u64 = fs::read_dir(&log_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.metadata().ok())
        .map(|metadata| metadata.len())
        .sum();
    
    println!("Compression level {}: {:?}, Size: {} KB", 
        level, duration, total_size / 1024);
    
    // Cleanup
    fs::remove_dir_all(log_dir).ok();
}

fn main() {
    let num_logs = 50000;
    
    for level in [1, 3, 6, 9] {
        benchmark_compression(level, num_logs);
    }
}
```

### Dynamic Compression Strategy

```rust
use scribe::{Logger, LoggerConfig};
use std::path::PathBuf;

pub fn create_logger_with_adaptive_compression(
    available_disk_space: u64,
) -> Logger {
    let compression_level = if available_disk_space < 1024 * 1024 * 1024 { // < 1 GB
        9 // Maximum compression
    } else if available_disk_space < 10 * 1024 * 1024 * 1024 { // < 10 GB
        6 // Balanced
    } else {
        3 // Fast compression
    };
    
    let config = LoggerConfig {
        enable_compression: true,
        compression_level,
        ..Default::default()
    };
    
    Logger::with_config(config).unwrap()
}
```

## File Rotation Strategy

### Size-Based Rotation

```rust
use scribe::LoggerConfig;

// Small files, frequent rotation (good for debugging)
let config_small = LoggerConfig {
    max_file_size: 1024 * 1024, // 1 MB
    max_files: 10,
    ..Default::default()
};

// Medium files (balanced)
let config_medium = LoggerConfig {
    max_file_size: 10 * 1024 * 1024, // 10 MB
    max_files: 5,
    ..Default::default()
};

// Large files (fewer rotations, less overhead)
let config_large = LoggerConfig {
    max_file_size: 100 * 1024 * 1024, // 100 MB
    max_files: 3,
    ..Default::default()
};
```

### Rotation Performance Impact

```rust
use scribe::{Logger, LoggerConfig};
use std::time::Instant;

fn benchmark_rotation_overhead(max_file_size: usize) -> std::time::Duration {
    let config = LoggerConfig {
        max_file_size,
        max_files: 3,
        enable_compression: false,
        ..Default::default()
    };
    
    let logger = Logger::with_config(config).unwrap();
    let start = Instant::now();
    
    // Write enough to trigger multiple rotations
    let total_data = max_file_size * 5;
    let message = "A".repeat(100); // 100 byte message
    let num_logs = total_data / 100;
    
    for _ in 0..num_logs {
        logger.info(&message);
    }
    
    start.elapsed()
}

fn main() {
    for size in [1024 * 1024, 5 * 1024 * 1024, 10 * 1024 * 1024] {
        let duration = benchmark_rotation_overhead(size);
        println!("Max file size: {} MB, Time: {:?}", 
            size / (1024 * 1024), duration);
    }
}
```

## Cleanup Strategy Optimization

### Time-Based Cleanup

```rust
use scribe::{Logger, LoggerConfig};
use std::time::Duration;

// Aggressive cleanup (save disk space)
let config_aggressive = LoggerConfig {
    max_files: 3,
    cleanup_threshold: Duration::from_secs(24 * 60 * 60), // 1 day
    ..Default::default()
};

// Moderate cleanup
let config_moderate = LoggerConfig {
    max_files: 5,
    cleanup_threshold: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
    ..Default::default()
};

// Conservative cleanup (keep more history)
let config_conservative = LoggerConfig {
    max_files: 10,
    cleanup_threshold: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
    ..Default::default()
};
```

### Manual Cleanup Control

```rust
use scribe::Logger;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn cleanup_old_logs(log_dir: &str, max_age_days: u64) -> std::io::Result<usize> {
    let max_age_secs = max_age_days * 24 * 60 * 60;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let mut removed_count = 0;
    
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        
        if let Ok(modified) = metadata.modified() {
            let modified_secs = modified
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            if now - modified_secs > max_age_secs {
                fs::remove_file(entry.path())?;
                removed_count += 1;
            }
        }
    }
    
    Ok(removed_count)
}
```

## I/O Pattern Optimization

### Batch Logging

```rust
use scribe::Logger;

pub struct BatchLogger {
    logger: Logger,
    batch: Vec<String>,
    batch_size: usize,
}

impl BatchLogger {
    pub fn new(logger: Logger, batch_size: usize) -> Self {
        Self {
            logger,
            batch: Vec::with_capacity(batch_size),
            batch_size,
        }
    }
    
    pub fn log(&mut self, message: String) {
        self.batch.push(message);
        
        if self.batch.len() >= self.batch_size {
            self.flush();
        }
    }
    
    pub fn flush(&mut self) {
        for message in self.batch.drain(..) {
            self.logger.info(&message);
        }
        self.logger.flush().ok();
    }
}

impl Drop for BatchLogger {
    fn drop(&mut self) {
        self.flush();
    }
}

// Usage
fn main() {
    let logger = Logger::new();
    let mut batch_logger = BatchLogger::new(logger, 100);
    
    for i in 0..1000 {
        batch_logger.log(format!("Message {}", i));
    }
    
    // Automatically flushed on drop
}
```

### Async Logging

```rust
use scribe::Logger;
use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;

pub struct AsyncLogger {
    sender: Sender<String>,
}

impl AsyncLogger {
    pub fn new(logger: Logger) -> Self {
        let (sender, receiver): (Sender<String>, Receiver<String>) = mpsc::channel();
        
        thread::spawn(move || {
            while let Ok(message) = receiver.recv() {
                logger.info(&message);
            }
            logger.flush().ok();
        });
        
        Self { sender }
    }
    
    pub fn log(&self, message: String) {
        self.sender.send(message).ok();
    }
}

// Usage
fn main() {
    let logger = Logger::new();
    let async_logger = AsyncLogger::new(logger);
    
    for i in 0..1000 {
        async_logger.log(format!("Message {}", i));
    }
    
    // Give time for async processing
    thread::sleep(std::time::Duration::from_secs(1));
}
```

## Thread Contention Reduction

### Thread-Local Buffers

```rust
use scribe::Logger;
use std::sync::Arc;
use std::cell::RefCell;

thread_local! {
    static THREAD_BUFFER: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

pub struct ThreadLocalLogger {
    logger: Arc<Logger>,
    buffer_size: usize,
}

impl ThreadLocalLogger {
    pub fn new(logger: Logger, buffer_size: usize) -> Self {
        Self {
            logger: Arc::new(logger),
            buffer_size,
        }
    }
    
    pub fn log(&self, message: String) {
        THREAD_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            buffer.push(message);
            
            if buffer.len() >= self.buffer_size {
                self.flush_internal(&mut buffer);
            }
        });
    }
    
    pub fn flush(&self) {
        THREAD_BUFFER.with(|buffer| {
            let mut buffer = buffer.borrow_mut();
            self.flush_internal(&mut buffer);
        });
    }
    
    fn flush_internal(&self, buffer: &mut Vec<String>) {
        for message in buffer.drain(..) {
            self.logger.info(&message);
        }
    }
}
```

### Lock-Free Logging

```rust
use scribe::Logger;
use crossbeam_channel::{bounded, Sender};
use std::thread;

pub struct LockFreeLogger {
    sender: Sender<String>,
}

impl LockFreeLogger {
    pub fn new(logger: Logger, buffer_capacity: usize) -> Self {
        let (sender, receiver) = bounded(buffer_capacity);
        
        thread::spawn(move || {
            for message in receiver {
                logger.info(&message);
            }
        });
        
        Self { sender }
    }
    
    pub fn log(&self, message: String) -> Result<(), String> {
        self.sender.try_send(message)
            .map_err(|e| format!("Failed to send log: {}", e))
    }
}
```

## Performance Monitoring

### Logging Metrics

```rust
use scribe::Logger;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct MonitoredLogger {
    logger: Logger,
    log_count: Arc<AtomicU64>,
    bytes_written: Arc<AtomicU64>,
    start_time: Instant,
}

impl MonitoredLogger {
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            log_count: Arc::new(AtomicU64::new(0)),
            bytes_written: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }
    
    pub fn log(&self, message: &str) {
        self.logger.info(message);
        self.log_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(message.len() as u64, Ordering::Relaxed);
    }
    
    pub fn get_stats(&self) -> LogStats {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let count = self.log_count.load(Ordering::Relaxed);
        let bytes = self.bytes_written.load(Ordering::Relaxed);
        
        LogStats {
            total_logs: count,
            total_bytes: bytes,
            logs_per_second: count as f64 / elapsed,
            bytes_per_second: bytes as f64 / elapsed,
        }
    }
}

pub struct LogStats {
    pub total_logs: u64,
    pub total_bytes: u64,
    pub logs_per_second: f64,
    pub bytes_per_second: f64,
}
```

## Configuration Presets

### High-Throughput Configuration

```rust
use scribe::LoggerConfig;

pub fn high_throughput_config() -> LoggerConfig {
    LoggerConfig {
        buffer_size: 65536,           // Large buffer
        max_file_size: 100 * 1024 * 1024, // 100 MB
        max_files: 10,
        enable_compression: true,
        compression_level: 1,          // Fast compression
        console_output: false,         // No console overhead
        ..Default::default()
    }
}
```

### Low-Latency Configuration

```rust
use scribe::LoggerConfig;

pub fn low_latency_config() -> LoggerConfig {
    LoggerConfig {
        buffer_size: 4096,            // Small buffer for quick flush
        max_file_size: 10 * 1024 * 1024, // 10 MB
        max_files: 5,
        enable_compression: false,    // No compression overhead
        console_output: false,
        ..Default::default()
    }
}
```

### Low-Memory Configuration

```rust
use scribe::LoggerConfig;

pub fn low_memory_config() -> LoggerConfig {
    LoggerConfig {
        buffer_size: 2048,            // Minimal buffer
        max_file_size: 1024 * 1024,   // 1 MB
        max_files: 3,
        enable_compression: true,
        compression_level: 9,          // Maximum compression
        console_output: false,
        ..Default::default()
    }
}
```

### Balanced Configuration

```rust
use scribe::LoggerConfig;

pub fn balanced_config() -> LoggerConfig {
    LoggerConfig {
        buffer_size: 8192,
        max_file_size: 10 * 1024 * 1024,
        max_files: 5,
        enable_compression: true,
        compression_level: 6,
        console_output: false,
        ..Default::default()
    }
}
```

## Best Practices Summary

1. **Start with defaults** and measure before optimizing
2. **Profile your workload** to identify bottlenecks
3. **Match buffer size** to log frequency
4. **Choose compression level** based on CPU/storage tradeoff
5. **Use larger files** when rotation overhead is noticeable
6. **Implement async logging** for high-throughput scenarios
7. **Monitor performance metrics** in production
8. **Test different configurations** with realistic workloads

## Next Steps

- Review [Troubleshooting](./TROUBLESHOOTING.md) for performance issues
- Check [Getting Started](./GETTING-STARTED.md) for basic usage
