# Scribe 使用指南

## 快速开始

### Android (Kotlin)

```kotlin
import com.scribe.Scribe

// 1. 初始化（在 Application.onCreate）
val config = Scribe.Config(
    autoFlushIntervalMs = 5000,          // 5秒自动刷新
    enableConsole = BuildConfig.DEBUG,   // Debug模式启用控制台
    minConsoleLevel = LogLevel.DEBUG,
    compression = true
)

Scribe.initialize(
    logDir = "${filesDir}/logs",
    config = config
).getOrElse { e ->
    Log.e("App", "Scribe init failed", e)
}

// 2. 使用日志
Scribe.d("MyTag", "Debug message")
Scribe.i("MyTag", "Info message")
Scribe.w("MyTag", "Warning message")
Scribe.e("MyTag", "Error message")

// 3. 可选：手动刷新
Scribe.flush()

// 4. 可选：获取统计
val stats = Scribe.getStats()
println("Stats: $stats")
```

### iOS (Swift)

```swift
import Scribe

// 1. 初始化（在 AppDelegate）
let config = ScribeConfig(
    autoFlushIntervalMs: 5000,
    enableConsole: true,
    compression: true
)

Task {
    do {
        try await Scribe.initialize(
            logDir: documentsDir + "/logs",
            config: config
        )
    } catch {
        print("Scribe init failed: \(error)")
    }
}

// 2. 使用日志
Scribe.d("MyTag", "Debug message")
Scribe.i("MyTag", "Info message")
Scribe.w("MyTag", "Warning message")
Scribe.e("MyTag", "Error message")

// 3. 可选：手动刷新
Task {
    try? await Scribe.flush()
}

// 4. 可选：获取统计
let stats = Scribe.getStats()
print("Stats: \(stats)")
```

## 配置选项

### JSON 配置格式

```json
{
  "auto_flush_interval_ms": 5000,
  "enable_console": true,
  "min_console_level": 1,
  "max_file_size_mb": 10,
  "max_file_count": 5,
  "compression": true,
  "encryption": false
}
```

### 配置说明

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `auto_flush_interval_ms` | number | 5000 | 自动刷新间隔（毫秒） |
| `enable_console` | boolean | false | 启用控制台输出 |
| `min_console_level` | number | 1 | 控制台最小级别（0-4） |
| `max_file_size_mb` | number | 10 | 单文件最大大小（MB） |
| `max_file_count` | number | 5 | 最大文件数量 |
| `compression` | boolean | true | 启用压缩 |
| `encryption` | boolean | false | 启用加密 |

### 日志级别

| 级别 | 值 | 说明 |
|------|---|------|
| Verbose | 0 | 详细日志 |
| Debug | 1 | 调试日志 |
| Info | 2 | 信息日志 |
| Warn | 3 | 警告日志 |
| Error | 4 | 错误日志 |

## 高级用法

### Scoped Logger (Kotlin)

```kotlin
class MainActivity : Activity() {
    private val logger = Scribe.logger("MainActivity")
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        logger.d("Activity created")
    }
    
    override fun onResume() {
        super.onResume()
        logger.i("Activity resumed")
    }
}
```

### Scoped Logger (Swift)

```swift
class ViewController: UIViewController {
    let logger = scribeLogger(label: "ViewController")
    
    override func viewDidLoad() {
        super.viewDidLoad()
        logger.d("View loaded")
    }
    
    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        logger.i("View will appear")
    }
}
```

### DSL 风格 (Kotlin)

```kotlin
scribeLogger("MyFeature") {
    d("Starting feature")
    i("Feature initialized")
    w("Potential issue detected")
}
```

## 性能优化

### 最佳实践

1. **使用 Scoped Logger**
   ```kotlin
   // ✅ 推荐：避免重复传递 label
   private val logger = Scribe.logger("MyClass")
   logger.d("Message")
   
   // ❌ 不推荐
   Scribe.d("MyClass", "Message")
   Scribe.d("MyClass", "Another message")
   ```

2. **批量写入后统一刷新**
   ```kotlin
   // ✅ 推荐
   repeat(100) { Scribe.d("Tag", "Message $it") }
   Scribe.flush() // 统一刷新
   
   // ❌ 不推荐：频繁刷新
   repeat(100) {
       Scribe.d("Tag", "Message $it")
       Scribe.flush() // 每次都刷新
   }
   ```

3. **开发环境启用控制台**
   ```kotlin
   val config = Scribe.Config(
       enableConsole = BuildConfig.DEBUG  // 仅Debug模式
   )
   ```

## 自动化功能

Scribe 内置了多项自动化功能，无需手动管理：

1. **自动刷新**
   - 后台线程定期刷新（默认5秒）
   - 可通过配置调整间隔

2. **自动清理**
   - Android: ShutdownHook 自动刷新
   - iOS: Actor deinit 自动清理

3. **自动日志轮转**
   - 文件大小达到限制自动轮转
   - 保持最新的 N 个文件

4. **崩溃恢复**
   - 自动检测未完成的日志
   - 启动时自动恢复

## 故障排查

### Android

**问题：日志未写入文件**
```kotlin
// 检查目录权限
val logDir = File("${filesDir}/logs")
println("Log dir exists: ${logDir.exists()}")
println("Log dir writable: ${logDir.canWrite()}")

// 检查初始化状态
val result = Scribe.initialize(logDir.absolutePath, config)
result.onFailure { e ->
    println("Init failed: $e")
}
```

**问题：性能影响**
```kotlin
// 增加刷新间隔
val config = Scribe.Config(
    autoFlushIntervalMs = 10000  // 10秒
)

// 禁用控制台输出（Release模式）
val config = Scribe.Config(
    enableConsole = false
)
```

### iOS

**问题：日志丢失**
```swift
// 确保在后台也刷新
func applicationDidEnterBackground(_ application: UIApplication) {
    Task {
        try? await Scribe.flush()
    }
}
```

**问题：内存占用**
```swift
// 减少文件大小和数量
let config = ScribeConfig(
    maxFileSizeMb: 5,
    maxFileCount: 3
)
```

## API 参考

### 核心 API

| 函数 | 返回值 | 说明 |
|------|--------|------|
| `initialize(logDir, config)` | `Result<Unit>` | 初始化 Scribe |
| `log(level, label, message)` | `Result<Unit>` | 写日志 |
| `flush()` | `Result<Unit>` | 手动刷新 |
| `getStats()` | `String` | 获取统计（JSON） |

### 便捷方法

| 函数 | 说明 |
|------|------|
| `v(label, message)` | Verbose 日志 |
| `d(label, message)` | Debug 日志 |
| `i(label, message)` | Info 日志 |
| `w(label, message)` | Warn 日志 |
| `e(label, message)` | Error 日志 |
| `logger(label)` | 创建 Scoped Logger |
