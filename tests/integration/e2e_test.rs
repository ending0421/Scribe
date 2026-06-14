use scribe::{LogFrame, LogLevel, ScribeMetrics, ErrorType};
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;
use flate2::read::GzDecoder;
use std::io::Read;

mod common;
use common::{create_test_recovery, compress_data, encrypt_data};

/// 端到端测试：完整的写入 -> 压缩 -> 加密 -> 持久化流程
#[test]
fn test_e2e_write_compress_encrypt_persist() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // 1. 创建日志帧
    let frames = vec![
        LogFrame::new(LogLevel::Info, "app".to_string(), "Application started".to_string()),
        LogFrame::new(LogLevel::Debug, "network".to_string(), "Connecting to server".to_string()),
        LogFrame::new(LogLevel::Warn, "auth".to_string(), "Retry authentication".to_string()),
        LogFrame::new(LogLevel::Error, "db".to_string(), "Connection timeout".to_string()),
    ];

    // 2. 序列化所有帧
    let mut buffer = Vec::new();
    for frame in &frames {
        let serialized = frame.serialize().unwrap();
        buffer.extend_from_slice(&serialized);
        metrics.record_write(serialized.len() as u64);
    }

    // 3. 压缩数据
    let compressed = compress_data(&buffer).unwrap();
    metrics.record_compression_time(150);
    assert!(compressed.len() < buffer.len());

    // 4. 加密数据
    let encrypted = encrypt_data(&compressed).unwrap();
    metrics.record_encryption_time(80);

    // 5. 持久化到磁盘
    let log_file = temp_dir.path().join("app.log.gz.enc");
    let mut file = File::create(&log_file).unwrap();
    file.write_all(&encrypted).unwrap();
    file.sync_all().unwrap();
    metrics.record_io_time(2000);
    metrics.record_flush();

    // 6. 验证 Metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_count, 4);
    assert!(snapshot.bytes_written > 0);
    assert_eq!(snapshot.flush_count, 1);
    assert!(snapshot.compression_time_us > 0);
    assert!(snapshot.encryption_time_us > 0);
    assert!(snapshot.io_time_us > 0);

    // 7. 验证文件已创建
    assert!(log_file.exists());
    assert!(log_file.metadata().unwrap().len() > 0);
}

/// 端到端测试：崩溃恢复 + Metrics 记录
#[test]
fn test_e2e_crash_recovery_with_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());
    let recovery = create_test_recovery(&temp_dir);

    // 1. 创建多个 mmap 文件（模拟崩溃前的状态）
    for i in 0..3 {
        let mmap_path = temp_dir.path().join(format!("buffer_{}.mmap", i));
        let mut file = File::create(&mmap_path).unwrap();

        for j in 0..5 {
            let frame = LogFrame::new(
                LogLevel::Info,
                format!("module{}", i),
                format!("Log message {} from buffer {}", j, i),
            );
            let serialized = frame.serialize().unwrap();
            file.write_all(&serialized).unwrap();
        }
    }

    // 2. 执行恢复
    let report = recovery.recover_all().unwrap();

    // 3. 记录恢复指标
    metrics.record_cleanup(
        report.files_processed as u64,
        report.bytes_recovered,
    );

    // 4. 验证恢复报告
    assert_eq!(report.files_processed, 3);
    assert_eq!(report.frames_recovered, 15); // 3 files * 5 frames
    assert_eq!(report.frames_corrupted, 0);
    assert!(report.bytes_recovered > 0);

    // 5. 验证 Metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.cleanup_count, 1);
    assert_eq!(snapshot.files_deleted, 3);
    assert_eq!(snapshot.bytes_freed, report.bytes_recovered);

    // 6. 验证 mmap 文件已被清理
    let remaining = recovery.scan_mmap_files().unwrap();
    assert_eq!(remaining.len(), 0);
}

