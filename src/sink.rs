use crate::LogLevel;
use parking_lot::RwLock;
use std::sync::Arc;

/// LogRecord - 日志记录结构
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub level: LogLevel,
    pub context: Option<String>,
    pub message: String,
    pub thread_name: Option<String>,
    pub caller: Option<String>,
}

/// LogSink trait - 日志输出抽象
pub trait LogSink: Send + Sync {
    /// 记录日志
    fn log(&self, record: &LogRecord);

    /// 判断是否应该记录此日志
    fn is_loggable(&self, level: LogLevel) -> bool {
        let _ = level;
        true
    }
}

/// ConsoleSink - 控制台输出
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
}

impl LogSink for ConsoleSink {
    fn log(&self, record: &LogRecord) {
        if !self.is_loggable(record.level) {
            return;
        }

        let context = record.context.as_deref().unwrap_or("Scribe");
        let thread_info = record
            .thread_name
            .as_ref()
            .map(|t| format!("[{}] ", t))
            .unwrap_or_default();

        println!(
            "{:?} {}{}: {}",
            record.level, thread_info, context, record.message
        );
    }

    fn is_loggable(&self, level: LogLevel) -> bool {
        level as u8 >= self.min_level as u8
    }
}

impl Default for ConsoleSink {
    fn default() -> Self {
        Self::new()
    }
}

/// SinkRegistry - 日志接收器注册表
pub struct SinkRegistry {
    sinks: RwLock<Vec<Arc<dyn LogSink>>>,
}

impl SinkRegistry {
    pub fn new() -> Self {
        Self {
            sinks: RwLock::new(Vec::new()),
        }
    }

    /// 注册一个 Sink
    pub fn register(&self, sink: Box<dyn LogSink>) {
        self.sinks.write().push(Arc::from(sink));
    }

    /// 清空所有 Sink
    pub fn clear(&self) {
        self.sinks.write().clear();
    }

    /// 分发日志到所有 Sink
    pub fn dispatch(&self, record: &LogRecord) {
        let sinks = self.sinks.read();
        for sink in sinks.iter() {
            sink.log(record);
        }
    }

    /// 获取 Sink 数量
    pub fn count(&self) -> usize {
        self.sinks.read().len()
    }
}

impl Default for SinkRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// 全局 SinkRegistry
use once_cell::sync::Lazy;
static GLOBAL_REGISTRY: Lazy<SinkRegistry> = Lazy::new(SinkRegistry::new);

/// 获取全局 Registry
pub fn registry() -> &'static SinkRegistry {
    &GLOBAL_REGISTRY
}

/// 注册一个 Sink
pub fn register_sink(sink: Box<dyn LogSink>) {
    registry().register(sink);
}

/// 清空所有 Sink
pub fn clear_sinks() {
    registry().clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_sink() {
        let sink = ConsoleSink::new();
        let record = LogRecord {
            level: LogLevel::Info,
            context: Some("Test".to_string()),
            message: "Test message".to_string(),
            thread_name: None,
            caller: None,
        };
        sink.log(&record);
    }

    #[test]
    fn test_sink_registry() {
        let registry = SinkRegistry::new();
        assert_eq!(registry.count(), 0);

        registry.register(Box::new(ConsoleSink::new()));
        assert_eq!(registry.count(), 1);

        registry.clear();
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_log_record() {
        let record = LogRecord {
            level: LogLevel::Debug,
            context: Some("MyModule".to_string()),
            message: "Debug message".to_string(),
            thread_name: Some("main".to_string()),
            caller: None,
        };

        assert_eq!(record.level as i32, LogLevel::Debug as i32);
        assert_eq!(record.context, Some("MyModule".to_string()));
    }
}
