//! Thread-local context and label management for Scribe.
//!
//! 注意：这些 API 已被简化的 FFI API 取代。
//! 直接使用 scribe_log() FFI 函数即可。

#![allow(dead_code)]

use std::cell::RefCell;

thread_local! {
    static THREAD_LABEL: RefCell<Option<String>> = const { RefCell::new(None) };
}

const MAX_LABEL_LENGTH: usize = 23;

/// 验证 label 长度
pub fn validate_label(label: &str) -> Result<(), String> {
    if label.len() > MAX_LABEL_LENGTH {
        Err(format!(
            "Label exceeds maximum length of {} bytes",
            MAX_LABEL_LENGTH
        ))
    } else {
        Ok(())
    }
}

/// 设置线程本地 label
pub fn set_thread_label(label: Option<String>) {
    THREAD_LABEL.with(|tl| {
        *tl.borrow_mut() = label;
    });
}

/// 获取线程本地 label
pub fn get_thread_label() -> Option<String> {
    THREAD_LABEL.with(|tl| tl.borrow().clone())
}

/// 清除线程本地 label
pub fn uproot() {
    set_thread_label(None);
}

/// 创建 label（用于兼容性）
pub fn label(tag: &str) -> String {
    tag.to_string()
}

/// 已废弃 - 使用 uproot() 代替
#[deprecated(note = "Use uproot() instead")]
pub fn tag(tag: &str) -> String {
    tag.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_validate_label() {
        // Valid labels
        assert!(validate_label("").is_ok());
        assert!(validate_label("TEST").is_ok());
        assert!(validate_label("12345678901234567890123").is_ok()); // Within limit

        // Invalid labels
        assert!(validate_label("123456789012345678901234").is_err()); // Exceeds limit
        assert!(validate_label(&"a".repeat(24)).is_err());
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
    fn test_uproot() {
        set_thread_label(Some("TEST".to_string()));
        assert_eq!(get_thread_label(), Some("TEST".to_string()));

        uproot();
        assert_eq!(get_thread_label(), None);
    }

    #[test]
    fn test_tag_length_boundary() {
        // Test exact boundary
        let max_label = "a".repeat(MAX_LABEL_LENGTH);
        assert!(validate_label(&max_label).is_ok());

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
    fn test_unicode_label_length() {
        // Unicode characters count as multiple bytes
        let unicode_label = "测试"; // 6 bytes, 2 characters
        assert!(validate_label(unicode_label).is_ok());

        // Create a label that's exactly MAX_LABEL_LENGTH bytes with unicode
        let long_unicode = "测".repeat(12); // 36 bytes - should fail
        assert!(validate_label(&long_unicode).is_err());
    }

    #[test]
    fn test_concurrent_thread_labels() {
        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let label = format!("THREAD_{}", i);
                    set_thread_label(Some(label.clone()));
                    thread::sleep(std::time::Duration::from_millis(10));
                    assert_eq!(get_thread_label(), Some(label));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }
}
