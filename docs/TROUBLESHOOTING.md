# Troubleshooting Guide

This guide helps you diagnose and resolve common issues when using Scribe.

## Common Errors

### 1. Failed to Create Log Directory

**Error Message:**
```
Error: Failed to create log directory: Permission denied (os error 13)
```

**Cause:** Insufficient permissions to create the log directory.

**Solution:**

```rust
use scribe::{Logger, LoggerConfig};
use std::path::PathBuf;

// Check and create directory with proper error handling
fn create_logger_with_fallback() -> Result<Logger, Box<dyn std::error::Error>> {
    let primary_dir = PathBuf::from("/var/log/myapp");
    let fallback_dir = PathBuf::from("./logs");
    
    let config = match Logger::with_config(LoggerConfig {
        log_dir: primary_dir.clone(),
        ..Default::default()
    }) {
        Ok(logger) => return Ok(logger),
        Err(_) => {
            eprintln!("Failed to use {}, falling back to {}", 
                primary_dir.display(), fallback_dir.display());
            LoggerConfig {
                log_dir: fallback_dir,
                ..Default::default()
            }
        }
    };
    
    Ok(Logger::with_config(config)?)
}
```

**Prevention:**

```rust
use std::fs;
use std::path::Path;

fn ensure_log_directory(path: &Path) -> std::io::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    
    // Verify write permissions
    let test_file = path.join(".write_test");
    fs::write(&test_file, b"test")?;
    fs::remove_file(test_file)?;
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_dir = PathBuf::from("./logs");
    ensure_log_directory(&log_dir)?;
    
    let logger = Logger::with_config(LoggerConfig {
        log_dir,
        ..Default::default()
    })?;
    
    Ok(())
}
```

### 2. Disk Space Exhausted

**Error Message:**
```
Error: No space left on device (os error 28)
```

**Cause:** Log files consuming all available disk space.

**Solution:**

```rust
use scribe::{Logger, LoggerConfig};
use std::path::Path;

fn get_available_space(path: &Path) -> Result<u64, std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::metadata(path)?;
        // This is a simplified example; use statvfs for actual implementation
        Ok(metadata.size())
    }
    
    #[cfg(windows)]
    {
        // Use GetDiskFreeSpaceEx on Windows
        Ok(u64::MAX) // Placeholder
    }
}

fn create_logger_with_space_check() -> Result<Logger, Box<dyn std::error::Error>> {
    let log_dir = PathBuf::from("./logs");
    let min_required_space = 100 * 1024 * 1024; // 100 MB
    
    if get_available_space(&log_dir)? < min_required_space {
        return Err("Insufficient disk space for logging".into());
    }
    
    let config = LoggerConfig {
        log_dir,
        max_file_size: 10 * 1024 * 1024,
        max_files: 5, // Limit total space usage
        enable_compression: true, // Reduce space usage
        compression_level: 9,
        ..Default::default()
    };
    
    Ok(Logger::with_config(config)?)
}
```

**Monitoring:**

```rust
use std::fs;
use std::path::Path;

pub fn monitor_disk_usage(log_dir: &Path) -> Result<DiskUsageStats, std::io::Error> {
    let mut total_size = 0u64;
    let mut file_count = 0usize;
    
    for entry in fs::read_dir(log_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            total_size += metadata.len();
            file_count += 1;
        }
    }
    
    Ok(DiskUsageStats {
        total_bytes: total_size,
        file_count,
    })
}

pub struct DiskUsageStats {
    pub total_bytes: u64,
    pub file_count: usize,
}

impl DiskUsageStats {
    pub fn total_mb(&self) -> f64 {
        self.total_bytes as f64 / (1024.0 * 1024.0)
    }
}
```

### 3. File Rotation Failures

**Error Message:**
```
Error: Failed to rotate log file: Resource temporarily unavailable
```

**Cause:** File is locked by another process or rotation operation in progress.

**Solution:**

```rust
use scribe::{Logger, LoggerConfig};
use std::time::Duration;
use std::thread;

fn log_with_retry(logger: &Logger, message: &str, max_retries: u32) -> Result<(), String> {
    for attempt in 0..max_retries {
        match logger.info(message) {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt < max_retries - 1 {
                    eprintln!("Log attempt {} failed: {}, retrying...", attempt + 1, e);
                    thread::sleep(Duration::from_millis(100));
                } else {
                    return Err(format!("Failed after {} attempts: {}", max_retries, e));
                }
            }
        }
    }
    Ok(())
}

// Usage
fn main() {
    let logger = Logger::new();
    
    if let Err(e) = log_with_retry(&logger, "Important message", 3) {
        eprintln!("Critical: {}", e);
    }
}
```

