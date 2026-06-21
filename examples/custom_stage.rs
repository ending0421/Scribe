//! 自定义 Pipeline Stage 示例
//!
//! 本示例展示如何实现和使用自定义的 Pipeline Stage：
//! 1. 实现一个日志过滤 Stage（按级别过滤）
//! 2. 实现一个内容转换 Stage（替换敏感信息）
//! 3. 实现一个统计 Stage（计算日志统计）
//! 4. 将自定义 Stage 集成到 Pipeline
//! 5. 演示错误处理和 Fallback 策略
//!
//! 运行命令：
//! ```bash
//! cargo run --example custom_stage
//! ```

use scribe::{
    PipelineStage, Pipeline, LogBatch, Result, ScribeError,
    LogFrame, LogLevel,
};
use scribe::pipeline::stage::Fallback;
use std::sync::Arc;
use parking_lot::Mutex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 自定义 Pipeline Stage 示例 ===\n");

    // ==================== 1. 创建自定义 Stages ====================
    println!("1. 创建自定义 Pipeline Stages...\n");

    // 日志过滤器 - 只保留 Warn 及以上级别
    let filter_stage = LogFilterStage {
        min_level: LogLevel::Warn,
    };
    println!("  ✓ 创建日志过滤器: 最低级别 = Warn");

    // 敏感信息脱敏器
    let sanitizer_stage = SensitiveDataSanitizerStage {
        patterns: vec![
            ("password", "***PASSWORD***"),
            ("token", "***TOKEN***"),
            ("secret", "***SECRET***"),
            ("credit_card", "***CARD***"),
        ],
    };
    println!("  ✓ 创建敏感信息脱敏器");

    // 统计收集器
    let stats_collector = Arc::new(Mutex::new(LogStatistics::default()));
    let stats_stage = StatisticsStage {
        stats: stats_collector.clone(),
    };
    println!("  ✓ 创建统计收集器");

    // 可能失败的 Stage（用于演示错误处理）
    let failing_stage = FlakyStage {
        fail_probability: 0.0, // 0% 失败率，稍后我们会测试失败情况
    };
    println!("  ✓ 创建测试 Stage（用于错误处理演示）\n");

    // ==================== 2. 构建 Pipeline ====================
    println!("2. 构建处理 Pipeline...\n");

    let pipeline = Pipeline::new()
        .add_stage(Box::new(filter_stage))          // Stage 1: 过滤
        .add_stage(Box::new(sanitizer_stage))       // Stage 2: 脱敏
        .add_stage(Box::new(stats_stage))           // Stage 3: 统计
        .add_stage(Box::new(failing_stage));        // Stage 4: 测试

    println!("  Pipeline 构建完成:");
    println!("    过滤器 -> 脱敏器 -> 统计器 -> 测试Stage\n");

    // ==================== 3. 准备测试数据 ====================
    println!("3. 准备测试日志数据...\n");

    let test_logs = vec![
        LogFrame::new(LogLevel::Debug, "app".to_string(), "调试信息".to_string()),
        LogFrame::new(LogLevel::Info, "auth".to_string(), "用户登录成功".to_string()),
        LogFrame::new(LogLevel::Warn, "security".to_string(), "检测到异常访问".to_string()),
        LogFrame::new(LogLevel::Error, "payment".to_string(), "支付失败 credit_card=1234-5678".to_string()),
        LogFrame::new(LogLevel::Error, "api".to_string(), "API 调用失败 token=abc123xyz".to_string()),
        LogFrame::new(LogLevel::Warn, "database".to_string(), "数据库连接慢".to_string()),
        LogFrame::new(LogLevel::Error, "auth".to_string(), "登录失败 password=secret123".to_string()),
    ];

    println!("  准备了 {} 条测试日志:", test_logs.len());
    for (i, log) in test_logs.iter().enumerate() {
        println!("    {}. [{:?}] [{}] {}", i + 1, log.level, log.tag, log.message);
    }
    println!();

    // ==================== 4. 处理日志 ====================
    println!("4. 通过 Pipeline 处理日志...\n");

    for (i, log) in test_logs.iter().enumerate() {
        println!("  处理日志 #{}", i + 1);

        // 序列化日志
        let serialized = log.serialize()?;
        let batch = LogBatch::new(serialized);

        // 通过 Pipeline 处理
        match pipeline.process(batch) {
            Ok(result) => {
                if result.size() > 0 {
                    // 尝试反序列化查看处理结果
                    if let Ok(processed_frame) = LogFrame::deserialize(&result.data) {
                        println!("    ✓ 处理成功:");
                        println!("      原始: [{:?}] {}", log.level, log.message);
                        println!("      处理后: [{:?}] {}", processed_frame.level, processed_frame.message);
                    }
                } else {
                    println!("    ⊘ 被过滤器丢弃（级别低于 Warn）");
                }
            }
            Err(e) => {
                println!("    ✗ 处理失败: {}", e);
            }
        }
        println!();
    }

    // ==================== 5. 显示统计信息 ====================
    println!("5. 显示处理统计...\n");

    let stats = stats_collector.lock();
    println!("  统计信息:");
    println!("    - 总处理数: {}", stats.total_processed);
    println!("    - Verbose: {}", stats.by_level[&LogLevel::Verbose]);
    println!("    - Debug: {}", stats.by_level[&LogLevel::Debug]);
    println!("    - Info: {}", stats.by_level[&LogLevel::Info]);
    println!("    - Warn: {}", stats.by_level[&LogLevel::Warn]);
    println!("    - Error: {}", stats.by_level[&LogLevel::Error]);
    println!("    - 总字节数: {} bytes", stats.total_bytes);
    println!();

    // ==================== 6. 测试错误处理 ====================
    println!("6. 测试错误处理机制...\n");

    // 创建一个会失败的 Pipeline
    let failing_pipeline = Pipeline::new()
        .add_stage(Box::new(FlakyStage { fail_probability: 1.0 })); // 100% 失败

    println!("  测试 Skip Fallback 策略:");
    let test_log = LogFrame::new(LogLevel::Error, "test".to_string(), "测试".to_string());
    let batch = LogBatch::new(test_log.serialize()?);

    match failing_pipeline.process(batch) {
        Ok(_) => {
            println!("    ✓ Pipeline 完成（Stage 失败但被跳过）");
        }
        Err(e) => {
            println!("    ✗ Pipeline 中止: {}", e);
        }
    }
    println!();

    // 测试 Abort 策略
    println!("  测试 Abort Fallback 策略:");
    let abort_pipeline = Pipeline::new()
        .add_stage(Box::new(AbortOnErrorStage));

    let test_log = LogFrame::new(LogLevel::Error, "test".to_string(), "测试".to_string());
    let batch = LogBatch::new(test_log.serialize()?);

    match abort_pipeline.process(batch) {
        Ok(_) => {
            println!("    ✓ Pipeline 完成");
        }
        Err(e) => {
            println!("    ✗ Pipeline 中止（预期行为）: {}", e);
        }
    }
    println!();

    println!("=== 示例完成 ===");
    Ok(())
}

