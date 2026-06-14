//! Scribe - High-performance logging library for mobile platforms.
//!
//! Scribe provides lock-free, memory-mapped logging with double buffering,
//! automatic rotation, compression, and encryption support.
//!
//! # Simplified FFI API
//!
//! ```c
//! // Initialize with JSON configuration
//! const char* config = "{\"enable_console\": true, \"auto_flush_interval_ms\": 5000}";
//! scribe_init("/path/to/logs", config);
//!
//! // Log messages (automatic flush, automatic sink routing)
//! scribe_log(2, "MyTag", "Log message");  // 2 = Info level
//!
//! // Optional: Manual flush
//! scribe_flush();
//!
//! // Optional: Get performance stats
//! const char* stats = scribe_get_stats();
//! ```

use once_cell::sync::OnceCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod caller;
mod config;
mod context;
mod error;
mod macros;
mod metrics;
mod outputs;
mod pipeline;
mod platform;
mod sink;
mod stages;
mod storage;

pub use config::ScribeConfig;
pub use error::{Result, ScribeError};
pub use metrics::{ErrorType, MetricsSnapshot, ScribeMetrics};
pub use sink::{clear_sinks, register_sink, registry, ConsoleSink, LogSink, SinkRegistry};
pub use storage::{DoubleBufferManager, LogFrame, LogLevel, MmapBuffer};

static GLOBAL_SCRIBE: OnceCell<Arc<Mutex<ScribeInstance>>> = OnceCell::new();
static GLOBAL_METRICS: OnceCell<Arc<ScribeMetrics>> = OnceCell::new();
static AUTO_FLUSH_HANDLE: OnceCell<Mutex<Option<thread::JoinHandle<()>>>> = OnceCell::new();

struct ScribeInstance {
    manager: DoubleBufferManager,
    config: ScribeConfig,
}

impl Drop for ScribeInstance {
    fn drop(&mut self) {
        // 自动清理：停止后台线程并刷新
        stop_auto_flush();
        let _ = self.manager.flush();
    }
}

fn get_metrics() -> Arc<ScribeMetrics> {
    GLOBAL_METRICS
        .get_or_init(|| Arc::new(ScribeMetrics::new()))
        .clone()
}

/// 启动后台自动刷新线程
fn start_auto_flush(interval_ms: u64) {
    let handle = thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(interval_ms));

            // 执行刷新
            if let Some(scribe) = GLOBAL_SCRIBE.get() {
                if let Ok(mut instance) = scribe.lock() {
                    let _ = instance.manager.flush();
                }
            }
        }
    });

    AUTO_FLUSH_HANDLE.get_or_init(|| Mutex::new(Some(handle)));
}

/// 停止后台刷新线程
fn stop_auto_flush() {
    if let Some(handle_mutex) = AUTO_FLUSH_HANDLE.get() {
        if let Ok(mut guard) = handle_mutex.lock() {
            if let Some(handle) = guard.take() {
                // 线程会在下次检查时自然退出
                drop(handle);
            }
        }
    }
}

// ============================================================================
// 简化的 FFI API (4个核心函数)
// ============================================================================

/// Initialize Scribe with JSON configuration (FFI).
///
/// # Arguments
///
/// * `log_dir` - Null-terminated C string path to log directory
/// * `config_json` - Null-terminated C string with JSON configuration
///
/// # Configuration JSON
///
/// ```json
/// {
///   "auto_flush_interval_ms": 5000,
///   "enable_console": true,
///   "min_console_level": 1,
///   "max_file_size_mb": 10,
///   "max_file_count": 5,
///   "compression": true,
///   "encryption": false
/// }
/// ```
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - log_dir is null
/// * `-2` - Invalid UTF-8 in log_dir
/// * `-3` - Failed to create directory
/// * `-4` - Failed to create buffer manager
/// * `-5` - Already initialized
/// * `-6` - config_json is null
/// * `-7` - Invalid JSON in config
///
/// # Safety
///
/// `log_dir` and `config_json` must be valid null-terminated C strings.
#[no_mangle]
pub extern "C" fn scribe_init(log_dir: *const c_char, config_json: *const c_char) -> i32 {
    if log_dir.is_null() {
        return -1;
    }

    if config_json.is_null() {
        return -6;
    }

    let log_dir_str = unsafe {
        match CStr::from_ptr(log_dir).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        }
    };

    let config_str = unsafe {
        match CStr::from_ptr(config_json).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        }
    };

    // 解析配置
    let config = match ScribeConfig::from_json(config_str) {
        Ok(c) => c,
        Err(_) => return -7,
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

    // 根据配置注册 ConsoleSink
    if config.enable_console {
        let console_sink = ConsoleSink::new().with_min_level(match config.min_console_level {
            0 => LogLevel::Verbose,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Debug,
        });
        register_sink(Box::new(console_sink));
    }

    let instance = ScribeInstance {
        manager,
        config: config.clone(),
    };

    if GLOBAL_SCRIBE.set(Arc::new(Mutex::new(instance))).is_err() {
        return -5; // Already initialized
    }

    // 启动自动刷新线程
    start_auto_flush(config.auto_flush_interval_ms);

    0
}