**Prevention:**

```rust
use scribe::LoggerConfig;

// Increase buffer size to reduce rotation frequency
let config = LoggerConfig {
    max_file_size: 50 * 1024 * 1024, // Larger files
    buffer_size: 32768, // Larger buffer
    ..Default::default()
};
```

### 4. Compression Failures

**Error Message:**
```
Error: Compression failed: invalid compression level
```

**Cause:** Invalid compression level specified.

**Solution:**

```rust
use scribe::LoggerConfig;

fn create_logger_with_safe_compression(level: u32) -> Result<Logger, Box<dyn std::error::Error>> {
    let safe_level = level.clamp(1, 9); // Ensure valid range
    
    let config = LoggerConfig {
        enable_compression: true,
        compression_level: safe_level,
        ..Default::default()
    };
    
    Ok(Logger::with_config(config)?)
}
```

**Handling Compression Errors:**

```rust
use flate2::Compression;
use std::fs::File;
use std::io::{self, Read, Write};

pub fn compress_log_file(
    input_path: &Path,
    output_path: &Path,
    level: u32,
) -> io::Result<()> {
    let mut input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut encoder = flate2::write::GzEncoder::new(
        output,
        Compression::new(level)
    );
    
    let mut buffer = vec![0; 8192];
    loop {
        let bytes_read = input.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        encoder.write_all(&buffer[..bytes_read])?;
    }
    
    encoder.finish()?;
    Ok(())
}
```

### 5. Buffer Overflow Issues

**Error Message:**
```
Warning: Log buffer full, dropping messages
```

**Cause:** Writing logs faster than they can be flushed to disk.

**Solution:**

```rust
use scribe::{Logger, LoggerConfig};
use std::sync::mpsc::{self, Sender};
use std::thread;

pub struct BufferedLogger {
    sender: Sender<String>,
    logger: Logger,
}

impl BufferedLogger {
    pub fn new(buffer_capacity: usize) -> Self {
        let logger = Logger::with_config(LoggerConfig {
            buffer_size: 65536, // Large internal buffer
            ..Default::default()
        }).unwrap();
        
        let (sender, receiver) = mpsc::sync_channel(buffer_capacity);
        let logger_clone = logger.clone();
        
        thread::spawn(move || {
            for message in receiver {
                if let Err(e) = logger_clone.info(&message) {
                    eprintln!("Failed to log: {}", e);
                }
            }
        });
        
        Self { sender, logger }
    }
    
    pub fn log(&self, message: String) -> Result<(), String> {
        self.sender.send(message)
            .map_err(|e| format!("Buffer full: {}", e))
    }
}
```

**Monitoring Buffer Health:**

```rust
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct BufferMonitor {
    pending_logs: Arc<AtomicUsize>,
    max_pending: usize,
}

impl BufferMonitor {
    pub fn new(max_pending: usize) -> Self {
        Self {
            pending_logs: Arc::new(AtomicUsize::new(0)),
            max_pending,
        }
    }
    
    pub fn log_started(&self) -> bool {
        let pending = self.pending_logs.fetch_add(1, Ordering::Relaxed);
        if pending >= self.max_pending {
            self.pending_logs.fetch_sub(1, Ordering::Relaxed);
            return false;
        }
        true
    }
    
    pub fn log_completed(&self) {
        self.pending_logs.fetch_sub(1, Ordering::Relaxed);
    }
    
    pub fn get_buffer_utilization(&self) -> f64 {
        let pending = self.pending_logs.load(Ordering::Relaxed);
        (pending as f64 / self.max_pending as f64) * 100.0
    }
}
```

## Debugging Techniques

### 1. Enable Detailed Logging

```rust
use scribe::{Logger, LoggerConfig, LogLevel};

fn create_debug_logger() -> Logger {
    let config = LoggerConfig {
        console_output: true, // Show logs in console
        log_level: LogLevel::Debug, // Verbose output
        ..Default::default()
    };
    
    Logger::with_config(config).unwrap()
}

fn main() {
    let logger = create_debug_logger();
    
    logger.debug("Starting application");
    logger.debug("Configuration loaded");
    logger.info("Application ready");
}
```

### 2. Log File Analysis

