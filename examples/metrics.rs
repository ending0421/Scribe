//! Metrics 性能指标使用示例
//!
//! 本示例展示如何使用 Scribe 的性能指标系统：
//! 1. 初始化和收集基本指标
//! 2. 记录不同类型的操作（写入、刷新、压缩等）
//! 3. 获取指标快照并分析
//! 4. 模拟导出到监控系统
//! 5. 定期重置指标
//! 6. 性能基准测试
//!
//! 运行命令：
//! ```bash
//! cargo run --example metrics
//! ```

use scribe::{
    ScribeMetrics, MetricsSnapshot, ErrorType,
    DoubleBufferManager, LogFrame, LogLevel,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scribe Metrics 使用示例 ===\n");

    // ==================== 1. 初始化 Metrics ====================
    println!("1. 初始化性能指标系统...\n");

    let metrics = ScribeMetrics::new();
    println!("  ✓ Metrics 已初始化");
    println!("  初始状态: {:?}\n", metrics.snapshot());

    // ==================== 2. 模拟基本操作 ====================
    println!("2. 模拟基本日志操作并收集指标...\n");

    let log_dir = PathBuf::from("/tmp/scribe_metrics_example");
    std::fs::create_dir_all(&log_dir)?;
    let mut manager = DoubleBufferManager::new(log_dir.clone())?;

    println!("  执行 50 次写入操作:");
    for i in 0..50 {
        let frame = LogFrame::new(
            LogLevel::Info,
            "metrics_test".to_string(),
            format!("测试消息 #{}", i),
        );

        let serialized = frame.serialize()?;
        let data_len = serialized.len() as u64;

        // 模拟写入操作的性能测量
        let start = Instant::now();

        let (buffer, idx) = manager.get_active_buffer();
        manager.increment_active_writers(idx);

        match buffer.write(&serialized) {
            Ok(_) => {
                let elapsed = start.elapsed().as_micros() as u64;

                // 记录成功写入
                metrics.record_write(data_len);
                metrics.record_io_time(elapsed);

                if i % 10 == 0 {
                    println!("    写入 #{}: {} bytes, 耗时 {} μs", i, data_len, elapsed);
                }
            }
            Err(_) => {
                metrics.record_write_failed();
                metrics.record_error(ErrorType::WriteFailed);
            }
        }

        manager.decrement_active_writers(idx);
    }
    println!("  ✓ 完成\n");

    // ==================== 3. 模拟压缩和加密 ====================
    println!("3. 模拟压缩和加密操作...\n");

    println!("  执行 10 次压缩:");
    for i in 0..10 {
        // 模拟压缩时间（实际应该是真实的压缩操作）
        let compression_time = 150 + (i * 10); // 微秒
        metrics.record_compression_time(compression_time);

        if i % 3 == 0 {
            println!("    压缩 #{}: {} μs", i, compression_time);
        }
    }
    println!("  ✓ 完成\n");

    println!("  执行 10 次加密:");
    for i in 0..10 {
        // 模拟加密时间
        let encryption_time = 200 + (i * 15); // 微秒
        metrics.record_encryption_time(encryption_time);

        if i % 3 == 0 {
            println!("    加密 #{}: {} μs", i, encryption_time);
        }
    }
    println!("  ✓ 完成\n");

    // ==================== 4. 模拟刷新操作 ====================
    println!("4. 执行缓冲区刷新...\n");

    for i in 0..5 {
        let (buffer, _) = manager.get_active_buffer();
        buffer.flush()?;
        metrics.record_flush();
        println!("  刷新 #{} 完成", i + 1);
    }
    println!("  ✓ 完成\n");

    // ==================== 5. 获取并分析指标快照 ====================
    println!("5. 获取性能指标快照并分析...\n");

    let snapshot = metrics.snapshot();
    print_detailed_metrics(&snapshot);

    // ==================== 6. 模拟错误场景 ====================
    println!("\n6. 模拟各种错误场景...\n");

    println!("  模拟错误:");
    metrics.record_error(ErrorType::BufferFull);
    metrics.record_error(ErrorType::BufferFull);
    metrics.record_error(ErrorType::DiskFull);
    metrics.record_error(ErrorType::CompressionFailed);
    metrics.record_error(ErrorType::EncryptionFailed);
    println!("    - 缓冲区满: 2 次");
    println!("    - 磁盘满: 1 次");
    println!("    - 压缩失败: 1 次");
    println!("    - 加密失败: 1 次");

    let snapshot = metrics.snapshot();
    println!("\n  更新后的错误统计:");
    println!("    - 缓冲区满: {}", snapshot.buffer_full_count);
    println!("    - 磁盘满: {}", snapshot.disk_full_count);
    println!("    - 压缩错误: {}", snapshot.compression_errors);
    println!("    - 加密错误: {}", snapshot.encryption_errors);
    println!();

    // ==================== 7. 导出到监控系统 ====================
    println!("7. 模拟导出到监控系统...\n");

    export_to_prometheus(&snapshot);
    println!();
    export_to_json(&snapshot);
    println!();

    // ==================== 8. 清理统计 ====================
    println!("\n8. 模拟清理操作...\n");

    metrics.record_cleanup(15, 1024 * 1024 * 5); // 删除 15 个文件，释放 5MB
    println!("  记录清理: 15 个文件, 5 MB");

    let snapshot = metrics.snapshot();
    println!("  清理统计:");
    println!("    - 清理次数: {}", snapshot.cleanup_count);
    println!("    - 删除文件: {}", snapshot.files_deleted);
    println!("    - 释放空间: {} bytes ({:.2} MB)",
        snapshot.bytes_freed,
        snapshot.bytes_freed as f64 / (1024.0 * 1024.0)
    );
    println!();

    // ==================== 9. 定期重置演示 ====================
    println!("9. 演示定期重置指标...\n");

    println!("  当前指标摘要:");
    let before_reset = metrics.snapshot();
    println!("    - 写入次数: {}", before_reset.writes_count);
    println!("    - 字节数: {}", before_reset.bytes_written);
    println!("    - 刷新次数: {}", before_reset.flush_count);

    println!("\n  执行重置...");
    metrics.reset();

    let after_reset = metrics.snapshot();
    println!("\n  重置后的指标:");
    println!("    - 写入次数: {}", after_reset.writes_count);
    println!("    - 字节数: {}", after_reset.bytes_written);
    println!("    - 刷新次数: {}", after_reset.flush_count);
    println!();

    // ==================== 10. 性能基准测试 ====================
    println!("10. 执行性能基准测试...\n");

    benchmark_write_performance(&mut manager, &metrics)?;
    println!();

    println!("=== 示例完成 ===");
    println!("\n提示: 在生产环境中，应该定期收集这些指标并发送到监控系统");
    println!("      建议每 60 秒收集一次快照，然后重置计数器");

    Ok(())
}

