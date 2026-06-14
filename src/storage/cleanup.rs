use crate::storage::LogLevel;
use crate::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Policy for cleaning up old log files.
///
/// CleanupPolicy defines rules for automatic log file deletion based on:
/// - Total size limits
/// - Individual file size limits
/// - Age-based retention (global and per-level)
/// - Storage threshold triggers
///
/// # Examples
///
/// ```
/// use scribe::CleanupPolicy;
/// use scribe::LogLevel;
/// use std::collections::HashMap;
///
/// let mut level_retention = HashMap::new();
/// level_retention.insert(LogLevel::Debug, 1);   // Keep debug logs for 1 day
/// level_retention.insert(LogLevel::Error, 30);  // Keep error logs for 30 days
///
/// let policy = CleanupPolicy {
///     max_total_size: 100 * 1024 * 1024,  // 100MB total
///     max_file_size: 10 * 1024 * 1024,    // 10MB per file
///     retention_days: 7,                   // Default 7 days
///     level_retention,
///     cleanup_threshold: 0.9,              // Trigger at 90% full
/// };
/// ```
pub struct CleanupPolicy {
    /// Maximum total size of all log files in bytes.
    pub max_total_size: usize,
    /// Maximum size of a single log file in bytes.
    pub max_file_size: usize,
    /// Default retention period in days.
    pub retention_days: u32,
    /// Per-level retention overrides (in days).
    pub level_retention: HashMap<LogLevel, u32>,
    /// Threshold (0.0-1.0) that triggers cleanup.
    pub cleanup_threshold: f32,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        let mut level_retention = HashMap::new();
        level_retention.insert(LogLevel::Debug, 1);
        level_retention.insert(LogLevel::Info, 3);
        level_retention.insert(LogLevel::Warn, 7);
        level_retention.insert(LogLevel::Error, 30);

        Self {
            max_total_size: 50 * 1024 * 1024, // 50MB
            max_file_size: 10 * 1024 * 1024,  // 10MB
            retention_days: 7,
            level_retention,
            cleanup_threshold: 0.9,
        }
    }
}

/// Metadata for a log file.
///
/// Represents a log file with its properties for cleanup decision-making.
pub struct LogFile {
    /// Path to the log file.
    pub path: PathBuf,
    /// Size of the file in bytes.
    pub size: usize,
    /// File creation timestamp.
    pub created_at: SystemTime,
    /// Minimum (highest severity) log level in this file.
    pub min_level: LogLevel,
}

impl LogFile {
    /// Returns the age of the file in days.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::LogFile;
    /// use scribe::LogLevel;
    /// use std::path::PathBuf;
    /// use std::time::SystemTime;
    ///
    /// let file = LogFile {
    ///     path: PathBuf::from("/tmp/app.log"),
    ///     size: 1024,
    ///     created_at: SystemTime::now(),
    ///     min_level: LogLevel::Info,
    /// };
    ///
    /// assert_eq!(file.age_days(), 0);
    /// ```
    pub fn age_days(&self) -> u32 {
        let now = SystemTime::now();
        let duration = now.duration_since(self.created_at).unwrap_or_default();
        (duration.as_secs() / 86400) as u32
    }
}

/// Report of cleanup operations.
///
/// Contains statistics about files deleted during cleanup.
pub struct CleanupReport {
    /// Number of files deleted.
    pub files_deleted: usize,
    /// Total bytes freed.
    pub bytes_freed: u64,
    /// Paths of deleted files.
    pub deleted_files: Vec<String>,
}

