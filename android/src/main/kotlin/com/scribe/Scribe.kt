package com.scribe

import androidx.annotation.Keep
import kotlinx.coroutines.*
import org.json.JSONObject

/**
 * Scribe - High-performance logging library
 *
 * Simplified API with automatic management.
 */
@Keep
object Scribe {

    /**
     * Log levels
     */
    enum class LogLevel(val value: Int) {
        VERBOSE(0),
        DEBUG(1),
        INFO(2),
        WARN(3),
        ERROR(4)
    }

    /**
     * Scribe configuration
     */
    data class Config(
        val autoFlushIntervalMs: Long = 5000,
        val enableConsole: Boolean = false,
        val minConsoleLevel: LogLevel = LogLevel.DEBUG,
        val maxFileSizeMb: Int = 10,
        val maxFileCount: Int = 5,
        val compression: Boolean = true,
        val encryption: Boolean = false
    ) {
        fun toJson(): String = JSONObject().apply {
            put("auto_flush_interval_ms", autoFlushIntervalMs)
            put("enable_console", enableConsole)
            put("min_console_level", minConsoleLevel.value)
            put("max_file_size_mb", maxFileSizeMb)
            put("max_file_count", maxFileCount)
            put("compression", compression)
            put("encryption", encryption)
        }.toString()
    }

    private var autoFlushJob: Job? = null
    private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

    // 加载本地库
    init {
        System.loadLibrary("scribe")
    }

    // === Native methods (简化为2个核心函数) ===

    @JvmStatic
    private external fun nativeInit(logDir: String, configJson: String): Int

    @JvmStatic
    private external fun nativeLog(level: Int, label: String, message: String): Int

    @JvmStatic
    private external fun nativeFlush(): Int

    @JvmStatic
    private external fun nativeGetStats(): String

    // === Public API (简化为2个必需 + 2个可选) ===

    /**
     * Initialize Scribe with automatic management
     *
     * @param logDir Directory to store log files
     * @param config Configuration (optional, uses defaults if null)
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun initialize(logDir: String, config: Config = Config()): Result<Unit> = runCatching {
        val result = nativeInit(logDir, config.toJson())
        if (result < 0) {
            throw ScribeException("Failed to initialize: error code $result")
        }

        // 启动自动刷新
        startAutoFlush(config.autoFlushIntervalMs)

        // 注册进程退出时自动清理
        Runtime.getRuntime().addShutdownHook(Thread {
            shutdown()
        })
    }

    /**
     * Log a message (core API)
     *
     * @param level Log level
     * @param label Log label/tag
     * @param message Log message
     */
    @JvmStatic
    fun log(level: LogLevel, label: String, message: String): Result<Unit> = runCatching {
        val result = nativeLog(level.value, label, message)
        if (result < 0) {
            throw ScribeException("Failed to log: error code $result")
        }
    }

    /**
     * Manual flush (optional, automatic flush is enabled by default)
     *
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun flush(): Result<Unit> = runCatching {
        val result = nativeFlush()
        if (result < 0) {
            throw ScribeException("Failed to flush: error code $result")
        }
    }

    /**
     * Get performance statistics (optional)
     *
     * @return JSON string with statistics
     */
    @JvmStatic
    fun getStats(): String = nativeGetStats()

    // === Convenience methods ===

    @JvmStatic
    fun v(label: String, message: String) {
        log(LogLevel.VERBOSE, label, message)
    }

    @JvmStatic
    fun d(label: String, message: String) {
        log(LogLevel.DEBUG, label, message)
    }

    @JvmStatic
    fun i(label: String, message: String) {
        log(LogLevel.INFO, label, message)
    }

    @JvmStatic
    fun w(label: String, message: String) {
        log(LogLevel.WARN, label, message)
    }

    @JvmStatic
    fun e(label: String, message: String) {
        log(LogLevel.ERROR, label, message)
    }

    // === Internal management ===

    private fun startAutoFlush(intervalMs: Long) {
        autoFlushJob?.cancel()
        autoFlushJob = scope.launch {
            while (isActive) {
                delay(intervalMs)
                flush()
            }
        }
    }

    private fun shutdown() {
        autoFlushJob?.cancel()
        flush()
        scope.cancel()
    }

    // === Scoped logging ===

    @JvmStatic
    fun logger(label: String): ScopedLogger = ScopedLogger(label)

    class ScopedLogger internal constructor(private val label: String) {
        fun v(message: String) = Scribe.v(label, message)
        fun d(message: String) = Scribe.d(label, message)
        fun i(message: String) = Scribe.i(label, message)
        fun w(message: String) = Scribe.w(label, message)
        fun e(message: String) = Scribe.e(label, message)
    }

    class ScribeException(message: String) : Exception(message)
}

// === Extension function ===

inline fun scribeLogger(label: String, block: Scribe.ScopedLogger.() -> Unit) {
    Scribe.logger(label).block()
}
