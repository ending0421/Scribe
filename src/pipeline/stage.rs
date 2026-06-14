use crate::Result;

/// Fallback strategy when a pipeline stage fails.
///
/// Determines how the pipeline should proceed after a stage error.
pub enum Fallback {
    /// Abort the entire pipeline with an error.
    Abort,
    /// Skip the failed stage and continue with the original data.
    Skip,
    /// Continue processing with the original data.
    Continue,
}

/// A batch of log data for pipeline processing.
///
/// LogBatch wraps binary log data as it flows through pipeline stages.
///
/// # Examples
///
/// ```
/// use scribe::LogBatch;
///
/// let batch = LogBatch::new(vec![1, 2, 3, 4]);
/// assert_eq!(batch.size(), 4);
/// ```
#[derive(Clone)]
pub struct LogBatch {
    /// The binary log data.
    pub data: Vec<u8>,
}

impl LogBatch {
    /// Creates a new LogBatch with data.
    ///
    /// # Arguments
    ///
    /// * `data` - The binary log data.
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Creates an empty LogBatch.
    pub fn empty() -> Self {
        Self { data: Vec::new() }
    }

    /// Returns the size of the batch in bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// A stage in the log processing pipeline.
///
/// PipelineStage defines a processing step that transforms log data.
/// Stages can be chained together in a Pipeline to create complex
/// processing workflows.
///
/// # Examples
///
/// ```
/// use scribe::{PipelineStage, LogBatch, Result};
///
/// struct CompressionStage;
///
/// impl PipelineStage for CompressionStage {
///     fn name(&self) -> &str {
///         "compression"
///     }
///
///     fn process(&self, data: LogBatch) -> Result<LogBatch> {
///         // Compress data here
///         Ok(data)
///     }
/// }
/// ```
pub trait PipelineStage: Send + Sync {
    /// Returns the name of this stage.
    fn name(&self) -> &str;

    /// Processes a batch of log data.
    ///
    /// # Arguments
    ///
    /// * `data` - The input log batch.
    ///
    /// # Returns
    ///
    /// * `Ok(LogBatch)` - The processed output batch.
    /// * `Err(ScribeError)` - If processing fails.
    fn process(&self, data: LogBatch) -> Result<LogBatch>;

    /// Handles errors during processing.
    ///
    /// # Arguments
    ///
    /// * `data` - The original input data.
    /// * `_error` - The error that occurred.
    ///
    /// # Returns
    ///
    /// A Fallback strategy indicating how to proceed.
    fn on_error(&self, data: LogBatch, _error: crate::ScribeError) -> Fallback {
        // 默认实现：跳过失败的 Stage
        Fallback::Skip
    }
}

/// A pipeline for chaining multiple processing stages.
///
/// Pipeline executes stages in sequence, passing the output of each stage
/// as input to the next. If a stage fails, its error handler determines
/// whether to abort, skip, or continue.
///
/// # Examples
///
/// ```
/// use scribe::{Pipeline, PipelineStage, LogBatch, Result};
///
/// struct CompressionStage;
/// struct EncryptionStage;
///
/// impl PipelineStage for CompressionStage {
///     fn name(&self) -> &str { "compression" }
///     fn process(&self, data: LogBatch) -> Result<LogBatch> { Ok(data) }
/// }
///
/// impl PipelineStage for EncryptionStage {
///     fn name(&self) -> &str { "encryption" }
///     fn process(&self, data: LogBatch) -> Result<LogBatch> { Ok(data) }
/// }
///
/// let pipeline = Pipeline::new()
///     .add_stage(Box::new(CompressionStage))
///     .add_stage(Box::new(EncryptionStage));
///
/// let input = LogBatch::new(vec![1, 2, 3]);
/// let output = pipeline.process(input).unwrap();
/// ```
pub struct Pipeline {
    stages: Vec<Box<dyn PipelineStage>>,
}

impl Pipeline {
    /// Creates a new empty pipeline.
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    /// Adds a stage to the pipeline.
    ///
    /// # Arguments
    ///
    /// * `stage` - A boxed pipeline stage.
    ///
    /// # Returns
    ///
    /// The pipeline with the stage added (builder pattern).
    pub fn add_stage(mut self, stage: Box<dyn PipelineStage>) -> Self {
        self.stages.push(stage);
        self
    }

