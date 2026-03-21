pub const CONFIG_FILE_NAME: &str = "camera_config.toml";
pub const DEFAULT_MODEL_PATH: &str = "models/HandLandmarkDetector.onnx";

pub fn default_detection_warmup_frames() -> u32 {
    2
}
pub fn default_lost_to_reset_roi() -> u32 {
    4
}
pub fn default_roi_expand_ratio() -> f32 {
    1.5
}
pub fn default_landmark_smooth_alpha() -> f32 {
    0.35
}
pub fn default_cursor_smooth_alpha() -> f32 {
    0.25
}
pub fn default_cursor_interp_alpha() -> f32 {
    0.42
}
pub fn default_index_finger_tip() -> usize {
    8
}
pub fn default_inference_hz() -> f32 {
    4.0
}
pub fn default_cursor_update_hz() -> f32 {
    30.0
}
pub fn default_min_bbox_ratio_track() -> f32 {
    0.035
}
pub fn default_min_bbox_ratio_scan() -> f32 {
    0.045
}
pub fn default_max_bbox_ratio() -> f32 {
    0.85
}
pub fn default_min_segment_ratio() -> f32 {
    0.003
}
pub fn default_max_segment_ratio() -> f32 {
    0.42
}
pub fn default_min_palm_area_ratio() -> f32 {
    0.00045
}