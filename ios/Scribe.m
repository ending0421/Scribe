//
//  Scribe.m
//  Scribe
//

#import "Scribe.h"

// C function declarations from Rust
extern int scribe_init(const char *log_dir);
extern int scribe_write(int level, const char *label, const char *message);
extern int scribe_flush(void);
extern int scribe_destroy(void);
extern int scribe_register_console(int min_level);
extern int scribe_clear_sinks(void);
extern int scribe_sink_count(void);

@implementation Scribe

+ (int)initWithLogDir:(NSString *)logDir {
    return scribe_init([logDir UTF8String]);
}

+ (int)write:(ScribeLogLevel)level label:(NSString *)label message:(NSString *)message {
    return scribe_write((int)level, [label UTF8String], [message UTF8String]);
}

+ (int)flush {
    return scribe_flush();
}

+ (int)destroy {
    return scribe_destroy();
}

+ (int)registerConsoleWithMinLevel:(ScribeLogLevel)minLevel {
    return scribe_register_console((int)minLevel);
}

+ (int)clearSinks {
    return scribe_clear_sinks();
}

+ (int)sinkCount {
    return scribe_sink_count();
}

// Convenience methods

+ (void)v:(NSString *)label message:(NSString *)message {
    [self write:ScribeLogLevelVerbose label:label message:message];
}

+ (void)d:(NSString *)label message:(NSString *)message {
    [self write:ScribeLogLevelDebug label:label message:message];
}

+ (void)i:(NSString *)label message:(NSString *)message {
    [self write:ScribeLogLevelInfo label:label message:message];
}

+ (void)w:(NSString *)label message:(NSString *)message {
    [self write:ScribeLogLevelWarn label:label message:message];
}

+ (void)e:(NSString *)label message:(NSString *)message {
    [self write:ScribeLogLevelError label:label message:message];
}

@end