/// 端到端测试：存储清理 + 保留策略
#[test]
fn test_e2e_storage_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // 1. 创建多个日志文件（模拟不同时间的日志）
    let log_files = vec![
        ("old_debug.log", LogLevel::Debug, 1024),
        ("old_info.log", LogLevel::Info, 2048),
        ("old_warn.log", LogLevel::Warn, 4096),
        ("old_error.log", LogLevel::Error, 8192),
        ("recent.log", LogLevel::Info, 1024),
    ];

    let mut total_size = 0u64;
    for (filename, level, size) in log_files {
        let file_path = temp_dir.path().join(filename);
        let mut file = File::create(&file_path).unwrap();
        let data = vec![0u8; size];
        file.write_all(&data).unwrap();
        total_size += size as u64;
    }

    // 2. 执行清理（模拟删除旧文件）
    let files_to_delete = vec![
        "old_debug.log",
        "old_info.log",
    ];

    let mut deleted_count = 0u64;
    let mut freed_bytes = 0u64;

    for filename in files_to_delete {
        let file_path = temp_dir.path().join(filename);
        if file_path.exists() {
            let size = file_path.metadata().unwrap().len();
            std::fs::remove_file(&file_path).unwrap();
            deleted_count += 1;
            freed_bytes += size;
        }
    }

    // 3. 记录清理指标
    metrics.record_cleanup(deleted_count, freed_bytes);

    // 4. 验证清理结果
    assert!(!temp_dir.path().join("old_debug.log").exists());
    assert!(!temp_dir.path().join("old_info.log").exists());
    assert!(temp_dir.path().join("old_warn.log").exists());
    assert!(temp_dir.path().join("old_error.log").exists());
    assert!(temp_dir.path().join("recent.log").exists());

    // 5. 验证 Metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.cleanup_count, 1);
    assert_eq!(snapshot.files_deleted, 2);
    assert_eq!(snapshot.bytes_freed, 1024 + 2048);
}

/// 端到端测试：多线程写入 + Metrics 准确性
#[test]
fn test_e2e_multithreaded_writes_with_metrics() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());
    let mut handles = vec![];

    // 1. 启动 5 个线程并发写入
    for thread_id in 0..5 {
        let metrics_clone = Arc::clone(&metrics);
        let temp_path = temp_dir.path().to_path_buf();

        let handle = thread::spawn(move || {
            let mut buffer = Vec::new();

            for i in 0..20 {
                let frame = LogFrame::new(
                    LogLevel::Info,
                    format!("thread{}", thread_id),
                    format!("Message {} from thread {}", i, thread_id),
                );

                let serialized = frame.serialize().unwrap();
                buffer.extend_from_slice(&serialized);
                metrics_clone.record_write(serialized.len() as u64);
            }

            // 压缩和写入
            let compressed = compress_data(&buffer).unwrap();
            metrics_clone.record_compression_time(100 + thread_id * 10);

            let log_file = temp_path.join(format!("thread_{}.log.gz", thread_id));
            let mut file = File::create(&log_file).unwrap();
            file.write_all(&compressed).unwrap();
            file.sync_all().unwrap();

            metrics_clone.record_io_time(1000 + thread_id * 100);
            metrics_clone.record_flush();
        });

        handles.push(handle);
    }

    // 2. 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }

    // 3. 验证 Metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_count, 100); // 5 threads * 20 messages
    assert!(snapshot.bytes_written > 0);
    assert_eq!(snapshot.flush_count, 5);
    assert!(snapshot.compression_time_us > 0);
    assert!(snapshot.io_time_us > 0);

    // 4. 验证所有文件已创建
    for i in 0..5 {
        let log_file = temp_dir.path().join(format!("thread_{}.log.gz", i));
        assert!(log_file.exists());
    }
}

/// 端到端测试：错误处理 + Metrics 记录
#[test]
fn test_e2e_error_handling_with_metrics() {
    let metrics = Arc::new(ScribeMetrics::new());

    // 1. 模拟各种错误场景
    // Buffer 满错误
    for _ in 0..3 {
        metrics.record_error(ErrorType::BufferFull);
        metrics.record_write_failed();
    }

    // 压缩错误
    for _ in 0..2 {
        metrics.record_error(ErrorType::Compression);
        metrics.record_write_failed();
    }

    // 加密错误
    metrics.record_error(ErrorType::Encryption);
    metrics.record_write_failed();

    // 磁盘满错误
    metrics.record_error(ErrorType::DiskFull);
    metrics.record_write_failed();

    // 2. 一些成功的写入
    for _ in 0..10 {
        metrics.record_write(512);
    }

    // 3. 验证 Metrics
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 10);
    assert_eq!(snapshot.writes_failed, 7);
    assert_eq!(snapshot.buffer_full_count, 3);
    assert_eq!(snapshot.compression_errors, 2);
    assert_eq!(snapshot.encryption_errors, 1);
    assert_eq!(snapshot.disk_full_count, 1);

    // 4. 验证成功率计算
    let success_rate = snapshot.write_success_rate();
    assert!((success_rate - 0.5882).abs() < 0.001); // 10 / 17
}