```rust
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub fn analyze_log_file(path: &Path) -> Result<LogAnalysis, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    
    let mut analysis = LogAnalysis::default();
    
    for line in reader.lines() {
        let line = line?;
        
        if line.contains("ERROR") {
            analysis.error_count += 1;
        } else if line.contains("WARNING") {
            analysis.warning_count += 1;
        } else if line.contains("INFO") {
            analysis.info_count += 1;
        }
        
        analysis.total_lines += 1;
    }
    
    Ok(analysis)
}

#[derive(Default)]
pub struct LogAnalysis {
    pub total_lines: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl LogAnalysis {
    pub fn print_summary(&self) {
        println!("Log Analysis:");
        println!("  Total lines: {}", self.total_lines);
        println!("  Errors: {}", self.error_count);
        println!("  Warnings: {}", self.warning_count);
        println!("  Info: {}", self.info_count);
    }
}
```

### 3. Performance Profiling

```rust
use scribe::Logger;
use std::time::Instant;

pub fn profile_logging_performance(logger: &Logger, num_logs: usize) {
    let start = Instant::now();
    
    for i in 0..num_logs {
        logger.info(&format!("Test message {}", i));
    }
    
    let duration = start.elapsed();
    let throughput = num_logs as f64 / duration.as_secs_f64();
    
    println!("Logging Performance:");
    println!("  Messages: {}", num_logs);
    println!("  Duration: {:?}", duration);
    println!("  Throughput: {:.2} logs/sec", throughput);
}
```

### 4. Memory Usage Tracking

```rust
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

pub fn get_memory_usage() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}
```

## Performance Issues

### Issue: Slow Logging Performance

**Symptoms:**
- High latency when writing logs
- Application slowdown during logging
- CPU usage spikes

**Diagnosis:**

```rust
use scribe::Logger;
use std::time::Instant;

pub fn diagnose_logging_speed(logger: &Logger) {
    let test_message = "Test message for performance diagnosis";
    let iterations = 1000;
    
    // Test without flush
    let start = Instant::now();
    for _ in 0..iterations {
        logger.info(test_message);
    }
    let without_flush = start.elapsed();
    
    // Test with flush
    let start = Instant::now();
    for _ in 0..iterations {
        logger.info(test_message);
        logger.flush().ok();
    }
    let with_flush = start.elapsed();
    
    println!("Performance Diagnosis:");
    println!("  Without flush: {:?} ({:.2} logs/sec)", 
        without_flush, 
        iterations as f64 / without_flush.as_secs_f64()
    );
    println!("  With flush: {:?} ({:.2} logs/sec)", 
        with_flush, 
        iterations as f64 / with_flush.as_secs_f64()
    );
}
```

**Solutions:**

1. **Increase Buffer Size:**

```rust
let config = LoggerConfig {
    buffer_size: 65536, // Increase from default 8192
    ..Default::default()
};
```

2. **Reduce Compression Level:**

```rust
let config = LoggerConfig {
    enable_compression: true,
    compression_level: 3, // Use faster compression
    ..Default::default()
};
```

3. **Use Async Logging:**

```rust
use tokio::sync::mpsc;

pub struct AsyncLogger {
    sender: mpsc::UnboundedSender<String>,
}

impl AsyncLogger {
    pub fn new(logger: Logger) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();
        
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                logger.info(&message);
            }
        });
        
        Self { sender }
    }
    
    pub fn log(&self, message: String) {
        self.sender.send(message).ok();
    }
}
```

### Issue: High Memory Usage

**Symptoms:**
- Increasing memory consumption over time
- Out of memory errors
- Memory leaks

**Diagnosis:**

```rust
use scribe::Logger;

pub fn diagnose_memory_usage(logger: &Logger) {
    let baseline = get_memory_usage();
    println!("Baseline memory: {} bytes", baseline);
    
    // Perform logging operations
    for i in 0..10000 {
        logger.info(&format!("Test message {}", i));
    }
    
    let after_logging = get_memory_usage();
    println!("After logging: {} bytes", after_logging);
    println!("Difference: {} bytes", after_logging.saturating_sub(baseline));
    
    // Force cleanup
    logger.flush().ok();
    std::thread::sleep(std::time::Duration::from_secs(1));
    
    let after_flush = get_memory_usage();
    println!("After flush: {} bytes", after_flush);
}
```

**Solutions:**

1. **Reduce Buffer Size:**

```rust
let config = LoggerConfig {
    buffer_size: 4096, // Smaller buffer
    ..Default::default()
};
```

2. **Limit Log Message Size:**

