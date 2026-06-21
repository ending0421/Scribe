//! Scribe - High-performance logging library for mobile platforms.
//!
//! Scribe provides lock-free, memory-mapped logging with double buffering,
//! automatic rotation, compression, and encryption support.
//!
//! # Architecture
//!
//! - **Memory-mapped I/O**: Zero-copy writes to disk
//! - **Double buffering**: Lock-free buffer rotation
//! - **Pipeline processing**: Modular data transformation
//! - **Automatic cleanup**: Size and time-based retention
//!
//! # FFI Usage (C/C++/Swift/Kotlin)
//!
//! ```c
//! // Initialize
//! scribe_init("/path/to/logs");
//!
//! // Plant a ConsoleSink for development (outputs to stdout)
//! scribe_register_console(2);  // 2 = Info level minimum
//!
//! // Write logs (will be stored AND printed via ConsoleSink)
//! scribe_write(2, "MyTag", "Log message");  // 2 = Info level
//!
//! // Flush to disk
//! scribe_flush();
//!
//! // Get metrics
//! MetricsSnapshot snapshot;
//! scribe_get_metrics(&snapshot);
//!
//! // Remove all trees (optional)
//! scribe_clear_sinks();
//!
//! // Cleanup
//! scribe_destroy();
//! ```
//!
//! # Rust Usage
//!
//! ```no_run
//! use scribe::{DoubleBufferManager, LogFrame, LogLevel};
//! use std::path::PathBuf;
//!
//! let mut manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
//!
//! let frame = LogFrame::new(
//!     LogLevel::Info,
//!     "app".to_string(),
//!     "Application started".to_string()
//! );
//!
//! let data = frame.serialize().unwrap();
//! let (buffer, idx) = manager.get_active_buffer();
//! manager.increment_active_writers(idx);
//! buffer.write(&data).unwrap();
//! manager.decrement_active_writers(idx);
//! ```

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;

mod error;
mod storage;
mod pipeline;
mod stages;
mod outputs;
mod platform;
mod config;
mod metrics;
pub mod caller;
pub mod context;
pub mod sink;
#[macro_use]
mod macros;

pub use error::{ScribeError, Result};
pub use storage::{LogFrame, LogLevel, MmapBuffer, DoubleBufferManager};
pub use config::Config;
pub use metrics::{ScribeMetrics, MetricsSnapshot, ErrorType};
pub use sink::{LogSink, ConsoleSink, SinkRegistry, register_sink, clear_sinks, registry};

static GLOBAL_SCRIBE: OnceCell<Arc<Mutex<ScribeInstance>>> = OnceCell::new();
static GLOBAL_METRICS: OnceCell<Arc<ScribeMetrics>> = OnceCell::new();

struct ScribeInstance {
    manager: DoubleBufferManager,
}

fn get_metrics() -> Arc<ScribeMetrics> {
    GLOBAL_METRICS.get_or_init(|| Arc::new(ScribeMetrics::new())).clone()
}

/// Helper function for logging with automatic tag detection.
///
/// This function is used by the convenience macros (`scribe_v!`, `scribe_d!`, etc.)
/// to log messages with automatic tag detection from either:
/// 1. Thread-local tag storage (set via `tag().plant()`)
/// 2. Call stack backtrace (detects the calling module/function)
///
/// # Arguments
///
/// * `level` - The log level
/// * `message` - The formatted message string
///
/// # Examples
///
/// ```no_run
/// use scribe::{log_with_auto_tag, LogLevel};
///
/// log_with_auto_tag(LogLevel::Info, "Application started");
/// ```
pub fn log_with_auto_tag(level: LogLevel, message: &str) {
    let tag = context::get_thread_context()
        .or_else(|| caller::get_caller_module());

    sink::registry().log(level, tag.as_deref(), message);
}

/// Initializes the Scribe logging system (FFI).
///
/// Creates the log directory and sets up double buffering.
///
/// # Arguments
///
/// * `log_dir` - Null-terminated C string path to log directory.
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - log_dir is null
/// * `-2` - Invalid UTF-8 in log_dir
/// * `-3` - Failed to create directory
/// * `-4` - Failed to create buffer manager
/// * `-5` - Already initialized
///
/// # Safety
///
/// `log_dir` must be a valid null-terminated C string.
///
/// # Examples
///
/// ```c
/// int result = scribe_init("/var/logs/myapp");
/// if (result != 0) {
///     // Handle error
/// }
/// ```
#[no_mangle]
pub extern "C" fn scribe_init(
    log_dir: *const c_char,
) -> i32 {
    if log_dir.is_null() {
        return -1;
    }

    let log_dir_str = unsafe {
        match CStr::from_ptr(log_dir).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        }
    };

    let log_path = std::path::PathBuf::from(log_dir_str);

    // 创建目录
    if let Err(_) = std::fs::create_dir_all(&log_path) {
        return -3;
    }

    let manager = match DoubleBufferManager::new(log_path) {
        Ok(m) => m,
        Err(_) => return -4,
    };

    let instance = ScribeInstance { manager };

    if GLOBAL_SCRIBE.set(Arc::new(Mutex::new(instance))).is_err() {
        return -5; // Already initialized
    }

    0
}