/// 端到端测试：完整工作流程（写入 -> 轮转 -> 压缩 -> 恢复）
#[test]
fn test_e2e_complete_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // === 阶段 1: 写入日志 ===
    let mmap_path = temp_dir.path().join("active.mmap");
    let mut mmap_file = File::create(&mmap_path).unwrap();

    let write_frames = vec![
        LogFrame::new(LogLevel::Info, "startup".to_string(), "System initializing".to_string()),
        LogFrame::new(LogLevel::Debug, "config".to_string(), "Loading configuration".to_string()),
        LogFrame::new(LogLevel::Info, "network".to_string(), "Server started on port 8080".to_string()),
        LogFrame::new(LogLevel::Warn, "memory".to_string(), "Memory usage at 75%".to_string()),
        LogFrame::new(LogLevel::Error, "database".to_string(), "Query timeout".to_string()),
    ];

    for frame in &write_frames {
        let serialized = frame.serialize().unwrap();
        mmap_file.write_all(&serialized).unwrap();
        metrics.record_write(serialized.len() as u64);
    }
    mmap_file.sync_all().unwrap();
    drop(mmap_file);

    // === 阶段 2: 模拟崩溃和恢复 ===
    let recovery = create_test_recovery(&temp_dir);
    let report = recovery.recover_all().unwrap();

    assert_eq!(report.files_processed, 1);
    assert_eq!(report.frames_recovered, 5);
    assert!(report.bytes_recovered > 0);

    metrics.record_cleanup(
        report.files_processed as u64,
        report.bytes_recovered,
    );

    // === 阶段 3: 验证恢复的数据 ===
    let recovered_files: Vec<_> = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("recovered_")
        })
        .collect();

    assert_eq!(recovered_files.len(), 1);

    // 解压并验证内容
    let recovered_file = File::open(recovered_files[0].path()).unwrap();
    let mut decoder = GzDecoder::new(recovered_file);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).unwrap();

    // 验证可以反序列化恢复的帧
    let mut offset = 0;
    let mut recovered_count = 0;
    while offset < decompressed.len() {
        if let Ok(frame) = LogFrame::deserialize(&decompressed[offset..]) {
            recovered_count += 1;
            let serialized = frame.serialize().unwrap();
            offset += serialized.len();
        } else {
            break;
        }
    }

    assert_eq!(recovered_count, 5);

    // === 阶段 4: 验证最终 Metrics ===
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 5);
    assert!(snapshot.bytes_written > 0);
    assert_eq!(snapshot.cleanup_count, 1);
    assert_eq!(snapshot.files_deleted, 1);
    assert_eq!(snapshot.bytes_freed, report.bytes_recovered);
}

/// 端到端测试：压力测试 - 大量数据写入和恢复
#[test]
fn test_e2e_stress_test() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // 1. 写入大量日志
    let mmap_path = temp_dir.path().join("stress.mmap");
    let mut mmap_file = File::create(&mmap_path).unwrap();

    let levels = vec![
        LogLevel::Verbose,
        LogLevel::Debug,
        LogLevel::Info,
        LogLevel::Warn,
        LogLevel::Error,
    ];

    for i in 0..1000 {
        let level = levels[i % levels.len()];
        let frame = LogFrame::new(
            level,
            format!("module_{}", i % 10),
            format!("Log message number {} with some content", i),
        );

        let serialized = frame.serialize().unwrap();
        mmap_file.write_all(&serialized).unwrap();
        metrics.record_write(serialized.len() as u64);
    }

    drop(mmap_file);

    // 2. 恢复
    let recovery = create_test_recovery(&temp_dir);
    let report = recovery.recover_all().unwrap();

    assert_eq!(report.frames_recovered, 1000);
    assert_eq!(report.frames_corrupted, 0);

    // 3. 验证 Metrics
    let snapshot = metrics.snapshot();
    assert_eq!(snapshot.writes_count, 1000);
    assert!(snapshot.bytes_written > 50000); // 至少 50KB
}

