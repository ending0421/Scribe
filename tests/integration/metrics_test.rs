use scribe::{ScribeMetrics, MetricsSnapshot, ErrorType};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_metrics_record_write() {
    let metrics = ScribeMetrics::new();

    metrics.record_write(1024);
    metrics.record_write(2048);
    metrics.record_write(512);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 3);
    assert_eq!(snapshot.bytes_written, 3584);
    assert_eq!(snapshot.writes_failed, 0);
}

#[test]
fn test_metrics_record_errors() {
    let metrics = ScribeMetrics::new();

    metrics.record_error(ErrorType::BufferFull);
    metrics.record_error(ErrorType::BufferFull);
    metrics.record_error(ErrorType::Compression);
    metrics.record_error(ErrorType::Encryption);
    metrics.record_error(ErrorType::DiskFull);
    metrics.record_error(ErrorType::WriteFailed);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.buffer_full_count, 2);
    assert_eq!(snapshot.compression_errors, 1);
    assert_eq!(snapshot.encryption_errors, 1);
    assert_eq!(snapshot.disk_full_count, 1);
    assert_eq!(snapshot.writes_failed, 1);
}

#[test]
fn test_metrics_record_flush() {
    let metrics = ScribeMetrics::new();

    for _ in 0..5 {
        metrics.record_flush();
    }

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.flush_count, 5);
}

#[test]
fn test_metrics_record_worker_wakeup() {
    let metrics = ScribeMetrics::new();

    for _ in 0..10 {
        metrics.record_worker_wakeup();
    }

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.worker_wakeups, 10);
}

#[test]
fn test_metrics_record_timing() {
    let metrics = ScribeMetrics::new();

    metrics.record_compression_time(1000);
    metrics.record_compression_time(1500);
    metrics.record_compression_time(2000);

    metrics.record_encryption_time(500);
    metrics.record_encryption_time(800);

    metrics.record_io_time(5000);
    metrics.record_io_time(6000);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.compression_time_us, 4500);
    assert_eq!(snapshot.encryption_time_us, 1300);
    assert_eq!(snapshot.io_time_us, 11000);
}

#[test]
fn test_metrics_record_cleanup() {
    let metrics = ScribeMetrics::new();

    metrics.record_cleanup(5, 10240);
    metrics.record_cleanup(3, 5120);
    metrics.record_cleanup(2, 2048);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.cleanup_count, 3);
    assert_eq!(snapshot.files_deleted, 10);
    assert_eq!(snapshot.bytes_freed, 17408);
}

#[test]
fn test_metrics_snapshot() {
    let metrics = ScribeMetrics::new();

    metrics.record_write(1000);
    metrics.record_write(2000);
    metrics.record_flush();
    metrics.record_error(ErrorType::BufferFull);

    let snapshot1 = metrics.snapshot();
    let snapshot2 = metrics.snapshot();

    // 快照应该反映相同的状态
    assert_eq!(snapshot1.writes_count, snapshot2.writes_count);
    assert_eq!(snapshot1.bytes_written, snapshot2.bytes_written);
    assert_eq!(snapshot1.flush_count, snapshot2.flush_count);
    assert_eq!(snapshot1.buffer_full_count, snapshot2.buffer_full_count);
}

#[test]
fn test_metrics_reset() {
    let metrics = ScribeMetrics::new();

    // 记录一些指标
    metrics.record_write(1000);
    metrics.record_write(2000);
    metrics.record_flush();
    metrics.record_error(ErrorType::Compression);
    metrics.record_cleanup(5, 10240);
    metrics.record_compression_time(1000);
    metrics.record_encryption_time(500);
    metrics.record_io_time(3000);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_count, 2);
    assert_eq!(snapshot.bytes_written, 3000);
    assert!(snapshot.compression_time_us > 0);

    // 重置所有指标
    metrics.reset();

    let snapshot_after = metrics.snapshot();
    assert_eq!(snapshot_after.writes_count, 0);
    assert_eq!(snapshot_after.writes_failed, 0);
    assert_eq!(snapshot_after.bytes_written, 0);
    assert_eq!(snapshot_after.flush_count, 0);
    assert_eq!(snapshot_after.worker_wakeups, 0);
    assert_eq!(snapshot_after.compression_time_us, 0);
    assert_eq!(snapshot_after.encryption_time_us, 0);
    assert_eq!(snapshot_after.io_time_us, 0);
    assert_eq!(snapshot_after.buffer_full_count, 0);
    assert_eq!(snapshot_after.disk_full_count, 0);
    assert_eq!(snapshot_after.compression_errors, 0);
    assert_eq!(snapshot_after.encryption_errors, 0);
    assert_eq!(snapshot_after.cleanup_count, 0);
    assert_eq!(snapshot_after.files_deleted, 0);
    assert_eq!(snapshot_after.bytes_freed, 0);
}