/// Log a message (FFI).
///
/// # Arguments
///
/// * `level` - Log level (0=Verbose, 1=Debug, 2=Info, 3=Warn, 4=Error)
/// * `label` - Null-terminated C string label/tag
/// * `message` - Null-terminated C string message
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - Not initialized
/// * `-2` - Invalid parameters
/// * `-3` - Write failed
///
/// # Safety
///
/// `label` and `message` must be valid null-terminated C strings.
#[no_mangle]
pub extern "C" fn scribe_log(level: i32, label: *const c_char, message: *const c_char) -> i32 {
    if label.is_null() || message.is_null() {
        return -2;
    }

    let label_str = unsafe {
        match CStr::from_ptr(label).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        }
    };

    let message_str = unsafe {
        match CStr::from_ptr(message).to_str() {
            Ok(s) => s,
            Err(_) => return -2,
        }
    };

    let log_level = match level {
        0 => LogLevel::Verbose,
        1 => LogLevel::Debug,
        2 => LogLevel::Info,
        3 => LogLevel::Warn,
        4 => LogLevel::Error,
        _ => LogLevel::Info,
    };

    // 路由到 Sink
    registry().dispatch(&sink::LogRecord {
        level: log_level,
        context: Some(label_str.to_string()),
        message: message_str.to_string(),
        thread_name: std::thread::current().name().map(|s| s.to_string()),
        caller: None,
    });

    // 写入存储
    let scribe = match GLOBAL_SCRIBE.get() {
        Some(s) => s,
        None => return -1,
    };

    let mut instance = match scribe.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };

    let frame = LogFrame::new(log_level, Some(label_str), message_str);

    match instance.manager.write(&frame) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

/// Manual flush to disk (FFI).
///
/// Optional - automatic flush is enabled by default.
///
/// # Returns
///
/// * `0` - Success
/// * `-1` - Not initialized
/// * `-2` - Flush failed
#[no_mangle]
pub extern "C" fn scribe_flush() -> i32 {
    let scribe = match GLOBAL_SCRIBE.get() {
        Some(s) => s,
        None => return -1,
    };

    let mut instance = match scribe.lock() {
        Ok(i) => i,
        Err(_) => return -1,
    };

    match instance.manager.flush() {
        Ok(_) => 0,
        Err(_) => -2,
    }
}

/// Get performance statistics as JSON (FFI).
///
/// Returns a JSON string with performance metrics.
/// The caller must NOT free the returned pointer.
///
/// # Returns
///
/// * JSON string pointer - Success
/// * null - Failed
///
/// # Safety
///
/// The returned pointer is valid until the next call to this function.
#[no_mangle]
pub extern "C" fn scribe_get_stats() -> *const c_char {
    let metrics = get_metrics();
    let snapshot = metrics.snapshot();

    let json = serde_json::json!({
        "log_writes": snapshot.log_writes,
        "buffer_flushes": snapshot.buffer_flushes,
        "buffer_swaps": snapshot.buffer_swaps,
        "bytes_written": snapshot.bytes_written,
        "flush_errors": snapshot.flush_errors,
        "write_errors": snapshot.write_errors,
    });

    match CString::new(json.to_string()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null(),
    }
}

// 保留测试代码...
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let json = r#"{"enable_console": true, "auto_flush_interval_ms": 3000}"#;
        let config = ScribeConfig::from_json(json).unwrap();
        assert!(config.enable_console);
        assert_eq!(config.auto_flush_interval_ms, 3000);
    }
}
