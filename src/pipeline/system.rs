use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TrySendError};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::preferences::PipelineConfig;
use crate::inference::HandLandmarkSession;

use super::config::{sanitize_pipeline_config, MAX_INTERP_STEPS};
use super::r#struct::{Frame, FrameProcessor, HandTrackingProcessor, WorkerResult, WorkerState};
use super::utils::{
    build_next_roi, draw_dot_rgb, draw_skeleton, is_valid_hand_detection, move_cursor_normalized,
    remap_landmarks_to_full_frame, to_frame_point,
};

impl HandTrackingProcessor {
    pub fn new(model_path: &Path, pipeline_config: PipelineConfig) -> Result<Self> {
        HandLandmarkSession::from_model_file(model_path)?;
        let config = sanitize_pipeline_config(pipeline_config);

        let (request_tx, request_rx) = mpsc::sync_channel::<Frame>(1);
        let (result_tx, result_rx) = mpsc::channel::<WorkerResult>();
        spawn_inference_worker(model_path.to_path_buf(), config.clone(), request_rx, result_tx);
        let last_inference_request_at = Instant::now() - Duration::from_secs_f32(1.0 / config.inference_hz);

        Ok(Self {
            config,
            frame_count: 0,
            error_count: 0,
            request_tx,
            result_rx,
            detected_streak: 0,
            smoothed_landmarks: None,
            last_valid_landmarks: None,
            smoothed_cursor_norm: None,
            cursor_target_norm: None,
            cursor_current_norm: None,
            last_inference_request_at,
            last_cursor_update_at: Instant::now(),
        })
    }

    fn inference_interval(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.config.inference_hz)
    }

    fn cursor_update_interval(&self) -> Duration {
        Duration::from_secs_f32(1.0 / self.config.cursor_update_hz)
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
                self.smoothed_cursor_norm = None;
                self.cursor_target_norm = None;
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
                    self.smoothed_cursor_norm = None;
                    self.cursor_target_norm = None;
                }
            }
        }
    }

    fn smooth_landmarks(&mut self, current: &[crate::inference::Landmark3D]) -> Vec<crate::inference::Landmark3D> {
        if let Some(prev) = &self.smoothed_landmarks {
            if prev.len() == current.len() {
                let alpha = self.config.landmark_smooth_alpha;
                let smoothed: Vec<crate::inference::Landmark3D> = prev
                    .iter()
                    .zip(current.iter())
                    .map(|(p, c)| crate::inference::Landmark3D {
                        x: p.x * (1.0 - alpha) + c.x * alpha,
                        y: p.y * (1.0 - alpha) + c.y * alpha,
                        z: p.z * (1.0 - alpha) + c.z * alpha,
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

    fn smooth_cursor_target(&mut self, target_x: f32, target_y: f32) -> (f32, f32) {
        let target_x = target_x.clamp(0.0, 1.0);
        let target_y = target_y.clamp(0.0, 1.0);
        let alpha = self.config.cursor_smooth_alpha;

        if let Some((prev_x, prev_y)) = self.smoothed_cursor_norm {
            let next_x = prev_x * (1.0 - alpha) + target_x * alpha;
            let next_y = prev_y * (1.0 - alpha) + target_y * alpha;
            self.smoothed_cursor_norm = Some((next_x, next_y));
            return (next_x, next_y);
        }

        self.smoothed_cursor_norm = Some((target_x, target_y));
        (target_x, target_y)
    }

    fn maybe_request_inference(&mut self, frame: &Frame, now: Instant) {
        if now.duration_since(self.last_inference_request_at) < self.inference_interval() {
            return;
        }

        match self.request_tx.try_send(frame.clone()) {
            Ok(_) => {
                self.last_inference_request_at = now;
            }
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {
                self.error_count = self.error_count.saturating_add(1);
                if self.error_count % 30 == 1 {
                    eprintln!("推論ワーカーが停止しています");
                }
            }
        }
    }

    fn update_cursor_with_interpolation(&mut self, now: Instant) {
        let Some(target) = self.cursor_target_norm else {
            return;
        };

        let dt = now.duration_since(self.last_cursor_update_at);
        if dt < self.cursor_update_interval() {
            return;
        }

        let steps = ((dt.as_secs_f32() * self.config.cursor_update_hz).floor() as u32).clamp(1, MAX_INTERP_STEPS);
        let alpha = self.config.cursor_interp_alpha;
        let mut current = self.cursor_current_norm.unwrap_or(target);
        for _ in 0..steps {
            current.0 = current.0 + (target.0 - current.0) * alpha;
            current.1 = current.1 + (target.1 - current.1) * alpha;
        }
        current.0 = current.0.clamp(0.0, 1.0);
        current.1 = current.1.clamp(0.0, 1.0);
        self.cursor_current_norm = Some(current);
        self.last_cursor_update_at = now;

        if let Err(e) = move_cursor_normalized(current.0, current.1) {
            self.error_count = self.error_count.saturating_add(1);
            if self.error_count % 30 == 1 {
                eprintln!("カーソル移動に失敗しました: {e:#}");
            }
        }
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
        let now = Instant::now();

        self.maybe_request_inference(&frame, now);
        self.absorb_worker_results();

        if self.detected_streak >= self.config.detection_warmup_frames {
            if let Some(lm) = self.last_valid_landmarks.clone() {
                let stable = self.smooth_landmarks(&lm);
                if let Some(&tip) = stable.get(self.config.index_finger_tip) {
                    if let Some((ix, iy)) = to_frame_point(tip, frame.width, frame.height) {
                        let tx = ix as f32 / frame.width.max(1) as f32;
                        let ty = iy as f32 / frame.height.max(1) as f32;
                        let (mx, my) = self.smooth_cursor_target(tx, ty);
                        self.cursor_target_norm = Some((mx, my));
                        draw_dot_rgb(&mut frame, (ix, iy), 5, [80, 180, 255]);
                    }
                }
                draw_skeleton(&mut frame, &stable);
            }
        }

        self.update_cursor_with_interpolation(now);
        Ok(frame)
    }
}

fn spawn_inference_worker(
    model_path: PathBuf,
    config: PipelineConfig,
    request_rx: Receiver<Frame>,
    result_tx: mpsc::Sender<WorkerResult>,
) {
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

                    let valid = is_valid_hand_detection(
                        &full_landmarks,
                        frame.width,
                        frame.height,
                        &mut state,
                        &config,
                    );

                    if valid {
                        state.lost_count = 0;
                        state.valid_streak = state.valid_streak.saturating_add(1);
                        state.roi = build_next_roi(&full_landmarks, frame.width, frame.height, &config);
                        WorkerResult {
                            landmarks: Some(full_landmarks),
                            error: None,
                        }
                    } else {
                        state.lost_count = state.lost_count.saturating_add(1);
                        state.valid_streak = 0;
                        if state.lost_count >= config.lost_to_reset_roi {
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