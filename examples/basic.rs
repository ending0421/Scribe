//! Scribe 使用示例

use scribe::{Config, LogFrame, LogLevel};
use std::path::PathBuf;

fn main() {
    println!("Scribe Example");

    // 创建配置
    let config = Config::default()
        .with_log_dir(PathBuf::from("/tmp/scribe_example"))
        .with_max_total_size(10 * 1024 * 1024) // 10MB
        .with_retention_days(3);

    println!(
        "Config: log_dir={:?}, max_size={}MB",
        config.log_dir,
        config.max_total_size / 1024 / 1024
    );

    // 创建日志帧
    let frame = LogFrame::new(
        LogLevel::Info,
        "example".to_string(),
        "This is a test log message".to_string(),
    );

    println!(
        "Created log frame: level={:?}, tag={}, message={}",
        frame.level, frame.tag, frame.message
    );

    // 序列化
    match frame.serialize() {
        Ok(data) => {
            println!("Serialized frame: {} bytes", data.len());

            // 反序列化验证
            match LogFrame::deserialize(&data) {
                Ok(deserialized) => {
                    println!("Deserialized successfully!");
                    println!("  Timestamp: {}", deserialized.timestamp);
                    println!("  Level: {:?}", deserialized.level);
                    println!("  Tag: {}", deserialized.tag);
                    println!("  Message: {}", deserialized.message);
                }
                Err(e) => eprintln!("Deserialization failed: {}", e),
            }
        }
        Err(e) => eprintln!("Serialization failed: {}", e),
    }
}