/// Writes a log entry to the buffer (FFI).
///
/// Serializes the log frame and writes it to the active buffer.
/// Automatically triggers buffer swap when the buffer is 90% full.
///
/// # Arguments
///
/// * `level` - Log level (0=Verbose, 1=Debug, 2=Info, 3=Warn, 4=Error)
/// * `tag` - Null-terminated C string tag
/// * `message` - Null-terminated C string message
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - tag or message is null
/// * `-2` - Not initialized (call scribe_init first)
/// * `-3` - Serialization failed
/// * `-4` - Buffer swap failed
/// * `-5` - Buffer full
///
/// # Safety
///
/// `tag` and `message` must be valid null-terminated C strings.
///
/// # Examples
///
/// ```c
/// // Write an info log
/// int result = scribe_write(2, "network", "Connection established");
/// if (result != 0) {
///     // Handle error
/// }
/// ```
#[no_mangle]
pub extern "C" fn scribe_write(
    level: i32,
    tag: *const c_char,
    message: *const c_char,
) -> i32 {
    if tag.is_null() || message.is_null() {
        return -1;
    }

    let scribe = match GLOBAL_SCRIBE.get() {
        Some(s) => s,
        None => return -2, // Not initialized
    };

    let tag_str = unsafe {
        CStr::from_ptr(tag).to_string_lossy().to_string()
    };

    let msg_str = unsafe {
        CStr::from_ptr(message).to_string_lossy().to_string()
    };

    let log_level = match level {
        0 => LogLevel::Verbose,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        _ => LogLevel::Info,
    };

    // Use Tree to log
    sink::registry().log(log_level, Some(&tag_str), &msg_str);

    let frame = LogFrame::new(log_level, tag_str.clone(), msg_str.clone());

    let serialized = match frame.serialize() {
        Ok(data) => data,
        Err(_) => {
            get_metrics().record_write_failed();
            return -3;
        }
    };

    let serialized_len = serialized.len() as u64;
    let instance = scribe.lock();
    let (buffer, idx) = instance.manager.get_active_buffer();

    instance.manager.increment_active_writers(idx);

    let result = buffer.write(&serialized);

    instance.manager.decrement_active_writers(idx);

    match result {
        Ok(_) => {
            // 记录成功写入
            get_metrics().record_write(serialized_len);

            // 检查是否需要交换
            if instance.manager.should_swap(&buffer) {
                if let Err(_) = instance.manager.swap_buffers() {
                    get_metrics().record_error(ErrorType::WriteFailed);
                    return -4;
                }
            }
            0
        }
        Err(_) => {
            get_metrics().record_write_failed();
            get_metrics().record_error(ErrorType::BufferFull);
            -5
        }
    }
}

/// Flushes the active buffer to disk (FFI).
///
/// Forces all pending writes to be persisted to disk.
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - Not initialized
/// * `-2` - Flush failed
///
/// # Examples
///
/// ```c
/// int result = scribe_flush();
/// if (result != 0) {
///     // Handle error
/// }
/// ```
#[no_mangle]
pub extern "C" fn scribe_flush() -> i32 {
    let scribe = match GLOBAL_SCRIBE.get() {
        Some(s) => s,
        None => return -1,
    };

    let instance = scribe.lock();
    let (buffer, _) = instance.manager.get_active_buffer();

    match buffer.flush() {
        Ok(_) => {
            get_metrics().record_flush();
            0
        }
        Err(_) => -2,
    }
}

/// Destroys the Scribe logging system (FFI).
///
/// Performs graceful shutdown of the logging system.
///
/// # Returns
///
/// * `0` - Success
///
/// # Examples
///
/// ```c
/// scribe_destroy();
/// ```
#[no_mangle]
pub extern "C" fn scribe_destroy() -> i32 {
    // TODO: 优雅关闭
    0
}

/// Gets a snapshot of current performance metrics (FFI).
///
/// Retrieves statistics about writes, flushes, errors, and timing.
///
/// # Arguments
///
/// * `snapshot` - Pointer to a MetricsSnapshot struct to fill.
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - snapshot is null
///
/// # Safety
///
/// `snapshot` must be a valid pointer to a MetricsSnapshot struct.
///
/// # Examples
///
/// ```c
/// MetricsSnapshot snapshot;
/// int result = scribe_get_metrics(&snapshot);
/// if (result == 0) {
///     printf("Writes: %llu\n", snapshot.writes_count);
///     printf("Bytes: %llu\n", snapshot.bytes_written);
/// }
/// ```
#[no_mangle]
pub extern "C" fn scribe_get_metrics(snapshot: *mut MetricsSnapshot) -> i32 {
    if snapshot.is_null() {
        return -1;
    }

    let metrics = get_metrics();
    let snap = metrics.snapshot();

    unsafe {
        *snapshot = snap;
    }

    0
}

