import Foundation

/// Log levels for Scribe
public enum LogLevel: Int, Sendable {
    case verbose = 0
    case debug = 1
    case info = 2
    case warn = 3
    case error = 4
}

/// Scribe configuration
public struct ScribeConfig: Sendable {
    public let autoFlushIntervalMs: Int
    public let enableConsole: Bool
    public let minConsoleLevel: LogLevel
    public let maxFileSizeMb: Int
    public let maxFileCount: Int
    public let compression: Bool
    public let encryption: Bool

    public init(
        autoFlushIntervalMs: Int = 5000,
        enableConsole: Bool = false,
        minConsoleLevel: LogLevel = .debug,
        maxFileSizeMb: Int = 10,
        maxFileCount: Int = 5,
        compression: Bool = true,
        encryption: Bool = false
    ) {
        self.autoFlushIntervalMs = autoFlushIntervalMs
        self.enableConsole = enableConsole
        self.minConsoleLevel = minConsoleLevel
        self.maxFileSizeMb = maxFileSizeMb
        self.maxFileCount = maxFileCount
        self.compression = compression
        self.encryption = encryption
    }

    func toJSON() -> String {
        """
        {
            "auto_flush_interval_ms": \(autoFlushIntervalMs),
            "enable_console": \(enableConsole),
            "min_console_level": \(minConsoleLevel.rawValue),
            "max_file_size_mb": \(maxFileSizeMb),
            "max_file_count": \(maxFileCount),
            "compression": \(compression),
            "encryption": \(encryption)
        }
        """
    }
}

/// Errors that can occur in Scribe operations
public enum ScribeError: Error, Sendable {
    case initializationFailed(Int)
    case logFailed(Int)
    case flushFailed(Int)
}

/// High-performance logging library for iOS
///
/// Simplified API with automatic management.
@available(iOS 13.0, macOS 10.15, *)
public actor Scribe {

    private static let shared = Scribe()
    private var isInitialized = false
    private var autoFlushTask: Task<Void, Never>?

    private init() {}

    // MARK: - C FFI Functions (简化为3个)

    @_silgen_name("scribe_init")
    private static func nativeInit(_ logDir: UnsafePointer<CChar>, _ configJson: UnsafePointer<CChar>) -> Int32

    @_silgen_name("scribe_log")
    private static func nativeLog(_ level: Int32, _ label: UnsafePointer<CChar>, _ message: UnsafePointer<CChar>) -> Int32

    @_silgen_name("scribe_flush")
    private static func nativeFlush() -> Int32

    @_silgen_name("scribe_get_stats")
    private static func nativeGetStats() -> UnsafePointer<CChar>?

    // MARK: - Public API (简化为2个必需 + 2个可选)

    /// Initialize Scribe with automatic management
    /// - Parameters:
    ///   - logDir: Directory path to store log files
    ///   - config: Configuration (uses defaults if not provided)
    /// - Throws: ScribeError if initialization fails
    public static func initialize(logDir: String, config: ScribeConfig = ScribeConfig()) async throws {
        try await shared.initializeImpl(logDir: logDir, config: config)
    }

    private func initializeImpl(logDir: String, config: ScribeConfig) throws {
        guard !isInitialized else { return }

        let result = logDir.withCString { logDirPtr in
            config.toJSON().withCString { configPtr in
                Self.nativeInit(logDirPtr, configPtr)
            }
        }

        guard result == 0 else {
            throw ScribeError.initializationFailed(Int(result))
        }

        isInitialized = true

        // 启动自动刷新
        startAutoFlush(intervalMs: config.autoFlushIntervalMs)
    }

    /// Log a message (core API)
    /// - Parameters:
    ///   - level: Log level
    ///   - label: Log label/tag
    ///   - message: Log message
    /// - Throws: ScribeError if log fails
    public static func log(level: LogLevel, label: String, message: String) throws {
        let result = label.withCString { labelPtr in
            message.withCString { messagePtr in
                nativeLog(Int32(level.rawValue), labelPtr, messagePtr)
            }
        }

        guard result == 0 else {
            throw ScribeError.logFailed(Int(result))
        }
    }

    /// Manual flush (optional, automatic flush is enabled by default)
    /// - Throws: ScribeError if flush fails
    public static func flush() async throws {
        try await shared.flushImpl()
    }

    private func flushImpl() throws {
        let result = Self.nativeFlush()
        guard result == 0 else {
            throw ScribeError.flushFailed(Int(result))
        }
    }

    /// Get performance statistics (optional)
    /// - Returns: JSON string with statistics
    public static func getStats() -> String {
        guard let ptr = nativeGetStats() else { return "{}" }
        return String(cString: ptr)
    }

    // MARK: - Convenience Methods

    public static func v(_ label: String, _ message: String) {
        try? log(level: .verbose, label: label, message: message)
    }

    public static func d(_ label: String, _ message: String) {
        try? log(level: .debug, label: label, message: message)
    }

    public static func i(_ label: String, _ message: String) {
        try? log(level: .info, label: label, message: message)
    }

    public static func w(_ label: String, _ message: String) {
        try? log(level: .warn, label: label, message: message)
    }

    public static func e(_ label: String, _ message: String) {
        try? log(level: .error, label: label, message: message)
    }

    // MARK: - Internal Management

    private func startAutoFlush(intervalMs: Int) {
        autoFlushTask?.cancel()
        autoFlushTask = Task {
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(intervalMs))
                try? await flushImpl()
            }
        }
    }

    deinit {
        autoFlushTask?.cancel()
    }
}

// MARK: - Scoped Logger

@available(iOS 13.0, macOS 10.15, *)
public struct ScopedLogger: Sendable {
    private let label: String

    public init(label: String) {
        self.label = label
    }

    public func v(_ message: String) {
        Scribe.v(label, message)
    }

    public func d(_ message: String) {
        Scribe.d(label, message)
    }

    public func i(_ message: String) {
        Scribe.i(label, message)
    }

    public func w(_ message: String) {
        Scribe.w(label, message)
    }

    public func e(_ message: String) {
        Scribe.e(label, message)
    }
}

// MARK: - Global Convenience

public func scribeLogger(label: String) -> ScopedLogger {
    ScopedLogger(label: label)
}
