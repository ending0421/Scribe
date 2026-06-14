use scribe::{DebugTree, Forest, LogLevel, plant, uproot_all, forest};

struct MainActivity;

impl MainActivity {
    fn on_create(&self) {
        // 自动从调用栈提取类名 "MainActivity"
        forest().log(LogLevel::Info, None, "Activity created");
    }

    fn on_resume(&self) {
        // 使用显式 tag（优先级高于自动提取）
        forest().log(LogLevel::Debug, Some("Lifecycle"), "Activity resumed");
    }

    fn handle_error(&self, error: &str) {
        forest().log(LogLevel::Error, None, &format!("Error occurred: {}", error));
    }
}

struct UserService;

impl UserService {
    fn fetch_user(&self, user_id: u32) {
        forest().log(LogLevel::Info, None, &format!("Fetching user {}", user_id));
    }

    fn update_profile(&self, user_id: u32) {
        forest().log(LogLevel::Warn, None, &format!("Updating profile for user {}", user_id));
    }
}

fn main() {
    println!("=== DebugTree Example ===\n");

    // 示例 1: 基本用法 - 记录所有级别
    println!("1. Basic usage - log all levels:");
    plant(Box::new(DebugTree::new()));

    let activity = MainActivity;
    activity.on_create();
    activity.on_resume();

    uproot_all();
    println!();

    // 示例 2: 使用最小级别过滤 - 只记录 Info 及以上
    println!("2. With min level filter (Info and above):");
    plant(Box::new(DebugTree::with_min_level(LogLevel::Info)));

    forest().log(LogLevel::Verbose, Some("test"), "This will be filtered out");
    forest().log(LogLevel::Debug, Some("test"), "This will also be filtered out");
    forest().log(LogLevel::Info, Some("test"), "This will appear");
    forest().log(LogLevel::Warn, Some("test"), "This will appear");
    forest().log(LogLevel::Error, Some("test"), "This will appear");

    uproot_all();
    println!();

    // 示例 3: 多个 DebugTree - 不同的过滤级别
    println!("3. Multiple DebugTrees with different filters:");
    plant(Box::new(DebugTree::with_min_level(LogLevel::Warn)));
    plant(Box::new(DebugTree::with_min_level(LogLevel::Info)));

    forest().log(LogLevel::Debug, Some("multi"), "Filtered by both");
    forest().log(LogLevel::Info, Some("multi"), "Appears once (second tree)");
    forest().log(LogLevel::Warn, Some("multi"), "Appears twice (both trees)");

    uproot_all();
    println!();

    // 示例 4: 自动类名提取
    println!("4. Auto class name extraction:");
    plant(Box::new(DebugTree::new()));

    let activity = MainActivity;
    activity.on_create();
    activity.handle_error("Network timeout");

    let service = UserService;
    service.fetch_user(123);
    service.update_profile(456);

    uproot_all();
    println!();

    // 示例 5: 显式 tag vs 自动提取
    println!("5. Explicit tag vs auto-extracted:");
    plant(Box::new(DebugTree::new()));

    // 使用显式 tag
    forest().log(LogLevel::Info, Some("CustomTag"), "With explicit tag");

    // 自动提取（fallback 到 "Scribe"）
    forest().log(LogLevel::Info, None, "Without tag (fallback to Scribe)");

    uproot_all();
    println!();

    // 示例 6: 多线程场景
    println!("6. Multi-threaded logging:");
    plant(Box::new(DebugTree::new()));

    let handles: Vec<_> = (0..3)
        .map(|i| {
            std::thread::Builder::new()
                .name(format!("worker-{}", i))
                .spawn(move || {
                    forest().log(
                        LogLevel::Info,
                        Some("Worker"),
                        &format!("Message from thread {}", i),
                    );
                })
                .unwrap()
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    uproot_all();
    println!();

    // 示例 7: 不同日志级别的典型用途
    println!("7. Typical use cases for different log levels:");
    plant(Box::new(DebugTree::new()));

    forest().log(LogLevel::Verbose, Some("App"), "Detailed trace: entering function X");
    forest().log(LogLevel::Debug, Some("App"), "Debug info: variable value = 42");
    forest().log(LogLevel::Info, Some("App"), "User logged in successfully");
    forest().log(LogLevel::Warn, Some("App"), "API rate limit approaching");
    forest().log(LogLevel::Error, Some("App"), "Failed to save data to database");

    uproot_all();
    println!();

    // 示例 8: 直接使用 Forest（非全局）
    println!("8. Using Forest directly (non-global):");
    let forest = Forest::new();
    forest.plant(Box::new(DebugTree::with_min_level(LogLevel::Warn)));

    forest.log(LogLevel::Info, Some("local"), "This won't appear (below Warn)");
    forest.log(LogLevel::Warn, Some("local"), "This will appear");
    forest.log(LogLevel::Error, Some("local"), "This will also appear");

    println!();

    // 示例 9: UTF-8 和特殊字符
    println!("9. UTF-8 and special characters:");
    plant(Box::new(DebugTree::new()));

    forest().log(LogLevel::Info, Some("i18n"), "Hello 世界 🌍");
    forest().log(LogLevel::Info, Some("i18n"), "Привет мир");
    forest().log(LogLevel::Info, Some("i18n"), "こんにちは世界");
    forest().log(LogLevel::Info, Some("multiline"), "Line 1\nLine 2\tTabbed");

    uproot_all();
    println!();

    println!("=== Example completed ===");
}