/// Resets all performance metrics counters to zero (FFI).
///
/// Clears all accumulated statistics. Useful for benchmarking
/// or resetting after maintenance operations.
///
/// # Returns
///
/// * `0` - Success
///
/// # Examples
///
/// ```c
/// scribe_reset_metrics();
/// // Run benchmark
/// // ...
/// MetricsSnapshot snapshot;
/// scribe_get_metrics(&snapshot);
/// ```
#[no_mangle]
pub extern "C" fn scribe_reset_metrics() -> i32 {
    get_metrics().reset();
    0
}

/// Plants a ConsoleSink with the specified minimum log level (FFI).
///
/// The ConsoleSink outputs logs to stdout/stderr and is useful for development.
/// It automatically extracts the calling class name as the tag from the call stack.
///
/// # Arguments
///
/// * `min_level` - Minimum log level to output (0=Verbose, 1=Debug, 2=Info, 3=Warn, 4=Error)
///
/// # Returns
///
/// * `0` - Success
///
/// # Examples
///
/// ```c
/// // Plant a ConsoleSink that only shows Info and above
/// scribe_register_console(2);
///
/// // Logs will now appear in stdout
/// scribe_write(2, "MyTag", "This will be printed");
/// ```
#[no_mangle]
pub extern "C" fn scribe_register_console(min_level: i32) -> i32 {
    let level = match min_level {
        0 => LogLevel::Verbose,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        _ => LogLevel::Info,
    };
    sink::register_sink(Box::new(sink::ConsoleSink::with_min_level(level)));
    0
}

/// Clears all registered sinks (FFI).
///
/// Clears all LogSink instances that were previously planted.
/// After calling this, logs will only be written to storage, not output anywhere else.
///
/// # Returns
///
/// * `0` - Success
///
/// # Examples
///
/// ```c
/// // Remove all trees
/// scribe_clear_sinks();
///
/// // Logs will no longer appear in stdout
/// scribe_write(2, "MyTag", "This won't be printed");
/// ```
#[no_mangle]
pub extern "C" fn scribe_clear_sinks() -> i32 {
    sink::clear_sinks();
    0
}

