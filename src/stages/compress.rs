use crate::pipeline::{Fallback, LogBatch, PipelineStage};
use crate::Result;

pub struct CompressStage {
    level: i32,
}

impl CompressStage {
    pub fn new(level: i32) -> Self {
        Self { level }
    }

    pub fn zstd(level: i32) -> Self {
        Self::new(level)
    }
}

impl PipelineStage for CompressStage {
    fn name(&self) -> &str {
        "compress"
    }

    fn process(&self, data: LogBatch) -> Result<LogBatch> {
        match zstd::encode_all(&data.data[..], self.level) {
            Ok(compressed) => Ok(LogBatch::new(compressed)),
            Err(e) => Err(crate::ScribeError::Compression(e.to_string())),
        }
    }

    fn on_error(&self, data: LogBatch, _error: crate::ScribeError) -> Fallback {
        // 压缩失败时跳过，使用原始数据
        Fallback::Skip
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress() {
        let stage = CompressStage::zstd(3);
        let data = vec![b'a'; 1000];
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();

        // 压缩后应该更小
        assert!(result.size() < data.len());

        // 验证可以解压缩
        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_level_1() {
        let stage = CompressStage::zstd(1);
        let data = vec![b'x'; 1000];
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();
        assert!(result.size() < data.len());

        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_level_9() {
        let stage = CompressStage::zstd(9);
        let data = vec![b'y'; 1000];
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();
        assert!(result.size() < data.len());

        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_empty_data() {
        let stage = CompressStage::zstd(3);
        let data = vec![];
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();

        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_large_data() {
        let stage = CompressStage::zstd(3);
        // 10MB+ 数据
        let data = vec![b'z'; 10 * 1024 * 1024 + 1000];
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();

        // 高度重复的数据应该压缩得很好
        assert!(result.size() < data.len() / 100);

        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compress_random_data() {
        let stage = CompressStage::zstd(3);
        // 随机数据不容易压缩
        let data: Vec<u8> = (0..1000).map(|i| (i * 123 + 456) as u8).collect();
        let batch = LogBatch::new(data.clone());

        let result = stage.process(batch).unwrap();

        let decompressed = zstd::decode_all(&result.data[..]).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_on_error() {
        let stage = CompressStage::zstd(3);
        let data = vec![1, 2, 3];
        let batch = LogBatch::new(data);
        let error = crate::ScribeError::Compression("test error".to_string());

        let fallback = stage.on_error(batch, error);
        assert!(matches!(fallback, Fallback::Skip));
    }

    #[test]
    fn test_stage_name() {
        let stage = CompressStage::zstd(3);
        assert_eq!(stage.name(), "compress");
    }
}