/// 端到端测试：Metrics 重置后的新周期
#[test]
fn test_e2e_metrics_reset_cycle() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // === 周期 1 ===
    for _ in 0..10 {
        metrics.record_write(1000);
    }
    metrics.record_flush();

    let snapshot1 = metrics.snapshot();
    assert_eq!(snapshot1.writes_count, 10);
    assert_eq!(snapshot1.flush_count, 1);

    // === 重置 ===
    metrics.reset();

    let snapshot_reset = metrics.snapshot();
    assert_eq!(snapshot_reset.writes_count, 0);
    assert_eq!(snapshot_reset.flush_count, 0);

    // === 周期 2 ===
    for _ in 0..5 {
        metrics.record_write(500);
    }
    metrics.record_flush();
    metrics.record_flush();

    let snapshot2 = metrics.snapshot();
    assert_eq!(snapshot2.writes_count, 5);
    assert_eq!(snapshot2.flush_count, 2);
}

/// 端到端测试：真实场景模拟
#[test]
fn test_e2e_realistic_application_scenario() {
    let temp_dir = TempDir::new().unwrap();
    let metrics = Arc::new(ScribeMetrics::new());

    // === 应用启动阶段 ===
    let startup_logs = vec![
        LogFrame::new(LogLevel::Info, "main".to_string(), "Application v1.0.0 starting".to_string()),
        LogFrame::new(LogLevel::Debug, "config".to_string(), "Reading config.json".to_string()),
        LogFrame::new(LogLevel::Info, "database".to_string(), "Connected to PostgreSQL".to_string()),
        LogFrame::new(LogLevel::Info, "server".to_string(), "HTTP server listening on 0.0.0.0:8080".to_string()),
    ];

    let mut buffer = Vec::new();
    for frame in startup_logs {
        let serialized = frame.serialize().unwrap();
        buffer.extend_from_slice(&serialized);
        metrics.record_write(serialized.len() as u64);
    }

    // === 正常运行阶段 ===
    for i in 0..50 {
        let frame = LogFrame::new(
            LogLevel::Debug,
            "api".to_string(),
            format!("Processing request {}", i),
        );
        let serialized = frame.serialize().unwrap();
        buffer.extend_from_slice(&serialized);
        metrics.record_write(serialized.len() as u64);
    }

    // 定期 flush
    for _ in 0..3 {
        metrics.record_flush();
        metrics.record_io_time(1500);
    }

    // === 出现一些警告 ===
    let warnings = vec![
        LogFrame::new(LogLevel::Warn, "memory".to_string(), "Memory usage high: 85%".to_string()),
        LogFrame::new(LogLevel::Warn, "cache".to_string(), "Cache hit rate low: 45%".to_string()),
    ];

    for frame in warnings {
        let serialized = frame.serialize().unwrap();
        buffer.extend_from_slice(&serialized);
        metrics.record_write(serialized.len() as u64);
    }

    // === 出现错误 ===
    metrics.record_error(ErrorType::BufferFull);
    metrics.record_write_failed();

    let error_frame = LogFrame::new(
        LogLevel::Error,
        "database".to_string(),
        "Connection lost, retrying...".to_string(),
    );
    let serialized = error_frame.serialize().unwrap();
    buffer.extend_from_slice(&serialized);
    metrics.record_write(serialized.len() as u64);

    // === 压缩和持久化 ===
    let compressed = compress_data(&buffer).unwrap();
    metrics.record_compression_time(250);

    let log_file = temp_dir.path().join("app.log.gz");
    let mut file = File::create(&log_file).unwrap();
    file.write_all(&compressed).unwrap();
    metrics.record_io_time(3000);
    metrics.record_flush();

    // === 验证最终状态 ===
    let snapshot = metrics.snapshot();

    assert_eq!(snapshot.writes_count, 57); // 4 + 50 + 2 + 1
    assert_eq!(snapshot.writes_failed, 1);
    assert_eq!(snapshot.flush_count, 4);
    assert_eq!(snapshot.buffer_full_count, 1);
    assert!(snapshot.bytes_written > 0);
    assert!(snapshot.compression_time_us > 0);
    assert!(snapshot.io_time_us > 0);

    // 成功率应该非常高
    assert!(snapshot.write_success_rate() > 0.98);

    // 验证文件存在
    assert!(log_file.exists());
    assert!(log_file.metadata().unwrap().len() > 0);
}
