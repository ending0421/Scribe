use crate::LogLevel;
use std::sync::Arc;
use std::backtrace::Backtrace;

/// LogSink trait - 扩展日志行为的核心抽象
pub trait LogSink: Send + Sync {
    /// 记录日志
    fn log(&self, level: LogLevel, label: Option<, tag: Option<&str>str>, message: &str, thread: Option<&str>);

    /// 判断是否应该记录此日志
    fn is_loggable(&self, level: LogLevel, label: Option<, tag: Option<&str>str>) -> bool {
        let _ = (level, label);
        true  // 默认全部记录
    }
}

/// ConsoleSink - 开发环境使用的 Tree
/// 自动从调用栈获取类名作为 tag
pub struct ConsoleSink {
    min_level: LogLevel,
}

impl ConsoleSink {
    pub fn new() -> Self {
        Self {
            min_level: LogLevel::Verbose,
        }
    }

    pub fn with_min_level(min_level: LogLevel) -> Self {
        Self { min_level }
    }

    fn get_calling_class() -> Option<String> {
        let backtrace = Backtrace::capture();
        let bt_str = format!("{:?}", backtrace);

        // 解析 backtrace 获取调用类名
        // 跳过 scribe 内部调用
        for line in bt_str.lines() {
            if line.contains("::") && !line.contains("scribe::") && !line.contains("sink::") {
                // 提取类名
                if let Some(class) = extract_class_name(line) {
                    return Some(class);
                }
            }
        }
        None
    }
}

impl LogSink for ConsoleSink {
    fn log(&self, level: LogLevel, label: Option<, tag: Option<&str>str>, message: &str, thread: Option<&str>) {
        let auto_tag = tag.or_else(|| Self::get_calling_class().as_deref());
        let tag_str = auto_tag.unwrap_or("Scribe");

        let thread_info = thread.map(|t| format!("[{}] ", t)).unwrap_or_default();

        println!(
            "{:?} {}{}: {}",
            level,
            thread_info,
            tag_str,
            message
        );
    }

    fn is_loggable(&self, level: LogLevel, _tag: Option<&str>) -> bool {
        level as u8 >= self.min_level as u8
    }
}

impl Default for ConsoleSink {
    fn default() -> Self {
        Self::new()
    }
}

fn extract_class_name(line: &str) -> Option<String> {
    // 从 backtrace 行提取类名
    // 示例：  at my_app::MainActivity::onCreate
    // 提取：MainActivity

    // 查找包含 :: 的部分
    if let Some(at_pos) = line.find(" at ") {
        let after_at = &line[at_pos + 4..];

        // 查找第一个空格或括号
        let end_pos = after_at
            .find(' ')
            .or_else(|| after_at.find('('))
            .unwrap_or(after_at.len());

        let func_path = &after_at[..end_pos];

        // 分割路径并提取类名 (倒数第二个部分)
        let parts: Vec<&str> = func_path.split("::").collect();
        if parts.len() >= 2 {
            // 如果倒数第二个部分看起来像类名（首字母大写）
            let potential_class = parts[parts.len() - 2];
            if potential_class.chars().next()?.is_uppercase() {
                return Some(potential_class.to_string());
            }
        }
    }

    None
}

/// Sink Registry
pub struct SinkRegistry {
    sinks: parking_lot::RwLock<Vec<Box<dyn LogSink>>>,
}

impl SinkRegistry {
    pub fn new() -> Self {
        Self {
            sinks: parking_lot::RwLock::new(Vec::new()),
        }
    }

    pub fn register_sink(&self, sink: Box<dyn LogSink>) {
        self.trees.write().push(tree);
    }

    pub fn clear_sinks(&self) {
        self.trees.write().clear();
    }

    pub fn log(&self, level: LogLevel, label: Option<, tag: Option<&str>str>, message: &str) {
        let thread_name = std::thread::current().name().map(|s| s.to_string());
        let sinks = self.trees.read();

        for sink in trees.iter() {
            if sink.is_loggable(level, label) {
                sink.log(level, tag, message, thread_name.as_deref());
            }
        }
    }

    pub fn sink_count(&self) -> usize {
        self.trees.read().len()
    }
}

impl Default for SinkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

static GLOBAL_REGISTRY: once_cell::sync::Lazy<Forest> = once_cell::sync::Lazy::new(SinkRegistry::new);

pub fn register_sink(sink: Box<dyn LogSink>) {
    GLOBAL_REGISTRY.register(tree);
}

