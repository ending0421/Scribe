# Scribe Android 示例使用

## 初始化

```kotlin
import com.scribe.Scribe

class MyApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        
        // 初始化 Scribe
        val logDir = "${filesDir.absolutePath}/logs"
        Scribe.init(logDir).getOrElse {
            Log.e("App", "Failed to init Scribe", it)
        }
        
        // 开发环境：注册控制台输出
        if (BuildConfig.DEBUG) {
            Scribe.registerConsole(Scribe.LogLevel.DEBUG)
        }
    }
    
    override fun onTerminate() {
        super.onTerminate()
        Scribe.destroy()
    }
}
```

## 基础使用

```kotlin
// 方式1: 直接调用
Scribe.d("MyTag", "Debug message")
Scribe.i("MyTag", "Info message")
Scribe.w("MyTag", "Warning message")
Scribe.e("MyTag", "Error message")

// 方式2: 使用 Result API
Scribe.write(Scribe.LogLevel.INFO, "MyTag", "Message")
    .onSuccess { println("Logged successfully") }
    .onFailure { e -> println("Failed: $e") }

// 方式3: Scoped Logger
val logger = Scribe.logger("MainActivity")
logger.d("Activity created")
logger.i("User logged in")

// 方式4: DSL 风格
scribeLogger("MyFeature") {
    d("Starting feature")
    i("Feature initialized")
}
```

## 协程使用

```kotlin
class MyViewModel : ViewModel() {
    
    fun saveData() = viewModelScope.launch {
        Scribe.i("ViewModel", "Saving data...")
        
        // 异步操作
        val result = saveToDatabase()
        
        // 异步刷新日志
        Scribe.flushAsync().getOrElse {
            Log.e("ViewModel", "Failed to flush", it)
        }
    }
}
```

## 性能最佳实践

```kotlin
// ✅ 推荐: 使用 Scoped Logger 避免重复传递 label
class MyActivity : Activity() {
    private val logger = Scribe.logger("MyActivity")
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        logger.d("onCreate")
    }
}

// ✅ 推荐: 批量写入后统一刷新
fun processBatch(items: List<Item>) {
    items.forEach { item ->
        Scribe.d("Processor", "Processing ${item.id}")
        process(item)
    }
    Scribe.flush() // 统一刷新
}

// ❌ 避免: 高频调用 flush
fun badExample() {
    Scribe.d("Tag", "Message")
    Scribe.flush() // 不必要的频繁刷新
}
```
