//! 崩溃恢复示例
//!
//! 本示例演示了 Scribe 的崩溃恢复能力：
//! 1. 写入一些日志数据
//! 2. 模拟程序崩溃（不执行 flush）
//! 3. 重新启动并恢复数据
//! 4. 验证数据完整性
//!
//! 运行命令：
//! ```bash
//! cargo run --example recovery
//! ```

use scribe::{
    DoubleBufferManager, LogFrame, LogLevel,
    Recovery, RecoveryReport,
};
use std::path::PathBuf;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scribe 崩溃恢复示例 ===\n");

    let log_dir = PathBuf::from("/tmp/scribe_recovery_example");
    std::fs::create_dir_all(&log_dir)?;

    println!("日志目录: {:?}\n", log_dir);

    // 询问用户要执行的操作
    println!("请选择操作:");
    println!("1. 写入日志并模拟崩溃");
    println!("2. 恢复崩溃前的数据");
    println!("3. 完整流程（写入 -> 崩溃 -> 恢复）");
    print!("\n请输入选项 (1-3): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim();

    match choice {
        "1" => simulate_crash(&log_dir)?,
        "2" => recover_data(&log_dir)?,
        "3" => {
            simulate_crash(&log_dir)?;
            println!("\n--- 模拟程序重启 ---\n");
            std::thread::sleep(std::time::Duration::from_secs(1));
            recover_data(&log_dir)?;
        }
        _ => {
            println!("无效选项，默认执行完整流程");
            simulate_crash(&log_dir)?;
            println!("\n--- 模拟程序重启 ---\n");
            std::thread::sleep(std::time::Duration::from_secs(1));
            recover_data(&log_dir)?;
        }
    }

    println!("\n=== 示例完成 ===");
    Ok(())
}

/// 模拟写入日志并崩溃
fn simulate_crash(log_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 阶段 1: 写入日志并模拟崩溃 ===\n");

    println!("1. 初始化缓冲区管理器...");
    let mut manager = DoubleBufferManager::new(log_dir.clone())?;
    println!("  ✓ 初始化完成\n");

    println!("2. 写入日志数据...");
    let log_messages = vec![
        ("app", "应用程序启动", LogLevel::Info),
        ("network", "连接到服务器 192.168.1.100:8080", LogLevel::Info),
        ("database", "数据库连接池初始化完成", LogLevel::Debug),
        ("auth", "用户登录: user123", LogLevel::Info),
        ("api", "处理 API 请求: GET /api/users", LogLevel::Debug),
        ("cache", "缓存命中率: 87.5%", LogLevel::Verbose),
        ("task", "后台任务开始执行", LogLevel::Info),
        ("network", "网络延迟检测: 45ms", LogLevel::Debug),
        ("security", "检测到可疑活动", LogLevel::Warn),
        ("performance", "响应时间: 123ms", LogLevel::Verbose),
    ];

    for (i, (tag, message, level)) in log_messages.iter().enumerate() {
        let frame = LogFrame::new(
            *level,
            tag.to_string(),
            format!("[{}] {}", i + 1, message),
        );

        let serialized = frame.serialize()?;
        let (buffer, idx) = manager.get_active_buffer();

        manager.increment_active_writers(idx);
        buffer.write(&serialized)?;
        manager.decrement_active_writers(idx);

        println!("  [{:?}] {}: {}", level, tag, message);
    }

    println!("\n  ✓ 成功写入 {} 条日志\n", log_messages.len());

    println!("3. 模拟程序崩溃...");
    println!("  ! 注意: 故意不调用 flush()");
    println!("  ! 数据仍在内存映射缓冲区中，但未完整同步到磁盘");
    println!("  ✓ 程序「崩溃」（正常退出，但模拟未清理）\n");

    // 重要：这里故意不调用 buffer.flush()
    // 这模拟了程序崩溃时的情况
    // 但是由于使用了 mmap，数据已经在操作系统的页缓存中

    Ok(())
}

/// 恢复崩溃前的数据
fn recover_data(log_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== 阶段 2: 恢复崩溃前的数据 ===\n");

    println!("1. 检查日志目录...");

    // 列出所有日志文件
    let mut log_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(log_dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("mmap") {
                if let Ok(metadata) = entry.metadata() {
                    log_files.push((path.clone(), metadata.len()));
                    println!("  - 发现日志文件: {} ({} bytes)",
                        path.file_name().unwrap().to_string_lossy(),
                        metadata.len()
                    );
                }
            }
        }
    }

    if log_files.is_empty() {
        println!("  ! 未找到日志文件，请先运行写入操作");
        return Ok(());
    }

    println!("  ✓ 找到 {} 个日志文件\n", log_files.len());

    println!("2. 执行数据恢复...");

    let mut total_recovered = 0;
    let mut total_corrupted = 0;
    let mut total_bytes = 0;

    for (file_path, size) in log_files {
        println!("\n  处理文件: {}", file_path.file_name().unwrap().to_string_lossy());

        match Recovery::scan_file(&file_path) {
            Ok(report) => {
                println!("    - 扫描范围: {} bytes", report.bytes_scanned);
                println!("    - 恢复成功: {} 条日志", report.frames_recovered);
                println!("    - 损坏数据: {} 条", report.frames_corrupted);

                if report.frames_corrupted > 0 {
                    println!("    ⚠ 警告: 检测到 {} 条损坏的日志", report.frames_corrupted);
                }

                total_recovered += report.frames_recovered;
                total_corrupted += report.frames_corrupted;
                total_bytes += report.bytes_scanned;

                // 显示恢复的日志内容（前 5 条）
                if !report.recovered_frames.is_empty() {
                    println!("\n    恢复的日志内容（前 5 条）:");
                    for (i, frame) in report.recovered_frames.iter().take(5).enumerate() {
                        println!("      {}. [{:?}] [{}] {}",
                            i + 1,
                            frame.level,
                            frame.tag,
                            frame.message
                        );
                    }

                    if report.recovered_frames.len() > 5 {
                        println!("      ... 还有 {} 条日志",
                            report.recovered_frames.len() - 5
                        );
                    }
                }
            }
            Err(e) => {
                println!("    ✗ 恢复失败: {}", e);
            }
        }
    }

    println!("\n3. 恢复统计信息:");
    println!("  总扫描字节数: {} bytes ({:.2} KB)", total_bytes, total_bytes as f64 / 1024.0);
    println!("  成功恢复: {} 条日志", total_recovered);
    println!("  损坏数据: {} 条", total_corrupted);

    if total_corrupted == 0 {
        println!("\n  ✓ 所有数据完整恢复，无损坏！");
    } else {
        let recovery_rate = (total_recovered as f64 / (total_recovered + total_corrupted) as f64) * 100.0;
        println!("\n  恢复率: {:.2}%", recovery_rate);
    }

    println!("\n4. 重新初始化缓冲区管理器...");
    let manager = DoubleBufferManager::new(log_dir.clone())?;
    println!("  ✓ 系统可以继续写入新日志\n");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_recovery_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        // 模拟崩溃
        simulate_crash(&log_dir).unwrap();

        // 恢复数据
        recover_data(&log_dir).unwrap();
    }

    #[test]
    fn test_recovery_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let log_dir = temp_dir.path().to_path_buf();

        // 直接恢复空目录
        let result = recover_data(&log_dir);
        assert!(result.is_ok());
    }
}
