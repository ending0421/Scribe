# Integration Guide

This guide covers how to integrate Scribe into different platforms and environments.

## Android Integration

### Prerequisites

- Android NDK installed
- Rust Android targets configured
- `cargo-ndk` tool installed

### Setup

1. **Install Required Tools**

```bash
# Install cargo-ndk
cargo install cargo-ndk

# Add Android targets
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

2. **Configure Cargo.toml for Android**

```toml
[lib]
name = "scribe"
crate-type = ["cdylib", "staticlib"]

[dependencies]
jni = "0.21"
android_logger = "0.13"

[profile.release]
opt-level = 3
lto = true
strip = true
```

3. **Create JNI Bindings**

Create `src/android/mod.rs`:

```rust
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jlong;
use crate::{Logger, LoggerConfig, LogLevel};
use std::path::PathBuf;

#[no_mangle]
pub extern "C" fn Java_com_example_scribe_ScribeLogger_nativeInit(
    env: JNIEnv,
    _class: JClass,
    log_dir: JString,
    max_file_size: jlong,
    max_files: jlong,
) -> jlong {
    let log_dir: String = env.get_string(log_dir)
        .expect("Couldn't get log directory")
        .into();
    
    let config = LoggerConfig {
        log_dir: PathBuf::from(log_dir),
        max_file_size: max_file_size as usize,
        max_files: max_files as usize,
        enable_compression: true,
        ..Default::default()
    };
    
    match Logger::with_config(config) {
        Ok(logger) => Box::into_raw(Box::new(logger)) as jlong,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn Java_com_example_scribe_ScribeLogger_nativeLog(
    env: JNIEnv,
    _class: JClass,
    logger_ptr: jlong,
    level: jlong,
    message: JString,
) {
    let logger = unsafe { &*(logger_ptr as *const Logger) };
    let message: String = env.get_string(message)
        .expect("Couldn't get message")
        .into();
    
    let log_level = match level {
        0 => LogLevel::Debug,
        1 => LogLevel::Info,
        2 => LogLevel::Warning,
        3 => LogLevel::Error,
        _ => LogLevel::Info,
    };
    
    logger.log(log_level, &message);
}

#[no_mangle]
pub extern "C" fn Java_com_example_scribe_ScribeLogger_nativeDestroy(
    _env: JNIEnv,
    _class: JClass,
    logger_ptr: jlong,
) {
    if logger_ptr != 0 {
        unsafe {
            let _ = Box::from_raw(logger_ptr as *mut Logger);
        }
    }
}
```

4. **Create Java Wrapper**

Create `ScribeLogger.java`:

```java
package com.example.scribe;

public class ScribeLogger {
    static {
        System.loadLibrary("scribe");
    }
    
    private long nativeHandle;
    
    public ScribeLogger(String logDir, long maxFileSize, long maxFiles) {
        nativeHandle = nativeInit(logDir, maxFileSize, maxFiles);
        if (nativeHandle == 0) {
            throw new RuntimeException("Failed to initialize Scribe logger");
        }
    }
    
    public void debug(String message) {
        nativeLog(nativeHandle, 0, message);
    }
    
    public void info(String message) {
        nativeLog(nativeHandle, 1, message);
    }
    
    public void warn(String message) {
        nativeLog(nativeHandle, 2, message);
    }
    
    public void error(String message) {
        nativeLog(nativeHandle, 3, message);
    }
    
    public void close() {
        if (nativeHandle != 0) {
            nativeDestroy(nativeHandle);
            nativeHandle = 0;
        }
    }
    
    @Override
    protected void finalize() throws Throwable {
        close();
        super.finalize();
    }
    
    private native long nativeInit(String logDir, long maxFileSize, long maxFiles);
    private native void nativeLog(long handle, long level, String message);
    private native void nativeDestroy(long handle);
}
```

5. **Build for Android**

```bash
# Build for all Android architectures
cargo ndk --target aarch64-linux-android --android-platform 21 -- build --release
cargo ndk --target armv7-linux-androideabi --android-platform 21 -- build --release
cargo ndk --target x86_64-linux-android --android-platform 21 -- build --release
cargo ndk --target i686-linux-android --android-platform 21 -- build --release
```

6. **Usage in Android App**

```java
public class MainActivity extends AppCompatActivity {
    private ScribeLogger logger;
    
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        
        String logDir = getExternalFilesDir(null) + "/logs";
        logger = new ScribeLogger(logDir, 10 * 1024 * 1024, 5);
        
        logger.info("Application started");
    }
    
    @Override
    protected void onDestroy() {
        logger.close();
        super.onDestroy();
    }
}
```

### Android Best Practices

1. **Use Internal Storage for Logs**

```java
// Use app-specific directory
String logDir = context.getFilesDir() + "/logs";
// Or external storage (requires permission)
String logDir = context.getExternalFilesDir(null) + "/logs";
```

2. **Handle Permissions**

Add to `AndroidManifest.xml` if using external storage:

```xml
<uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
<uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
```

3. **Lifecycle Management**

```java
public class LoggerManager {
    private static ScribeLogger instance;
    
    public static synchronized ScribeLogger getInstance(Context context) {
        if (instance == null) {
            String logDir = context.getFilesDir() + "/logs";
            instance = new ScribeLogger(logDir, 10 * 1024 * 1024, 5);
        }
        return instance;
    }
    
    public static void cleanup() {
        if (instance != null) {
            instance.close();
            instance = null;
        }
    }
}
```

## iOS Integration

### Prerequisites

- Xcode installed
- Rust iOS targets configured
- `cargo-lipo` or `cargo-xcode` installed

### Setup

1. **Install Required Tools**

```bash
# Install cargo-lipo
cargo install cargo-lipo

# Add iOS targets
rustup target add aarch64-apple-ios
rustup target add x86_64-apple-ios
rustup target add aarch64-apple-ios-sim
```

2. **Configure Cargo.toml for iOS**

```toml
[lib]
name = "scribe"
crate-type = ["staticlib", "cdylib"]

[dependencies]
# iOS-specific dependencies if needed
```

3. **Create C Bindings**

Create `src/ios/mod.rs`:

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use crate::{Logger, LoggerConfig, LogLevel};

#[repr(C)]
pub struct ScribeLoggerHandle {
    logger: *mut Logger,
}

#[no_mangle]
pub extern "C" fn scribe_logger_create(
    log_dir: *const c_char,
    max_file_size: usize,
    max_files: usize,
) -> *mut ScribeLoggerHandle {
    let log_dir = unsafe {
        if log_dir.is_null() {
            return std::ptr::null_mut();
        }
        CStr::from_ptr(log_dir)
    };
    
    let log_dir_str = match log_dir.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let config = LoggerConfig {
        log_dir: PathBuf::from(log_dir_str),
        max_file_size,
        max_files,
        enable_compression: true,
        ..Default::default()
    };
    
    match Logger::with_config(config) {
        Ok(logger) => {
            let handle = Box::new(ScribeLoggerHandle {
                logger: Box::into_raw(Box::new(logger)),
            });
            Box::into_raw(handle)
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn scribe_logger_log(
    handle: *mut ScribeLoggerHandle,
    level: u32,
    message: *const c_char,
) {
    if handle.is_null() || message.is_null() {
        return;
    }
    
    let handle = unsafe { &*handle };
    let logger = unsafe { &*handle.logger };
    
    let message = unsafe { CStr::from_ptr(message) };
    let message_str = match message.to_str() {
        Ok(s) => s,
        Err(_) => return,
    };
    
    let log_level = match level {
        0 => LogLevel::Debug,
        1 => LogLevel::Info,
        2 => LogLevel::Warning,
        3 => LogLevel::Error,
        _ => LogLevel::Info,
    };
    
    logger.log(log_level, message_str);
}

#[no_mangle]
pub extern "C" fn scribe_logger_destroy(handle: *mut ScribeLoggerHandle) {
    if !handle.is_null() {
        unsafe {
            let handle = Box::from_raw(handle);
            let _ = Box::from_raw(handle.logger);
        }
    }
}
```

4. **Create Header File**

Create `scribe.h`:

```c
#ifndef SCRIBE_H
#define SCRIBE_H

#include <stddef.h>
#include <stdint.h>

typedef struct ScribeLoggerHandle ScribeLoggerHandle;

ScribeLoggerHandle* scribe_logger_create(
    const char* log_dir,
    size_t max_file_size,
    size_t max_files
);

void scribe_logger_log(
    ScribeLoggerHandle* handle,
    uint32_t level,
    const char* message
);

void scribe_logger_destroy(ScribeLoggerHandle* handle);

#endif // SCRIBE_H
```

5. **Create Swift Wrapper**

Create `ScribeLogger.swift`:

```swift
import Foundation

public enum LogLevel: UInt32 {
    case debug = 0
    case info = 1
    case warning = 2
    case error = 3
}

public class ScribeLogger {
    private var handle: OpaquePointer?
    
    public init?(logDir: String, maxFileSize: Int, maxFiles: Int) {
        handle = scribe_logger_create(logDir, maxFileSize, maxFiles)
        if handle == nil {
            return nil
        }
    }
    
    deinit {
        if let handle = handle {
            scribe_logger_destroy(handle)
        }
    }
    
    public func log(level: LogLevel, message: String) {
        guard let handle = handle else { return }
        message.withCString { cString in
            scribe_logger_log(handle, level.rawValue, cString)
        }
    }
    
    public func debug(_ message: String) {
        log(level: .debug, message: message)
    }
    
    public func info(_ message: String) {
        log(level: .info, message: message)
    }
    
    public func warn(_ message: String) {
        log(level: .warning, message: message)
    }
    
    public func error(_ message: String) {
        log(level: .error, message: message)
    }
}
```

6. **Build for iOS**

```bash
# Build universal library
cargo lipo --release

# Or build for specific targets
cargo build --target aarch64-apple-ios --release
cargo build --target x86_64-apple-ios --release
```

7. **Usage in iOS App**

```swift
class AppDelegate: UIResponder, UIApplicationDelegate {
    var logger: ScribeLogger?
    
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        let documentsPath = NSSearchPathForDirectoriesInDomains(
            .documentDirectory,
            .userDomainMask,
            true
        )[0]
        let logDir = "\(documentsPath)/logs"
        
        logger = ScribeLogger(
            logDir: logDir,
            maxFileSize: 10 * 1024 * 1024,
            maxFiles: 5
        )
        
        logger?.info("Application started")
        
        return true
    }
}
```

### iOS Best Practices

1. **Use Documents Directory for Logs**

```swift
let documentsPath = FileManager.default.urls(
    for: .documentDirectory,
    in: .userDomainMask
).first!
let logDir = documentsPath.appendingPathComponent("logs").path
```

2. **Handle App Lifecycle**

```swift
class LoggerManager {
    static let shared = LoggerManager()
    private var logger: ScribeLogger?
    
    private init() {
        setupLogger()
        
        NotificationCenter.default.addObserver(
            self,
            selector: #selector(applicationWillTerminate),
            name: UIApplication.willTerminateNotification,
            object: nil
        )
    }
    
    private func setupLogger() {
        let documentsPath = FileManager.default.urls(
            for: .documentDirectory,
            in: .userDomainMask
        ).first!
        let logDir = documentsPath.appendingPathComponent("logs").path
        
        logger = ScribeLogger(
            logDir: logDir,
            maxFileSize: 10 * 1024 * 1024,
            maxFiles: 5
        )
    }
    
    @objc private func applicationWillTerminate() {
        logger = nil // Trigger deinit
    }
    
    func log(_ message: String, level: LogLevel = .info) {
        logger?.log(level: level, message: message)
    }
}
```

## Configuration Best Practices

### Development vs Production

```rust
pub fn create_logger_for_environment(is_production: bool) -> Result<Logger, std::io::Error> {
    let config = if is_production {
        LoggerConfig {
            log_dir: PathBuf::from("/var/log/myapp"),
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_files: 20,
            buffer_size: 32768,
            enable_compression: true,
            compression_level: 9,
            console_output: false,
            ..Default::default()
        }
    } else {
        LoggerConfig {
            log_dir: PathBuf::from("./logs"),
            max_file_size: 10 * 1024 * 1024, // 10 MB
            max_files: 3,
            buffer_size: 8192,
            enable_compression: false,
            console_output: true,
            ..Default::default()
        }
    };
    
    Logger::with_config(config)
}
```

### Environment Variables

```rust
use std::env;

pub fn create_logger_from_env() -> Result<Logger, Box<dyn std::error::Error>> {
    let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "./logs".to_string());
    let max_file_size = env::var("LOG_MAX_FILE_SIZE")
        .unwrap_or_else(|_| "10485760".to_string())
        .parse()?;
    let max_files = env::var("LOG_MAX_FILES")
        .unwrap_or_else(|_| "5".to_string())
        .parse()?;
    let enable_compression = env::var("LOG_ENABLE_COMPRESSION")
        .unwrap_or_else(|_| "false".to_string())
        .parse()?;
    
    let config = LoggerConfig {
        log_dir: PathBuf::from(log_dir),
        max_file_size,
        max_files,
        enable_compression,
        ..Default::default()
    };
    
    Ok(Logger::with_config(config)?)
}
```

### Configuration File

```rust
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct LoggerSettings {
    pub log_dir: String,
    pub max_file_size: usize,
    pub max_files: usize,
    pub buffer_size: usize,
    pub enable_compression: bool,
    pub compression_level: u32,
}

impl Default for LoggerSettings {
    fn default() -> Self {
        Self {
            log_dir: "./logs".to_string(),
            max_file_size: 10 * 1024 * 1024,
            max_files: 5,
            buffer_size: 8192,
            enable_compression: false,
            compression_level: 6,
        }
    }
}

pub fn load_config(path: &str) -> Result<LoggerConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let settings: LoggerSettings = toml::from_str(&content)?;
    
    Ok(LoggerConfig {
        log_dir: PathBuf::from(settings.log_dir),
        max_file_size: settings.max_file_size,
        max_files: settings.max_files,
        buffer_size: settings.buffer_size,
        enable_compression: settings.enable_compression,
        compression_level: settings.compression_level,
        ..Default::default()
    })
}
```

Example `logger.toml`:

```toml
log_dir = "./logs"
max_file_size = 10485760  # 10 MB
max_files = 5
buffer_size = 8192
enable_compression = true
compression_level = 6
```

## Thread Safety

Scribe is designed to be thread-safe. Share logger instances across threads using `Arc`:

```rust
use std::sync::Arc;
use scribe::Logger;

fn main() {
    let logger = Arc::new(Logger::new());
    
    let handles: Vec<_> = (0..10).map(|i| {
        let logger = Arc::clone(&logger);
        std::thread::spawn(move || {
            logger.info(&format!("Thread {} started", i));
        })
    }).collect();
    
    for handle in handles {
        handle.join().unwrap();
    }
}
```

## Next Steps

- Review [Performance Tuning](./PERFORMANCE-TUNING.md) for optimization tips
- Check [Troubleshooting](./TROUBLESHOOTING.md) for common issues
