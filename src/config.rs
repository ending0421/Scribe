use std::collections::HashMap;
use std::path::PathBuf;
use crate::storage::LogLevel;

/// Configuration for the Scribe logging system.
///
/// Config provides comprehensive settings for log storage, rotation,
/// retention, compression, and encryption.
///
/// # Examples
///
/// ```
/// use scribe::Config;
/// use std::path::PathBuf;
///
/// // Use default configuration
/// let config = Config::default();
///
/// // Customize with builder pattern
/// let config = Config::default()
///     .with_log_dir(PathBuf::from("/var/logs/myapp"))
///     .with_max_total_size(100 * 1024 * 1024)  // 100MB
///     .with_retention_days(30)
///     .with_encryption(true);
/// ```
#[derive(Clone)]
pub struct Config {
    /// Directory where log files are stored.
    pub log_dir: PathBuf,
    /// Size of each memory-mapped buffer in bytes.
    pub buffer_capacity: usize,
    /// Maximum total size of all log files in bytes.
    pub max_total_size: usize,
    /// Maximum size of a single log file in bytes.
    pub max_file_size: usize,
    /// Default retention period in days.
    pub retention_days: u32,
    /// Per-level retention overrides (in days).
    pub level_retention: HashMap<LogLevel, u32>,
    /// Enable compression when rotating log files.
    pub compress_on_rotation: bool,
    /// Compression level (0-9, higher = better compression, slower).
    pub compression_level: i32,
    /// Enable encryption for log files.
    pub enable_encryption: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut level_retention = HashMap::new();
        level_retention.insert(LogLevel::Debug, 1);
        level_retention.insert(LogLevel::Info, 3);
        level_retention.insert(LogLevel::Warn, 7);
        level_retention.insert(LogLevel::Error, 30);

        Self {
            log_dir: PathBuf::from("/tmp/scribe_logs"),
            buffer_capacity: 4 * 1024 * 1024,  // 4MB
            max_total_size: 50 * 1024 * 1024,  // 50MB
            max_file_size: 10 * 1024 * 1024,   // 10MB
            retention_days: 7,
            level_retention,
            compress_on_rotation: true,
            compression_level: 3,
            enable_encryption: false,
        }
    }
}

impl Config {
    /// Sets the log directory.
    ///
    /// # Arguments
    ///
    /// * `log_dir` - Path to the log directory.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::Config;
    /// use std::path::PathBuf;
    ///
    /// let config = Config::default()
    ///     .with_log_dir(PathBuf::from("/var/logs/myapp"));
    /// ```
    pub fn with_log_dir(mut self, log_dir: PathBuf) -> Self {
        self.log_dir = log_dir;
        self
    }

    /// Sets the maximum total size for all log files.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum size in bytes.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::Config;
    ///
    /// let config = Config::default()
    ///     .with_max_total_size(100 * 1024 * 1024);  // 100MB
    /// ```
    pub fn with_max_total_size(mut self, size: usize) -> Self {
        self.max_total_size = size;
        self
    }

    /// Sets the default retention period.
    ///
    /// # Arguments
    ///
    /// * `days` - Number of days to retain logs.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::Config;
    ///
    /// let config = Config::default()
    ///     .with_retention_days(30);
    /// ```
    pub fn with_retention_days(mut self, days: u32) -> Self {
        self.retention_days = days;
        self
    }

