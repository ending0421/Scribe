//! Pipeline processing for log data transformation.
//!
//! This module provides a flexible pipeline architecture for processing log data
//! through multiple stages. Stages can compress, encrypt, filter, or transform
//! log data in sequence.
//!
//! # Components
//!
//! - [`PipelineStage`] - A single processing step
//! - [`Pipeline`] - Chain of stages
//! - [`Router`] - Conditional pipeline selection
//! - [`LogBatch`] - Data container for pipeline processing
//!
//! # Examples
//!
//! ```
//! use scribe::{Pipeline, Router, LogLevel};
//!
//! // Create a pipeline
//! let pipeline = Pipeline::new();
//!
//! // Create a router with conditional routing
//! let router = Router::new()
//!     .route(|frame| frame.level == LogLevel::Error, pipeline)
//!     .default(Pipeline::new());
//! ```

pub mod router;
pub mod stage;

pub use router::Router;
pub use stage::{Fallback, LogBatch, PipelineStage};