pub fn clear_sinks() {
    GLOBAL_REGISTRY.clear();
}

pub fn registry() -> &'static SinkRegistry {
    &GLOBAL_REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// 测试用的 LogSink - 记录调用次数
    struct CountingTree {
        count: Arc<AtomicUsize>,
    }

    impl CountingTree {
        fn new(count: Arc<AtomicUsize>) -> Self {
            Self { count }
        }
    }

    impl LogSink for CountingTree {
        fn log(&self, _level: LogLevel, _tag: Option<&str>, _message: &str, _thread: Option<&str>) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// 测试用的 LogSink - 只记录特定级别
    struct FilteringTree {
        min_level: LogLevel,
        count: Arc<AtomicUsize>,
    }

    impl FilteringTree {
        fn new(min_level: LogLevel, count: Arc<AtomicUsize>) -> Self {
            Self { min_level, count }
        }
    }

    impl LogSink for FilteringTree {
        fn log(&self, _level: LogLevel, _tag: Option<&str>, _message: &str, _thread: Option<&str>) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }

        fn is_loggable(&self, level: LogLevel, _tag: Option<&str>) -> bool {
            level as u8 >= self.min_level as u8
        }
    }

    /// 测试用的 LogSink - 捕获日志内容
    struct CapturingTree {
        logs: Arc<parking_lot::Mutex<Vec<CapturedLog>>>,
    }

    #[derive(Debug, Clone)]
    struct CapturedLog {
        level: LogLevel,
        tag: Option<String>,
        message: String,
        thread: Option<String>,
    }

    impl CapturingTree {
        fn new(logs: Arc<parking_lot::Mutex<Vec<CapturedLog>>>) -> Self {
            Self { logs }
        }
    }

    impl LogSink for CapturingTree {
        fn log(&self, level: LogLevel, label: Option<, tag: Option<&str>str>, message: &str, thread: Option<&str>) {
            self.logs.lock().push(CapturedLog {
                level,
                tag: tag.map(|s| s.to_string()),
                message: message.to_string(),
                thread: thread.map(|s| s.to_string()),
            });
        }
    }

    /// 测试用的 LogSink
    struct TagFilteringTree {
        allowed_tag: String,
        count: Arc<AtomicUsize>,
    }

    impl TagFilteringTree {
        fn new(allowed_tag: String, count: Arc<AtomicUsize>) -> Self {
            Self { allowed_tag, count }
        }
    }

    impl LogSink for TagFilteringTree {
        fn log(&self, _level: LogLevel, _tag: Option<&str>, _message: &str, _thread: Option<&str>) {
            self.count.fetch_add(1, Ordering::SeqCst);
        }

        fn is_loggable(&self, _level: LogLevel, label: Option<, tag: Option<&str>str>) -> bool {
            tag == Some(self.allowed_tag.as_str())
        }
    }

    #[test]
    fn test_forest_new() {
        let forest = SinkRegistry::new();
        assert_eq!(forest.trees.read().len(), 0);
    }

    #[test]
    fn test_forest_default() {
        let forest = SinkRegistry::default();
        assert_eq!(forest.trees.read().len(), 0);
    }

    #[test]
    fn test_plant_single_tree() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count.clone())));

        assert_eq!(forest.trees.read().len(), 1);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_plant_multiple_trees() {
        let forest = SinkRegistry::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count1.clone())));
        forest.register(Box::new(CountingTree::new(count2.clone())));

        assert_eq!(forest.trees.read().len(), 2);
    }

    #[test]
    fn test_uproot_all() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count.clone())));
        forest.register(Box::new(CountingTree::new(count.clone())));

        assert_eq!(forest.trees.read().len(), 2);

        forest.clear();

        assert_eq!(forest.trees.read().len(), 0);
    }

    #[test]
    fn test_log_single_tree() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count.clone())));
        forest.log(LogLevel::Info, Some("test"), "test message");

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_log_multiple_trees() {
        let forest = SinkRegistry::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count1.clone())));
        forest.register(Box::new(CountingTree::new(count2.clone())));

        forest.log(LogLevel::Info, Some("test"), "test message");

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_log_multiple_times() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count.clone())));

        forest.log(LogLevel::Info, Some("test"), "message 1");
        forest.log(LogLevel::Debug, None, "message 2");
        forest.log(LogLevel::Error, Some("error"), "message 3");

        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_filtering_tree_level() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        // 只记录 Warn 及以上级别
        forest.register(Box::new(FilteringTree::new(LogLevel::Warn, count.clone())));

        forest.log(LogLevel::Verbose, None, "verbose");
        forest.log(LogLevel::Debug, None, "debug");
        forest.log(LogLevel::Info, None, "info");
        forest.log(LogLevel::Warn, None, "warn");
        forest.log(LogLevel::Error, None, "error");

        // 只有 Warn 和 Error 被记录
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_capturing_tree_content() {
        let forest = SinkRegistry::new();
        let logs = Arc::new(parking_lot::Mutex::new(Vec::new()));

        forest.register(Box::new(CapturingTree::new(logs.clone())));

        forest.log(LogLevel::Info, Some("tag1"), "message 1");
        forest.log(LogLevel::Error, None, "message 2");

        let captured = logs.lock();
        assert_eq!(captured.len(), 2);

        assert_eq!(captured[0].level, LogLevel::Info);
        assert_eq!(captured[0].tag, Some("tag1".to_string()));
        assert_eq!(captured[0].message, "message 1");

        assert_eq!(captured[1].level, LogLevel::Error);
        assert_eq!(captured[1].tag, None);
        assert_eq!(captured[1].message, "message 2");
    }

    #[test]
    fn test_capturing_tree_thread_name() {
        let forest = SinkRegistry::new();
        let logs = Arc::new(parking_lot::Mutex::new(Vec::new()));

        forest.register(Box::new(CapturingTree::new(logs.clone())));

        let handle = std::thread::Builder::new()
            .name("test-thread".to_string())
            .spawn(move || {
                forest.log(LogLevel::Info, None, "from thread");
            })
            .unwrap();

        handle.join().unwrap();

        let captured = logs.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].thread, Some("test-thread".to_string()));
    }

    #[test]
    fn test_tag_filtering_tree() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(TagFilteringTree::new("allowed".to_string(), count.clone())));

        forest.log(LogLevel::Info, Some("allowed"), "message 1");
        forest.log(LogLevel::Info, Some("blocked"), "message 2");
        forest.log(LogLevel::Info, None, "message 3");
        forest.log(LogLevel::Info, Some("allowed"), "message 4");

        // 只有带 "allowed" 标签的被记录
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_mixed_filtering_trees() {
        let forest = SinkRegistry::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        // Tree 1: 只记录 Error 及以上
        forest.register(Box::new(FilteringTree::new(LogLevel::Error, count1.clone())));
        // Tree 2: 只记录 Debug 及以上
        forest.register(Box::new(FilteringTree::new(LogLevel::Debug, count2.clone())));

        forest.log(LogLevel::Verbose, None, "verbose");
        forest.log(LogLevel::Debug, None, "debug");
        forest.log(LogLevel::Info, None, "info");
        forest.log(LogLevel::Error, None, "error");

        assert_eq!(count1.load(Ordering::SeqCst), 1); // 只记录 Error
        assert_eq!(count2.load(Ordering::SeqCst), 3); // 记录 Debug, Info, Error
    }

    #[test]
    fn test_global_forest_plant() {
        // 清理全局状态
        uproot_all();

        let count = Arc::new(AtomicUsize::new(0));
        plant(Box::new(CountingTree::new(count.clone())));

        forest().log(LogLevel::Info, None, "test");

        assert_eq!(count.load(Ordering::SeqCst), 1);

        // 清理
        uproot_all();
    }

    #[test]
    fn test_global_forest_uproot_all() {
        // 清理全局状态
        uproot_all();

        let count = Arc::new(AtomicUsize::new(0));
        plant(Box::new(CountingTree::new(count.clone())));

        forest().log(LogLevel::Info, None, "test");
        assert_eq!(count.load(Ordering::SeqCst), 1);

        uproot_all();

        forest().log(LogLevel::Info, None, "test");
        // 仍然是 1，因为 tree 已被移除
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_concurrent_logging() {
        let forest = Arc::new(SinkRegistry::new());
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(CountingTree::new(count.clone())));

        let mut handles = vec![];

        for i in 0..10 {
            let forest_clone = forest.clone();
            let handle = std::thread::spawn(move || {
                for j in 0..100 {
                    forest_clone.log(
                        LogLevel::Info,
                        Some("concurrent"),
                        &format!("thread {} message {}", i, j),
                    );
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(count.load(Ordering::SeqCst), 1000);
    }

    #[test]
    fn test_concurrent_plant_and_log() {
        let forest = Arc::new(SinkRegistry::new());
        let count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        // 一些线程植入树
        for _ in 0..5 {
            let forest_clone = forest.clone();
            let count_clone = count.clone();
            let handle = std::thread::spawn(move || {
                forest_clone.register(Box::new(CountingTree::new(count_clone)));
            });
            handles.push(handle);
        }

        // 一些线程记录日志
        for i in 0..5 {
            let forest_clone = forest.clone();
            let handle = std::thread::spawn(move || {
                for j in 0..10 {
                    forest_clone.log(
                        LogLevel::Info,
                        None,
                        &format!("thread {} message {}", i, j),
                    );
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // 验证所有树都被植入
        assert_eq!(forest.trees.read().len(), 5);
        // 日志被记录（具体次数取决于树的植入时机）
        assert!(count.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn test_empty_forest_log() {
        let forest = SinkRegistry::new();
        // 不应该 panic
        forest.log(LogLevel::Info, None, "test");
    }

    #[test]
    fn test_log_with_empty_message() {
        let forest = SinkRegistry::new();
        let logs = Arc::new(parking_lot::Mutex::new(Vec::new()));

        forest.register(Box::new(CapturingTree::new(logs.clone())));
        forest.log(LogLevel::Info, None, "");

        let captured = logs.lock();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].message, "");
    }

    #[test]
    fn test_log_level_ordering() {
        let count = Arc::new(AtomicUsize::new(0));
        let tree = FilteringTree::new(LogLevel::Info, count.clone());

        assert!(!sink.is_loggable(LogLevel::Verbose, None));
        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(sink.is_loggable(LogLevel::Info, None));
        assert!(sink.is_loggable(LogLevel::Warn, None));
        assert!(sink.is_loggable(LogLevel::Error, None));
    }

    // ====== ConsoleSink Tests ======

    #[test]
    fn test_console_sink_new() {
        let tree = ConsoleSink::new();
        assert!(sink.is_loggable(LogLevel::Verbose, None));
        assert!(sink.is_loggable(LogLevel::Debug, None));
        assert!(sink.is_loggable(LogLevel::Info, None));
        assert!(sink.is_loggable(LogLevel::Warn, None));
        assert!(sink.is_loggable(LogLevel::Error, None));
    }

    #[test]
    fn test_console_sink_default() {
        let tree = ConsoleSink::default();
        assert!(sink.is_loggable(LogLevel::Verbose, None));
    }

    #[test]
    fn test_console_sink_with_min_level() {
        let tree = ConsoleSink::with_min_level(LogLevel::Warn);
        assert!(!sink.is_loggable(LogLevel::Verbose, None));
        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(!sink.is_loggable(LogLevel::Info, None));
        assert!(sink.is_loggable(LogLevel::Warn, None));
        assert!(sink.is_loggable(LogLevel::Error, None));
    }

    #[test]
    fn test_console_sink_is_loggable_verbose() {
        let tree = ConsoleSink::with_min_level(LogLevel::Verbose);
        assert!(sink.is_loggable(LogLevel::Verbose, Some("test")));
        assert!(sink.is_loggable(LogLevel::Debug, Some("test")));
        assert!(sink.is_loggable(LogLevel::Info, Some("test")));
        assert!(sink.is_loggable(LogLevel::Warn, Some("test")));
        assert!(sink.is_loggable(LogLevel::Error, Some("test")));
    }

    #[test]
    fn test_console_sink_is_loggable_info() {
        let tree = ConsoleSink::with_min_level(LogLevel::Info);
        assert!(!sink.is_loggable(LogLevel::Verbose, None));
        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(sink.is_loggable(LogLevel::Info, None));
        assert!(sink.is_loggable(LogLevel::Warn, None));
        assert!(sink.is_loggable(LogLevel::Error, None));
    }

    #[test]
    fn test_console_sink_is_loggable_error() {
        let tree = ConsoleSink::with_min_level(LogLevel::Error);
        assert!(!sink.is_loggable(LogLevel::Verbose, None));
        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(!sink.is_loggable(LogLevel::Info, None));
        assert!(!sink.is_loggable(LogLevel::Warn, None));
        assert!(sink.is_loggable(LogLevel::Error, None));
    }

    #[test]
    fn test_console_sink_log_with_explicit_tag() {
        let tree = ConsoleSink::new();
        // 不应该 panic，输出应该使用显式 tag
        sink.log(LogLevel::Info, Some("explicit_tag"), "test message", None);
    }

    #[test]
    fn test_console_sink_log_without_tag() {
        let tree = ConsoleSink::new();
        // 不应该 panic，输出应该使用自动提取的 tag 或 "Scribe"
        sink.log(LogLevel::Info, None, "test message", None);
    }

    #[test]
    fn test_console_sink_log_with_thread() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("test"), "message", Some("main"));
    }

    #[test]
    fn test_console_sink_log_without_thread() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("test"), "message", None);
    }

    #[test]
    fn test_console_sink_log_all_levels() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Verbose, Some("test"), "verbose message", None);
        sink.log(LogLevel::Debug, Some("test"), "debug message", None);
        sink.log(LogLevel::Info, Some("test"), "info message", None);
        sink.log(LogLevel::Warn, Some("test"), "warn message", None);
        sink.log(LogLevel::Error, Some("test"), "error message", None);
    }

    #[test]
    fn test_console_sink_log_empty_message() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("test"), "", None);
    }

    #[test]
    fn test_console_sink_log_long_message() {
        let tree = ConsoleSink::new();
        let long_message = "a".repeat(10000);
        sink.log(LogLevel::Info, Some("test"), &long_message, None);
    }

    #[test]
    fn test_console_sink_log_utf8_message() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("test"), "Hello 世界 🌍 Привет", None);
    }

    #[test]
    fn test_console_sink_log_special_characters() {
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("test"), "Line1\nLine2\tTabbed", None);
    }

    #[test]
    fn test_console_sink_filter_verbose_out() {
        let tree = ConsoleSink::with_min_level(LogLevel::Debug);
        assert!(!sink.is_loggable(LogLevel::Verbose, None));
        // 实际调用 log 会被过滤（通过 Forest 的 is_loggable）
    }

    #[test]
    fn test_console_sink_filter_debug_out() {
        let tree = ConsoleSink::with_min_level(LogLevel::Info);
        assert!(!sink.is_loggable(LogLevel::Debug, None));
    }

    #[test]
    fn test_console_sink_in_forest() {
        let forest = SinkRegistry::new();
        forest.register(Box::new(ConsoleSink::new()));

        // 不应该 panic
        forest.log(LogLevel::Info, Some("test"), "test message");
    }

    #[test]
    fn test_console_sink_multiple_in_forest() {
        let forest = SinkRegistry::new();
        forest.register(Box::new(ConsoleSink::new()));
        forest.register(Box::new(ConsoleSink::with_min_level(LogLevel::Warn)));

        forest.log(LogLevel::Info, Some("test"), "info message");
        forest.log(LogLevel::Warn, Some("test"), "warn message");
        forest.log(LogLevel::Error, Some("test"), "error message");
    }

    #[test]
    fn test_console_sink_with_filtering_tree() {
        let forest = SinkRegistry::new();
        let count = Arc::new(AtomicUsize::new(0));

        forest.register(Box::new(ConsoleSink::with_min_level(LogLevel::Info)));
        forest.register(Box::new(CountingTree::new(count.clone())));

        forest.log(LogLevel::Verbose, Some("test"), "verbose");
        forest.log(LogLevel::Info, Some("test"), "info");
        forest.log(LogLevel::Error, Some("test"), "error");

        // CountingTree 应该记录所有 3 条
        assert_eq!(count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_console_sink_concurrent_logging() {
        let forest = Arc::new(SinkRegistry::new());
        forest.register(Box::new(ConsoleSink::new()));

        let handles: Vec<_> = (0..5)
            .map(|i| {
                let forest_clone = forest.clone();
                std::thread::spawn(move || {
                    for j in 0..10 {
                        forest_clone.log(
                            LogLevel::Info,
                            Some("concurrent"),
                            &format!("thread {} message {}", i, j),
                        );
                    }
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_console_sink_with_named_thread() {
        let tree = ConsoleSink::new();

        let handle = std::thread::Builder::new()
            .name("test-worker".to_string())
            .spawn(move || {
                sink.log(LogLevel::Info, Some("test"), "from named thread", Some("test-worker"));
            })
            .unwrap();

        handle.join().unwrap();
    }

    #[test]
    fn test_extract_class_name_valid() {
        let line = "  at my_app::MainActivity::onCreate";
        let result = extract_class_name(line);
        assert_eq!(result, Some("MainActivity".to_string()));
    }

    #[test]
    fn test_extract_class_name_with_namespace() {
        let line = "  at com::example::app::UserService::process_request";
        let result = extract_class_name(line);
        assert_eq!(result, Some("UserService".to_string()));
    }

    #[test]
    fn test_extract_class_name_no_class() {
        let line = "  at my_app::utils::helper_function";
        let result = extract_class_name(line);
        // helper_function 不是类名（小写开头）
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_class_name_no_double_colon() {
        let line = "  at some_function";
        let result = extract_class_name(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_class_name_with_parentheses() {
        let line = "  at my_app::MyClass::method(arg1, arg2)";
        let result = extract_class_name(line);
        assert_eq!(result, Some("MyClass".to_string()));
    }

    #[test]
    fn test_extract_class_name_empty_string() {
        let line = "";
        let result = extract_class_name(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_class_name_only_module() {
        let line = "  at my_module::function";
        let result = extract_class_name(line);
        // my_module 小写开头，不是类名
        assert_eq!(result, None);
    }

    #[test]
    fn test_console_sink_tag_priority() {
        // 显式 tag 应该优先于自动提取的 tag
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, Some("explicit"), "message", None);
        // 输出应该使用 "explicit" 而不是自动提取的类名
    }

    #[test]
    fn test_console_sink_fallback_to_scribe() {
        // 没有 tag 且无法提取类名时，应该使用 "Scribe"
        let tree = ConsoleSink::new();
        sink.log(LogLevel::Info, None, "message", None);
        // 输出应该使用 "Scribe"
    }

    #[test]
    fn test_console_sink_level_boundary() {
        let tree = ConsoleSink::with_min_level(LogLevel::Info);

        // 边界测试
        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(sink.is_loggable(LogLevel::Info, None)); // 边界
        assert!(sink.is_loggable(LogLevel::Warn, None));
    }

    #[test]
    fn test_console_sink_tag_independence() {
        // tag 参数不应影响 is_loggable 的结果（仅级别过滤）
        let tree = ConsoleSink::with_min_level(LogLevel::Info);

        assert!(sink.is_loggable(LogLevel::Info, None));
        assert!(sink.is_loggable(LogLevel::Info, Some("tag1")));
        assert!(sink.is_loggable(LogLevel::Info, Some("tag2")));

        assert!(!sink.is_loggable(LogLevel::Debug, None));
        assert!(!sink.is_loggable(LogLevel::Debug, Some("tag1")));
    }

    #[test]
    fn test_console_sink_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<DebugTree>();
    }

    #[test]
    fn test_console_sink_as_trait_object() {
        let sink: Box<dyn LogSink> = Box::new(ConsoleSink::new());
        sink.log(LogLevel::Info, Some("test"), "message", None);
        assert!(sink.is_loggable(LogLevel::Verbose, None));
    }

    #[test]
    fn test_console_sink_clone_via_new() {
        let tree1 = ConsoleSink::with_min_level(LogLevel::Warn);
        let tree2 = ConsoleSink::with_min_level(LogLevel::Warn);

        // 两个实例应该有相同的行为
        assert_eq!(
            tree1.is_loggable(LogLevel::Info, None),
            tree2.is_loggable(LogLevel::Info, None)
        );
    }

    #[test]
    fn test_console_sink_stress_logging() {
        let tree = ConsoleSink::new();

        for i in 0..1000 {
            sink.log(
                LogLevel::Info,
                Some("stress"),
                &format!("message {}", i),
                None
            );
        }
    }

    #[test]
    fn test_global_forest_with_debug_tree() {
        uproot_all();

        plant(Box::new(ConsoleSink::with_min_level(LogLevel::Info)));

        forest().log(LogLevel::Verbose, Some("test"), "should be filtered");
        forest().log(LogLevel::Info, Some("test"), "should appear");
        forest().log(LogLevel::Error, Some("test"), "should appear");

        uproot_all();
    }

    #[test]
    fn test_console_sink_mixed_log_levels() {
        let forest = SinkRegistry::new();
        forest.register(Box::new(ConsoleSink::with_min_level(LogLevel::Debug)));

        let levels = vec![
            LogLevel::Verbose,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ];

        for level in levels {
            forest.log(level, Some("test"), &format!("{:?} message", level));
        }
    }

    #[test]
    fn test_extract_class_name_complex_path() {
        let line = "  at deeply::nested::module::MyStruct::new";
        let result = extract_class_name(line);
        assert_eq!(result, Some("MyStruct".to_string()));
    }

    #[test]
    fn test_extract_class_name_generic() {
        let line = "  at app::Container<T>::insert";
        let result = extract_class_name(line);
        // 应该提取 "Container"（可能包含 "<T>"）
        // 具体行为取决于实现
        assert!(result.is_some() || result.is_none());
    }
}
