use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

const MAX_LABEL_LENGTH: usize = 128; // Configurable label length limit

thread_local! {
    static THREAD_LABEL: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
}

/// 验证 Label 长度
pub fn validate_label(tag: &str) -> crate::Result<()> {
    if tag.len() > MAX_LABEL_LENGTH {
        return Err(crate::ScribeError::Mmap(
            format!("Label exceeds maximum length: {} > {}", tag.len(), MAX_LABEL_LENGTH)
        ));
    }
    Ok(())
}

/// 设置临时 Tag（仅当前线程）
pub fn set_thread_label(tag: Option<String>) {
    THREAD_LABEL.with(|t| {
        *t.borrow_mut() = tag;
    });
}

/// 获取当前线程的临时 Tag
pub fn get_thread_label() -> Option<String> {
    THREAD_LABEL.with(|t| t.borrow().clone())
}

/// LabeledLogger - 支持链式调用的 Label 设置
pub struct LabeledLogger {
    label: String,
}

impl LabeledLogger {
    pub fn new(label: String) -> Self {
        Self { label }
    }

    pub fn v(&self, message: &str) {
        crate::sink::registry().log(
            crate::LogLevel::Verbose,
            Some(&self.label),
            message,
        );
    }

    pub fn d(&self, message: &str) {
        crate::sink::registry().log(
            crate::LogLevel::Debug,
            Some(&self.label),
            message,
        );
    }

    pub fn i(&self, message: &str) {
        crate::sink::registry().log(
            crate::LogLevel::Info,
            Some(&self.label),
            message,
        );
    }

    pub fn w(&self, message: &str) {
        crate::sink::registry().log(
            crate::LogLevel::Warn,
            Some(&self.label),
            message,
        );
    }

    pub fn e(&self, message: &str) {
        crate::sink::registry().log(
            crate::LogLevel::Error,
            Some(&self.label),
            message,
        );
    }

    /// Sets this label as the thread-local label for all subsequent log calls
    /// in the current thread (until `uproot()` is called).
    pub fn plant(&self) {
        set_thread_label(Some(self.label.clone()));
    }
}

/// Clears the thread-local tag.
pub fn uproot() {
    set_thread_label(None);
}

/// 创建带 Label 的 Logger
pub fn label(tag: &str) -> LabeledLogger {
    LabeledLogger::new(tag.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_validate_label() {
        // Valid tags
        assert!(validate_label("").is_ok());
        assert!(validate_label("TEST").is_ok());
        assert!(validate_label("12345678901234567890123").is_ok()); // Within limit

        // Invalid tags
        assert!(validate_label("123456789012345678901234").is_err()); // Exceeds limit
        assert!(validate_label("a".repeat(24).as_str()).is_err());
    }

    #[test]
    fn test_thread_label() {
        // Initially None
        assert_eq!(get_thread_label(), None);

        // Set and get
        set_thread_label(Some("TEST".to_string()));
        assert_eq!(get_thread_label(), Some("TEST".to_string()));

        // Clear
        set_thread_label(None);
        assert_eq!(get_thread_label(), None);
    }

    #[test]
    fn test_thread_label_isolation() {
        set_thread_label(Some("MAIN".to_string()));

        let handle = thread::spawn(|| {
            // Thread should have its own label storage
            assert_eq!(get_thread_label(), None);
            set_thread_label(Some("THREAD".to_string()));
            assert_eq!(get_thread_label(), Some("THREAD".to_string()));
        });

        handle.join().unwrap();

        // Main thread label should be unchanged
        assert_eq!(get_thread_label(), Some("MAIN".to_string()));

        // Cleanup
        set_thread_label(None);
    }

    #[test]
    fn test_tagged_logger_creation() {
        let logger = tag("TEST");
        assert_eq!(logger.tag, "TEST");
    }

    #[test]
    fn test_tagged_logger_new() {
        let logger = LabeledLogger::new("CUSTOM".to_string());
        assert_eq!(logger.tag, "CUSTOM");
    }

    #[test]
    fn test_tag_length_boundary() {
        // Test exact boundary
        let max_label = "a".repeat(MAX_LABEL_LENGTH);
        assert!(validate_label(&max_tag).is_ok());

        let over_max = "a".repeat(MAX_LABEL_LENGTH + 1);
        assert!(validate_label(&over_max).is_err());
    }

    #[test]
    fn test_thread_label_multiple_sets() {
        set_thread_label(Some("FIRST".to_string()));
        assert_eq!(get_thread_label(), Some("FIRST".to_string()));

        set_thread_label(Some("SECOND".to_string()));
        assert_eq!(get_thread_label(), Some("SECOND".to_string()));

        set_thread_label(None);
        assert_eq!(get_thread_label(), None);
    }

    #[test]
    fn test_empty_tag() {
        let logger = tag("");
        assert_eq!(logger.tag, "");
    }

    #[test]
    fn test_unicode_tag_length() {
        // Unicode characters count as multiple bytes
        let unicode_label = "测试"; // 6 bytes, 2 characters
        assert!(validate_label(unicode_tag).is_ok());

        // Create a label that's exactly MAX_LABEL_LENGTH bytes with unicode
        let long_unicode = "测".repeat(12); // 36 bytes - should fail
        assert!(validate_label(&long_unicode).is_err());
    }

    #[test]
    fn test_concurrent_thread_tags() {
        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let label = format!("THREAD_{}", i);
                    set_thread_label(Some(tag.clone()));
                    thread::sleep(std::time::Duration::from_millis(10));
                    assert_eq!(get_thread_label(), Some(tag));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
