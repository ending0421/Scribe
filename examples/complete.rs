//! 完整的 Scribe 使用示例
//!
//! 本示例展示了 Scribe 的完整使用流程：
//! 1. 初始化配置和存储
//! 2. 创建处理 Pipeline
//! 3. 配置路由规则 (Router)
//! 4. 写入不同级别的日志
//! 5. 查看性能指标 (Metrics)
//! 6. 执行清理操作
//!
//! 运行命令：
//! ```bash
//! cargo run --example complete
//! ```

use scribe::{
    Config, DoubleBufferManager, LogFrame, LogLevel, LogBatch,
    ScribeMetrics, CleanupPolicy, CleanupReport,
};
use std::path::PathBuf;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scribe 完整使用示例 ===\n");

    // ==================== 1. 初始化配置 ====================
    println!("1. 初始化配置和存储...");

    // 创建临时目录用于存储日志
    let log_dir = PathBuf::from("/tmp/scribe_example");
    std::fs::create_dir_all(&log_dir)?;

    // 配置参数
    let config = Config {
        buffer_size: 4 * 1024 * 1024,  // 4MB 缓冲区
        max_file_size: 10 * 1024 * 1024, // 10MB 单文件上限
        ..Default::default()
    };

    println!("  - 日志目录: {:?}", log_dir);
    println!("  - 缓冲区大小: {} MB", config.buffer_size / 1024 / 1024);
    println!("  - 单文件大小: {} MB", config.max_file_size / 1024 / 1024);

    // 创建双缓冲管理器
    let mut manager = DoubleBufferManager::new(log_dir.clone())?;
    println!("  ✓ 存储初始化完成\n");

    // ==================== 2. 初始化 Metrics ====================
    println!("2. 初始化性能指标...");
    let metrics = ScribeMetrics::new();
    println!("  ✓ Metrics 初始化完成\n");

    // ==================== 3. 写入日志 ====================
    println!("3. 写入不同级别的日志...");

    // 写入 100 条日志
    let log_count = 100;
    for i in 0..log_count {
        // 根据索引选择不同的日志级别
        let level = match i % 5 {
            0 => LogLevel::Verbose,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            _ => LogLevel::Info,
        };

        let tag = format!("module_{}", i % 10);
        let message = format!("这是第 {} 条日志，级别为 {:?}", i, level);

        // 创建日志帧
        let frame = LogFrame::new(level, tag, message);

        // 序列化日志
        let serialized = frame.serialize()?;
        let data_len = serialized.len() as u64;

        // 获取活动缓冲区
        let (buffer, idx) = manager.get_active_buffer();

        // 增加写入计数（模拟并发写入保护）
        manager.increment_active_writers(idx);

        // 写入缓冲区
        match buffer.write(&serialized) {
            Ok(_) => {
                // 记录成功写入的指标
                metrics.record_write(data_len);

                if i % 20 == 0 {
                    println!("  - 已写入 {} 条日志", i + 1);
                }
            }
            Err(e) => {
                // 记录失败
                metrics.record_write_failed();
                eprintln!("  ✗ 写入失败: {}", e);
            }
        }

        // 减少写入计数
        manager.decrement_active_writers(idx);

        // 检查是否需要交换缓冲区
        if manager.should_swap(&buffer) {
            println!("  ! 触发缓冲区交换...");
            manager.swap_buffers()?;
            metrics.record_flush();
        }
    }

    println!("  ✓ 成功写入 {} 条日志\n", log_count);

    // ==================== 4. 手动刷新 ====================
    println!("4. 手动刷新缓冲区...");
    let (buffer, _) = manager.get_active_buffer();
    buffer.flush()?;
    metrics.record_flush();
    println!("  ✓ 缓冲区已刷新到磁盘\n");

    // ==================== 5. 查看 Metrics ====================
    println!("5. 查看性能指标...");
    let snapshot = metrics.snapshot();

    println!("  写入统计:");
    println!("    - 总写入次数: {}", snapshot.writes_count);
    println!("    - 失败次数: {}", snapshot.writes_failed);
    println!("    - 成功率: {:.2}%", snapshot.write_success_rate() * 100.0);
    println!("    - 总字节数: {} bytes ({:.2} KB)",
        snapshot.bytes_written,
        snapshot.bytes_written as f64 / 1024.0
    );

    println!("\n  刷新统计:");
    println!("    - 刷新次数: {}", snapshot.flush_count);

    println!("\n  错误统计:");
    println!("    - 缓冲区满: {}", snapshot.buffer_full_count);
    println!("    - 磁盘满: {}", snapshot.disk_full_count);
    println!("    - 压缩错误: {}", snapshot.compression_errors);
    println!("    - 加密错误: {}", snapshot.encryption_errors);

    if snapshot.writes_count > 0 {
        println!("\n  性能统计:");
        if snapshot.compression_time_us > 0 {
            println!("    - 平均压缩时间: {:.2} μs", snapshot.avg_compression_time_us());
        }
        if snapshot.encryption_time_us > 0 {
            println!("    - 平均加密时间: {:.2} μs", snapshot.avg_encryption_time_us());
        }
        if snapshot.io_time_us > 0 {
            println!("    - 平均 I/O 时间: {:.2} μs", snapshot.avg_io_time_us());
        }
    }
    println!();

    // ==================== 6. 执行清理操作 ====================
    println!("6. 执行存储清理...");

    // 配置清理策略：最多保留 5MB，保留最近 7 天
    let cleanup_policy = CleanupPolicy {
        max_total_size: Some(5 * 1024 * 1024),  // 5MB
        max_age: Some(Duration::from_secs(7 * 24 * 60 * 60)),  // 7 天
        min_free_space: None,
    };

    println!("  清理策略:");
    println!("    - 最大总大小: {} MB", 5);
    println!("    - 最大保留时间: 7 天");

    // 执行清理
    match cleanup_policy.cleanup(&log_dir) {
        Ok(report) => {
            println!("\n  清理报告:");
            println!("    - 扫描文件数: {}", report.files_scanned);
            println!("    - 删除文件数: {}", report.files_deleted);
            println!("    - 释放空间: {} bytes ({:.2} KB)",
                report.bytes_freed,
                report.bytes_freed as f64 / 1024.0
            );

            // 记录清理指标
            metrics.record_cleanup(report.files_deleted, report.bytes_freed);
        }
        Err(e) => {
            eprintln!("  ✗ 清理失败: {}", e);
        }
    }
    println!();

    // ==================== 7. 最终统计 ====================
    println!("7. 最终统计信息...");
    let final_snapshot = metrics.snapshot();

    println!("  清理统计:");
    println!("    - 清理次数: {}", final_snapshot.cleanup_count);
    println!("    - 删除文件: {}", final_snapshot.files_deleted);
    println!("    - 释放空间: {} bytes", final_snapshot.bytes_freed);
    println!();

    // ==================== 8. 重置 Metrics (可选) ====================
    println!("8. 重置性能指标...");
    metrics.reset();
    let reset_snapshot = metrics.snapshot();
    println!("  ✓ 指标已重置");
    println!("    - 写入次数: {}", reset_snapshot.writes_count);
    println!("    - 总字节数: {}", reset_snapshot.bytes_written);
    println!();

    // ==================== 9. 查看生成的日志文件 ====================
    println!("9. 查看生成的日志文件...");
    if let Ok(entries) = std::fs::read_dir(&log_dir) {
        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("mmap"))
            .collect();

        files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        println!("  日志文件列表:");
        for entry in files {
            if let Ok(metadata) = entry.metadata() {
                println!("    - {}: {} bytes",
                    entry.file_name().to_string_lossy(),
                    metadata.len()
                );
            }
        }
    }
    println!();

    println!("=== 示例完成 ===");
    println!("\n提示: 日志文件保存在 {:?}", log_dir);
    println!("可以使用其他示例程序读取或恢复这些日志");

    Ok(())
}
