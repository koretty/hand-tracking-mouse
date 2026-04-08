use std::sync::mpsc::{Receiver, SyncSender};
use std::time::Instant;

use anyhow::Result;

use crate::preferences::PipelineConfig;
use crate::inference::{Landmark3D, RoiRect};

#[derive(Clone, Debug)]
pub struct Frame {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

pub trait FrameProcessor {
    fn process(&mut self, frame: Frame) -> Result<Frame>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ClickGesture {
    Left,
    Right,
}

pub struct HandTrackingProcessor {
    pub(super) config: PipelineConfig,
    pub(super) frame_count: u64,
    pub(super) error_count: u32,
    pub(super) request_tx: SyncSender<Frame>,
    pub(super) result_rx: Receiver<WorkerResult>,
    pub(super) detected_streak: u32,
    pub(super) smoothed_landmarks: Option<Vec<Landmark3D>>,
    pub(super) last_valid_landmarks: Option<Vec<Landmark3D>>,
    pub(super) smoothed_cursor_norm: Option<(f32, f32)>,
    pub(super) cursor_target_norm: Option<(f32, f32)>,
    pub(super) cursor_current_norm: Option<(f32, f32)>,
    pub(super) active_click_gesture: Option<ClickGesture>,
    pub(super) last_inference_request_at: Instant,
    pub(super) last_cursor_update_at: Instant,
    pub(super) last_click_at: Instant,
}

#[derive(Clone)]
pub(super) struct WorkerState {
    pub roi: Option<RoiRect>,
    pub lost_count: u32,
    pub prev_center: Option<(f32, f32)>,
    pub center_stuck_count: u32,
    pub valid_streak: u32,
}

pub(super) struct WorkerResult {
    pub landmarks: Option<Vec<Landmark3D>>,
    pub error: Option<String>,
}