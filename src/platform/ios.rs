#[cfg(target_os = "ios")]
pub fn log_to_console(level: i32, tag: &str, message: &str) {
    // iOS os_log 集成
    // 实际实现需要 System framework
    println!("[iOS] [{}] {}: {}", level, tag, message);
}
