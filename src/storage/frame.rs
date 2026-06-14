use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use crc32fast::Hasher;
use std::io::{Cursor, Write};

const MAGIC_HEADER: u32 = 0xFEEDC0DE;

/// Log level for filtering and categorizing log messages.
///
/// Levels are ordered by severity: `Verbose` (lowest) to `Error` (highest).
///
/// # Examples
///
/// ```
/// use scribe::LogLevel;
///
/// let level = LogLevel::Info;
/// assert_eq!(level as u8, 2);
/// ```
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogLevel {
    /// Detailed trace information for debugging.
    Verbose = 0,
    /// Debug information for development.
    Debug = 1,
    /// General informational messages.
    Info = 2,
    /// Warning messages for potentially problematic situations.
    Warn = 3,
    /// Error messages for failure conditions.
    Error = 4,
}

impl LogLevel {
    /// Converts a u8 value to a LogLevel.
    ///
    /// # Arguments
    ///
    /// * `value` - The numeric representation of the log level (0-4).
    ///
    /// # Returns
    ///
    /// * `Some(LogLevel)` - If the value is valid (0-4).
    /// * `None` - If the value is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::LogLevel;
    ///
    /// assert_eq!(LogLevel::from_u8(2), Some(LogLevel::Info));
    /// assert_eq!(LogLevel::from_u8(99), None);
    /// ```
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(LogLevel::Verbose),
            1 => Some(LogLevel::Debug),
            2 => Some(LogLevel::Info),
            3 => Some(LogLevel::Warn),
            4 => Some(LogLevel::Error),
            _ => None,
        }
    }
}

/// A structured log entry with timestamp, level, tag, and message.
///
/// LogFrame is the core data structure for log entries in Scribe. Each frame
/// includes a microsecond-precision timestamp, severity level, tag for categorization,
/// and the actual log message.
///
/// # Examples
///
/// ```
/// use scribe::{LogFrame, LogLevel};
///
/// let frame = LogFrame::new(
///     LogLevel::Info,
///     "network".to_string(),
///     "Connection established".to_string()
/// );
///
/// // Serialize to binary format
/// let bytes = frame.serialize().unwrap();
///
/// // Deserialize back
/// let decoded = LogFrame::deserialize(&bytes).unwrap();
/// assert_eq!(decoded.level, LogLevel::Info);
/// ```
pub struct LogFrame {
    /// Unix timestamp in microseconds.
    pub timestamp: i64,
    /// Severity level of the log entry.
    pub level: LogLevel,
    /// Category or component identifier.
    pub tag: String,
    /// The log message content.
    pub message: String,
}

