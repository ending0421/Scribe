use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    BufferFull,
    DiskFull,
    Compression,
    Encryption,
    WriteFailed,
}

/// 全局性能和统计指标收集器
pub struct ScribeMetrics {
    // 写入统计
    pub writes_count: AtomicU64,
    pub writes_failed: AtomicU64,
    pub bytes_written: AtomicU64,

    // 性能统计
    pub flush_count: AtomicU64,
    pub worker_wakeups: AtomicU64,
    pub compression_time_us: AtomicU64,
    pub encryption_time_us: AtomicU64,
    pub io_time_us: AtomicU64,

    // 错误统计
    pub buffer_full_count: AtomicU64,
    pub disk_full_count: AtomicU64,
    pub compression_errors: AtomicU64,
    pub encryption_errors: AtomicU64,

    // 清理统计
    pub cleanup_count: AtomicU64,
    pub files_deleted: AtomicU64,
    pub bytes_freed: AtomicU64,
}

impl ScribeMetrics {
    /// 创建新的指标收集器，所有计数器初始化为 0
    pub fn new() -> Self {
        Self {
            writes_count: AtomicU64::new(0),
            writes_failed: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),

            flush_count: AtomicU64::new(0),
            worker_wakeups: AtomicU64::new(0),
            compression_time_us: AtomicU64::new(0),
            encryption_time_us: AtomicU64::new(0),
            io_time_us: AtomicU64::new(0),

            buffer_full_count: AtomicU64::new(0),
            disk_full_count: AtomicU64::new(0),
            compression_errors: AtomicU64::new(0),
            encryption_errors: AtomicU64::new(0),

            cleanup_count: AtomicU64::new(0),
            files_deleted: AtomicU64::new(0),
            bytes_freed: AtomicU64::new(0),
        }
    }

    /// 记录一次成功的写入操作
    pub fn record_write(&self, bytes: u64) {
        self.writes_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// 记录一次写入失败
    pub fn record_write_failed(&self) {
        self.writes_failed.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录错误
    pub fn record_error(&self, error_type: ErrorType) {
        match error_type {
            ErrorType::BufferFull => {
                self.buffer_full_count.fetch_add(1, Ordering::Relaxed);
            }
            ErrorType::DiskFull => {
                self.disk_full_count.fetch_add(1, Ordering::Relaxed);
            }
            ErrorType::Compression => {
                self.compression_errors.fetch_add(1, Ordering::Relaxed);
            }
            ErrorType::Encryption => {
                self.encryption_errors.fetch_add(1, Ordering::Relaxed);
            }
            ErrorType::WriteFailed => {
                self.writes_failed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// 记录一次 flush 操作
    pub fn record_flush(&self) {
        self.flush_count.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录后台工作线程唤醒
    pub fn record_worker_wakeup(&self) {
        self.worker_wakeups.fetch_add(1, Ordering::Relaxed);
    }

    /// 记录压缩耗时（微秒）
    pub fn record_compression_time(&self, microseconds: u64) {
        self.compression_time_us
            .fetch_add(microseconds, Ordering::Relaxed);
    }

    /// 记录加密耗时（微秒）
    pub fn record_encryption_time(&self, microseconds: u64) {
        self.encryption_time_us
            .fetch_add(microseconds, Ordering::Relaxed);
    }

    /// 记录 I/O 耗时（微秒）
    pub fn record_io_time(&self, microseconds: u64) {
        self.io_time_us.fetch_add(microseconds, Ordering::Relaxed);
    }

    /// 记录清理操作
    pub fn record_cleanup(&self, files_deleted: u64, bytes_freed: u64) {
        self.cleanup_count.fetch_add(1, Ordering::Relaxed);
        self.files_deleted
            .fetch_add(files_deleted, Ordering::Relaxed);
        self.bytes_freed.fetch_add(bytes_freed, Ordering::Relaxed);
    }

    /// 获取当前所有指标的快照
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            writes_count: self.writes_count.load(Ordering::Relaxed),
            writes_failed: self.writes_failed.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),

            flush_count: self.flush_count.load(Ordering::Relaxed),
            worker_wakeups: self.worker_wakeups.load(Ordering::Relaxed),
            compression_time_us: self.compression_time_us.load(Ordering::Relaxed),
            encryption_time_us: self.encryption_time_us.load(Ordering::Relaxed),
            io_time_us: self.io_time_us.load(Ordering::Relaxed),

            buffer_full_count: self.buffer_full_count.load(Ordering::Relaxed),
            disk_full_count: self.disk_full_count.load(Ordering::Relaxed),
            compression_errors: self.compression_errors.load(Ordering::Relaxed),
            encryption_errors: self.encryption_errors.load(Ordering::Relaxed),

            cleanup_count: self.cleanup_count.load(Ordering::Relaxed),
            files_deleted: self.files_deleted.load(Ordering::Relaxed),
            bytes_freed: self.bytes_freed.load(Ordering::Relaxed),
        }
    }

    /// 重置所有计数器为 0
    pub fn reset(&self) {
        self.writes_count.store(0, Ordering::Relaxed);
        self.writes_failed.store(0, Ordering::Relaxed);
        self.bytes_written.store(0, Ordering::Relaxed);

        self.flush_count.store(0, Ordering::Relaxed);
        self.worker_wakeups.store(0, Ordering::Relaxed);
        self.compression_time_us.store(0, Ordering::Relaxed);
        self.encryption_time_us.store(0, Ordering::Relaxed);
        self.io_time_us.store(0, Ordering::Relaxed);

        self.buffer_full_count.store(0, Ordering::Relaxed);
        self.disk_full_count.store(0, Ordering::Relaxed);
        self.compression_errors.store(0, Ordering::Relaxed);
        self.encryption_errors.store(0, Ordering::Relaxed);

        self.cleanup_count.store(0, Ordering::Relaxed);
        self.files_deleted.store(0, Ordering::Relaxed);
        self.bytes_freed.store(0, Ordering::Relaxed);
    }
}

impl Default for ScribeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// 指标的不可变快照，用于读取和展示
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MetricsSnapshot {
    // 写入统计
    pub writes_count: u64,
    pub writes_failed: u64,
    pub bytes_written: u64,

    // 性能统计
    pub flush_count: u64,
    pub worker_wakeups: u64,
    pub compression_time_us: u64,
    pub encryption_time_us: u64,
    pub io_time_us: u64,

    // 错误统计
    pub buffer_full_count: u64,
    pub disk_full_count: u64,
    pub compression_errors: u64,
    pub encryption_errors: u64,

    // 清理统计
    pub cleanup_count: u64,
    pub files_deleted: u64,
    pub bytes_freed: u64,
}

impl MetricsSnapshot {
    /// 计算写入成功率（0.0 - 1.0）
    pub fn write_success_rate(&self) -> f64 {
        let total = self.writes_count + self.writes_failed;
        if total == 0 {
            1.0
        } else {
            self.writes_count as f64 / total as f64
        }
    }

    /// 计算平均每次压缩耗时（微秒）
    pub fn avg_compression_time_us(&self) -> f64 {
        if self.writes_count == 0 {
            0.0
        } else {
            self.compression_time_us as f64 / self.writes_count as f64
        }
    }

    /// 计算平均每次加密耗时（微秒）
    pub fn avg_encryption_time_us(&self) -> f64 {
        if self.writes_count == 0 {
            0.0
        } else {
            self.encryption_time_us as f64 / self.writes_count as f64
        }
    }

    /// 计算平均每次 I/O 耗时（微秒）
    pub fn avg_io_time_us(&self) -> f64 {
        if self.flush_count == 0 {
            0.0
        } else {
            self.io_time_us as f64 / self.flush_count as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_new() {
        let metrics = ScribeMetrics::new();
        let snapshot = metrics.snapshot();

        assert_eq!(snapshot.writes_count, 0);
        assert_eq!(snapshot.writes_failed, 0);
        assert_eq!(snapshot.bytes_written, 0);
    }

    #[test]
    fn test_record_write() {
        let metrics = ScribeMetrics::new();

        metrics.record_write(100);
        metrics.record_write(200);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.writes_count, 2);
        assert_eq!(snapshot.bytes_written, 300);
    }

    #[test]
    fn test_record_errors() {
        let metrics = ScribeMetrics::new();

        metrics.record_error(ErrorType::BufferFull);
        metrics.record_error(ErrorType::BufferFull);
        metrics.record_error(ErrorType::Compression);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.buffer_full_count, 2);
        assert_eq!(snapshot.compression_errors, 1);
    }

    #[test]
    fn test_record_flush() {
        let metrics = ScribeMetrics::new();

        metrics.record_flush();
        metrics.record_flush();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.flush_count, 2);
    }

    #[test]
    fn test_record_timing() {
        let metrics = ScribeMetrics::new();

        metrics.record_compression_time(1000);
        metrics.record_encryption_time(2000);
        metrics.record_io_time(3000);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.compression_time_us, 1000);
        assert_eq!(snapshot.encryption_time_us, 2000);
        assert_eq!(snapshot.io_time_us, 3000);
    }

    #[test]
    fn test_record_cleanup() {
        let metrics = ScribeMetrics::new();

        metrics.record_cleanup(5, 10240);
        metrics.record_cleanup(3, 5120);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.cleanup_count, 2);
        assert_eq!(snapshot.files_deleted, 8);
        assert_eq!(snapshot.bytes_freed, 15360);
    }

    #[test]
    fn test_reset() {
        let metrics = ScribeMetrics::new();

        metrics.record_write(100);
        metrics.record_flush();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.writes_count, 1);
        assert_eq!(snapshot.flush_count, 1);

        metrics.reset();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.writes_count, 0);
        assert_eq!(snapshot.flush_count, 0);
        assert_eq!(snapshot.bytes_written, 0);
    }

    #[test]
    fn test_snapshot_calculations() {
        let metrics = ScribeMetrics::new();

        // 模拟 10 次成功写入，2 次失败
        for _ in 0..10 {
            metrics.record_write(100);
            metrics.record_compression_time(500);
            metrics.record_encryption_time(300);
        }
        metrics.record_write_failed();
        metrics.record_write_failed();

        // 模拟 3 次 flush
        for _ in 0..3 {
            metrics.record_flush();
            metrics.record_io_time(10000);
        }

        let snapshot = metrics.snapshot();

        // 成功率应该是 10/12 = 0.833...
        assert!((snapshot.write_success_rate() - 0.8333).abs() < 0.001);

        // 平均压缩时间
        assert_eq!(snapshot.avg_compression_time_us(), 500.0);

        // 平均加密时间
        assert_eq!(snapshot.avg_encryption_time_us(), 300.0);

        // 平均 I/O 时间
        assert_eq!(snapshot.avg_io_time_us(), 10000.0);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;

        let metrics = Arc::new(ScribeMetrics::new());
        let mut handles = vec![];

        // 启动 10 个线程，每个写入 100 次
        for _ in 0..10 {
            let metrics_clone = Arc::clone(&metrics);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    metrics_clone.record_write(1);
                }
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.writes_count, 1000);
        assert_eq!(snapshot.bytes_written, 1000);
    }
}
