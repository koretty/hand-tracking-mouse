//! Inference module design
//! - `system`: ONNX Runtime session lifecycle and inference execution
//! - `struct`: inference data structures
//! - `config`: inference constants
//! - `utils`: tensor preprocessing and landmark parsing helpers

pub mod config;
pub mod r#struct;
pub mod system;
pub mod utils;

pub use config::{LANDMARK_COUNT, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH};
pub use r#struct::{HandLandmarkSession, Landmark3D, RoiRect};