/// 打印详细的指标信息
fn print_detailed_metrics(snapshot: &MetricsSnapshot) {
    println!("  ==================== 指标详情 ====================");

    println!("\n  【写入性能】");
    println!("    总写入次数: {}", snapshot.writes_count);
    println!("    失败次数: {}", snapshot.writes_failed);
    println!("    成功率: {:.2}%", snapshot.write_success_rate() * 100.0);
    println!("    总字节数: {} bytes ({:.2} KB)",
        snapshot.bytes_written,
        snapshot.bytes_written as f64 / 1024.0
    );

    if snapshot.writes_count > 0 {
        println!("    平均每次写入: {:.2} bytes",
            snapshot.bytes_written as f64 / snapshot.writes_count as f64
        );
    }

    println!("\n  【缓冲区管理】");
    println!("    刷新次数: {}", snapshot.flush_count);
    println!("    Worker 唤醒: {}", snapshot.worker_wakeups);
    println!("    缓冲区满次数: {}", snapshot.buffer_full_count);

    println!("\n  【性能时间】");
    if snapshot.compression_time_us > 0 {
        println!("    总压缩时间: {} μs ({:.2} ms)",
            snapshot.compression_time_us,
            snapshot.compression_time_us as f64 / 1000.0
        );
        println!("    平均压缩时间: {:.2} μs", snapshot.avg_compression_time_us());
    }

    if snapshot.encryption_time_us > 0 {
        println!("    总加密时间: {} μs ({:.2} ms)",
            snapshot.encryption_time_us,
            snapshot.encryption_time_us as f64 / 1000.0
        );
        println!("    平均加密时间: {:.2} μs", snapshot.avg_encryption_time_us());
    }

    if snapshot.io_time_us > 0 {
        println!("    总 I/O 时间: {} μs ({:.2} ms)",
            snapshot.io_time_us,
            snapshot.io_time_us as f64 / 1000.0
        );
        println!("    平均 I/O 时间: {:.2} μs", snapshot.avg_io_time_us());
    }

    println!("\n  【错误统计】");
    println!("    缓冲区满: {}", snapshot.buffer_full_count);
    println!("    磁盘满: {}", snapshot.disk_full_count);
    println!("    压缩错误: {}", snapshot.compression_errors);
    println!("    加密错误: {}", snapshot.encryption_errors);

    println!("\n  【清理统计】");
    println!("    清理次数: {}", snapshot.cleanup_count);
    println!("    删除文件: {}", snapshot.files_deleted);
    println!("    释放空间: {} bytes ({:.2} KB)",
        snapshot.bytes_freed,
        snapshot.bytes_freed as f64 / 1024.0
    );

    println!("\n  ==================================================");
}

