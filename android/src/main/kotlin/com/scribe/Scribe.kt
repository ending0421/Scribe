package com.scribe

import androidx.annotation.Keep
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Scribe - High-performance logging library
 *
 * Rust-based logging engine with crash recovery, compression, and encryption support.
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

    // 加载本地库
    init {
        System.loadLibrary("scribe")
    }

    // === Native methods ===

    @JvmStatic
    private external fun nativeInit(logDir: String): Int

    @JvmStatic
    private external fun nativeWrite(level: Int, label: String, message: String): Int

    @JvmStatic
    private external fun nativeFlush(): Int

    @JvmStatic
    private external fun nativeDestroy(): Int

    @JvmStatic
    private external fun nativeRegisterConsole(minLevel: Int): Int

    @JvmStatic
    private external fun nativeClearSinks(): Int

    @JvmStatic
    private external fun nativeSinkCount(): Int

    // === Public API ===

    /**
     * Initialize Scribe logging system
     *
     * @param logDir Directory to store log files
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun init(logDir: String): Result<Unit> = runCatching {
        val result = nativeInit(logDir)
        if (result < 0) {
            throw ScribeException("Failed to initialize Scribe: error code $result")
        }
    }

    /**
     * Initialize Scribe logging system (suspend version)
     */
    @JvmStatic
    suspend fun initAsync(logDir: String): Result<Unit> = withContext(Dispatchers.IO) {
        init(logDir)
    }

    /**
     * Write a log message
     *
     * @param level Log level
     * @param label Log label/tag
     * @param message Log message
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun write(level: LogLevel, label: String, message: String): Result<Unit> = runCatching {
        val result = nativeWrite(level.value, label, message)
        if (result < 0) {
            throw ScribeException("Failed to write log: error code $result")
        }
    }

    /**
     * Flush all buffered logs to disk
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
     * Flush logs asynchronously
     */
    @JvmStatic
    suspend fun flushAsync(): Result<Unit> = withContext(Dispatchers.IO) {
        flush()
    }

    /**
     * Destroy and cleanup Scribe
     *
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun destroy(): Result<Unit> = runCatching {
        val result = nativeDestroy()
        if (result < 0) {
            throw ScribeException("Failed to destroy: error code $result")
        }
    }

    /**
     * Register a console sink for development
     *
     * @param minLevel Minimum log level
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun registerConsole(minLevel: LogLevel): Result<Unit> = runCatching {
        val result = nativeRegisterConsole(minLevel.value)
        if (result < 0) {
            throw ScribeException("Failed to register console: error code $result")
        }
    }

    /**
     * Clear all registered sinks
     *
     * @return Result indicating success or failure
     */
    @JvmStatic
    fun clearSinks(): Result<Unit> = runCatching {
        val result = nativeClearSinks()
        if (result < 0) {
            throw ScribeException("Failed to clear sinks: error code $result")
        }
    }

    /**
     * Get the number of registered sinks
     *
     * @return Number of sinks
     */
    @JvmStatic
    fun sinkCount(): Int = nativeSinkCount()

    // === Convenience methods ===

    @JvmStatic
    fun v(label: String, message: String) {
        write(LogLevel.VERBOSE, label, message)
    }

    @JvmStatic
    fun d(label: String, message: String) {
        write(LogLevel.DEBUG, label, message)
    }

    @JvmStatic
    fun i(label: String, message: String) {
        write(LogLevel.INFO, label, message)
    }

    @JvmStatic
    fun w(label: String, message: String) {
        write(LogLevel.WARN, label, message)
    }

    @JvmStatic
    fun e(label: String, message: String) {
        write(LogLevel.ERROR, label, message)
    }

    // === Scoped logging ===

    /**
     * Create a scoped logger with a fixed label
     */
    @JvmStatic
    fun logger(label: String): ScopedLogger = ScopedLogger(label)

    /**
     * Scoped logger with fixed label
     */
    class ScopedLogger internal constructor(private val label: String) {
        fun v(message: String) = Scribe.v(label, message)
        fun d(message: String) = Scribe.d(label, message)
        fun i(message: String) = Scribe.i(label, message)
        fun w(message: String) = Scribe.w(label, message)
        fun e(message: String) = Scribe.e(label, message)
    }

    /**
     * Custom exception for Scribe errors
     */
    class ScribeException(message: String) : Exception(message)
}

/**
 * Extension function for easier usage
 */
inline fun scribeLogger(label: String, block: Scribe.ScopedLogger.() -> Unit) {
    Scribe.logger(label).block()
}
