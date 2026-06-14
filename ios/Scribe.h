//
//  Scribe.h
//  Scribe
//
//  High-performance logging library for iOS
//

#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

/// Log levels
typedef NS_ENUM(NSInteger, ScribeLogLevel) {
    ScribeLogLevelVerbose = 0,
    ScribeLogLevelDebug = 1,
    ScribeLogLevelInfo = 2,
    ScribeLogLevelWarn = 3,
    ScribeLogLevelError = 4
};

/// Scribe logging interface
@interface Scribe : NSObject

/// Initialize Scribe with log directory
/// @param logDir Directory to store log files
/// @return 0 on success, negative error code on failure
+ (int)initWithLogDir:(NSString *)logDir;

/// Write a log message
/// @param level Log level
/// @param label Log label/tag
/// @param message Log message
/// @return 0 on success, negative error code on failure
+ (int)write:(ScribeLogLevel)level label:(NSString *)label message:(NSString *)message;

/// Flush all buffered logs to disk
/// @return 0 on success, negative error code on failure
+ (int)flush;

/// Destroy and cleanup Scribe
/// @return 0 on success, negative error code on failure
+ (int)destroy;

/// Register a console sink for development
/// @param minLevel Minimum log level
/// @return 0 on success, negative error code on failure
+ (int)registerConsoleWithMinLevel:(ScribeLogLevel)minLevel;

/// Clear all registered sinks
/// @return 0 on success, negative error code on failure
+ (int)clearSinks;

/// Get the number of registered sinks
/// @return Number of sinks
+ (int)sinkCount;

// Convenience methods

+ (void)v:(NSString *)label message:(NSString *)message;
+ (void)d:(NSString *)label message:(NSString *)message;
+ (void)i:(NSString *)label message:(NSString *)message;
+ (void)w:(NSString *)label message:(NSString *)message;
+ (void)e:(NSString *)label message:(NSString *)message;

@end

NS_ASSUME_NONNULL_END
