     STDIN
   1 import Foundation
   2 import ScribeFFI
   3 
   4 /// Log levels supported by Scribe
   5 public enum LogLevel: Int32 {
   6     case verbose = 0
   7     case debug = 1
   8     case info = 2
   9     case warn = 3
  10     case error = 4
  11 }
  12 
  13 /// Performance statistics from Scribe
  14 public struct Statistics {
  15     public let totalWrites: UInt64
  16     public let totalFlushes: UInt64
  17     public let totalErrors: UInt64
  18     public let bytesWritten: UInt64
  19     public let lastCleanupTime: UInt64
  20     
  21     init(from stats: ScribeStats) {
  22         self.totalWrites = stats.total_writes
  23         self.totalFlushes = stats.total_flushes
  24         self.totalErrors = stats.total_errors
  25         self.bytesWritten = stats.bytes_written
  26         self.lastCleanupTime = stats.last_cleanup_time
  27     }
  28 }
  29 
  30 /// Scribe logging system
  31 public enum Scribe {
  32     
  33     /// Initialize the Scribe logging system
  34     /// - Parameter configPath: Optional path to configuration file
  35     /// - Throws: ScribeError if initialization fails
  36     public static func initialize(configPath: String? = nil) throws {
  37         let result: Int32
  38         if let path = configPath {
  39             result = scribe_init(path)
  40         } else {
  41             result = scribe_init(nil)
  42         }
  43         
  44         if result != 0 {
  45             throw ScribeError.initializationFailed(code: result)
  46         }
  47     }
  48     
  49     /// Log a message
  50     /// - Parameters:
  51     ///   - level: Log level
  52     ///   - label: Tag or label for the message
  53     ///   - message: The log message
  54     /// - Throws: ScribeError if logging fails
  55     public static func log(_ level: LogLevel, label: String, message: String) throws {
  56         let result = scribe_log(level.rawValue, label, message)
  57         if result != 0 {
  58             throw ScribeError.logFailed(code: result)
  59         }
  60     }
  61     
  62     /// Flush all pending log data to disk
  63     /// - Throws: ScribeError if flush fails
  64     public static func flush() throws {
  65         let result = scribe_flush()
  66         if result != 0 {
  67             throw ScribeError.flushFailed(code: result)
  68         }
  69     }
  70     
  71     /// Get performance statistics
  72     /// - Returns: Statistics object
  73     /// - Throws: ScribeError if getting stats fails
  74     public static func getStatistics() throws -> Statistics {
  75         var stats = ScribeStats()
  76         let result = scribe_get_stats(&stats)
  77         if result != 0 {
  78             throw ScribeError.getStatsFailed(code: result)
  79         }
  80         return Statistics(from: stats)
  81     }
  82 }
  83 
  84 /// Errors that can occur when using Scribe
  85 public enum ScribeError: Error {
  86     case initializationFailed(code: Int32)
  87     case logFailed(code: Int32)
  88     case flushFailed(code: Int32)
  89     case getStatsFailed(code: Int32)
  90 }
  91 
  92 extension ScribeError: LocalizedError {
  93     public var errorDescription: String? {
  94         switch self {
  95         case .initializationFailed(let code):
  96             return "Scribe initialization failed with code: \(code)"
  97         case .logFailed(let code):
  98             return "Scribe log failed with code: \(code)"
  99         case .flushFailed(let code):
 100             return "Scribe flush failed with code: \(code)"
 101         case .getStatsFailed(let code):
 102             return "Scribe get statistics failed with code: \(code)"
 103         }
 104     }
 105 }
 106 
 107 // MARK: - Convenience methods
 108 
 109 extension Scribe {
 110     /// Log a verbose message
 111     public static func verbose(_ message: String, label: String = "App") {
 112         try? log(.verbose, label: label, message: message)
 113     }
 114     
 115     /// Log a debug message
 116     public static func debug(_ message: String, label: String = "App") {
 117         try? log(.debug, label: label, message: message)
 118     }
 119     
 120     /// Log an info message
 121     public static func info(_ message: String, label: String = "App") {
 122         try? log(.info, label: label, message: message)
 123     }
 124     
 125     /// Log a warning message
 126     public static func warn(_ message: String, label: String = "App") {
 127         try? log(.warn, label: label, message: message)
 128     }
 129     
 130     /// Log an error message
 131     public static func error(_ message: String, label: String = "App") {
 132         try? log(.error, label: label, message: message)
 133     }
 134 }