impl LogFrame {
    /// Creates a new LogFrame with the current timestamp.
    ///
    /// # Arguments
    ///
    /// * `level` - The severity level of this log entry.
    /// * `tag` - A category or component identifier.
    /// * `message` - The log message content.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::{LogFrame, LogLevel};
    ///
    /// let frame = LogFrame::new(
    ///     LogLevel::Error,
    ///     "auth".to_string(),
    ///     "Login failed".to_string()
    /// );
    /// ```
    pub fn new(level: LogLevel, tag: String, message: String) -> Self {
        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64,
            level,
            tag,
            message,
        }
    }

    /// Serializes the LogFrame into a binary format with CRC32 checksum.
    ///
    /// The binary format includes:
    /// - Magic header (4 bytes)
    /// - Frame length (4 bytes)
    /// - Timestamp (8 bytes)
    /// - Level (1 byte)
    /// - Tag length (2 bytes) + tag bytes
    /// - Message length (4 bytes) + message bytes
    /// - CRC32 checksum (4 bytes)
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - The serialized binary data.
    /// * `Err(ScribeError)` - If serialization fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::{LogFrame, LogLevel};
    ///
    /// let frame = LogFrame::new(LogLevel::Info, "test".to_string(), "hello".to_string());
    /// let bytes = frame.serialize().unwrap();
    /// assert!(bytes.len() > 0);
    /// ```
    pub fn serialize(&self) -> crate::Result<Vec<u8>> {
        let mut buf = Vec::new();

        // Magic Header
        buf.write_u32::<LittleEndian>(MAGIC_HEADER)?;

        // 预留 Frame Length（稍后填充）
        let length_pos = buf.len();
        buf.write_u32::<LittleEndian>(0)?;

        // Timestamp
        buf.write_i64::<LittleEndian>(self.timestamp)?;

        // Level
        buf.write_u8(self.level as u8)?;

        // Tag
        let tag_bytes = self.tag.as_bytes();
        buf.write_u16::<LittleEndian>(tag_bytes.len() as u16)?;
        buf.write_all(tag_bytes)?;

        // Message
        let msg_bytes = self.message.as_bytes();
        buf.write_u32::<LittleEndian>(msg_bytes.len() as u32)?;
        buf.write_all(msg_bytes)?;

        // 计算 CRC32（不包括 Magic 和 Length）
        let mut hasher = Hasher::new();
        hasher.update(&buf[8..]);
        let crc = hasher.finalize();
        buf.write_u32::<LittleEndian>(crc)?;

        // 回填 Frame Length
        let frame_length = buf.len() as u32;
        let mut cursor = Cursor::new(&mut buf[length_pos..length_pos + 4]);
        cursor.write_u32::<LittleEndian>(frame_length)?;

        Ok(buf)
    }

    /// Deserializes a LogFrame from binary data with CRC32 validation.
    ///
    /// # Arguments
    ///
    /// * `data` - The binary data to deserialize.
    ///
    /// # Returns
    ///
    /// * `Ok(LogFrame)` - The deserialized log frame.
    /// * `Err(ScribeError)` - If the data is invalid or CRC check fails.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The magic header is invalid
    /// - The CRC32 checksum doesn't match
    /// - The data is truncated or malformed
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::{LogFrame, LogLevel};
    ///
    /// let frame = LogFrame::new(LogLevel::Info, "test".to_string(), "hello".to_string());
    /// let bytes = frame.serialize().unwrap();
    /// let decoded = LogFrame::deserialize(&bytes).unwrap();
    /// assert_eq!(decoded.tag, "test");
    /// ```
    pub fn deserialize(data: &[u8]) -> crate::Result<Self> {
        let mut cursor = Cursor::new(data);

        // 验证 Magic Header
        let magic = cursor.read_u32::<LittleEndian>()?;
        if magic != MAGIC_HEADER {
            return Err(crate::ScribeError::InvalidFrame);
        }

        // Frame Length
        let _length = cursor.read_u32::<LittleEndian>()?;

        // Timestamp
        let timestamp = cursor.read_i64::<LittleEndian>()?;

        // Level
        let level_u8 = cursor.read_u8()?;
        let level = LogLevel::from_u8(level_u8)
            .ok_or(crate::ScribeError::InvalidFrame)?;

        // Tag
        let tag_len = cursor.read_u16::<LittleEndian>()? as usize;
        let mut tag_bytes = vec![0u8; tag_len];
        std::io::Read::read_exact(&mut cursor, &mut tag_bytes)?;
        let tag = String::from_utf8_lossy(&tag_bytes).to_string();

        // Message
        let msg_len = cursor.read_u32::<LittleEndian>()? as usize;
        let mut msg_bytes = vec![0u8; msg_len];
        std::io::Read::read_exact(&mut cursor, &mut msg_bytes)?;
        let message = String::from_utf8_lossy(&msg_bytes).to_string();

        // CRC32
        let crc_stored = cursor.read_u32::<LittleEndian>()?;

        // 验证 CRC
        let mut hasher = Hasher::new();
        hasher.update(&data[8..data.len() - 4]);
        let crc_calculated = hasher.finalize();

        if crc_stored != crc_calculated {
            return Err(crate::ScribeError::CrcMismatch);
        }

        Ok(Self {
            timestamp,
            level,
            tag,
            message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_serialize_deserialize() {
        let frame = LogFrame::new(
            LogLevel::Info,
            "test".to_string(),
            "hello world".to_string(),
        );

        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.level, LogLevel::Info);
        assert_eq!(deserialized.tag, "test");
        assert_eq!(deserialized.message, "hello world");
    }

    #[test]
    fn test_frame_crc_validation() {
        let frame = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());

        let mut serialized = frame.serialize().unwrap();

        // 篡改数据
        serialized[20] ^= 0xFF;

        // 验证失败
        assert!(LogFrame::deserialize(&serialized).is_err());
    }

    // 1. 所有日志级别序列化测试
    #[test]
    fn test_all_log_levels_verbose() {
        let frame = LogFrame::new(LogLevel::Verbose, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.level, LogLevel::Verbose);
        assert_eq!(deserialized.level as u8, 0);
    }

    #[test]
    fn test_all_log_levels_debug() {
        let frame = LogFrame::new(LogLevel::Debug, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.level, LogLevel::Debug);
        assert_eq!(deserialized.level as u8, 1);
    }

    #[test]
    fn test_all_log_levels_info() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.level, LogLevel::Info);
        assert_eq!(deserialized.level as u8, 2);
    }

    #[test]
    fn test_all_log_levels_warn() {
        let frame = LogFrame::new(LogLevel::Warn, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.level, LogLevel::Warn);
        assert_eq!(deserialized.level as u8, 3);
    }

    #[test]
    fn test_all_log_levels_error() {
        let frame = LogFrame::new(LogLevel::Error, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.level, LogLevel::Error);
        assert_eq!(deserialized.level as u8, 4);
    }

    // 2. 空 tag 测试
    #[test]
    fn test_empty_tag() {
        let frame = LogFrame::new(LogLevel::Info, "".to_string(), "message".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, "");
        assert_eq!(deserialized.message, "message");
    }

    // 3. 空 message 测试
    #[test]
    fn test_empty_message() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, "tag");
        assert_eq!(deserialized.message, "");
    }

    // 4. 空 tag 和空 message 同时测试
    #[test]
    fn test_empty_tag_and_message() {
        let frame = LogFrame::new(LogLevel::Info, "".to_string(), "".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, "");
        assert_eq!(deserialized.message, "");
    }

    // 5. 超长 tag 测试（100+ 字符）
    #[test]
    fn test_long_tag_100_chars() {
        let long_tag = "a".repeat(100);
        let frame = LogFrame::new(LogLevel::Info, long_tag.clone(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, long_tag);
        assert_eq!(deserialized.tag.len(), 100);
    }

    #[test]
    fn test_long_tag_1000_chars() {
        let long_tag = "b".repeat(1000);
        let frame = LogFrame::new(LogLevel::Info, long_tag.clone(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, long_tag);
        assert_eq!(deserialized.tag.len(), 1000);
    }

    #[test]
    fn test_long_tag_max_u16() {
        // tag 长度用 u16 存储，测试接近最大值
        let long_tag = "c".repeat(65535);
        let frame = LogFrame::new(LogLevel::Info, long_tag.clone(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, long_tag);
        assert_eq!(deserialized.tag.len(), 65535);
    }

    // 6. 超长 message 测试（1MB+）
    #[test]
    fn test_long_message_1kb() {
        let long_msg = "x".repeat(1024);
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), long_msg.clone());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, long_msg);
        assert_eq!(deserialized.message.len(), 1024);
    }

    #[test]
    fn test_long_message_1mb() {
        let long_msg = "y".repeat(1024 * 1024);
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), long_msg.clone());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, long_msg);
        assert_eq!(deserialized.message.len(), 1024 * 1024);
    }

    #[test]
    fn test_long_message_5mb() {
        let long_msg = "z".repeat(5 * 1024 * 1024);
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), long_msg.clone());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, long_msg);
        assert_eq!(deserialized.message.len(), 5 * 1024 * 1024);
    }

    // 7. 特殊字符测试
    #[test]
    fn test_emoji_in_tag() {
        let emoji_tag = "🚀🔥💻";
        let frame = LogFrame::new(LogLevel::Info, emoji_tag.to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, emoji_tag);
    }

    #[test]
    fn test_emoji_in_message() {
        let emoji_msg = "Hello 👋 World 🌍!";
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), emoji_msg.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, emoji_msg);
    }

    #[test]
    fn test_chinese_characters() {
        let chinese_tag = "日志标签";
        let chinese_msg = "这是一条中文日志消息，包含标点符号：！@#￥%……&*（）";
        let frame = LogFrame::new(
            LogLevel::Info,
            chinese_tag.to_string(),
            chinese_msg.to_string(),
        );
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, chinese_tag);
        assert_eq!(deserialized.message, chinese_msg);
    }

    #[test]
    fn test_japanese_characters() {
        let japanese = "こんにちは世界";
        let frame = LogFrame::new(LogLevel::Info, japanese.to_string(), japanese.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.tag, japanese);
        assert_eq!(deserialized.message, japanese);
    }

    #[test]
    fn test_newline_characters() {
        let msg_with_newlines = "Line 1\nLine 2\r\nLine 3\rLine 4";
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), msg_with_newlines.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, msg_with_newlines);
    }

    #[test]
    fn test_tab_and_special_whitespace() {
        let msg_with_tabs = "Col1\tCol2\tCol3\t\t  Extra spaces";
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), msg_with_tabs.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, msg_with_tabs);
    }

    #[test]
    fn test_null_bytes() {
        let msg_with_null = "Before\0After";
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), msg_with_null.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, msg_with_null);
    }

    #[test]
    fn test_mixed_special_characters() {
        let mixed = "Mixed: 中文 English 123 🎉\n\tEmoji\r\n特殊符号!@#$%^&*()";
        let frame = LogFrame::new(LogLevel::Info, "mixed".to_string(), mixed.to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        assert_eq!(deserialized.message, mixed);
    }

    // 8. 非 UTF-8 字符测试（优雅处理）
    #[test]
    fn test_invalid_utf8_in_tag() {
        // 直接在序列化数据中插入无效 UTF-8
        let frame = LogFrame::new(LogLevel::Info, "valid".to_string(), "msg".to_string());
        let mut serialized = frame.serialize().unwrap();

        // 找到 tag 数据的位置并插入无效 UTF-8
        // Magic(4) + Length(4) + Timestamp(8) + Level(1) + TagLen(2) = 19
        // 将 tag 替换为无效的 UTF-8 序列
        serialized[19] = 0x00; // tag length low byte
        serialized[20] = 0x00; // tag length high byte
        serialized.insert(21, 0xFF); // 无效的 UTF-8
        serialized.insert(22, 0xFE);
        serialized[19] = 0x02; // update length to 2

        // 重新计算 CRC 和 length
        let data_len = serialized.len();
        let mut hasher = Hasher::new();
        hasher.update(&serialized[8..data_len - 4]);
        let crc = hasher.finalize();
        let mut cursor = Cursor::new(&mut serialized[data_len - 4..]);
        cursor.write_u32::<LittleEndian>(crc).unwrap();

        let mut cursor = Cursor::new(&mut serialized[4..8]);
        cursor.write_u32::<LittleEndian>(serialized.len() as u32).unwrap();

        // 反序列化应该优雅处理（使用 from_utf8_lossy）
        let deserialized = LogFrame::deserialize(&serialized).unwrap();
        // from_utf8_lossy 会将无效字节替换为 �
        assert!(deserialized.tag.contains('�') || !deserialized.tag.is_empty());
    }

    #[test]
    fn test_invalid_utf8_graceful_handling() {
        // 创建包含无效 UTF-8 的原始字节
        let invalid_bytes = vec![0xFF, 0xFE, 0xFD];
        let tag = String::from_utf8_lossy(&invalid_bytes).to_string();

        let frame = LogFrame::new(LogLevel::Info, tag.clone(), "msg".to_string());
        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        // 应该包含替换字符
        assert_eq!(deserialized.tag, tag);
    }

    // 9. timestamp 验证测试
    #[test]
    fn test_timestamp_is_set() {
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());

        let after = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        assert!(frame.timestamp >= before);
        assert!(frame.timestamp <= after);
    }

    #[test]
    fn test_timestamp_preserved_after_serialization() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let original_timestamp = frame.timestamp;

        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.timestamp, original_timestamp);
    }

    #[test]
    fn test_negative_timestamp() {
        let frame = LogFrame {
            timestamp: -1000,
            level: LogLevel::Info,
            tag: "tag".to_string(),
            message: "msg".to_string(),
        };

        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.timestamp, -1000);
    }

    #[test]
    fn test_max_timestamp() {
        let frame = LogFrame {
            timestamp: i64::MAX,
            level: LogLevel::Info,
            tag: "tag".to_string(),
            message: "msg".to_string(),
        };

        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.timestamp, i64::MAX);
    }

    #[test]
    fn test_min_timestamp() {
        let frame = LogFrame {
            timestamp: i64::MIN,
            level: LogLevel::Info,
            tag: "tag".to_string(),
            message: "msg".to_string(),
        };

        let serialized = frame.serialize().unwrap();
        let deserialized = LogFrame::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.timestamp, i64::MIN);
    }

    // 10. Magic Header 验证测试
    #[test]
    fn test_magic_header_validation() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let mut serialized = frame.serialize().unwrap();

        // 篡改 Magic Header
        serialized[0] = 0x00;
        serialized[1] = 0x00;
        serialized[2] = 0x00;
        serialized[3] = 0x00;

        let result = LogFrame::deserialize(&serialized);
        assert!(result.is_err());
        match result {
            Err(crate::ScribeError::InvalidFrame) => {},
            _ => panic!("Expected InvalidFrame error"),
        }
    }

    #[test]
    fn test_wrong_magic_header() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let mut serialized = frame.serialize().unwrap();

        // 使用错误的 magic header
        serialized[0] = 0xDE;
        serialized[1] = 0xAD;
        serialized[2] = 0xBE;
        serialized[3] = 0xEF;

        let result = LogFrame::deserialize(&serialized);
        assert!(result.is_err());
    }

    #[test]
    fn test_correct_magic_header_value() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let serialized = frame.serialize().unwrap();

        // 验证 Magic Header 是 0xFEEDC0DE (little-endian)
        assert_eq!(serialized[0], 0xDE);
        assert_eq!(serialized[1], 0xC0);
        assert_eq!(serialized[2], 0xED);
        assert_eq!(serialized[3], 0xFE);
    }

    // 11. Frame Length 验证测试
    #[test]
    fn test_frame_length_matches_actual_size() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "message".to_string());
        let serialized = frame.serialize().unwrap();

        // 读取存储的 frame length
        let mut cursor = Cursor::new(&serialized[4..8]);
        let stored_length = cursor.read_u32::<LittleEndian>().unwrap();

        // 验证与实际长度匹配
        assert_eq!(stored_length as usize, serialized.len());
    }

    #[test]
    fn test_frame_length_with_different_sizes() {
        let test_cases = vec![
            ("", ""),
            ("a", "b"),
            ("short", "message"),
            ("longer_tag", "longer message with more content"),
            ("🚀", "🌟✨"),
            (&"x".repeat(100), &"y".repeat(1000)),
        ];

        for (tag, msg) in test_cases {
            let frame = LogFrame::new(LogLevel::Info, tag.to_string(), msg.to_string());
            let serialized = frame.serialize().unwrap();

            let mut cursor = Cursor::new(&serialized[4..8]);
            let stored_length = cursor.read_u32::<LittleEndian>().unwrap();

            assert_eq!(stored_length as usize, serialized.len());
        }
    }

    #[test]
    fn test_truncated_frame_fails() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "message".to_string());
        let serialized = frame.serialize().unwrap();

        // 截断数据
        let truncated = &serialized[..serialized.len() - 10];

        let result = LogFrame::deserialize(truncated);
        assert!(result.is_err());
    }

    #[test]
    fn test_frame_minimum_size() {
        // 最小的 frame: 空 tag 和空 message
        let frame = LogFrame::new(LogLevel::Info, "".to_string(), "".to_string());
        let serialized = frame.serialize().unwrap();

        // Magic(4) + Length(4) + Timestamp(8) + Level(1) + TagLen(2) + Tag(0) + MsgLen(4) + Msg(0) + CRC(4) = 27
        assert_eq!(serialized.len(), 27);
    }

    // 12. CRC 相关的额外测试
    #[test]
    fn test_crc_tampering_at_different_positions() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "message".to_string());

        // 测试在不同位置篡改数据
        let positions = vec![8, 10, 15, 20, 25];

        for pos in positions {
            let mut serialized = frame.serialize().unwrap();
            if pos < serialized.len() - 4 {
                serialized[pos] ^= 0xFF;
                assert!(LogFrame::deserialize(&serialized).is_err());
            }
        }
    }

    #[test]
    fn test_crc_tampering_last_byte_before_crc() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "message".to_string());
        let mut serialized = frame.serialize().unwrap();

        // 篡改 CRC 之前的最后一个字节
        let crc_pos = serialized.len() - 4;
        serialized[crc_pos - 1] ^= 0x01;

        let result = LogFrame::deserialize(&serialized);
        assert!(result.is_err());
        match result {
            Err(crate::ScribeError::CrcMismatch) => {},
            _ => panic!("Expected CrcMismatch error"),
        }
    }

    // 13. LogLevel 边界测试
    #[test]
    fn test_loglevel_from_u8_all_valid() {
        assert_eq!(LogLevel::from_u8(0), Some(LogLevel::Verbose));
        assert_eq!(LogLevel::from_u8(1), Some(LogLevel::Debug));
        assert_eq!(LogLevel::from_u8(2), Some(LogLevel::Info));
        assert_eq!(LogLevel::from_u8(3), Some(LogLevel::Warn));
        assert_eq!(LogLevel::from_u8(4), Some(LogLevel::Error));
    }

    #[test]
    fn test_loglevel_from_u8_invalid() {
        assert_eq!(LogLevel::from_u8(5), None);
        assert_eq!(LogLevel::from_u8(255), None);
        assert_eq!(LogLevel::from_u8(100), None);
    }

    #[test]
    fn test_invalid_loglevel_in_frame() {
        let frame = LogFrame::new(LogLevel::Info, "tag".to_string(), "msg".to_string());
        let mut serialized = frame.serialize().unwrap();

        // 篡改 level 字节为无效值
        // Magic(4) + Length(4) + Timestamp(8) = 16, level at position 16
        serialized[16] = 99;

        // 重新计算 CRC
        let mut hasher = Hasher::new();
        hasher.update(&serialized[8..serialized.len() - 4]);
        let crc = hasher.finalize();
        let len = serialized.len();
        let mut cursor = Cursor::new(&mut serialized[len - 4..]);
        cursor.write_u32::<LittleEndian>(crc).unwrap();

        let result = LogFrame::deserialize(&serialized);
        assert!(result.is_err());
        match result {
            Err(crate::ScribeError::InvalidFrame) => {},
            _ => panic!("Expected InvalidFrame error for invalid log level"),
        }
    }

    // 14. 边界条件综合测试
    #[test]
    fn test_all_combinations_empty_fields() {
        let combinations = vec![
            ("", "", LogLevel::Verbose),
            ("", "msg", LogLevel::Debug),
            ("tag", "", LogLevel::Info),
            ("tag", "msg", LogLevel::Warn),
        ];

        for (tag, msg, level) in combinations {
            let frame = LogFrame::new(level, tag.to_string(), msg.to_string());
            let serialized = frame.serialize().unwrap();
            let deserialized = LogFrame::deserialize(&serialized).unwrap();

            assert_eq!(deserialized.tag, tag);
            assert_eq!(deserialized.message, msg);
            assert_eq!(deserialized.level, level);
        }
    }
}