    /// Processes data through all stages in sequence.
    ///
    /// # Arguments
    ///
    /// * `data` - The input log batch.
    ///
    /// # Returns
    ///
    /// * `Ok(LogBatch)` - The final processed output.
    /// * `Err(ScribeError)` - If a stage aborts the pipeline.
    pub fn process(&self, mut data: LogBatch) -> Result<LogBatch> {
        for stage in &self.stages {
            let original_data = data.clone(); // 保留原始数据副本
            match stage.process(data) {
                Ok(result) => {
                    data = result;
                }
                Err(e) => {
                    match stage.on_error(original_data.clone(), e) {
                        Fallback::Abort => {
                            return Err(crate::ScribeError::Mmap("Pipeline aborted".to_string()));
                        }
                        Fallback::Skip => {
                            // 跳过这个 Stage，继续使用原数据
                            data = original_data;
                            continue;
                        }
                        Fallback::Continue => {
                            // 继续使用原数据
                            data = original_data;
                            continue;
                        }
                    }
                }
            }
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestStage {
        name: String,
        should_fail: bool,
        fallback: Option<Fallback>,
    }

    impl TestStage {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                should_fail: false,
                fallback: None,
            }
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }

        fn with_fallback(mut self, fallback: Fallback) -> Self {
            self.fallback = Some(fallback);
            self
        }
    }

    impl PipelineStage for TestStage {
        fn name(&self) -> &str {
            &self.name
        }

        fn process(&self, data: LogBatch) -> Result<LogBatch> {
            if self.should_fail {
                Err(crate::ScribeError::Mmap("Test error".to_string()))
            } else {
                // Append stage name to data to track processing order
                let mut new_data = data.data.clone();
                new_data.extend(self.name.as_bytes());
                Ok(LogBatch::new(new_data))
            }
        }

        fn on_error(&self, data: LogBatch, error: crate::ScribeError) -> Fallback {
            if let Some(ref fallback) = self.fallback {
                match fallback {
                    Fallback::Abort => Fallback::Abort,
                    Fallback::Skip => Fallback::Skip,
                    Fallback::Continue => Fallback::Continue,
                }
            } else {
                Fallback::Skip
            }
        }
    }

    // 1. Pipeline::new() 创建空 Pipeline
    #[test]
    fn test_pipeline_new() {
        let pipeline = Pipeline::new();
        assert_eq!(pipeline.stages.len(), 0);
    }

    // 2. Pipeline::add_stage() 添加 Stage
    #[test]
    fn test_pipeline_add_stage() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(TestStage::new("stage2")))
            .add_stage(Box::new(TestStage::new("stage3")));

