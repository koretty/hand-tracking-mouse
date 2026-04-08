use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::config::{
    default_click_cooldown_ms, default_click_pinch_press_ratio, default_click_pinch_release_ratio,
    default_cursor_interp_alpha, default_cursor_smooth_alpha, default_cursor_update_hz,
    default_detection_warmup_frames, default_index_finger_tip, default_inference_hz,
    default_landmark_smooth_alpha, default_lost_to_reset_roi, default_max_bbox_ratio,
    default_max_segment_ratio, default_min_bbox_ratio_scan, default_min_bbox_ratio_track,
    default_min_palm_area_ratio, default_min_segment_ratio, default_roi_expand_ratio,
    DEFAULT_MODEL_PATH,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub preferred_camera_name: Option<String>,
    pub model_path: String,
    #[serde(default)]
    pub pipeline: PipelineConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    #[serde(default = "default_detection_warmup_frames")]
    pub detection_warmup_frames: u32,
    #[serde(default = "default_lost_to_reset_roi")]
    pub lost_to_reset_roi: u32,
    #[serde(default = "default_roi_expand_ratio")]
    pub roi_expand_ratio: f32,
    #[serde(default = "default_landmark_smooth_alpha")]
    pub landmark_smooth_alpha: f32,
    #[serde(default = "default_cursor_smooth_alpha")]
    pub cursor_smooth_alpha: f32,
    #[serde(default = "default_cursor_interp_alpha")]
    pub cursor_interp_alpha: f32,
    #[serde(default = "default_click_pinch_press_ratio")]
    pub click_pinch_press_ratio: f32,
    #[serde(default = "default_click_pinch_release_ratio")]
    pub click_pinch_release_ratio: f32,
    #[serde(default = "default_click_cooldown_ms")]
    pub click_cooldown_ms: u32,
    #[serde(default = "default_index_finger_tip")]
    pub index_finger_tip: usize,
    #[serde(default = "default_inference_hz")]
    pub inference_hz: f32,
    #[serde(default = "default_cursor_update_hz")]
    pub cursor_update_hz: f32,
    #[serde(default = "default_min_bbox_ratio_track")]
    pub min_bbox_ratio_track: f32,
    #[serde(default = "default_min_bbox_ratio_scan")]
    pub min_bbox_ratio_scan: f32,
    #[serde(default = "default_max_bbox_ratio")]
    pub max_bbox_ratio: f32,
    #[serde(default = "default_min_segment_ratio")]
    pub min_segment_ratio: f32,
    #[serde(default = "default_max_segment_ratio")]
    pub max_segment_ratio: f32,
    #[serde(default = "default_min_palm_area_ratio")]
    pub min_palm_area_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct ConfigStore {
    pub(super) path: PathBuf,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            preferred_camera_name: None,
            model_path: String::from(DEFAULT_MODEL_PATH),
            pipeline: PipelineConfig::default(),
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            detection_warmup_frames: default_detection_warmup_frames(),
            lost_to_reset_roi: default_lost_to_reset_roi(),
            roi_expand_ratio: default_roi_expand_ratio(),
            landmark_smooth_alpha: default_landmark_smooth_alpha(),
            cursor_smooth_alpha: default_cursor_smooth_alpha(),
            cursor_interp_alpha: default_cursor_interp_alpha(),
            click_pinch_press_ratio: default_click_pinch_press_ratio(),
            click_pinch_release_ratio: default_click_pinch_release_ratio(),
            click_cooldown_ms: default_click_cooldown_ms(),
            index_finger_tip: default_index_finger_tip(),
            inference_hz: default_inference_hz(),
            cursor_update_hz: default_cursor_update_hz(),
            min_bbox_ratio_track: default_min_bbox_ratio_track(),
            min_bbox_ratio_scan: default_min_bbox_ratio_scan(),
            max_bbox_ratio: default_max_bbox_ratio(),
            min_segment_ratio: default_min_segment_ratio(),
            max_segment_ratio: default_max_segment_ratio(),
            min_palm_area_ratio: default_min_palm_area_ratio(),
        }
    }
}