    /// Enables or disables encryption.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable encryption.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::Config;
    ///
    /// let config = Config::default()
    ///     .with_encryption(true);
    /// ```
    pub fn with_encryption(mut self, enabled: bool) -> Self {
        self.enable_encryption = enabled;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();

        // 验证所有默认值
        assert_eq!(config.log_dir, PathBuf::from("/tmp/scribe_logs"));
        assert_eq!(config.buffer_capacity, 4 * 1024 * 1024);
        assert_eq!(config.max_total_size, 50 * 1024 * 1024);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.retention_days, 7);
        assert_eq!(config.compress_on_rotation, true);
        assert_eq!(config.compression_level, 3);
        assert_eq!(config.enable_encryption, false);
    }

    #[test]
    fn test_default_level_retention() {
        let config = Config::default();

        // 验证 level_retention HashMap
        assert_eq!(config.level_retention.len(), 4);
        assert_eq!(config.level_retention.get(&LogLevel::Debug), Some(&1));
        assert_eq!(config.level_retention.get(&LogLevel::Info), Some(&3));
        assert_eq!(config.level_retention.get(&LogLevel::Warn), Some(&7));
        assert_eq!(config.level_retention.get(&LogLevel::Error), Some(&30));
    }

    #[test]
    fn test_with_log_dir() {
        let custom_path = PathBuf::from("/custom/log/path");
        let config = Config::default().with_log_dir(custom_path.clone());

        assert_eq!(config.log_dir, custom_path);

        // 验证其他字段未改变
        assert_eq!(config.buffer_capacity, 4 * 1024 * 1024);
        assert_eq!(config.max_total_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_with_log_dir_empty_path() {
        let empty_path = PathBuf::from("");
        let config = Config::default().with_log_dir(empty_path.clone());

        assert_eq!(config.log_dir, empty_path);
    }

    #[test]
    fn test_with_log_dir_relative_path() {
        let relative_path = PathBuf::from("./logs");
        let config = Config::default().with_log_dir(relative_path.clone());

        assert_eq!(config.log_dir, relative_path);
    }

    #[test]
    fn test_with_max_total_size() {
        let config = Config::default().with_max_total_size(100 * 1024 * 1024);

        assert_eq!(config.max_total_size, 100 * 1024 * 1024);

        // 验证其他字段未改变
        assert_eq!(config.log_dir, PathBuf::from("/tmp/scribe_logs"));
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_with_max_total_size_zero() {
        let config = Config::default().with_max_total_size(0);

        assert_eq!(config.max_total_size, 0);
    }

    #[test]
    fn test_with_max_total_size_very_large() {
        let large_size = usize::MAX;
        let config = Config::default().with_max_total_size(large_size);

        assert_eq!(config.max_total_size, large_size);
    }

    #[test]
    fn test_with_max_total_size_small_value() {
        let config = Config::default().with_max_total_size(1024);

        assert_eq!(config.max_total_size, 1024);
    }

    #[test]
    fn test_with_retention_days() {
        let config = Config::default().with_retention_days(30);

        assert_eq!(config.retention_days, 30);

        // 验证其他字段未改变
        assert_eq!(config.log_dir, PathBuf::from("/tmp/scribe_logs"));
        assert_eq!(config.max_total_size, 50 * 1024 * 1024);
    }

    #[test]
    fn test_with_retention_days_zero() {
        let config = Config::default().with_retention_days(0);

        assert_eq!(config.retention_days, 0);
    }

    #[test]
    fn test_with_retention_days_very_large() {
        let config = Config::default().with_retention_days(u32::MAX);

        assert_eq!(config.retention_days, u32::MAX);
    }

    #[test]
    fn test_with_retention_days_one() {
        let config = Config::default().with_retention_days(1);

        assert_eq!(config.retention_days, 1);
    }

    #[test]
    fn test_with_encryption_enabled() {
        let config = Config::default().with_encryption(true);

        assert_eq!(config.enable_encryption, true);

        // 验证其他字段未改变
        assert_eq!(config.log_dir, PathBuf::from("/tmp/scribe_logs"));
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_with_encryption_disabled() {
        let config = Config::default().with_encryption(false);

        assert_eq!(config.enable_encryption, false);
    }

    #[test]
    fn test_with_encryption_toggle() {
        let config = Config::default()
            .with_encryption(true)
            .with_encryption(false);

        assert_eq!(config.enable_encryption, false);
    }

    #[test]
    fn test_builder_chain_all_methods() {
        let config = Config::default()
            .with_log_dir(PathBuf::from("/var/logs"))
            .with_max_total_size(200 * 1024 * 1024)
            .with_retention_days(60)
            .with_encryption(true);

        assert_eq!(config.log_dir, PathBuf::from("/var/logs"));
        assert_eq!(config.max_total_size, 200 * 1024 * 1024);
        assert_eq!(config.retention_days, 60);
        assert_eq!(config.enable_encryption, true);
    }

    #[test]
    fn test_builder_chain_partial() {
        let config = Config::default()
            .with_max_total_size(100 * 1024 * 1024)
            .with_encryption(true);

        assert_eq!(config.max_total_size, 100 * 1024 * 1024);
        assert_eq!(config.enable_encryption, true);

        // 验证未设置的字段保持默认值
        assert_eq!(config.log_dir, PathBuf::from("/tmp/scribe_logs"));
        assert_eq!(config.retention_days, 7);
    }

    #[test]
    fn test_builder_chain_override() {
        let config = Config::default()
            .with_retention_days(30)
            .with_retention_days(60);

        assert_eq!(config.retention_days, 60);
    }

    #[test]
    fn test_config_clone() {
        let config1 = Config::default()
            .with_log_dir(PathBuf::from("/test"))
            .with_retention_days(15);

        let config2 = config1.clone();

        assert_eq!(config2.log_dir, PathBuf::from("/test"));
        assert_eq!(config2.retention_days, 15);
        assert_eq!(config2.level_retention.len(), 4);
    }

    #[test]
    fn test_level_retention_immutability_after_build() {
        let config = Config::default();

        // 验证 level_retention 包含预期的键值对
        assert!(config.level_retention.contains_key(&LogLevel::Debug));
        assert!(config.level_retention.contains_key(&LogLevel::Info));
        assert!(config.level_retention.contains_key(&LogLevel::Warn));
        assert!(config.level_retention.contains_key(&LogLevel::Error));
    }

    #[test]
    fn test_multiple_configs_independent() {
        let config1 = Config::default().with_retention_days(10);
        let config2 = Config::default().with_retention_days(20);

        assert_eq!(config1.retention_days, 10);
        assert_eq!(config2.retention_days, 20);
    }

    #[test]
    fn test_builder_order_independence() {
        let config1 = Config::default()
            .with_encryption(true)
            .with_retention_days(30)
            .with_log_dir(PathBuf::from("/logs1"));

        let config2 = Config::default()
            .with_log_dir(PathBuf::from("/logs1"))
            .with_retention_days(30)
            .with_encryption(true);

        assert_eq!(config1.log_dir, config2.log_dir);
        assert_eq!(config1.retention_days, config2.retention_days);
        assert_eq!(config1.enable_encryption, config2.enable_encryption);
    }

    #[test]
    fn test_edge_case_extreme_values() {
        let config = Config::default()
            .with_max_total_size(usize::MAX)
            .with_retention_days(u32::MAX);

        assert_eq!(config.max_total_size, usize::MAX);
        assert_eq!(config.retention_days, u32::MAX);
    }

    #[test]
    fn test_edge_case_minimum_values() {
        let config = Config::default()
            .with_max_total_size(0)
            .with_retention_days(0);

        assert_eq!(config.max_total_size, 0);
        assert_eq!(config.retention_days, 0);
    }

    #[test]
    fn test_compression_settings_preserved() {
        let config = Config::default()
            .with_encryption(true);

        // 验证未被 builder 修改的压缩设置保持默认值
        assert_eq!(config.compress_on_rotation, true);
        assert_eq!(config.compression_level, 3);
    }

    #[test]
    fn test_buffer_capacity_preserved() {
        let config = Config::default()
            .with_log_dir(PathBuf::from("/test"));

        // 验证 buffer_capacity 保持默认值
        assert_eq!(config.buffer_capacity, 4 * 1024 * 1024);
    }

    #[test]
    fn test_max_file_size_preserved() {
        let config = Config::default()
            .with_max_total_size(100 * 1024 * 1024);

        // 验证 max_file_size 保持默认值
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
    }
}
