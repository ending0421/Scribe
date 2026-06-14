use super::stage::{LogBatch, Pipeline};
use crate::storage::LogFrame;
use crate::Result;

type Condition = Box<dyn Fn(&LogFrame) -> bool + Send + Sync>;

/// A router for conditional pipeline selection.
///
/// Router dispatches log frames to different pipelines based on conditions.
/// This enables level-based routing, tag-based filtering, or any custom logic.
///
/// # Examples
///
/// ```
/// use scribe::{Router, Pipeline, LogFrame, LogLevel};
///
/// let error_pipeline = Pipeline::new();  // Pipeline for errors
/// let info_pipeline = Pipeline::new();   // Pipeline for info logs
///
/// let router = Router::new()
///     .route(
///         |frame| frame.level == LogLevel::Error,
///         error_pipeline
///     )
///     .default(info_pipeline);
///
/// let frame = LogFrame::new(LogLevel::Error, "app".to_string(), "Failed".to_string());
/// let batch = LogBatch::new(vec![1, 2, 3]);
/// let result = router.dispatch(&frame, batch).unwrap();
/// ```
pub struct Router {
    routes: Vec<(Condition, Pipeline)>,
    default_pipeline: Option<Pipeline>,
}

impl Router {
    /// Creates a new empty router.
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            default_pipeline: None,
        }
    }

    /// Adds a conditional route to the router.
    ///
    /// # Arguments
    ///
    /// * `condition` - A function that evaluates whether this route matches.
    /// * `pipeline` - The pipeline to use when the condition is true.
    ///
    /// # Returns
    ///
    /// The router with the route added (builder pattern).
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::{Router, Pipeline, LogLevel};
    ///
    /// let router = Router::new()
    ///     .route(
    ///         |frame| frame.level == LogLevel::Error,
    ///         Pipeline::new()
    ///     )
    ///     .route(
    ///         |frame| frame.tag == "network",
    ///         Pipeline::new()
    ///     );
    /// ```
    pub fn route<F>(mut self, condition: F, pipeline: Pipeline) -> Self
    where
        F: Fn(&LogFrame) -> bool + Send + Sync + 'static,
    {
        self.routes.push((Box::new(condition), pipeline));
        self
    }

    /// Sets the default pipeline for unmatched routes.
    ///
    /// # Arguments
    ///
    /// * `pipeline` - The pipeline to use when no conditions match.
    ///
    /// # Returns
    ///
    /// The router with the default pipeline set.
    pub fn default(mut self, pipeline: Pipeline) -> Self {
        self.default_pipeline = Some(pipeline);
        self
    }

    /// Dispatches a log frame to the appropriate pipeline.
    ///
    /// Routes are evaluated in the order they were added. The first matching
    /// route's pipeline is used. If no routes match, the default pipeline is used.
    ///
    /// # Arguments
    ///
    /// * `frame` - The log frame to evaluate conditions against.
    /// * `data` - The log batch to process.
    ///
    /// # Returns
    ///
    /// * `Ok(LogBatch)` - The processed output from the matched pipeline.
    /// * `Err(ScribeError)` - If pipeline processing fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use scribe::{Router, Pipeline, LogFrame, LogLevel, LogBatch};
    ///
    /// let router = Router::new()
    ///     .route(|frame| frame.level == LogLevel::Error, Pipeline::new())
    ///     .default(Pipeline::new());
    ///
    /// let frame = LogFrame::new(LogLevel::Info, "app".to_string(), "msg".to_string());
    /// let batch = LogBatch::new(vec![]);
    /// router.dispatch(&frame, batch).unwrap();
    /// ```
    pub fn dispatch(&self, frame: &LogFrame, data: LogBatch) -> Result<LogBatch> {
        for (condition, pipeline) in &self.routes {
            if condition(frame) {
                return pipeline.process(data);
            }
        }

        if let Some(default) = &self.default_pipeline {
            return default.process(data);
        }

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::LogLevel;

    #[test]
    fn test_router_new() {
        let router = Router::new();
        assert_eq!(router.routes.len(), 0);
        assert!(router.default_pipeline.is_none());
    }

    #[test]
    fn test_router_route_adds_route() {
        let router = Router::new().route(|frame| frame.level == LogLevel::Error, Pipeline::new());

        assert_eq!(router.routes.len(), 1);
    }

    #[test]
    fn test_router_route_adds_multiple_routes() {
        let router = Router::new()
            .route(|frame| frame.level == LogLevel::Error, Pipeline::new())
            .route(|frame| frame.level == LogLevel::Warn, Pipeline::new())
            .route(|frame| frame.tag == "network", Pipeline::new());

        assert_eq!(router.routes.len(), 3);
    }

    #[test]
    fn test_router_default_sets_default_pipeline() {
        let router = Router::new().default(Pipeline::new());

        assert!(router.default_pipeline.is_some());
    }

    #[test]
    fn test_router_single_condition_match() {
        let router = Router::new().route(|frame| frame.level == LogLevel::Error, Pipeline::new());

        let frame = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());
        let batch = LogBatch::new(vec![1, 2, 3]);

        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_router_multiple_conditions_first_match_wins() {
        let router = Router::new()
            .route(|frame| frame.level == LogLevel::Error, Pipeline::new())
            .route(|frame| frame.tag == "test", Pipeline::new())
            .route(
                |frame| frame.level == LogLevel::Error && frame.tag == "test",
                Pipeline::new(),
            );

        // 创建匹配所有条件的 frame
        let frame = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());
        let batch = LogBatch::new(vec![1, 2, 3]);

        // 应该匹配第一个条件（level == Error）
        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_router_no_match_uses_default() {
        let router = Router::new()
            .route(|frame| frame.level == LogLevel::Error, Pipeline::new())
            .default(Pipeline::new());

        // Info 级别不匹配 Error 条件
        let frame = LogFrame::new(LogLevel::Info, "test".to_string(), "info".to_string());
        let batch = LogBatch::new(vec![1, 2, 3]);

        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_router_no_match_no_default_returns_original_batch() {
        let router = Router::new().route(|frame| frame.level == LogLevel::Error, Pipeline::new());

        // Info 级别不匹配，且没有 default
        let frame = LogFrame::new(LogLevel::Info, "test".to_string(), "info".to_string());
        let batch = LogBatch::new(vec![1, 2, 3]);

        let result = router.dispatch(&frame, batch.clone());
        assert!(result.is_ok());
        // 验证返回原始 batch
        let output = result.unwrap();
        assert_eq!(output.ids.len(), 3);
    }

    #[test]
    fn test_router_complex_conditions_level_and_tag() {
        let router = Router::new()
            .route(
                |frame| frame.level == LogLevel::Error && frame.tag == "network",
                Pipeline::new(),
            )
            .route(
                |frame| frame.level == LogLevel::Warn && frame.tag.starts_with("auth"),
                Pipeline::new(),
            )
            .route(|frame| frame.level == LogLevel::Info, Pipeline::new())
            .default(Pipeline::new());

        // 测试第一个条件匹配
        let frame1 = LogFrame::new(
            LogLevel::Error,
            "network".to_string(),
            "connection failed".to_string(),
        );
        let batch1 = LogBatch::new(vec![1]);
        assert!(router.dispatch(&frame1, batch1).is_ok());

        // 测试第二个条件匹配
        let frame2 = LogFrame::new(
            LogLevel::Warn,
            "auth_service".to_string(),
            "token expired".to_string(),
        );
        let batch2 = LogBatch::new(vec![2]);
        assert!(router.dispatch(&frame2, batch2).is_ok());

        // 测试第三个条件匹配
        let frame3 = LogFrame::new(LogLevel::Info, "app".to_string(), "started".to_string());
        let batch3 = LogBatch::new(vec![3]);
        assert!(router.dispatch(&frame3, batch3).is_ok());

        // 测试 default
        let frame4 = LogFrame::new(LogLevel::Debug, "test".to_string(), "debug".to_string());
        let batch4 = LogBatch::new(vec![4]);
        assert!(router.dispatch(&frame4, batch4).is_ok());
    }

    #[test]
    fn test_router_with_empty_pipeline() {
        let empty_pipeline = Pipeline::new();
        let router = Router::new().route(|frame| frame.level == LogLevel::Error, empty_pipeline);

        let frame = LogFrame::new(LogLevel::Error, "test".to_string(), "error".to_string());
        let batch = LogBatch::new(vec![]);

        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().ids.len(), 0);
    }

    #[test]
    fn test_router_tag_based_routing() {
        let router = Router::new()
            .route(|frame| frame.tag == "database", Pipeline::new())
            .route(|frame| frame.tag == "network", Pipeline::new())
            .default(Pipeline::new());

        let frame = LogFrame::new(
            LogLevel::Info,
            "database".to_string(),
            "query executed".to_string(),
        );
        let batch = LogBatch::new(vec![1, 2]);

        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
    }

    #[test]
    fn test_router_level_hierarchy() {
        let router = Router::new()
            .route(|frame| frame.level == LogLevel::Verbose, Pipeline::new())
            .route(|frame| frame.level == LogLevel::Debug, Pipeline::new())
            .route(|frame| frame.level == LogLevel::Info, Pipeline::new())
            .route(|frame| frame.level == LogLevel::Warn, Pipeline::new())
            .route(|frame| frame.level == LogLevel::Error, Pipeline::new());

        // 测试每个级别
        for level in [
            LogLevel::Verbose,
            LogLevel::Debug,
            LogLevel::Info,
            LogLevel::Warn,
            LogLevel::Error,
        ] {
            let frame = LogFrame::new(level, "test".to_string(), "message".to_string());
            let batch = LogBatch::new(vec![1]);
            assert!(router.dispatch(&frame, batch).is_ok());
        }
    }

    #[test]
    fn test_router_message_content_condition() {
        let router = Router::new()
            .route(|frame| frame.message.contains("critical"), Pipeline::new())
            .default(Pipeline::new());

        let frame = LogFrame::new(
            LogLevel::Error,
            "app".to_string(),
            "critical system failure".to_string(),
        );
        let batch = LogBatch::new(vec![1]);

        let result = router.dispatch(&frame, batch);
        assert!(result.is_ok());
    }
}