#[test]
fn test_metrics_concurrent_writes() {
    let metrics = Arc::new(ScribeMetrics::new());
    let mut handles = vec![];

    // 启动 10 个线程，每个记录 100 次写入
    for i in 0..10 {
        let metrics_clone = Arc::clone(&metrics);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                metrics_clone.record_write(100 + i * 10 + j);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_count, 1000);
}

#[test]
fn test_metrics_concurrent_mixed_operations() {
    let metrics = Arc::new(ScribeMetrics::new());
    let mut handles = vec![];

    // 线程1: 记录写入
    let m1 = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
        for _ in 0..200 {
            m1.record_write(512);
        }
    }));

    // 线程2: 记录 flush
    let m2 = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
        for _ in 0..50 {
            m2.record_flush();
            thread::sleep(Duration::from_micros(10));
        }
    }));

    // 线程3: 记录错误
    let m3 = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
        for _ in 0..30 {
            m3.record_error(ErrorType::BufferFull);
            m3.record_error(ErrorType::Compression);
        }
    }));

    // 线程4: 记录时间
    let m4 = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
        for _ in 0..100 {
            m4.record_compression_time(100);
            m4.record_encryption_time(50);
            m4.record_io_time(200);
        }
    }));

    // 线程5: 记录清理
    let m5 = Arc::clone(&metrics);
    handles.push(thread::spawn(move || {
        for i in 0..20 {
            m5.record_cleanup(i + 1, (i + 1) * 1024);
        }
    }));

    for handle in handles {
        handle.join().unwrap();
    }

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 200);
    assert_eq!(snapshot.bytes_written, 200 * 512);
    assert_eq!(snapshot.flush_count, 50);
    assert_eq!(snapshot.buffer_full_count, 30);
    assert_eq!(snapshot.compression_errors, 30);
    assert_eq!(snapshot.compression_time_us, 10000);
    assert_eq!(snapshot.encryption_time_us, 5000);
    assert_eq!(snapshot.io_time_us, 20000);
    assert_eq!(snapshot.cleanup_count, 20);
}

#[test]
fn test_metrics_snapshot_calculations() {
    let metrics = ScribeMetrics::new();

    // 场景1: 所有写入成功
    metrics.record_write(1000);
    metrics.record_write(2000);
    metrics.record_write(3000);

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.write_success_rate(), 1.0);

    // 场景2: 部分写入失败
    metrics.reset();
    for _ in 0..8 {
        metrics.record_write(1000);
    }
    metrics.record_write_failed();
    metrics.record_write_failed();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.write_success_rate(), 0.8); // 8/10
}

#[test]
fn test_metrics_avg_compression_time() {
    let metrics = ScribeMetrics::new();

    // 记录 5 次写入，每次压缩时间不同
    metrics.record_write(1000);
    metrics.record_compression_time(100);

    metrics.record_write(2000);
    metrics.record_compression_time(200);

    metrics.record_write(3000);
    metrics.record_compression_time(300);

    metrics.record_write(4000);
    metrics.record_compression_time(400);

    metrics.record_write(5000);
    metrics.record_compression_time(500);

    let snapshot = metrics.snapshot();

    // 总压缩时间: 1500, 总写入: 5, 平均: 300
    assert_eq!(snapshot.avg_compression_time_us(), 300.0);
}

#[test]
fn test_metrics_avg_encryption_time() {
    let metrics = ScribeMetrics::new();

    for i in 1..=10 {
        metrics.record_write(1000);
        metrics.record_encryption_time(i * 50);
    }

    let snapshot = metrics.snapshot();

    // 总加密时间: 50+100+150+...+500 = 2750
    // 平均: 2750 / 10 = 275
    assert_eq!(snapshot.avg_encryption_time_us(), 275.0);
}