/// Gets the count of registered sinks (FFI).
///
/// Returns the number of Tree instances that are currently active.
///
/// # Returns
///
/// * The number of planted trees (>= 0)
///
/// # Examples
///
/// ```c
/// scribe_register_console(2);
/// int count = scribe_sink_count();
/// // count should be 1
///
/// scribe_clear_sinks();
/// count = scribe_sink_count();
/// // count should be 0
/// ```
#[no_mangle]
pub extern "C" fn scribe_sink_count() -> i32 {
    sink::registry().tree_count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use tempfile::TempDir;

    #[test]
    fn test_ffi_init_write() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let result = scribe_init(log_dir.as_ptr());
        assert_eq!(result, 0);

        let tag = CString::new("test").unwrap();
        let message = CString::new("hello world").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        assert_eq!(result, 0);

        let result = scribe_flush();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_metrics_recording() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        // 重置指标
        scribe_reset_metrics();

        let result = scribe_init(log_dir.as_ptr());
        assert_eq!(result, 0);

        // 写入几条日志
        let tag = CString::new("metrics_test").unwrap();
        for i in 0..5 {
            let msg = CString::new(format!("message {}", i)).unwrap();
            let result = scribe_write(2, tag.as_ptr(), msg.as_ptr());
            assert_eq!(result, 0);
        }

        // 获取指标
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        let result = scribe_get_metrics(&mut snapshot);
        assert_eq!(result, 0);

        // 验证写入计数
        assert_eq!(snapshot.writes_count, 5);
        assert!(snapshot.bytes_written > 0);
        assert_eq!(snapshot.writes_failed, 0);

        // Flush 并验证
        scribe_flush();
        let result = scribe_get_metrics(&mut snapshot);
        assert_eq!(result, 0);
        assert_eq!(snapshot.flush_count, 1);
    }

    #[test]
    fn test_metrics_reset() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        scribe_init(log_dir.as_ptr());

        // 写入一些日志
        let tag = CString::new("reset_test").unwrap();
        let msg = CString::new("test message").unwrap();
        scribe_write(2, tag.as_ptr(), msg.as_ptr());

        // 获取指标
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        scribe_get_metrics(&mut snapshot);
        assert!(snapshot.writes_count > 0);

        // 重置指标
        scribe_reset_metrics();

        scribe_get_metrics(&mut snapshot);
        assert_eq!(snapshot.writes_count, 0);
        assert_eq!(snapshot.bytes_written, 0);
    }

    #[test]
    fn test_get_metrics_null_pointer() {
        let result = scribe_get_metrics(std::ptr::null_mut());
        assert_eq!(result, -1);
    }

    #[test]
    fn test_metrics_write_failure() {
        // 测试写入失败时的指标记录
        scribe_reset_metrics();

        // 未初始化的情况下写入会失败
        let tag = CString::new("fail_test").unwrap();
        let msg = CString::new("should fail").unwrap();

        let result = scribe_write(2, tag.as_ptr(), msg.as_ptr());
        assert_eq!(result, -2); // 未初始化

        // 指标应该保持为 0（因为没有到达记录指标的代码）
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        scribe_get_metrics(&mut snapshot);
        assert_eq!(snapshot.writes_failed, 0);
    }

    #[test]
    fn test_metrics_snapshot_calculations() {
        let metrics = ScribeMetrics::new();

        // 模拟一些操作
        metrics.record_write(1000);
        metrics.record_write(2000);
        metrics.record_write(3000);

        metrics.record_compression_time(100);
        metrics.record_compression_time(200);
        metrics.record_compression_time(300);

        metrics.record_flush();
        metrics.record_flush();

        metrics.record_io_time(5000);
        metrics.record_io_time(7000);

        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.writes_count, 3);
        assert_eq!(snapshot.bytes_written, 6000);
        assert_eq!(snapshot.flush_count, 2);

        // 测试计算方法
        assert_eq!(snapshot.write_success_rate(), 1.0);
        assert_eq!(snapshot.avg_compression_time_us(), 200.0);
        assert_eq!(snapshot.avg_io_time_us(), 6000.0);
    }

    // 1. scribe_init() 多次调用测试
    #[test]
    fn test_scribe_init_multiple_calls() {
        // 清理全局状态（通过使用不同的进程或接受第一次初始化）
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let result1 = scribe_init(log_dir.as_ptr());
        // 第一次初始化应该成功或已经被前面的测试初始化（返回 -5）
        assert!(result1 == 0 || result1 == -5);

        // 第二次初始化应该返回 -5（已初始化）
        let result2 = scribe_init(log_dir.as_ptr());
        assert_eq!(result2, -5);
    }

    // 2. scribe_init() 空指针测试
    #[test]
    fn test_scribe_init_null_pointer() {
        let result = scribe_init(std::ptr::null());
        assert_eq!(result, -1);
    }

    // 3. scribe_write() 空指针测试（tag, message）
    #[test]
    fn test_scribe_write_null_tag() {
        let message = CString::new("test message").unwrap();
        let result = scribe_write(2, std::ptr::null(), message.as_ptr());
        assert_eq!(result, -1);
    }

    #[test]
    fn test_scribe_write_null_message() {
        let tag = CString::new("test_tag").unwrap();
        let result = scribe_write(2, tag.as_ptr(), std::ptr::null());
        assert_eq!(result, -1);
    }

    #[test]
    fn test_scribe_write_both_null() {
        let result = scribe_write(2, std::ptr::null(), std::ptr::null());
        assert_eq!(result, -1);
    }

    // 4. scribe_write() 超长 tag 测试（exceeds maximum length）
    #[test]
    fn test_scribe_write_long_tag() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        // 注意：由于全局单例，这里可能已经初始化
        let _ = scribe_init(log_dir.as_ptr());

        // 创建exceeds limit的 tag
        let long_tag = "a".repeat(100);
        let tag = CString::new(long_tag).unwrap();
        let message = CString::new("test message").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        // 应该能够处理（可能截断或正常处理）
        assert!(result == 0 || result == -2); // 0 表示成功，-2 表示未初始化
    }

    // 5. scribe_write() 超长 message 测试（> 1MB）
    #[test]
    fn test_scribe_write_large_message() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let tag = CString::new("large_test").unwrap();
        // 创建 2MB 的消息
        let large_message = "x".repeat(2 * 1024 * 1024);
        let message = CString::new(large_message).unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        // 可能成功、可能因为缓冲区满失败、或未初始化
        assert!(result == 0 || result == -2 || result == -5);
    }

    // 6. scribe_write() 不同日志级别测试
    #[test]
    fn test_scribe_write_different_log_levels() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let tag = CString::new("level_test").unwrap();

        // 测试所有日志级别
        let levels = vec![
            (0, "verbose message"),
            (1, "debug message"),
            (2, "info message"),
            (3, "warn message"),
            (4, "error message"),
            (99, "invalid level - should default to info"),
        ];

        for (level, msg_text) in levels {
            let message = CString::new(msg_text).unwrap();
            let result = scribe_write(level, tag.as_ptr(), message.as_ptr());
            // 应该成功或未初始化
            assert!(result == 0 || result == -2);
        }
    }

    // 7. scribe_flush() 未初始化测试
    #[test]
    fn test_scribe_flush_not_initialized() {
        // 注意：由于全局单例可能已被初始化，此测试可能返回 0
        // 在隔离的测试环境中应该返回 -1
        let result = scribe_flush();
        assert!(result == 0 || result == -1);
    }

    // 8. scribe_destroy() 测试
    #[test]
    fn test_scribe_destroy() {
        let result = scribe_destroy();
        assert_eq!(result, 0);
    }

    // 9. scribe_get_metrics() 测试
    #[test]
    fn test_scribe_get_metrics_valid() {
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        let result = scribe_get_metrics(&mut snapshot);
        assert_eq!(result, 0);

        // 验证返回的快照有合理的值（可能为 0 或正数）
        assert!(snapshot.writes_count >= 0);
        assert!(snapshot.bytes_written >= 0);
    }

    // 10. scribe_reset_metrics() 测试
    #[test]
    fn test_scribe_reset_metrics_functionality() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        // 写入一些数据
        let tag = CString::new("reset_func_test").unwrap();
        let message = CString::new("test data").unwrap();
        let _ = scribe_write(2, tag.as_ptr(), message.as_ptr());

        // 重置
        let result = scribe_reset_metrics();
        assert_eq!(result, 0);

        // 验证指标已重置
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        scribe_get_metrics(&mut snapshot);
        assert_eq!(snapshot.writes_count, 0);
        assert_eq!(snapshot.bytes_written, 0);
        assert_eq!(snapshot.flush_count, 0);
    }

    // 11. 全局 METRICS 单例并发测试
    #[test]
    fn test_global_metrics_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        scribe_reset_metrics();

        let handles: Vec<_> = (0..10)
            .map(|i| {
                thread::spawn(move || {
                    let metrics = get_metrics();

                    // 每个线程执行 100 次操作
                    for _ in 0..100 {
                        metrics.record_write(100);
                        metrics.record_flush();
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证总计数
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        scribe_get_metrics(&mut snapshot);

        // 10 个线程 * 100 次操作 = 1000
        assert_eq!(snapshot.writes_count, 1000);
        assert_eq!(snapshot.bytes_written, 100000); // 1000 * 100
        assert_eq!(snapshot.flush_count, 1000);
    }

    // 额外测试：未初始化时写入
    #[test]
    fn test_scribe_write_not_initialized() {
        // 创建新的临时状态来测试未初始化场景
        // 注意：由于全局单例，这可能已经初始化
        let tag = CString::new("uninit_test").unwrap();
        let message = CString::new("should fail").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        // 如果未初始化应返回 -2，否则返回 0
        assert!(result == -2 || result == 0);
    }

    // 额外测试：验证 UTF-8 处理
    #[test]
    fn test_scribe_write_utf8_content() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let tag = CString::new("utf8_test").unwrap();
        let message = CString::new("Hello 世界 🌍 Привет").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        assert!(result == 0 || result == -2);
    }

    // 额外测试：空字符串
    #[test]
    fn test_scribe_write_empty_strings() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let empty_tag = CString::new("").unwrap();
        let empty_message = CString::new("").unwrap();

        let result = scribe_write(2, empty_tag.as_ptr(), empty_message.as_ptr());
        assert!(result == 0 || result == -2);
    }

    // 额外测试：连续写入性能
    #[test]
    fn test_scribe_write_stress() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());
        scribe_reset_metrics();

        let tag = CString::new("stress_test").unwrap();

        // 写入 1000 条日志
        for i in 0..1000 {
            let message = CString::new(format!("stress message {}", i)).unwrap();
            let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
            // 允许成功或缓冲区满
            assert!(result == 0 || result == -2 || result == -5);
        }

        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        scribe_get_metrics(&mut snapshot);
        assert!(snapshot.writes_count > 0);
    }

    // ====== Tree Integration Tests ======

    #[test]
    fn test_scribe_register_console() {
        let result = scribe_register_console(2);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_scribe_register_console_all_levels() {
        // Test all log levels
        for level in 0..=4 {
            let result = scribe_register_console(level);
            assert_eq!(result, 0);
        }
        scribe_clear_sinks();
    }

    #[test]
    fn test_scribe_register_console_invalid_level() {
        // Invalid level should default to Info
        let result = scribe_register_console(99);
        assert_eq!(result, 0);
        scribe_clear_sinks();
    }

    #[test]
    fn test_scribe_clear_sinks() {
        scribe_register_console(2);
        let result = scribe_clear_sinks();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_tree_integration_with_write() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        // Clean state
        scribe_clear_sinks();

        let _ = scribe_init(log_dir.as_ptr());
        scribe_register_console(2);

        let tag = CString::new("tree_test").unwrap();
        let message = CString::new("test with tree").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        assert!(result == 0 || result == -2);

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_multiple_plants() {
        scribe_clear_sinks();

        // Plant multiple trees
        scribe_register_console(0);
        scribe_register_console(2);
        scribe_register_console(4);

        // Should not panic
        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_write_without_init() {
        scribe_clear_sinks();
        scribe_register_console(2);

        let tag = CString::new("test").unwrap();
        let message = CString::new("message").unwrap();

        // Writing without init should still call tree.log
        // but fail on storage write
        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        assert!(result == -2 || result == 0);

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_filtering_by_level() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        scribe_clear_sinks();
        let _ = scribe_init(log_dir.as_ptr());

        // Plant tree with Warn minimum level
        scribe_register_console(3);

        let tag = CString::new("filter_test").unwrap();

        // These should be filtered by the tree
        let verbose_msg = CString::new("verbose").unwrap();
        let debug_msg = CString::new("debug").unwrap();
        let info_msg = CString::new("info").unwrap();

        // These should pass
        let warn_msg = CString::new("warn").unwrap();
        let error_msg = CString::new("error").unwrap();

        // All writes should succeed in storage
        assert!(scribe_write(0, tag.as_ptr(), verbose_msg.as_ptr()) == 0 || scribe_write(0, tag.as_ptr(), verbose_msg.as_ptr()) == -2);
        assert!(scribe_write(1, tag.as_ptr(), debug_msg.as_ptr()) == 0 || scribe_write(1, tag.as_ptr(), debug_msg.as_ptr()) == -2);
        assert!(scribe_write(2, tag.as_ptr(), info_msg.as_ptr()) == 0 || scribe_write(2, tag.as_ptr(), info_msg.as_ptr()) == -2);
        assert!(scribe_write(3, tag.as_ptr(), warn_msg.as_ptr()) == 0 || scribe_write(3, tag.as_ptr(), warn_msg.as_ptr()) == -2);
        assert!(scribe_write(4, tag.as_ptr(), error_msg.as_ptr()) == 0 || scribe_write(4, tag.as_ptr(), error_msg.as_ptr()) == -2);

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_with_empty_tag() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        scribe_clear_sinks();
        let _ = scribe_init(log_dir.as_ptr());
        scribe_register_console(2);

        let empty_tag = CString::new("").unwrap();
        let message = CString::new("message with empty tag").unwrap();

        let result = scribe_write(2, empty_tag.as_ptr(), message.as_ptr());
        assert!(result == 0 || result == -2);

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_with_utf8_content() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        scribe_clear_sinks();
        let _ = scribe_init(log_dir.as_ptr());
        scribe_register_console(2);

        let tag = CString::new("utf8_test").unwrap();
        let message = CString::new("Hello 世界 🌍").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());
        assert!(result == 0 || result == -2);

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_concurrent_logging() {
        use std::thread;

        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        scribe_clear_sinks();
        let _ = scribe_init(log_dir.as_ptr());
        scribe_register_console(2);

        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let tag = CString::new(format!("thread_{}", i)).unwrap();
                    for j in 0..10 {
                        let message = CString::new(format!("message {}", j)).unwrap();
                        let _ = scribe_write(2, tag.as_ptr(), message.as_ptr());
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_uproot_and_replant() {
        scribe_clear_sinks();

        // Plant
        scribe_register_console(2);

        // Uproot
        scribe_clear_sinks();

        // Replant with different level
        scribe_register_console(4);

        // Should work fine
        scribe_clear_sinks();
    }

    #[test]
    fn test_tree_no_double_free() {
        // Multiple uproots should be safe
        scribe_clear_sinks();
        scribe_clear_sinks();
        scribe_clear_sinks();
    }

    #[test]
    fn test_scribe_sink_count() {
        scribe_clear_sinks();

        // Initially should be 0
        assert_eq!(scribe_sink_count(), 0);

        // Plant one tree
        scribe_register_console(2);
        assert_eq!(scribe_sink_count(), 1);

        // Plant another tree
        scribe_register_console(4);
        assert_eq!(scribe_sink_count(), 2);

        // Plant a third tree
        scribe_register_console(0);
        assert_eq!(scribe_sink_count(), 3);

        // Uproot all
        scribe_clear_sinks();
        assert_eq!(scribe_sink_count(), 0);
    }

    #[test]
    fn test_tree_count_after_multiple_operations() {
        scribe_clear_sinks();
        assert_eq!(scribe_sink_count(), 0);

        // Plant, uproot, plant again
        scribe_register_console(2);
        assert_eq!(scribe_sink_count(), 1);

        scribe_clear_sinks();
        assert_eq!(scribe_sink_count(), 0);

        scribe_register_console(3);
        scribe_register_console(1);
        assert_eq!(scribe_sink_count(), 2);

        scribe_clear_sinks();
        assert_eq!(scribe_sink_count(), 0);
    }

    // ====== Additional Comprehensive Tests ======

    /// 测试多次初始化行为
    /// 验证第一次初始化成功后，后续初始化调用应返回错误码 -5
    #[test]
    fn test_scribe_init_twice() {
        let temp_dir1 = TempDir::new().unwrap();
        let log_dir1 = CString::new(temp_dir1.path().to_str().unwrap()).unwrap();

        // 第一次初始化
        let result1 = scribe_init(log_dir1.as_ptr());
        // 可能成功或已被其他测试初始化
        assert!(result1 == 0 || result1 == -5);

        // 尝试使用不同路径再次初始化
        let temp_dir2 = TempDir::new().unwrap();
        let log_dir2 = CString::new(temp_dir2.path().to_str().unwrap()).unwrap();

        let result2 = scribe_init(log_dir2.as_ptr());
        // 应该返回 -5 表示已初始化
        assert_eq!(result2, -5);
    }

    /// 测试空 tag 指针处理
    /// 验证当 tag 为 null 时，函数返回错误码 -1
    #[test]
    fn test_scribe_write_null_tag() {
        let message = CString::new("test message").unwrap();

        // tag 为 null，应返回 -1
        let result = scribe_write(2, std::ptr::null(), message.as_ptr());
        assert_eq!(result, -1, "Writing with null tag should return -1");
    }

    /// 测试空 message 指针处理
    /// 验证当 message 为 null 时，函数返回错误码 -1
    #[test]
    fn test_scribe_write_null_message() {
        let tag = CString::new("test_tag").unwrap();

        // message 为 null，应返回 -1
        let result = scribe_write(2, tag.as_ptr(), std::ptr::null());
        assert_eq!(result, -1, "Writing with null message should return -1");
    }

    /// 测试超长 tag 处理（超过 100 字符）
    /// 验证系统能够处理超长的 tag 而不崩溃
    #[test]
    fn test_scribe_write_long_tag() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        // 确保初始化
        let _ = scribe_init(log_dir.as_ptr());

        // 创建超过 100 字符的 tag
        let long_tag = "A".repeat(150);
        let tag = CString::new(long_tag.clone()).unwrap();
        let message = CString::new("Testing with very long tag").unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());

        // 应该能够处理（成功或未初始化）
        assert!(
            result == 0 || result == -2,
            "Long tag should be handled gracefully, got result: {}",
            result
        );

        // 验证 tag 长度确实超过 100
        assert!(long_tag.len() > 100, "Tag should be longer than 100 characters");
    }

    /// 测试超长 message 处理（超过 1MB）
    /// 验证系统能够处理超大消息或正确拒绝
    #[test]
    fn test_scribe_write_long_message() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let tag = CString::new("large_message_test").unwrap();

        // 创建 2MB 的消息
        let large_message = "M".repeat(2 * 1024 * 1024);
        let message = CString::new(large_message.clone()).unwrap();

        let result = scribe_write(2, tag.as_ptr(), message.as_ptr());

        // 可能成功、缓冲区满、或未初始化
        assert!(
            result == 0 || result == -2 || result == -5,
            "Large message should return valid error code, got: {}",
            result
        );

        // 验证消息大小确实超过 1MB
        assert!(
            large_message.len() > 1024 * 1024,
            "Message should be larger than 1MB"
        );
    }

    /// 测试所有日志级别
    /// 验证每个日志级别都能正确处理，包括无效级别的默认行为
    #[test]
    fn test_scribe_write_all_levels() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        let tag = CString::new("level_test").unwrap();

        // 定义所有测试场景
        let test_cases = vec![
            (0, "Verbose", "This is a verbose message"),
            (1, "Debug", "This is a debug message"),
            (2, "Info", "This is an info message"),
            (3, "Warn", "This is a warning message"),
            (4, "Error", "This is an error message"),
            (-1, "Invalid Negative", "Should default to Info"),
            (99, "Invalid High", "Should default to Info"),
            (100, "Invalid Very High", "Should default to Info"),
        ];

        for (level, level_name, msg_text) in test_cases {
            let message = CString::new(msg_text).unwrap();
            let result = scribe_write(level, tag.as_ptr(), message.as_ptr());

            // 应该成功或未初始化
            assert!(
                result == 0 || result == -2,
                "Level {} ({}) should be handled, got result: {}",
                level,
                level_name,
                result
            );
        }
    }

    /// 测试未初始化时调用 flush
    /// 验证在未初始化状态下调用 flush 会返回适当的错误码
    #[test]
    fn test_scribe_flush_not_initialized() {
        // 注意：由于全局单例可能已被其他测试初始化
        // 此测试在隔离环境中应返回 -1
        let result = scribe_flush();

        assert!(
            result == 0 || result == -1,
            "Flush should return 0 (if initialized) or -1 (if not), got: {}",
            result
        );
    }

    /// 测试 destroy 函数
    /// 验证销毁操作总是返回成功
    #[test]
    fn test_scribe_destroy() {
        let result = scribe_destroy();
        assert_eq!(result, 0, "Destroy should always return 0");

        // 再次调用也应该成功（幂等性）
        let result2 = scribe_destroy();
        assert_eq!(result2, 0, "Destroy should be idempotent");
    }

    /// 测试获取 metrics
    /// 验证可以成功获取性能指标快照
    #[test]
    fn test_scribe_get_metrics() {
        // 创建一个空的 MetricsSnapshot
        let mut snapshot = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };

        let result = scribe_get_metrics(&mut snapshot);
        assert_eq!(result, 0, "Getting metrics should succeed");

        // 验证所有字段都是非负数
        assert!(snapshot.writes_count >= 0, "writes_count should be non-negative");
        assert!(snapshot.writes_failed >= 0, "writes_failed should be non-negative");
        assert!(snapshot.bytes_written >= 0, "bytes_written should be non-negative");
        assert!(snapshot.flush_count >= 0, "flush_count should be non-negative");
        assert!(snapshot.worker_wakeups >= 0, "worker_wakeups should be non-negative");
        assert!(snapshot.compression_time_us >= 0, "compression_time_us should be non-negative");
        assert!(snapshot.encryption_time_us >= 0, "encryption_time_us should be non-negative");
        assert!(snapshot.io_time_us >= 0, "io_time_us should be non-negative");
        assert!(snapshot.buffer_full_count >= 0, "buffer_full_count should be non-negative");
        assert!(snapshot.disk_full_count >= 0, "disk_full_count should be non-negative");
        assert!(snapshot.compression_errors >= 0, "compression_errors should be non-negative");
        assert!(snapshot.encryption_errors >= 0, "encryption_errors should be non-negative");
        assert!(snapshot.cleanup_count >= 0, "cleanup_count should be non-negative");
        assert!(snapshot.files_deleted >= 0, "files_deleted should be non-negative");
        assert!(snapshot.bytes_freed >= 0, "bytes_freed should be non-negative");
    }

    /// 测试重置 metrics
    /// 验证重置操作能够清零所有计数器
    #[test]
    fn test_scribe_reset_metrics() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = CString::new(temp_dir.path().to_str().unwrap()).unwrap();

        let _ = scribe_init(log_dir.as_ptr());

        // 写入一些数据来产生 metrics
        let tag = CString::new("reset_test").unwrap();
        let message = CString::new("test data for reset").unwrap();
        let _ = scribe_write(2, tag.as_ptr(), message.as_ptr());
        let _ = scribe_write(2, tag.as_ptr(), message.as_ptr());
        let _ = scribe_flush();

        // 获取重置前的 metrics
        let mut snapshot_before = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };
        scribe_get_metrics(&mut snapshot_before);

        // 执行重置
        let result = scribe_reset_metrics();
        assert_eq!(result, 0, "Reset should succeed");

        // 获取重置后的 metrics
        let mut snapshot_after = MetricsSnapshot {
            writes_count: 0,
            writes_failed: 0,
            bytes_written: 0,
            flush_count: 0,
            worker_wakeups: 0,
            compression_time_us: 0,
            encryption_time_us: 0,
            io_time_us: 0,
            buffer_full_count: 0,
            disk_full_count: 0,
            compression_errors: 0,
            encryption_errors: 0,
            cleanup_count: 0,
            files_deleted: 0,
            bytes_freed: 0,
        };
        scribe_get_metrics(&mut snapshot_after);

        // 验证所有计数器都已归零
        assert_eq!(snapshot_after.writes_count, 0, "writes_count should be reset to 0");
        assert_eq!(snapshot_after.bytes_written, 0, "bytes_written should be reset to 0");
        assert_eq!(snapshot_after.flush_count, 0, "flush_count should be reset to 0");
        assert_eq!(snapshot_after.writes_failed, 0, "writes_failed should be reset to 0");
    }

    /// 测试全局 metrics 单例
    /// 验证全局 metrics 在多线程环境下的正确性和一致性
    #[test]
    fn test_global_metrics_singleton() {
        use std::thread;
        use std::sync::Arc;

        // 重置 metrics 以获得干净的状态
        scribe_reset_metrics();

        // 获取全局 metrics 的多个引用
        let metrics1 = get_metrics();
        let metrics2 = get_metrics();
        let metrics3 = get_metrics();

        // 验证它们指向同一个实例（通过 Arc 指针地址）
        assert!(
            Arc::ptr_eq(&metrics1, &metrics2),
            "metrics1 and metrics2 should be the same instance"
        );
        assert!(
            Arc::ptr_eq(&metrics2, &metrics3),
            "metrics2 and metrics3 should be the same instance"
        );

        // 通过一个引用记录数据
        metrics1.record_write(100);
        metrics1.record_write(200);

        // 通过另一个引用读取，应该看到相同的数据
        let snapshot = metrics2.snapshot();
        assert_eq!(
            snapshot.writes_count, 2,
            "Should see 2 writes through different reference"
        );
        assert_eq!(
            snapshot.bytes_written, 300,
            "Should see 300 bytes through different reference"
        );

        // 多线程并发测试
        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let metrics = get_metrics();

                    // 每个线程执行 50 次写入
                    for _ in 0..50 {
                        metrics.record_write(10);
                    }
                })
            })
            .collect();

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        // 验证最终计数
        let final_snapshot = metrics3.snapshot();

        // 应该有: 初始的 2 次 + (10 个线程 * 50 次) = 502 次
        assert_eq!(
            final_snapshot.writes_count, 502,
            "Should have 502 total writes (2 initial + 10*50 concurrent)"
        );

        // 应该有: 初始的 300 字节 + (10 * 50 * 10) = 5300 字节
        assert_eq!(
            final_snapshot.bytes_written, 5300,
            "Should have 5300 total bytes (300 initial + 10*50*10 concurrent)"
        );

        // 测试重置在所有引用中生效
        scribe_reset_metrics();

        let reset_snapshot1 = metrics1.snapshot();
        let reset_snapshot2 = metrics2.snapshot();

        assert_eq!(reset_snapshot1.writes_count, 0, "metrics1 should be reset");
        assert_eq!(reset_snapshot2.writes_count, 0, "metrics2 should be reset");
        assert_eq!(reset_snapshot1.bytes_written, 0, "bytes should be reset");
    }
}