// ==================== 自定义 Stage 实现 ====================

/// 日志过滤器 - 根据日志级别过滤
struct LogFilterStage {
    min_level: LogLevel,
}

impl PipelineStage for LogFilterStage {
    fn name(&self) -> &str {
        "log_filter"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        // 反序列化日志
        let frame = LogFrame::deserialize(&data.data)?;

        // 检查日志级别
        if frame.level >= self.min_level {
            // 级别足够，保留日志
            Ok(data)
        } else {
            // 级别太低，返回空批次
            Ok(LogBatch::empty())
        }
    }

    fn on_error(&self, data: LogBatch, _error: ScribeError) -> Fallback {
        // 过滤器失败时跳过，保留原始数据
        Fallback::Skip
    }
}

/// 敏感数据脱敏器 - 替换敏感信息
struct SensitiveDataSanitizerStage {
    patterns: Vec<(&'static str, &'static str)>,
}

impl PipelineStage for SensitiveDataSanitizerStage {
    fn name(&self) -> &str {
        "sensitive_data_sanitizer"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        // 反序列化日志
        let mut frame = LogFrame::deserialize(&data.data)?;

        // 替换敏感信息
        for (pattern, replacement) in &self.patterns {
            if frame.message.contains(pattern) {
                // 简单替换：查找包含敏感词的部分并替换
                let parts: Vec<&str> = frame.message.split(pattern).collect();
                if parts.len() > 1 {
                    // 找到敏感词，替换后面的值
                    frame.message = format!("{}{}={}", parts[0], pattern, replacement);
                }
            }
        }

        // 重新序列化
        let sanitized = frame.serialize()?;
        Ok(LogBatch::new(sanitized))
    }