#[test]
fn test_metrics_avg_io_time() {
    let metrics = ScribeMetrics::new();

    for i in 1..=5 {
        metrics.record_flush();
        metrics.record_io_time(i * 1000);
    }

    let snapshot = metrics.snapshot();

    // 总 I/O 时间: 1000+2000+3000+4000+5000 = 15000
    // 平均: 15000 / 5 = 3000
    assert_eq!(snapshot.avg_io_time_us(), 3000.0);
}

#[test]
fn test_metrics_zero_division_protection() {
    let metrics = ScribeMetrics::new();
    let snapshot = metrics.snapshot();

    // 没有任何操作时，计算不应该 panic
    assert_eq!(snapshot.write_success_rate(), 1.0);
    assert_eq!(snapshot.avg_compression_time_us(), 0.0);
    assert_eq!(snapshot.avg_encryption_time_us(), 0.0);
    assert_eq!(snapshot.avg_io_time_us(), 0.0);
}

#[test]
fn test_metrics_realistic_scenario() {
    let metrics = ScribeMetrics::new();

    // 模拟实际使用场景
    // 1. 启动阶段 - 100 次成功写入
    for _ in 0..100 {
        metrics.record_write(256);
        metrics.record_compression_time(50);
        metrics.record_encryption_time(30);
    }

    // 2. 正常运行 - 触发几次 flush
    for _ in 0..5 {
        metrics.record_flush();
        metrics.record_io_time(2000);
    }

    // 3. 出现一些错误
    metrics.record_error(ErrorType::BufferFull);
    metrics.record_write_failed();

    // 4. 继续写入
    for _ in 0..50 {
        metrics.record_write(512);
        metrics.record_compression_time(60);
    }

    // 5. 执行清理
    metrics.record_cleanup(3, 10240);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 150);
    assert_eq!(snapshot.bytes_written, 100 * 256 + 50 * 512);
    assert_eq!(snapshot.writes_failed, 1);
    assert_eq!(snapshot.flush_count, 5);
    assert_eq!(snapshot.buffer_full_count, 1);
    assert_eq!(snapshot.cleanup_count, 1);
    assert_eq!(snapshot.files_deleted, 3);
    assert_eq!(snapshot.bytes_freed, 10240);

    // 验证计算
    assert!((snapshot.write_success_rate() - 0.9934).abs() < 0.001); // 150/151
    assert!(snapshot.avg_compression_time_us() > 0.0);
    assert_eq!(snapshot.avg_io_time_us(), 2000.0);
}

#[test]
fn test_metrics_stress_test() {
    let metrics = Arc::new(ScribeMetrics::new());
    let mut handles = vec![];

    // 高并发场景 - 20 个线程同时操作
    for thread_id in 0..20 {
        let metrics_clone = Arc::clone(&metrics);
        let handle = thread::spawn(move || {
            for i in 0..500 {
                match thread_id % 5 {
                    0 => metrics_clone.record_write(i),
                    1 => metrics_clone.record_flush(),
                    2 => metrics_clone.record_error(ErrorType::BufferFull),
                    3 => metrics_clone.record_compression_time(i),
                    4 => metrics_clone.record_cleanup(1, i),
                    _ => unreachable!(),
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let snapshot = metrics.snapshot();

    // 每类操作有 4 个线程，每个执行 500 次 = 2000 次
    assert_eq!(snapshot.writes_count, 2000);
    assert_eq!(snapshot.flush_count, 2000);
    assert_eq!(snapshot.buffer_full_count, 2000);
    assert_eq!(snapshot.cleanup_count, 2000);
}

#[test]
fn test_metrics_record_write_failed() {
    let metrics = ScribeMetrics::new();

    metrics.record_write_failed();
    metrics.record_write_failed();
    metrics.record_write_failed();

    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_failed, 3);
}

#[test]
fn test_metrics_all_error_types() {
    let metrics = ScribeMetrics::new();

    metrics.record_error(ErrorType::BufferFull);
    metrics.record_error(ErrorType::DiskFull);
    metrics.record_error(ErrorType::Compression);
    metrics.record_error(ErrorType::Encryption);
    metrics.record_error(ErrorType::WriteFailed);

    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.buffer_full_count, 1);
    assert_eq!(snapshot.disk_full_count, 1);
    assert_eq!(snapshot.compression_errors, 1);
    assert_eq!(snapshot.encryption_errors, 1);
    assert_eq!(snapshot.writes_failed, 1);
}

#[test]
fn test_metrics_default_trait() {
    let metrics = ScribeMetrics::default();
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 0);
    assert_eq!(snapshot.bytes_written, 0);
}