/// 导出为 Prometheus 格式
fn export_to_prometheus(snapshot: &MetricsSnapshot) {
    println!("  Prometheus 格式导出:");
    println!("    ```");
    println!("    # HELP scribe_writes_total Total number of write operations");
    println!("    # TYPE scribe_writes_total counter");
    println!("    scribe_writes_total {}", snapshot.writes_count);
    println!();
    println!("    # HELP scribe_writes_failed_total Total number of failed writes");
    println!("    # TYPE scribe_writes_failed_total counter");
    println!("    scribe_writes_failed_total {}", snapshot.writes_failed);
    println!();
    println!("    # HELP scribe_bytes_written_total Total bytes written");
    println!("    # TYPE scribe_bytes_written_total counter");
    println!("    scribe_bytes_written_total {}", snapshot.bytes_written);
    println!();
    println!("    # HELP scribe_flush_total Total number of flush operations");
    println!("    # TYPE scribe_flush_total counter");
    println!("    scribe_flush_total {}", snapshot.flush_count);
    println!();
    println!("    # HELP scribe_buffer_full_total Number of buffer full errors");
    println!("    # TYPE scribe_buffer_full_total counter");
    println!("    scribe_buffer_full_total {}", snapshot.buffer_full_count);
    println!("    ```");
}

/// 导出为 JSON 格式
fn export_to_json(snapshot: &MetricsSnapshot) {
    println!("  JSON 格式导出:");
    println!("    ```json");
    println!("    {{");
    println!("      \"writes\": {{");
    println!("        \"total\": {},", snapshot.writes_count);
    println!("        \"failed\": {},", snapshot.writes_failed);
    println!("        \"success_rate\": {:.4}", snapshot.write_success_rate());
    println!("      }},");
    println!("      \"bytes\": {{");
    println!("        \"written\": {}", snapshot.bytes_written);
    println!("      }},");
    println!("      \"flush\": {{");
    println!("        \"count\": {}", snapshot.flush_count);
    println!("      }},");
    println!("      \"performance\": {{");
    println!("        \"avg_compression_us\": {:.2},", snapshot.avg_compression_time_us());
    println!("        \"avg_encryption_us\": {:.2},", snapshot.avg_encryption_time_us());
    println!("        \"avg_io_us\": {:.2}", snapshot.avg_io_time_us());
    println!("      }},");
    println!("      \"errors\": {{");
    println!("        \"buffer_full\": {},", snapshot.buffer_full_count);
    println!("        \"disk_full\": {},", snapshot.disk_full_count);
    println!("        \"compression\": {},", snapshot.compression_errors);
    println!("        \"encryption\": {}", snapshot.encryption_errors);
    println!("      }}");
    println!("    }}");
    println!("    ```");
}

