use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread;

use anyhow::Result;

use crate::inference::{
    HandLandmarkSession, Landmark3D, RoiRect, LANDMARK_COUNT, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH,
};

use super::types::{Frame, FrameProcessor};

#[allow(dead_code)]
pub const DEFAULT_ONNX_MODEL_PATH: &str = "models/HandLandmarkDetector.onnx";
const DETECTION_WARMUP_FRAMES: u32 = 2;
const LOST_TO_RESET_ROI: u32 = 4;
const ROI_EXPAND_RATIO: f32 = 1.5;
const SMOOTH_ALPHA: f32 = 0.35;
const MIN_BBOX_RATIO_TRACK: f32 = 0.035;
const MIN_BBOX_RATIO_SCAN: f32 = 0.045;
const MAX_BBOX_RATIO: f32 = 0.85;
const MIN_SEGMENT_RATIO: f32 = 0.003;
const MAX_SEGMENT_RATIO: f32 = 0.42;
const MIN_PALM_AREA_RATIO: f32 = 0.00045;

// MediaPipe Hands と同等の 21 点接続順。
const HAND_CONNECTIONS: [(usize, usize); 21] = [
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

pub struct HandTrackingProcessor {
    frame_count: u64,
    error_count: u32,
    request_tx: SyncSender<Frame>,
    result_rx: Receiver<WorkerResult>,
    detected_streak: u32,
    smoothed_landmarks: Option<Vec<Landmark3D>>,
    last_valid_landmarks: Option<Vec<Landmark3D>>,
}

#[derive(Clone)]
struct WorkerState {
    roi: Option<RoiRect>,
    lost_count: u32,
    prev_center: Option<(f32, f32)>,
    center_stuck_count: u32,
    valid_streak: u32,
}

struct WorkerResult {
    landmarks: Option<Vec<Landmark3D>>,
    error: Option<String>,
}

impl HandTrackingProcessor {
    pub fn new(model_path: &Path) -> Result<Self> {
        // 起動時にモデルロード可否だけ先に検証して、既存挙動を保つ。
        HandLandmarkSession::from_model_file(model_path)?;

        let (request_tx, request_rx) = mpsc::sync_channel::<Frame>(1);
        let (result_tx, result_rx) = mpsc::channel::<WorkerResult>();
        spawn_inference_worker(model_path.to_path_buf(), request_rx, result_tx);

        Ok(Self {
            frame_count: 0,
            error_count: 0,
            request_tx,
            result_rx,
            detected_streak: 0,
            smoothed_landmarks: None,
            last_valid_landmarks: None,
        })
    }

    fn absorb_worker_results(&mut self) {
        while let Ok(result) = self.result_rx.try_recv() {
            if let Some(err) = result.error {
                self.error_count = self.error_count.saturating_add(1);
                if self.error_count % 30 == 1 {
                    eprintln!("推論ワーカーエラー(継続): {err}");
                }
                self.detected_streak = 0;
                self.last_valid_landmarks = None;
                self.smoothed_landmarks = None;
                continue;
            }

            self.error_count = 0;
            match result.landmarks {
                Some(lm) => {
                    self.detected_streak = self.detected_streak.saturating_add(1);
                    self.last_valid_landmarks = Some(lm);
                }
                None => {
                    self.detected_streak = 0;
                    self.last_valid_landmarks = None;
                    self.smoothed_landmarks = None;
                }
            }
        }
    }

    fn smooth_landmarks(&mut self, current: &[Landmark3D]) -> Vec<Landmark3D> {
        if let Some(prev) = &self.smoothed_landmarks {
            if prev.len() == current.len() {
                let smoothed: Vec<Landmark3D> = prev
                    .iter()
                    .zip(current.iter())
                    .map(|(p, c)| Landmark3D {
                        x: p.x * (1.0 - SMOOTH_ALPHA) + c.x * SMOOTH_ALPHA,
                        y: p.y * (1.0 - SMOOTH_ALPHA) + c.y * SMOOTH_ALPHA,
                        z: p.z * (1.0 - SMOOTH_ALPHA) + c.z * SMOOTH_ALPHA,
                    })
                    .collect();
                self.smoothed_landmarks = Some(smoothed.clone());
                return smoothed;
            }
        }

        let fresh = current.to_vec();
        self.smoothed_landmarks = Some(fresh.clone());
        fresh
    }
}

pub struct NoopProcessor;

impl FrameProcessor for NoopProcessor {
    fn process(&mut self, frame: Frame) -> Result<Frame> {
        Ok(frame)
    }
}

impl FrameProcessor for HandTrackingProcessor {
    fn process(&mut self, mut frame: Frame) -> Result<Frame> {
        self.frame_count = self.frame_count.saturating_add(1);

        // 推論は別スレッドに投げる。満杯時は古い要求を優先してこのフレームの投下をスキップ。
        match self.request_tx.try_send(frame.clone()) {
            Ok(_) => {}
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {
                self.error_count = self.error_count.saturating_add(1);
                if self.error_count % 30 == 1 {
                    eprintln!("推論ワーカーが停止しています");
                }
            }
        }

        self.absorb_worker_results();

        if self.detected_streak >= DETECTION_WARMUP_FRAMES {
            if let Some(lm) = self.last_valid_landmarks.clone() {
                let stable = self.smooth_landmarks(&lm);
                draw_skeleton(&mut frame, &stable);
            }
        }

        Ok(frame)
    }
}

fn spawn_inference_worker(model_path: PathBuf, request_rx: Receiver<Frame>, result_tx: mpsc::Sender<WorkerResult>) {
    thread::spawn(move || {
        let mut session = match HandLandmarkSession::from_model_file(&model_path) {
            Ok(s) => s,
            Err(e) => {
                let _ = result_tx.send(WorkerResult {
                    landmarks: None,
                    error: Some(format!("モデルロード失敗: {e:#}")),
                });
                return;
            }
        };

        let mut state = WorkerState {
            roi: None,
            lost_count: 0,
            prev_center: None,
            center_stuck_count: 0,
            valid_streak: 0,
        };

        while let Ok(frame) = request_rx.recv() {
            let roi_for_infer = state.roi;
            let infer_result = session.run_on_frame_with_roi(&frame, roi_for_infer);

            let result = match infer_result {
                Ok(landmarks) => {
                    let full_landmarks = remap_landmarks_to_full_frame(
                        &landmarks,
                        roi_for_infer,
                        frame.width,
                        frame.height,
                    );

                    let valid = is_valid_hand_detection(&full_landmarks, frame.width, frame.height, &mut state);

                    if valid {
                        state.lost_count = 0;
                        state.valid_streak = state.valid_streak.saturating_add(1);
                        state.roi = build_next_roi(&full_landmarks, frame.width, frame.height);
                        WorkerResult {
                            landmarks: Some(full_landmarks),
                            error: None,
                        }
                    } else {
                        state.lost_count = state.lost_count.saturating_add(1);
                        state.valid_streak = 0;
                        if state.lost_count >= LOST_TO_RESET_ROI {
                            state.roi = None;
                            state.prev_center = None;
                            state.center_stuck_count = 0;
                        }

                        WorkerResult {
                            landmarks: None,
                            error: None,
                        }
                    }
                }
                Err(e) => WorkerResult {
                    landmarks: None,
                    error: Some(format!("推論失敗: {e:#}")),
                },
            };

            if result_tx.send(result).is_err() {
                break;
            }
        }
    });
}

fn remap_landmarks_to_full_frame(
    landmarks: &[Landmark3D],
    roi: Option<RoiRect>,
    frame_w: usize,
    frame_h: usize,
) -> Vec<Landmark3D> {
    let Some(roi) = roi else {
        return landmarks.to_vec();
    };

    landmarks
        .iter()
        .map(|lm| {
            let local_x = map_coord(lm.x, roi.width as f32, MODEL_INPUT_WIDTH as f32)
                .clamp(0.0, roi.width.saturating_sub(1) as f32);
            let local_y = map_coord(lm.y, roi.height as f32, MODEL_INPUT_HEIGHT as f32)
                .clamp(0.0, roi.height.saturating_sub(1) as f32);

            let full_x = (roi.x as f32 + local_x).clamp(0.0, frame_w.saturating_sub(1) as f32);
            let full_y = (roi.y as f32 + local_y).clamp(0.0, frame_h.saturating_sub(1) as f32);

            Landmark3D {
                x: full_x / frame_w.max(1) as f32,
                y: full_y / frame_h.max(1) as f32,
                z: lm.z,
            }
        })
        .collect()
}

fn build_next_roi(landmarks: &[Landmark3D], frame_w: usize, frame_h: usize) -> Option<RoiRect> {
    let points: Vec<(i32, i32)> = landmarks
        .iter()
        .filter_map(|lm| to_frame_point(*lm, frame_w, frame_h))
        .collect();
    if points.len() < LANDMARK_COUNT / 2 {
        return None;
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for &(x, y) in &points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    let bbox_w = (max_x - min_x).max(1) as f32;
    let bbox_h = (max_y - min_y).max(1) as f32;
    let cx = (min_x + max_x) as f32 * 0.5;
    let cy = (min_y + max_y) as f32 * 0.5;
    let roi_w = (bbox_w * ROI_EXPAND_RATIO).max(frame_w as f32 * 0.2);
    let roi_h = (bbox_h * ROI_EXPAND_RATIO).max(frame_h as f32 * 0.2);

    let x0 = (cx - roi_w * 0.5).floor().clamp(0.0, frame_w.saturating_sub(1) as f32) as usize;
    let y0 = (cy - roi_h * 0.5).floor().clamp(0.0, frame_h.saturating_sub(1) as f32) as usize;
    let x1 = (cx + roi_w * 0.5).ceil().clamp(1.0, frame_w as f32) as usize;
    let y1 = (cy + roi_h * 0.5).ceil().clamp(1.0, frame_h as f32) as usize;

    Some(RoiRect {
        x: x0,
        y: y0,
        width: x1.saturating_sub(x0).max(1),
        height: y1.saturating_sub(y0).max(1),
    })
}

fn is_valid_hand_detection(
    landmarks: &[Landmark3D],
    frame_w: usize,
    frame_h: usize,
    state: &mut WorkerState,
) -> bool {
    if landmarks.len() < LANDMARK_COUNT {
        return false;
    }

    let points: Vec<(i32, i32)> = landmarks
        .iter()
        .filter_map(|lm| to_frame_point(*lm, frame_w, frame_h))
        .collect();
    if points.len() < LANDMARK_COUNT / 2 {
        return false;
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;
    let mut near_border = 0_u32;
    let border_x = (frame_w as f32 * 0.015) as i32;
    let border_y = (frame_h as f32 * 0.015) as i32;

    for &(x, y) in &points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);

        if x <= border_x
            || x >= frame_w.saturating_sub(1) as i32 - border_x
            || y <= border_y
            || y >= frame_h.saturating_sub(1) as i32 - border_y
        {
            near_border += 1;
        }
    }

    let bbox_w = (max_x - min_x).max(0) as usize;
    let bbox_h = (max_y - min_y).max(0) as usize;

    let min_bbox = if state.roi.is_some() {
        MIN_BBOX_RATIO_TRACK
    } else {
        MIN_BBOX_RATIO_SCAN
    };

    if bbox_w < (frame_w as f32 * min_bbox) as usize || bbox_h < (frame_h as f32 * min_bbox) as usize {
        return false;
    }

    if bbox_w > (frame_w as f32 * MAX_BBOX_RATIO) as usize || bbox_h > (frame_h as f32 * MAX_BBOX_RATIO) as usize {
        return false;
    }

    if near_border as usize > LANDMARK_COUNT * 3 / 4 {
        return false;
    }

    let frame_diag = ((frame_w * frame_w + frame_h * frame_h) as f32).sqrt().max(1.0);
    let palm_width = point_distance(&points, 5, 17);
    let wrist_to_middle = point_distance(&points, 0, 9);

    if palm_width < frame_diag * 0.015 || wrist_to_middle < frame_diag * 0.012 {
        return false;
    }

    let palm_area = triangle_area(points[0], points[5], points[17]);
    if palm_area < (frame_w * frame_h) as f32 * MIN_PALM_AREA_RATIO {
        return false;
    }

    let mut valid_segments = 0_u32;
    for (a, b) in HAND_CONNECTIONS {
        let seg = point_distance(&points, a, b);
        if seg >= frame_diag * MIN_SEGMENT_RATIO && seg <= frame_diag * MAX_SEGMENT_RATIO {
            valid_segments += 1;
        }
    }
    if valid_segments < 15 {
        return false;
    }

    let center = (
        (min_x + max_x) as f32 * 0.5 / frame_w.max(1) as f32,
        (min_y + max_y) as f32 * 0.5 / frame_h.max(1) as f32,
    );

    if let Some(prev) = state.prev_center {
        let dx = center.0 - prev.0;
        let dy = center.1 - prev.1;
        let dist = (dx * dx + dy * dy).sqrt();

        // 1フレームで飛びすぎる中心移動は誤検出として除外。
        if dist > 0.55 {
            state.prev_center = Some(center);
            state.center_stuck_count = 0;
            return false;
        }

        // 中央付近で固定され続ける幽霊ランドマークを除外。
        if dist < 0.004 && (0.35..=0.65).contains(&center.0) && (0.35..=0.65).contains(&center.1) {
            state.center_stuck_count = state.center_stuck_count.saturating_add(1);
            if state.center_stuck_count >= 6 {
                state.prev_center = Some(center);
                return false;
            }
        } else {
            state.center_stuck_count = 0;
        }
    }

    state.prev_center = Some(center);
    true
}

fn point_distance(points: &[(i32, i32)], a: usize, b: usize) -> f32 {
    let (x0, y0) = points[a];
    let (x1, y1) = points[b];
    let dx = (x1 - x0) as f32;
    let dy = (y1 - y0) as f32;
    (dx * dx + dy * dy).sqrt()
}

fn triangle_area(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> f32 {
    ((b.0 - a.0) as f32 * (c.1 - a.1) as f32 - (b.1 - a.1) as f32 * (c.0 - a.0) as f32).abs() * 0.5
}

fn draw_skeleton(frame: &mut Frame, landmarks: &[Landmark3D]) {
    let points: Vec<(i32, i32)> = landmarks
        .iter()
        .filter_map(|lm| to_frame_point(*lm, frame.width, frame.height))
        .collect();

    for (a, b) in HAND_CONNECTIONS {
        if let (Some(&p0), Some(&p1)) = (points.get(a), points.get(b)) {
            draw_line_rgb(frame, p0, p1, [0, 255, 0]);
        }
    }

    for &p in &points {
        draw_dot_rgb(frame, p, 2, [255, 80, 80]);
    }
}

fn to_frame_point(lm: Landmark3D, frame_w: usize, frame_h: usize) -> Option<(i32, i32)> {
    if !lm.x.is_finite() || !lm.y.is_finite() {
        return None;
    }

    let x = map_coord(lm.x, frame_w as f32, MODEL_INPUT_WIDTH as f32);
    let y = map_coord(lm.y, frame_h as f32, MODEL_INPUT_HEIGHT as f32);

    let px = x.round().clamp(0.0, frame_w.saturating_sub(1) as f32) as i32;
    let py = y.round().clamp(0.0, frame_h.saturating_sub(1) as f32) as i32;
    Some((px, py))
}

fn map_coord(v: f32, frame_size: f32, model_size: f32) -> f32 {
    if (0.0..=1.2).contains(&v) {
        return v * frame_size;
    }
    if (-1.2..=1.2).contains(&v) {
        return ((v + 1.0) * 0.5) * frame_size;
    }
    v * (frame_size / model_size)
}

fn draw_dot_rgb(frame: &mut Frame, center: (i32, i32), radius: i32, color: [u8; 3]) {
    let (cx, cy) = center;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy <= radius * radius {
                set_pixel_rgb(frame, cx + dx, cy + dy, color);
            }
        }
    }
}

fn draw_line_rgb(frame: &mut Frame, p0: (i32, i32), p1: (i32, i32), color: [u8; 3]) {
    let (mut x0, mut y0) = p0;
    let (x1, y1) = p1;

    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        set_pixel_rgb(frame, x0, y0, color);
        if x0 == x1 && y0 == y1 {
            break;
        }

        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn set_pixel_rgb(frame: &mut Frame, x: i32, y: i32, color: [u8; 3]) {
    if x < 0 || y < 0 {
        return;
    }

    let x = x as usize;
    let y = y as usize;
    if x >= frame.width || y >= frame.height {
        return;
    }

    let idx = (y * frame.width + x) * 3;
    if idx + 2 >= frame.data.len() {
        return;
    }

    frame.data[idx] = color[0];
    frame.data[idx + 1] = color[1];
    frame.data[idx + 2] = color[2];
}