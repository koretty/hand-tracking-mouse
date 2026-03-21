//! UI module design
//! - `system`: window lifecycle and UI orchestration
//! - `struct`: UI data structures
//! - `config`: UI constants
//! - `utils`: UI helper functions

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use r#struct::PreviewWindow;
pub use utils::choose_camera;
