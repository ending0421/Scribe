import Foundation

/// Log levels for Scribe
public enum LogLevel: Int, Sendable {
    case verbose = 0
    case debug = 1
    case info = 2
    case warn = 3
    case error = 4
}

/// Errors that can occur in Scribe operations
public enum ScribeError: Error, Sendable {
    case initializationFailed(Int)
    case writeFailed(Int)
    case flushFailed(Int)
    case destroyFailed(Int)
    case registerConsoleFailed(Int)
    case clearSinksFailed(Int)
}

/// High-performance logging library for iOS
///
/// Scribe provides Rust-based logging with crash recovery, compression, and encryption support.
@available(iOS 13.0, macOS 10.15, *)
public actor Scribe {

    private static let shared = Scribe()
    private var isInitialized = false

    private init() {}

    // MARK: - C FFI Functions

    @_silgen_name("scribe_init")
    private static func nativeInit(_ logDir: UnsafePointer<CChar>) -> Int32

    @_silgen_name("scribe_write")
    private static func nativeWrite(_ level: Int32, _ label: UnsafePointer<CChar>, _ message: UnsafePointer<CChar>) -> Int32

    @_silgen_name("scribe_flush")
    private static func nativeFlush() -> Int32

    @_silgen_name("scribe_destroy")
    private static func nativeDestroy() -> Int32

    @_silgen_name("scribe_register_console")
    private static func nativeRegisterConsole(_ minLevel: Int32) -> Int32

    @_silgen_name("scribe_clear_sinks")
    private static func nativeClearSinks() -> Int32

    @_silgen_name("scribe_sink_count")
    private static func nativeSinkCount() -> Int32

    // MARK: - Public API

    /// Initialize Scribe with log directory
    /// - Parameter logDir: Directory path to store log files
    /// - Throws: ScribeError if initialization fails
    public static func initialize(logDir: String) async throws {
        try await shared.initializeImpl(logDir: logDir)
    }

    private func initializeImpl(logDir: String) throws {
        guard !isInitialized else { return }

        let result = logDir.withCString { ptr in
            Self.nativeInit(ptr)
        }

        guard result == 0 else {
            throw ScribeError.initializationFailed(Int(result))
        }

        isInitialized = true
    }

    /// Write a log message
    /// - Parameters:
    ///   - level: Log level
    ///   - label: Log label/tag
    ///   - message: Log message
    /// - Throws: ScribeError if write fails
    public static func write(level: LogLevel, label: String, message: String) throws {
        let result = label.withCString { labelPtr in
            message.withCString { messagePtr in
                nativeWrite(Int32(level.rawValue), labelPtr, messagePtr)
            }
        }

        guard result == 0 else {
            throw ScribeError.writeFailed(Int(result))
        }
    }

    /// Flush all buffered logs to disk
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

    /// Destroy and cleanup Scribe
    /// - Throws: ScribeError if destroy fails
    public static func destroy() async throws {
        try await shared.destroyImpl()
    }

    private func destroyImpl() throws {
        guard isInitialized else { return }

        let result = Self.nativeDestroy()
        guard result == 0 else {
            throw ScribeError.destroyFailed(Int(result))
        }

        isInitialized = false
    }

    /// Register a console sink for development
    /// - Parameter minLevel: Minimum log level
    /// - Throws: ScribeError if registration fails
    public static func registerConsole(minLevel: LogLevel) throws {
        let result = nativeRegisterConsole(Int32(minLevel.rawValue))
        guard result == 0 else {
            throw ScribeError.registerConsoleFailed(Int(result))
        }
    }

    /// Clear all registered sinks
    /// - Throws: ScribeError if clear fails
    public static func clearSinks() throws {
        let result = nativeClearSinks()
        guard result == 0 else {
            throw ScribeError.clearSinksFailed(Int(result))
        }
    }

    /// Get the number of registered sinks
    /// - Returns: Number of sinks
    public static func sinkCount() -> Int {
        Int(nativeSinkCount())
    }

    // MARK: - Convenience Methods

    public static func v(_ label: String, _ message: String) {
        try? write(level: .verbose, label: label, message: message)
    }

    public static func d(_ label: String, _ message: String) {
        try? write(level: .debug, label: label, message: message)
    }

    public static func i(_ label: String, _ message: String) {
        try? write(level: .info, label: label, message: message)
    }

    public static func w(_ label: String, _ message: String) {
        try? write(level: .warn, label: label, message: message)
    }

    public static func e(_ label: String, _ message: String) {
        try? write(level: .error, label: label, message: message)
    }
}

// MARK: - Scoped Logger

/// Scoped logger with fixed label
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

/// Create a scoped logger
public func scribeLogger(label: String) -> ScopedLogger {
    ScopedLogger(label: label)
}
