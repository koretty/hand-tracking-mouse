//! Pipeline module design
//! - `system`: processing flow orchestration and worker lifecycle
//! - `struct`: shared data structures and runtime state
//! - `config`: constants and config sanitization
//! - `utils`: pure helpers for geometry, validation, drawing, and OS cursor bridge

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use r#struct::{Frame, FrameProcessor, HandTrackingProcessor};
pub use system::NoopProcessor;
