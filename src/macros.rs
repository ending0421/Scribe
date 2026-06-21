//! Convenient logging macros for Scribe.
//!
//! This module provides ergonomic macros for logging at different levels,
//! with automatic tag detection from thread-local storage or call stack.
//!
//! # Usage
//!
//! ```rust,no_run
//! use scribe::{scribe_d, scribe_i, scribe_e};
//!
//! // Simple logging with format support
//! scribe_i!("Server started on port {}", 8080);
//! scribe_d!("Processing request from {}", client_addr);
//! scribe_e!("Failed to connect: {}", error);
//!
//! // With explicit tags
//! scribe_tag_i!("network", "Connection established");
//! scribe_tag_e!("database", "Query failed: {}", sql);
//! ```

/// Logs a verbose-level message with automatic tag detection.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_v;
/// scribe_v!("Detailed trace information");
/// scribe_v!("Processing item {} of {}", current, total);
/// ```
#[macro_export]
macro_rules! scribe_v {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Verbose, &format!($($arg)*))
    };
}

/// Logs a debug-level message with automatic tag detection.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_d;
/// scribe_d!("Debug information");
/// scribe_d!("Variable value: {}", value);
/// ```
#[macro_export]
macro_rules! scribe_d {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Debug, &format!($($arg)*))
    };
}

/// Logs an info-level message with automatic tag detection.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_i;
/// scribe_i!("Application started");
/// scribe_i!("User {} logged in", username);
/// ```
#[macro_export]
macro_rules! scribe_i {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Info, &format!($($arg)*))
    };
}

/// Logs a warning-level message with automatic tag detection.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_w;
/// scribe_w!("Deprecated API called");
/// scribe_w!("Retry attempt {} of {}", attempt, max_retries);
/// ```
#[macro_export]
macro_rules! scribe_w {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Warn, &format!($($arg)*))
    };
}

/// Logs an error-level message with automatic tag detection.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_e;
/// scribe_e!("Fatal error occurred");
/// scribe_e!("Failed to process: {}", error);
/// ```
#[macro_export]
macro_rules! scribe_e {
    ($($arg:tt)*) => {
        $crate::log_with_auto_tag($crate::LogLevel::Error, &format!($($arg)*))
    };
}

/// Logs a verbose-level message with explicit tag.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_tag_v;
/// scribe_tag_v!("network", "Packet received");
/// scribe_tag_v!("cache", "Cache hit for key: {}", key);
/// ```
#[macro_export]
macro_rules! scribe_tag_v {
    ($tag:expr, $($arg:tt)*) => {
        $crate::context::tag($tag).v(&format!($($arg)*))
    };
}

/// Logs a debug-level message with explicit tag.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_tag_d;
/// scribe_tag_d!("database", "Query executed");
/// scribe_tag_d!("auth", "Token validated for user: {}", user_id);
/// ```
#[macro_export]
macro_rules! scribe_tag_d {
    ($tag:expr, $($arg:tt)*) => {
        $crate::context::tag($tag).d(&format!($($arg)*))
    };
}

/// Logs an info-level message with explicit tag.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_tag_i;
/// scribe_tag_i!("server", "Listening on port 8080");
/// scribe_tag_i!("startup", "Initialization complete in {}ms", duration);
/// ```
#[macro_export]
macro_rules! scribe_tag_i {
    ($tag:expr, $($arg:tt)*) => {
        $crate::context::tag($tag).i(&format!($($arg)*))
    };
}

/// Logs a warning-level message with explicit tag.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_tag_w;
/// scribe_tag_w!("memory", "High memory usage detected");
/// scribe_tag_w!("performance", "Slow query: {}ms", duration);
/// ```
#[macro_export]
macro_rules! scribe_tag_w {
    ($tag:expr, $($arg:tt)*) => {
        $crate::context::tag($tag).w(&format!($($arg)*))
    };
}

/// Logs an error-level message with explicit tag.
///
/// # Examples
///
/// ```rust,no_run
/// # use scribe::scribe_tag_e;
/// scribe_tag_e!("io", "Failed to read file");
/// scribe_tag_e!("network", "Connection timeout: {}", url);
/// ```
#[macro_export]
macro_rules! scribe_tag_e {
    ($tag:expr, $($arg:tt)*) => {
        $crate::context::tag($tag).e(&format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use crate::{tree, tag, LogLevel};

    #[test]
    fn test_verbose_macro() {
        scribe_v!("Test verbose message");
        scribe_v!("Formatted: {}", 42);
    }

    #[test]
    fn test_debug_macro() {
        scribe_d!("Test debug message");
        scribe_d!("Value: {}", "test");
    }

    #[test]
    fn test_info_macro() {
        scribe_i!("Test info message");
        scribe_i!("Count: {}", 100);
    }

    #[test]
    fn test_warn_macro() {
        scribe_w!("Test warning message");
        scribe_w!("Threshold: {}%", 90);
    }

    #[test]
    fn test_error_macro() {
        scribe_e!("Test error message");
        scribe_e!("Error code: {}", -1);
    }

    #[test]
    fn test_tagged_verbose_macro() {
        scribe_tag_v!("test_tag", "Tagged verbose message");
        scribe_tag_v!("test_tag", "Value: {}", 42);
    }

    #[test]
    fn test_tagged_debug_macro() {
        scribe_tag_d!("test_tag", "Tagged debug message");
        scribe_tag_d!("test_tag", "Data: {}", "test");
    }

    #[test]
    fn test_tagged_info_macro() {
        scribe_tag_i!("test_tag", "Tagged info message");
        scribe_tag_i!("test_tag", "Status: {}", "OK");
    }

    #[test]
    fn test_tagged_warn_macro() {
        scribe_tag_w!("test_tag", "Tagged warning message");
        scribe_tag_w!("test_tag", "Usage: {}%", 85);
    }

    #[test]
    fn test_tagged_error_macro() {
        scribe_tag_e!("test_tag", "Tagged error message");
        scribe_tag_e!("test_tag", "Failed: {}", "reason");
    }

    #[test]
    fn test_complex_formatting() {
        scribe_i!("Multiple values: {}, {}, {}", 1, "two", 3.0);
        scribe_tag_d!("complex", "Struct: {:?}", vec![1, 2, 3]);
    }

    #[test]
    fn test_macro_with_thread_tag() {
        // Set thread-local tag
        context::tag("thread_tag").plant();

        // Macros should use the thread-local tag
        scribe_i!("This should use thread_tag");
        scribe_d!("Debug with thread tag");

        // Clear tag
        context::uproot();
    }

    #[test]
    fn test_macro_without_tag() {
        // No thread tag set, should fall back to backtrace
        scribe_i!("No explicit tag");
    }
}