/// 性能基准测试
fn benchmark_write_performance(
    manager: &mut DoubleBufferManager,
    metrics: &ScribeMetrics,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("  执行写入性能基准测试:");
    println!("    测试参数: 1000 次写入");

    // 重置指标
    metrics.reset();

    let iterations = 1000;
    let start = Instant::now();

    for i in 0..iterations {
        let frame = LogFrame::new(
            LogLevel::Info,
            "benchmark".to_string(),
            format!("基准测试消息 #{}", i),
        );

        let serialized = frame.serialize()?;
        let data_len = serialized.len() as u64;

        let (buffer, idx) = manager.get_active_buffer();
        manager.increment_active_writers(idx);

        let write_start = Instant::now();
        match buffer.write(&serialized) {
            Ok(_) => {
                let elapsed = write_start.elapsed().as_micros() as u64;
                metrics.record_write(data_len);
                metrics.record_io_time(elapsed);
            }
            Err(_) => {
                metrics.record_write_failed();
            }
        }

        manager.decrement_active_writers(idx);
    }

    let total_elapsed = start.elapsed();
    let snapshot = metrics.snapshot();

    println!("\n  【基准测试结果】");
    println!("    总耗时: {:.2} ms", total_elapsed.as_secs_f64() * 1000.0);
    println!("    总写入: {}", snapshot.writes_count);
    println!("    失败数: {}", snapshot.writes_failed);
    println!("    总字节: {} bytes ({:.2} KB)",
        snapshot.bytes_written,
        snapshot.bytes_written as f64 / 1024.0
    );
    println!("    吞吐量: {:.2} writes/sec",
        iterations as f64 / total_elapsed.as_secs_f64()
    );
    println!("    带宽: {:.2} KB/sec",
        (snapshot.bytes_written as f64 / 1024.0) / total_elapsed.as_secs_f64()
    );

    if snapshot.io_time_us > 0 {
        println!("    平均延迟: {:.2} μs", snapshot.avg_io_time_us());
    }

    Ok(())
}

// ==================== 辅助函数：定期收集指标 ====================

/// 模拟定期收集指标的后台任务
#[allow(dead_code)]
fn periodic_metrics_collection(metrics: &ScribeMetrics, interval: Duration) {
    println!("\n  【定期指标收集任务】");
    println!("    收集间隔: {:?}", interval);
    println!("    这个函数可以在后台线程中运行\n");

    // 在实际应用中，这应该在一个独立的线程中运行
    // 示例代码：
    println!("    示例代码:");
    println!("    ```rust");
    println!("    std::thread::spawn(move || {{");
    println!("        loop {{");
    println!("            std::thread::sleep(Duration::from_secs(60));");
    println!("            let snapshot = metrics.snapshot();");
    println!("            send_to_monitoring_system(&snapshot);");
    println!("            metrics.reset();");
    println!("        }}");
    println!("    }});");
    println!("    ```");
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_basic() {
        let metrics = ScribeMetrics::new();

        metrics.record_write(1000);
        metrics.record_write(2000);
        metrics.record_flush();

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.writes_count, 2);
        assert_eq!(snapshot.bytes_written, 3000);
        assert_eq!(snapshot.flush_count, 1);
    }

    #[test]
    fn test_metrics_reset() {
        let metrics = ScribeMetrics::new();

        metrics.record_write(1000);
        metrics.record_flush();

        let before = metrics.snapshot();
        assert!(before.writes_count > 0);

        metrics.reset();

        let after = metrics.snapshot();
        assert_eq!(after.writes_count, 0);
        assert_eq!(after.bytes_written, 0);
        assert_eq!(after.flush_count, 0);
    }

    #[test]
    fn test_metrics_calculations() {
        let metrics = ScribeMetrics::new();

        metrics.record_write(1000);
        metrics.record_write(2000);
        metrics.record_compression_time(100);
        metrics.record_compression_time(200);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.write_success_rate(), 1.0);
        assert_eq!(snapshot.avg_compression_time_us(), 150.0);
    }

    #[test]
    fn test_error_tracking() {
        let metrics = ScribeMetrics::new();

        metrics.record_error(ErrorType::BufferFull);
        metrics.record_error(ErrorType::BufferFull);
        metrics.record_error(ErrorType::DiskFull);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.buffer_full_count, 2);
        assert_eq!(snapshot.disk_full_count, 1);
    }
}