```rust
pub fn log_truncated(logger: &Logger, message: &str, max_len: usize) {
    if message.len() > max_len {
        logger.info(&format!("{}... (truncated)", &message[..max_len]));
    } else {
        logger.info(message);
    }
}
```

3. **Regular Cleanup:**

```rust
use std::time::Duration;
use std::thread;

pub fn start_cleanup_thread(log_dir: PathBuf, interval: Duration) {
    thread::spawn(move || {
        loop {
            thread::sleep(interval);
            if let Err(e) = cleanup_old_logs(&log_dir, 7) {
                eprintln!("Cleanup failed: {}", e);
            }
        }
    });
}
```

### Issue: Log Files Not Rotating

**Symptoms:**
- Single log file growing too large
- No rotation despite reaching max_file_size
- Disk space filling up

**Diagnosis:**

```rust
use std::fs;

pub fn diagnose_rotation_issue(log_dir: &Path) {
    println!("Checking log directory: {}", log_dir.display());
    
    match fs::read_dir(log_dir) {
        Ok(entries) => {
            for (i, entry) in entries.enumerate() {
                if let Ok(entry) = entry {
                    if let Ok(metadata) = entry.metadata() {
                        println!("  File {}: {} ({} bytes)", 
                            i + 1,
                            entry.file_name().to_string_lossy(),
                            metadata.len()
                        );
                    }
                }
            }
        }
        Err(e) => println!("Error reading directory: {}", e),
    }
}
```

**Solutions:**

1. **Verify Configuration:**

```rust
let config = LoggerConfig {
    max_file_size: 10 * 1024 * 1024, // Ensure this is set
    max_files: 5, // Ensure this is set
    ..Default::default()
};
```

2. **Manual Rotation:**

```rust
pub fn force_rotation(logger: &Logger) -> Result<(), std::io::Error> {
    logger.flush()?;
    logger.rotate()?;
    Ok(())
}
```

## FAQ

### Q: Why are my logs delayed?

A: Logs are buffered before being written to disk. Call `logger.flush()` to force immediate write:

```rust
logger.info("Critical event");
logger.flush()?; // Ensure immediate write
```

### Q: How do I debug compression issues?

A: Disable compression temporarily to isolate the issue:

```rust
let config = LoggerConfig {
    enable_compression: false,
    ..Default::default()
};
```

### Q: Why are old logs not being deleted?

A: Check your cleanup configuration:

```rust
let config = LoggerConfig {
    max_files: 5, // Only keep 5 most recent files
    ..Default::default()
};
```

### Q: How do I handle logging errors gracefully?

A: Implement error handling with fallback:

```rust
fn safe_log(logger: &Logger, message: &str) {
    if let Err(e) = logger.info(message) {
        // Fallback to stderr
        eprintln!("[LOG ERROR: {}] {}", e, message);
    }
}
```

### Q: How do I test logging without creating files?

A: Use an in-memory or test logger:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestLogger {
        messages: std::sync::Mutex<Vec<String>>,
    }
    
    impl TestLogger {
        fn new() -> Self {
            Self {
                messages: std::sync::Mutex::new(Vec::new()),
            }
        }
        
        fn log(&self, message: String) {
            self.messages.lock().unwrap().push(message);
        }
        
        fn get_messages(&self) -> Vec<String> {
            self.messages.lock().unwrap().clone()
        }
    }
    
    #[test]
    fn test_logging() {
        let logger = TestLogger::new();
        logger.log("Test message".to_string());
        assert_eq!(logger.get_messages().len(), 1);
    }
}
```

## Getting Help

If you continue to experience issues:

1. Check the [GitHub Issues](https://github.com/yourusername/scribe/issues)
2. Review the [Getting Started Guide](./GETTING-STARTED.md)
3. Consult the [Performance Tuning Guide](./PERFORMANCE-TUNING.md)
4. Enable debug logging and collect diagnostics
5. Create a minimal reproducible example

### Collecting Diagnostics

```rust
pub fn collect_diagnostics(logger: &Logger) {
    println!("=== Scribe Diagnostics ===");
    println!("Logger configuration:");
    // Print relevant configuration
    
    println!("\nDisk usage:");
    if let Ok(stats) = monitor_disk_usage(logger.log_dir()) {
        println!("  Total: {:.2} MB", stats.total_mb());
        println!("  Files: {}", stats.file_count);
    }
    
    println!("\nPerformance:");
    profile_logging_performance(logger, 1000);
    
    println!("\nMemory usage:");
    println!("  Allocated: {} bytes", get_memory_usage());
}
```