impl CleanupPolicy {
    /// Performs cleanup on the given log files according to the policy.
    ///
    /// # Cleanup Strategy
    ///
    /// 1. Delete files exceeding retention period (by level or global)
    /// 2. If still over size limit, delete by priority score (age + size + level)
    ///
    /// # Arguments
    ///
    /// * `files` - Slice of log files to evaluate for cleanup.
    ///
    /// # Returns
    ///
    /// * `Ok(CleanupReport)` - Report of deleted files and freed space.
    /// * `Err(ScribeError)` - If cleanup operation fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use scribe::{CleanupPolicy, LogFile, LogLevel};
    /// use std::path::PathBuf;
    /// use std::time::SystemTime;
    ///
    /// let policy = CleanupPolicy::default();
    /// let files = vec![
    ///     LogFile {
    ///         path: PathBuf::from("/tmp/old.log"),
    ///         size: 1024,
    ///         created_at: SystemTime::now(),
    ///         min_level: LogLevel::Debug,
    ///     }
    /// ];
    ///
    /// let report = policy.cleanup(&files).unwrap();
    /// println!("Deleted {} files, freed {} bytes", report.files_deleted, report.bytes_freed);
    /// ```
    pub fn cleanup(&self, files: &[LogFile]) -> Result<CleanupReport> {
        let mut report = CleanupReport {
            files_deleted: 0,
            bytes_freed: 0,
            deleted_files: Vec::new(),
        };

        // 第一步：删除超过保留期的文件
        for file in files {
            if self.should_delete_by_time(file) {
                if let Ok(_) = std::fs::remove_file(&file.path) {
                    report.files_deleted += 1;
                    report.bytes_freed += file.size as u64;
                    report
                        .deleted_files
                        .push(file.path.to_string_lossy().to_string());
                }
            }
        }

        // 第二步：如果超过大小限制，按优先级删除
        let remaining_files: Vec<_> = files
            .iter()
            .filter(|f| {
                !report
                    .deleted_files
                    .contains(&f.path.to_string_lossy().to_string())
            })
            .collect();

        let total_size: usize = remaining_files.iter().map(|f| f.size).sum();

        if total_size > self.max_total_size {
            let mut sorted_files = remaining_files;
            sorted_files.sort_by_key(|f| self.priority_score(f));

            let target_size = (self.max_total_size as f32 * 0.8) as usize;
            let mut current_size = total_size;

            for file in sorted_files {
                if current_size <= target_size {
                    break;
                }

                if let Ok(_) = std::fs::remove_file(&file.path) {
                    report.files_deleted += 1;
                    report.bytes_freed += file.size as u64;
                    report
                        .deleted_files
                        .push(file.path.to_string_lossy().to_string());
                    current_size -= file.size;
                }
            }
        }

        Ok(report)
    }

    fn should_delete_by_time(&self, file: &LogFile) -> bool {
        let age = file.age_days();
        let retention = self
            .level_retention
            .get(&file.min_level)
            .unwrap_or(&self.retention_days);

        age > *retention
    }

