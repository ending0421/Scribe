# Scribe API 文档

## 概述

Scribe 是一个高性能的跨平台日志库，提供以下特性：

- 🚀 **高性能**：基于 mmap 的零拷贝写入
- 🔒 **线程安全**：无锁并发设计
- 💾 **崩溃恢复**：自动恢复未完成的日志
- 🗜️ **压缩支持**：zstd 压缩（5:1 压缩率）
- 🔐 **加密支持**：ChaCha20 加密
- 🔄 **自动管理**：自动刷新、自动清理、自动轮转

## 架构设计

```
用户代码
    ↓
Kotlin/Swift API (语言适配层)
    ↓
Rust FFI (4个核心函数)
    ↓
Rust 核心引擎
    ├─ 后台刷新线程
    ├─ 配置驱动的 Sink
    ├─ Drop trait 自动清理
    └─ mmap 存储
```

## Rust FFI API

### 核心函数

#### scribe_init

初始化 Scribe 日志系统。

```c
int scribe_init(const char* log_dir, const char* config_json);
```

**参数：**
- `log_dir`: 日志目录路径（C 字符串）
- `config_json`: JSON 配置字符串

**返回值：**
- `0`: 成功
- `-1`: log_dir 为 null
- `-2`: 无效的 UTF-8
- `-3`: 创建目录失败
- `-4`: 创建 buffer 失败
- `-5`: 已经初始化
- `-6`: config_json 为 null
- `-7`: 无效的 JSON

**配置示例：**
```json
{
  "auto_flush_interval_ms": 5000,
  "enable_console": true,
  "min_console_level": 1,
  "compression": true
}
```

#### scribe_log

写入日志消息。

```c
int scribe_log(int level, const char* label, const char* message);
```

**参数：**
- `level`: 日志级别（0=Verbose, 1=Debug, 2=Info, 3=Warn, 4=Error）
- `label`: 日志标签（C 字符串）
- `message`: 日志消息（C 字符串）

**返回值：**
- `0`: 成功
- `-1`: 未初始化
- `-2`: 无效参数
- `-3`: 写入失败

#### scribe_flush

手动刷新日志到磁盘（可选）。

```c
int scribe_flush();
```

**返回值：**
- `0`: 成功
- `-1`: 未初始化
- `-2`: 刷新失败

**注意：** 自动刷新默认启用，通常不需要手动调用。

#### scribe_get_stats

获取性能统计信息（JSON 格式）。

```c
const char* scribe_get_stats();
```

**返回值：**
- JSON 字符串指针（成功）
- NULL（失败）

**JSON 格式：**
```json
{
  "log_writes": 1000,
  "buffer_flushes": 10,
  "buffer_swaps": 5,
  "bytes_written": 102400,
  "flush_errors": 0,
  "write_errors": 0
}
```

## Android API (Kotlin)

### 初始化

```kotlin
import com.scribe.Scribe

val config = Scribe.Config(
    autoFlushIntervalMs = 5000,
    enableConsole = BuildConfig.DEBUG,
    minConsoleLevel = LogLevel.DEBUG,
    maxFileSizeMb = 10,
    maxFileCount = 5,
    compression = true,
    encryption = false
)

Scribe.initialize(
    logDir = "${filesDir}/logs",
    config = config
).onSuccess {
    println("Scribe initialized")
}.onFailure { e ->
    Log.e("App", "Init failed", e)
}
```

### 日志方法

```kotlin
// 基础方法
Scribe.v("Tag", "Verbose message")
Scribe.d("Tag", "Debug message")
Scribe.i("Tag", "Info message")
Scribe.w("Tag", "Warning message")
Scribe.e("Tag", "Error message")

// Scoped Logger
val logger = Scribe.logger("MyClass")
logger.d("Debug message")
logger.i("Info message")

// DSL 风格
scribeLogger("MyFeature") {
    d("Starting")
    i("Initialized")
}
```

### 配置类

