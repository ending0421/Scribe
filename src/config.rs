use serde::{Deserialize, Serialize};

/// Scribe 配置结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScribeConfig {
    /// 自动刷新间隔（毫秒）
    #[serde(default = "default_auto_flush_interval")]
    pub auto_flush_interval_ms: u64,

    /// 启用控制台输出
    #[serde(default)]
    pub enable_console: bool,

    /// 控制台最小日志级别
    #[serde(default = "default_min_console_level")]
    pub min_console_level: i32,

    /// 单个日志文件最大大小（MB）
    #[serde(default = "default_max_file_size")]
    pub max_file_size_mb: usize,

    /// 最大日志文件数量
    #[serde(default = "default_max_file_count")]
    pub max_file_count: usize,

    /// 启用压缩
    #[serde(default = "default_compression")]
    pub compression: bool,

    /// 启用加密
    #[serde(default)]
    pub encryption: bool,
}

fn default_auto_flush_interval() -> u64 { 5000 }
fn default_min_console_level() -> i32 { 1 }
fn default_max_file_size() -> usize { 10 }
fn default_max_file_count() -> usize { 5 }
fn default_compression() -> bool { true }

impl Default for ScribeConfig {
    fn default() -> Self {
        Self {
            auto_flush_interval_ms: default_auto_flush_interval(),
            enable_console: false,
            min_console_level: default_min_console_level(),
            max_file_size_mb: default_max_file_size(),
            max_file_count: default_max_file_count(),
            compression: default_compression(),
            encryption: false,
        }
    }
}

impl ScribeConfig {
    /// 从 JSON 字符串解析配置
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// 转换为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScribeConfig::default();
        assert_eq!(config.auto_flush_interval_ms, 5000);
        assert!(!config.enable_console);
        assert!(config.compression);
    }

    #[test]
    fn test_json_serialization() {
        let config = ScribeConfig::default();
        let json = config.to_json().unwrap();
        let parsed = ScribeConfig::from_json(&json).unwrap();
        assert_eq!(config.auto_flush_interval_ms, parsed.auto_flush_interval_ms);
    }

    #[test]
    fn test_partial_json() {
        let json = r#"{"enable_console": true}"#;
        let config = ScribeConfig::from_json(json).unwrap();
        assert!(config.enable_console);
        assert_eq!(config.auto_flush_interval_ms, 5000); // 使用默认值
    }
}