    fn priority_score(&self, file: &LogFile) -> u64 {
        let age_score = file.age_days() as u64 * 100;
        let size_score = (file.size / 1024) as u64;
        let level_score = match file.min_level {
            LogLevel::Debug => 1000,
            LogLevel::Verbose => 900,
            LogLevel::Info => 500,
            LogLevel::Warn => 100,
            LogLevel::Error => 0,
        };

        age_score + size_score + level_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::time::Duration;

    // Helper function to create a test file
    fn create_test_file(path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    // 1. CleanupPolicy::default() 测试
    #[test]
    fn test_cleanup_policy_default() {
        let policy = CleanupPolicy::default();

        assert_eq!(policy.max_total_size, 50 * 1024 * 1024);
        assert_eq!(policy.max_file_size, 10 * 1024 * 1024);
        assert_eq!(policy.retention_days, 7);
        assert_eq!(policy.cleanup_threshold, 0.9);

        // 验证各级别默认保留时间
        assert_eq!(*policy.level_retention.get(&LogLevel::Debug).unwrap(), 1);
        assert_eq!(*policy.level_retention.get(&LogLevel::Info).unwrap(), 3);
        assert_eq!(*policy.level_retention.get(&LogLevel::Warn).unwrap(), 7);
        assert_eq!(*policy.level_retention.get(&LogLevel::Error).unwrap(), 30);
    }

    // 2. cleanup() 完整流程测试
    #[test]
    fn test_cleanup_complete_workflow() {
        let temp_dir = std::env::temp_dir().join("scribe_test_cleanup_workflow");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // 创建测试文件
        let file1_path = temp_dir.join("file1.log");
        let file2_path = temp_dir.join("file2.log");
        create_test_file(&file1_path, "test content 1").unwrap();
        create_test_file(&file2_path, "test content 2").unwrap();

        let policy = CleanupPolicy {
            max_total_size: 100,
            max_file_size: 50,
            retention_days: 1,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let files = vec![
            LogFile {
                path: file1_path.clone(),
                size: 14,
                created_at: SystemTime::now() - Duration::from_secs(3 * 86400),
                min_level: LogLevel::Debug,
            },
            LogFile {
                path: file2_path.clone(),
                size: 14,
                created_at: SystemTime::now() - Duration::from_secs(3 * 86400),
                min_level: LogLevel::Error,
            },
        ];

        let report = policy.cleanup(&files).unwrap();

        // 两个文件都超过保留期（1天），应该被删除
        assert_eq!(report.files_deleted, 2);
        assert_eq!(report.bytes_freed, 28);
        assert_eq!(report.deleted_files.len(), 2);

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // 3. cleanup() 空目录测试
    #[test]
    fn test_cleanup_empty_directory() {
        let policy = CleanupPolicy::default();
        let files: Vec<LogFile> = vec![];

        let report = policy.cleanup(&files).unwrap();

        assert_eq!(report.files_deleted, 0);
        assert_eq!(report.bytes_freed, 0);
        assert_eq!(report.deleted_files.len(), 0);
    }

    // 4. cleanup() 时间策略测试
    #[test]
    fn test_cleanup_by_time() {
        let temp_dir = std::env::temp_dir().join("scribe_test_cleanup_time");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let old_file_path = temp_dir.join("old.log");
        let new_file_path = temp_dir.join("new.log");
        create_test_file(&old_file_path, "old content").unwrap();
        create_test_file(&new_file_path, "new content").unwrap();

        let policy = CleanupPolicy::default();

        let files = vec![
            LogFile {
                path: old_file_path.clone(),
                size: 1024,
                created_at: SystemTime::now() - Duration::from_secs(30 * 86400), // 30 days old
                min_level: LogLevel::Debug,                                      // Debug 保留 1 天
            },
            LogFile {
                path: new_file_path.clone(),
                size: 1024,
                created_at: SystemTime::now(), // Fresh file
                min_level: LogLevel::Debug,
            },
        ];

        let report = policy.cleanup(&files).unwrap();

        // 只有旧文件应该被删除
        assert_eq!(report.files_deleted, 1);
        assert_eq!(report.bytes_freed, 1024);
        assert!(!old_file_path.exists());
        assert!(new_file_path.exists());

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // 5. cleanup() 大小策略测试
    #[test]
    fn test_cleanup_by_size() {
        let temp_dir = std::env::temp_dir().join("scribe_test_cleanup_size");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file1_path = temp_dir.join("file1.log");
        let file2_path = temp_dir.join("file2.log");
        create_test_file(&file1_path, "content1").unwrap();
        create_test_file(&file2_path, "content2").unwrap();

        let policy = CleanupPolicy {
            max_total_size: 100, // 100 bytes total limit
            max_file_size: 50,
            retention_days: 365, // 不按时间删除
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let files = vec![
            LogFile {
                path: file1_path.clone(),
                size: 80, // 大文件，老文件
                created_at: SystemTime::now() - Duration::from_secs(10 * 86400),
                min_level: LogLevel::Debug,
            },
            LogFile {
                path: file2_path.clone(),
                size: 80, // 大文件，新文件
                created_at: SystemTime::now() - Duration::from_secs(1 * 86400),
                min_level: LogLevel::Debug,
            },
        ];

        let report = policy.cleanup(&files).unwrap();

        // 总大小 160 > 100，应该删除优先级高的文件（老的 Debug 文件）
        assert!(report.files_deleted >= 1);
        assert!(report.bytes_freed >= 80);

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // 6. cleanup() 混合策略测试
    #[test]
    fn test_cleanup_mixed_strategy() {
        let temp_dir = std::env::temp_dir().join("scribe_test_cleanup_mixed");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file1_path = temp_dir.join("file1.log");
        let file2_path = temp_dir.join("file2.log");
        let file3_path = temp_dir.join("file3.log");
        create_test_file(&file1_path, "1").unwrap();
        create_test_file(&file2_path, "2").unwrap();
        create_test_file(&file3_path, "3").unwrap();

        let policy = CleanupPolicy {
            max_total_size: 50,
            max_file_size: 50,
            retention_days: 2,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let files = vec![
            LogFile {
                path: file1_path.clone(),
                size: 30,
                created_at: SystemTime::now() - Duration::from_secs(5 * 86400), // 超时
                min_level: LogLevel::Debug,
            },
            LogFile {
                path: file2_path.clone(),
                size: 30,
                created_at: SystemTime::now(), // 不超时但总大小超限
                min_level: LogLevel::Info,
            },
            LogFile {
                path: file3_path.clone(),
                size: 30,
                created_at: SystemTime::now(),
                min_level: LogLevel::Error,
            },
        ];

        let report = policy.cleanup(&files).unwrap();

        // file1 因时间被删除，file2/file3 可能因大小被删除
        assert!(report.files_deleted >= 1);
        assert!(!file1_path.exists()); // file1 肯定被删除

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    // 7. cleanup() 文件删除失败测试
    #[test]
    fn test_cleanup_file_deletion_failure() {
        let policy = CleanupPolicy::default();

        // 使用不存在的文件路径
        let files = vec![LogFile {
            path: PathBuf::from("/nonexistent/path/file.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(30 * 86400),
            min_level: LogLevel::Debug,
        }];

        let report = policy.cleanup(&files).unwrap();

        // 删除失败不应该被计入报告
        assert_eq!(report.files_deleted, 0);
        assert_eq!(report.bytes_freed, 0);
    }

    // 8. priority_score() 不同级别测试
    #[test]
    fn test_priority_score_different_levels() {
        let policy = CleanupPolicy::default();
        let base_time = SystemTime::now() - Duration::from_secs(5 * 86400);

        let debug_file = LogFile {
            path: PathBuf::from("debug.log"),
            size: 1024,
            created_at: base_time,
            min_level: LogLevel::Debug,
        };

        let verbose_file = LogFile {
            path: PathBuf::from("verbose.log"),
            size: 1024,
            created_at: base_time,
            min_level: LogLevel::Verbose,
        };

        let info_file = LogFile {
            path: PathBuf::from("info.log"),
            size: 1024,
            created_at: base_time,
            min_level: LogLevel::Info,
        };

        let warn_file = LogFile {
            path: PathBuf::from("warn.log"),
            size: 1024,
            created_at: base_time,
            min_level: LogLevel::Warn,
        };

        let error_file = LogFile {
            path: PathBuf::from("error.log"),
            size: 1024,
            created_at: base_time,
            min_level: LogLevel::Error,
        };

        // 验证优先级顺序：Debug > Verbose > Info > Warn > Error
        assert!(policy.priority_score(&debug_file) > policy.priority_score(&verbose_file));
        assert!(policy.priority_score(&verbose_file) > policy.priority_score(&info_file));
        assert!(policy.priority_score(&info_file) > policy.priority_score(&warn_file));
        assert!(policy.priority_score(&warn_file) > policy.priority_score(&error_file));
    }

    #[test]
    fn test_priority_score_age_impact() {
        let policy = CleanupPolicy::default();

        let old_file = LogFile {
            path: PathBuf::from("old.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(10 * 86400), // 10 days
            min_level: LogLevel::Info,
        };

        let new_file = LogFile {
            path: PathBuf::from("new.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(1 * 86400), // 1 day
            min_level: LogLevel::Info,
        };

        // 更老的文件应该有更高的优先级分数
        assert!(policy.priority_score(&old_file) > policy.priority_score(&new_file));
    }

    #[test]
    fn test_priority_score_size_impact() {
        let policy = CleanupPolicy::default();

        let large_file = LogFile {
            path: PathBuf::from("large.log"),
            size: 10 * 1024 * 1024, // 10MB
            created_at: SystemTime::now(),
            min_level: LogLevel::Info,
        };

        let small_file = LogFile {
            path: PathBuf::from("small.log"),
            size: 1024, // 1KB
            created_at: SystemTime::now(),
            min_level: LogLevel::Info,
        };

        // 更大的文件应该有更高的优先级分数
        assert!(policy.priority_score(&large_file) > policy.priority_score(&small_file));
    }

    // 9. should_delete_by_time() 边界测试
    #[test]
    fn test_should_delete_by_time_exactly_at_retention() {
        let policy = CleanupPolicy {
            max_total_size: 100 * 1024 * 1024,
            max_file_size: 10 * 1024 * 1024,
            retention_days: 7,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let file_exactly_7_days = LogFile {
            path: PathBuf::from("exact.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(7 * 86400),
            min_level: LogLevel::Info,
        };

        // 正好 7 天不应该删除（需要超过）
        assert!(!policy.should_delete_by_time(&file_exactly_7_days));
    }

    #[test]
    fn test_should_delete_by_time_just_over_retention() {
        let policy = CleanupPolicy {
            max_total_size: 100 * 1024 * 1024,
            max_file_size: 10 * 1024 * 1024,
            retention_days: 7,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let file_over_7_days = LogFile {
            path: PathBuf::from("over.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(8 * 86400),
            min_level: LogLevel::Info,
        };

        // 超过 7 天应该删除
        assert!(policy.should_delete_by_time(&file_over_7_days));
    }

    #[test]
    fn test_should_delete_by_time_with_level_override() {
        let mut level_retention = HashMap::new();
        level_retention.insert(LogLevel::Debug, 1);
        level_retention.insert(LogLevel::Error, 30);

        let policy = CleanupPolicy {
            max_total_size: 100 * 1024 * 1024,
            max_file_size: 10 * 1024 * 1024,
            retention_days: 7,
            level_retention,
            cleanup_threshold: 0.9,
        };

        let debug_file_2_days = LogFile {
            path: PathBuf::from("debug.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(2 * 86400),
            min_level: LogLevel::Debug,
        };

        let error_file_10_days = LogFile {
            path: PathBuf::from("error.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(10 * 86400),
            min_level: LogLevel::Error,
        };

        // Debug 保留 1 天，2 天的应该删除
        assert!(policy.should_delete_by_time(&debug_file_2_days));

        // Error 保留 30 天，10 天的不应该删除
        assert!(!policy.should_delete_by_time(&error_file_10_days));
    }

    #[test]
    fn test_should_delete_by_time_fresh_file() {
        let policy = CleanupPolicy::default();

        let fresh_file = LogFile {
            path: PathBuf::from("fresh.log"),
            size: 1024,
            created_at: SystemTime::now(),
            min_level: LogLevel::Debug,
        };

        // 新文件不应该删除
        assert!(!policy.should_delete_by_time(&fresh_file));
    }

    // 10. CleanupReport 生成测试
    #[test]
    fn test_cleanup_report_generation() {
        let temp_dir = std::env::temp_dir().join("scribe_test_report");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let file1_path = temp_dir.join("report1.log");
        let file2_path = temp_dir.join("report2.log");
        create_test_file(&file1_path, "test1").unwrap();
        create_test_file(&file2_path, "test2").unwrap();

        let policy = CleanupPolicy {
            max_total_size: 1,
            max_file_size: 1,
            retention_days: 0,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let files = vec![
            LogFile {
                path: file1_path.clone(),
                size: 5,
                created_at: SystemTime::now() - Duration::from_secs(2 * 86400),
                min_level: LogLevel::Debug,
            },
            LogFile {
                path: file2_path.clone(),
                size: 5,
                created_at: SystemTime::now() - Duration::from_secs(1 * 86400),
                min_level: LogLevel::Info,
            },
        ];

        let report = policy.cleanup(&files).unwrap();

        // 验证报告内容
        assert!(report.files_deleted > 0);
        assert!(report.bytes_freed > 0);
        assert_eq!(report.deleted_files.len(), report.files_deleted);

        // 验证路径在报告中
        for deleted in &report.deleted_files {
            assert!(deleted.contains("report") && deleted.contains(".log"));
        }

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_cleanup_report_no_deletions() {
        let policy = CleanupPolicy {
            max_total_size: 1000 * 1024 * 1024,
            max_file_size: 100 * 1024 * 1024,
            retention_days: 365,
            level_retention: HashMap::new(),
            cleanup_threshold: 0.9,
        };

        let files = vec![LogFile {
            path: PathBuf::from("keep.log"),
            size: 1024,
            created_at: SystemTime::now(),
            min_level: LogLevel::Info,
        }];

        let report = policy.cleanup(&files).unwrap();

        // 没有文件应该被删除
        assert_eq!(report.files_deleted, 0);
        assert_eq!(report.bytes_freed, 0);
        assert_eq!(report.deleted_files.len(), 0);
    }

    // 额外测试：LogFile::age_days()
    #[test]
    fn test_log_file_age_days() {
        let file_today = LogFile {
            path: PathBuf::from("today.log"),
            size: 1024,
            created_at: SystemTime::now(),
            min_level: LogLevel::Info,
        };

        let file_5_days = LogFile {
            path: PathBuf::from("5days.log"),
            size: 1024,
            created_at: SystemTime::now() - Duration::from_secs(5 * 86400),
            min_level: LogLevel::Info,
        };

        assert_eq!(file_today.age_days(), 0);
        assert_eq!(file_5_days.age_days(), 5);
    }
}