```kotlin
data class Config(
    val autoFlushIntervalMs: Long = 5000,
    val enableConsole: Boolean = false,
    val minConsoleLevel: LogLevel = LogLevel.DEBUG,
    val maxFileSizeMb: Int = 10,
    val maxFileCount: Int = 5,
    val compression: Boolean = true,
    val encryption: Boolean = false
)
```

### 日志级别

```kotlin
enum class LogLevel(val value: Int) {
    VERBOSE(0),
    DEBUG(1),
    INFO(2),
    WARN(3),
    ERROR(4)
}
```

## iOS API (Swift)

### 初始化

```swift
import Scribe

let config = ScribeConfig(
    autoFlushIntervalMs: 5000,
    enableConsole: true,
    minConsoleLevel: .debug,
    maxFileSizeMb: 10,
    maxFileCount: 5,
    compression: true,
    encryption: false
)

Task {
    do {
        try await Scribe.initialize(
            logDir: documentsDir + "/logs",
            config: config
        )
        print("Scribe initialized")
    } catch {
        print("Init failed: \(error)")
    }
}
```

### 日志方法

```swift
// 基础方法
Scribe.v("Tag", "Verbose message")
Scribe.d("Tag", "Debug message")
Scribe.i("Tag", "Info message")
Scribe.w("Tag", "Warning message")
Scribe.e("Tag", "Error message")

// Scoped Logger
let logger = scribeLogger(label: "MyClass")
logger.d("Debug message")
logger.i("Info message")
```

### 配置结构

```swift
public struct ScribeConfig: Sendable {
    public let autoFlushIntervalMs: Int
    public let enableConsole: Bool
    public let minConsoleLevel: LogLevel
    public let maxFileSizeMb: Int
    public let maxFileCount: Int
    public let compression: Bool
    public let encryption: Bool
}
```

### 日志级别

```swift
public enum LogLevel: Int, Sendable {
    case verbose = 0
    case debug = 1
    case info = 2
    case warn = 3
    case error = 4
}
```

## 自动化功能

### 自动刷新

- 后台线程定期刷新（可配置间隔）
- 默认 5 秒
- 无需手动管理

### 自动清理

- **Android**: `Runtime.addShutdownHook()` 自动刷新
- **iOS**: Actor `deinit` 自动清理
- 进程退出时自动执行

### 自动轮转

- 文件大小达到限制自动轮转
- 保持最新的 N 个文件
- 旧文件自动删除

### 崩溃恢复

- 启动时自动检测
- 恢复未完成的日志
- 无需手动干预

## 性能特性

| 特性 | 指标 |
|------|------|
| 写入延迟 | < 100ns |
| 吞吐量 | > 5K logs/sec |
| 压缩率 | 5:1 (zstd) |
| 内存占用 | < 10MB |
| 线程安全 | 无锁并发 |

## 错误处理

### Kotlin

```kotlin
Scribe.initialize(logDir, config)
    .onSuccess { /* 成功 */ }
    .onFailure { error ->
        when (error) {
            is ScribeException -> {
                // 处理 Scribe 特定错误
            }
            else -> {
                // 其他错误
            }
        }
    }
```

### Swift

```swift
do {
    try await Scribe.initialize(logDir: logDir, config: config)
} catch ScribeError.initializationFailed(let code) {
    print("Init failed with code: \(code)")
} catch {
    print("Unknown error: \(error)")
}
```

## 线程安全

Scribe 完全线程安全，可以从任何线程调用：

- **Rust**: 使用 `Mutex` 和 `Arc` 保护共享状态
- **Kotlin**: 使用协程和 `CoroutineScope`
- **Swift**: 使用 `Actor` 隔离

## 最佳实践

1. **初始化时机**
   - Android: `Application.onCreate()`
   - iOS: `AppDelegate.didFinishLaunching`

2. **配置策略**
   - Debug 模式：启用控制台，低刷新间隔
   - Release 模式：禁用控制台，高刷新间隔

3. **性能优化**
   - 使用 Scoped Logger 避免重复传递标签
   - 批量写入后统一刷新
   - 合理设置文件大小和数量

4. **错误处理**
   - 初始化失败要有降级方案
   - 写入错误不应影响主逻辑
   - 定期检查统计信息
