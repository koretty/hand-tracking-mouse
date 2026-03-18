use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub preferred_camera_name: Option<String>,
    pub model_path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            preferred_camera_name: None,
            model_path: String::from("models/HandLandmarkDetector.onnx"),
        }
    }
}