        assert_eq!(pipeline.stages.len(), 3);
        assert_eq!(pipeline.stages[0].name(), "stage1");
        assert_eq!(pipeline.stages[1].name(), "stage2");
        assert_eq!(pipeline.stages[2].name(), "stage3");
    }

    // 3. Pipeline 空 Stage 列表处理
    #[test]
    fn test_pipeline_empty_stages() {
        let pipeline = Pipeline::new();
        let batch = LogBatch::new(vec![1, 2, 3, 4]);
        let result = pipeline.process(batch).unwrap();
        assert_eq!(result.data, vec![1, 2, 3, 4]);
    }

    // 4. Pipeline 单个 Stage 处理
    #[test]
    fn test_pipeline_single_stage() {
        let pipeline = Pipeline::new().add_stage(Box::new(TestStage::new("stage1")));

        let batch = LogBatch::new(vec![1, 2, 3]);
        let result = pipeline.process(batch).unwrap();

        // Original data + "stage1"
        let expected = vec![1, 2, 3, 115, 116, 97, 103, 101, 49]; // [1,2,3] + "stage1" in bytes
        assert_eq!(result.data, expected);
    }

    // 5. Pipeline 多个 Stage 串行处理
    #[test]
    fn test_pipeline_multiple_stages_serial() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("A")))
            .add_stage(Box::new(TestStage::new("B")))
            .add_stage(Box::new(TestStage::new("C")));

        let batch = LogBatch::new(vec![1]);
        let result = pipeline.process(batch).unwrap();

        // Original data + "A" + "B" + "C"
        let expected = vec![1, 65, 66, 67]; // [1] + "A" + "B" + "C" in bytes
        assert_eq!(result.data, expected);
    }

    // 6. Fallback::Abort 测试
    #[test]
    fn test_fallback_abort() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(
                TestStage::new("stage2")
                    .with_failure()
                    .with_fallback(Fallback::Abort),
            ))
            .add_stage(Box::new(TestStage::new("stage3")));

        let batch = LogBatch::new(vec![1, 2, 3]);
        let result = pipeline.process(batch);

        assert!(result.is_err());
        if let Err(crate::ScribeError::Mmap(msg)) = result {
            assert_eq!(msg, "Pipeline aborted");
        } else {
            panic!("Expected Pipeline aborted error");
        }
    }

    // 7. Fallback::Skip 测试
    #[test]
    fn test_fallback_skip() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(
                TestStage::new("stage2")
                    .with_failure()
                    .with_fallback(Fallback::Skip),
            ))
            .add_stage(Box::new(TestStage::new("stage3")));

        let batch = LogBatch::new(vec![1]);
        let result = pipeline.process(batch).unwrap();

        // stage1 processed, stage2 skipped (failed), stage3 processed on stage1 output
        // [1] + "stage1" + "stage3"
        let expected = vec![1, 115, 116, 97, 103, 101, 49, 115, 116, 97, 103, 101, 51];
        assert_eq!(result.data, expected);
    }

    // 8. Fallback::Continue 测试
    #[test]
    fn test_fallback_continue() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(
                TestStage::new("stage2")
                    .with_failure()
                    .with_fallback(Fallback::Continue),
            ))
            .add_stage(Box::new(TestStage::new("stage3")));

        let batch = LogBatch::new(vec![1]);
        let result = pipeline.process(batch).unwrap();

        // stage1 processed, stage2 failed but continued, stage3 processed on stage1 output
        // [1] + "stage1" + "stage3"
        let expected = vec![1, 115, 116, 97, 103, 101, 49, 115, 116, 97, 103, 101, 51];
        assert_eq!(result.data, expected);
    }

    // 9. Stage 错误传播测试
    #[test]
    fn test_stage_error_propagation() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(
                TestStage::new("failing_stage")
                    .with_failure()
                    .with_fallback(Fallback::Abort),
            ))
            .add_stage(Box::new(TestStage::new("stage3")));

        let batch = LogBatch::new(vec![1, 2, 3]);
        let result = pipeline.process(batch);

        // Error should propagate and abort pipeline
        assert!(result.is_err());
    }

    // 10. LogBatch::new() 测试
    #[test]
    fn test_logbatch_new() {
        let data = vec![1, 2, 3, 4, 5];
        let batch = LogBatch::new(data.clone());
        assert_eq!(batch.data, data);
    }

    // 11. LogBatch::empty() 测试
    #[test]
    fn test_logbatch_empty() {
        let batch = LogBatch::empty();
        assert_eq!(batch.data.len(), 0);
        assert!(batch.data.is_empty());
    }

    // 12. LogBatch::size() 测试
    #[test]
    fn test_logbatch_size() {
        let batch_empty = LogBatch::empty();
        assert_eq!(batch_empty.size(), 0);

        let batch_small = LogBatch::new(vec![1, 2, 3]);
        assert_eq!(batch_small.size(), 3);

        let batch_large = LogBatch::new(vec![0; 1024]);
        assert_eq!(batch_large.size(), 1024);
    }

    // Additional test: existing test compatibility
    #[test]
    fn test_pipeline_success() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1")))
            .add_stage(Box::new(TestStage::new("stage2")));

        let batch = LogBatch::new(vec![1, 2, 3]);
        let result = pipeline.process(batch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipeline_skip_on_error() {
        let pipeline = Pipeline::new()
            .add_stage(Box::new(TestStage::new("stage1").with_failure()))
            .add_stage(Box::new(TestStage::new("stage2")));

        let batch = LogBatch::new(vec![1, 2, 3]);
        let result = pipeline.process(batch);
        assert!(result.is_ok());
    }
}
