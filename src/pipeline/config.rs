use crate::preferences::PipelineConfig;
use crate::inference::LANDMARK_COUNT;

#[allow(dead_code)]
pub const DEFAULT_ONNX_MODEL_PATH: &str = "models/HandLandmarkDetector.onnx";

pub const MAX_INTERP_STEPS: u32 = 6;
pub const BORDER_MARGIN_RATIO: f32 = 0.015;
pub const PALM_WIDTH_MIN_DIAG_RATIO: f32 = 0.015;
pub const WRIST_TO_MIDDLE_MIN_DIAG_RATIO: f32 = 0.012;
pub const CENTER_JUMP_MAX_DIST: f32 = 0.55;
pub const CENTER_STUCK_MAX_DIST: f32 = 0.004;
pub const CENTER_STUCK_RANGE_MIN: f32 = 0.35;
pub const CENTER_STUCK_RANGE_MAX: f32 = 0.65;
pub const MIN_VALID_SEGMENTS: u32 = 15;
pub const WRIST_LANDMARK: usize = 0;
pub const THUMB_TIP_LANDMARK: usize = 4;
pub const INDEX_MCP_LANDMARK: usize = 5;
pub const MIDDLE_MCP_LANDMARK: usize = 9;
pub const MIDDLE_TIP_LANDMARK: usize = 12;
pub const PINKY_MCP_LANDMARK: usize = 17;

// MediaPipe Hands と同等の 21 点接続順。
pub const HAND_CONNECTIONS: [(usize, usize); 21] = [
    (0, 1),
    (1, 2),
    (2, 3),
    (3, 4),
    (0, 5),
    (5, 6),
    (6, 7),
    (7, 8),
    (5, 9),
    (9, 10),
    (10, 11),
    (11, 12),
    (9, 13),
    (13, 14),
    (14, 15),
    (15, 16),
    (13, 17),
    (0, 17),
    (17, 18),
    (18, 19),
    (19, 20),
];

pub fn sanitize_pipeline_config(mut config: PipelineConfig) -> PipelineConfig {
    config.landmark_smooth_alpha = config.landmark_smooth_alpha.clamp(0.01, 1.0);
    config.cursor_smooth_alpha = config.cursor_smooth_alpha.clamp(0.01, 1.0);
    config.cursor_interp_alpha = config.cursor_interp_alpha.clamp(0.01, 1.0);
    config.click_pinch_press_ratio = config.click_pinch_press_ratio.clamp(0.05, 1.5);
    config.click_pinch_release_ratio = config
        .click_pinch_release_ratio
        .clamp(config.click_pinch_press_ratio + 0.01, 2.0);
    config.click_cooldown_ms = config.click_cooldown_ms.clamp(50, 3000);
    config.inference_hz = config.inference_hz.max(0.5);
    config.cursor_update_hz = config.cursor_update_hz.max(1.0);
    config.index_finger_tip = config.index_finger_tip.min(LANDMARK_COUNT.saturating_sub(1));
    config.detection_warmup_frames = config.detection_warmup_frames.max(1);
    config.lost_to_reset_roi = config.lost_to_reset_roi.max(1);
    config.min_bbox_ratio_track = config.min_bbox_ratio_track.clamp(0.001, 1.0);
    config.min_bbox_ratio_scan = config.min_bbox_ratio_scan.clamp(0.001, 1.0);
    config.max_bbox_ratio = config.max_bbox_ratio.clamp(0.05, 1.0);
    config.min_segment_ratio = config.min_segment_ratio.clamp(0.0001, 1.0);
    config.max_segment_ratio = config.max_segment_ratio.clamp(config.min_segment_ratio, 2.0);
    config.min_palm_area_ratio = config.min_palm_area_ratio.clamp(0.00001, 1.0);
    config.roi_expand_ratio = config.roi_expand_ratio.clamp(0.5, 4.0);
    config
}