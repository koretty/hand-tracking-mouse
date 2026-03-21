//! Preferences module design
//! - `system`: local preference load/save operations
//! - `struct`: persisted config data structures
//! - `config`: default values and constants
//! - `utils`: path/helper routines

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use r#struct::{AppConfig, ConfigStore, PipelineConfig};
