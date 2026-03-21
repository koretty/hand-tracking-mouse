//! Camera module design
//! - `system`: camera session lifecycle
//! - `struct`: camera-related data structures
//! - `config`: camera module constants
//! - `utils`: camera query helpers

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use r#struct::{CameraDevice, CameraSession};
pub use utils::list_cameras;
