#[cfg(target_os = "android")]
pub fn log_to_console(level: i32, tag: &str, message: &str) {
    // Platform logging integration
    // 实际实现需要 NDK 支持
    println!("[Platform] [{}] {}: {}", level, tag, message);
}