    fn on_error(&self, data: LogBatch, _error: ScribeError) -> Fallback {
        // 脱敏失败时继续使用原始数据（安全考虑可能要改为 Abort）
        Fallback::Skip
    }
}

/// 统计收集器 - 收集日志统计信息
use std::collections::HashMap;

#[derive(Default)]
struct LogStatistics {
    total_processed: u64,
    by_level: HashMap<LogLevel, u64>,
    total_bytes: u64,
}

struct StatisticsStage {
    stats: Arc<Mutex<LogStatistics>>,
}

impl PipelineStage for StatisticsStage {
    fn name(&self) -> &str {
        "statistics_collector"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        // 反序列化日志以获取级别
        let frame = LogFrame::deserialize(&data.data)?;

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_processed += 1;
        *stats.by_level.entry(frame.level).or_insert(0) += 1;
        stats.total_bytes += data.size() as u64;

        // 返回原始数据（统计不修改数据）
        Ok(data)
    }

    fn on_error(&self, data: LogBatch, _error: ScribeError) -> Fallback {
        // 统计失败不影响数据流
        Fallback::Skip
    }
}

/// 不稳定的 Stage - 用于测试错误处理
struct FlakyStage {
    fail_probability: f64, // 0.0 - 1.0
}

impl PipelineStage for FlakyStage {
    fn name(&self) -> &str {
        "flaky_stage"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        // 模拟随机失败
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hash, Hasher};

        let mut hasher = RandomState::new().build_hasher();
        std::time::SystemTime::now().hash(&mut hasher);
        let random = (hasher.finish() % 100) as f64 / 100.0;

        if random < self.fail_probability {
            Err(ScribeError::Mmap("模拟的随机失败".to_string()))
        } else {
            Ok(data)
        }
    }

    fn on_error(&self, data: LogBatch, _error: ScribeError) -> Fallback {
        // 失败时跳过
        Fallback::Skip
    }
}

/// 遇到错误就中止的 Stage
struct AbortOnErrorStage;

impl PipelineStage for AbortOnErrorStage {
    fn name(&self) -> &str {
        "abort_on_error"
    }

    fn process(&self, _data: LogBatch) -> Result<LogBatch> {
        // 故意失败
        Err(ScribeError::Mmap("故意触发的错误".to_string()))
    }

    fn on_error(&self, data: LogBatch, _error: ScribeError) -> Fallback {
        // 遇到错误就中止整个 Pipeline
        Fallback::Abort
    }
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_filter() {
        let filter = LogFilterStage {
            min_level: LogLevel::Warn,
        };

        // 测试通过的日志
        let error_log = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());
        let batch = LogBatch::new(error_log.serialize().unwrap());
        let result = filter.process(batch).unwrap();
        assert!(result.size() > 0);

        // 测试被过滤的日志
        let debug_log = LogFrame::new(LogLevel::Debug, "test".to_string(), "debug".to_string());
        let batch = LogBatch::new(debug_log.serialize().unwrap());
        let result = filter.process(batch).unwrap();
        assert_eq!(result.size(), 0);
    }

    #[test]
    fn test_sanitizer() {
        let sanitizer = SensitiveDataSanitizerStage {
            patterns: vec![("password", "***")],
        };

        let log = LogFrame::new(
            LogLevel::Error,
            "auth".to_string(),
            "login failed password=secret123".to_string(),
        );

        let batch = LogBatch::new(log.serialize().unwrap());
        let result = sanitizer.process(batch).unwrap();

        let processed = LogFrame::deserialize(&result.data).unwrap();
        assert!(processed.message.contains("***"));
        assert!(!processed.message.contains("secret123"));
    }

    #[test]
    fn test_statistics() {
        let stats = Arc::new(Mutex::new(LogStatistics::default()));
        let stage = StatisticsStage {
            stats: stats.clone(),
        };

        for _ in 0..5 {
            let log = LogFrame::new(LogLevel::Info, "test".to_string(), "test".to_string());
            let batch = LogBatch::new(log.serialize().unwrap());
            stage.process(batch).unwrap();
        }

        let stats = stats.lock();
        assert_eq!(stats.total_processed, 5);
    }
}
