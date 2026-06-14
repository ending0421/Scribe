//! Storage layer for memory-mapped log buffers.
//!
//! This module provides high-performance, lock-free log storage using memory-mapped
//! files and double buffering.
//!
//! # Components
//!
//! - [`LogFrame`] - Structured log entry with serialization
//! - [`LogLevel`] - Log severity levels
//! - [`MmapBuffer`] - Lock-free memory-mapped buffer
//! - [`DoubleBufferManager`] - Atomic buffer rotation
//! - [`CleanupPolicy`] - Automatic log file cleanup
//!
//! # Examples
//!
//! ```no_run
//! use scribe::{DoubleBufferManager, LogFrame, LogLevel};
//! use std::path::PathBuf;
//!
//! let mut manager = DoubleBufferManager::new(PathBuf::from("/tmp/logs")).unwrap();
//!
//! // Write logs
//! let frame = LogFrame::new(LogLevel::Info, "app".to_string(), "Started".to_string());
//! let data = frame.serialize().unwrap();
//!
//! let (buffer, idx) = manager.get_active_buffer();
//! manager.increment_active_writers(idx);
//! buffer.write(&data).unwrap();
//! manager.decrement_active_writers(idx);
//! ```

pub mod buffer;
pub mod cleanup;
pub mod frame;
pub mod manager;
pub mod recovery;

pub use buffer::MmapBuffer;
pub use frame::{LogFrame, LogLevel};
pub use manager::DoubleBufferManager;
