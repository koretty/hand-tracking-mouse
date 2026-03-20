use anyhow::Result;

use crate::inference::{HandLandmarkSession, LANDMARK_COUNT, Landmark3D, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH};

use super::types::{Frame, FrameProcessor};

const HAND_CONNECTIONS: [(usize, usize); 21] = [
    (0, 1),(1, 2),(2, 3),(3, 4),
    (0, 5),(5, 6),(6, 7),(7, 8),
    (5, 9),(9, 10),(10, 11),(11, 12),
    (9, 13),(13, 14),(14, 15),(15, 16),
    (13, 17),(0, 17),(17, 18),(18, 19),(19, 20)
];

pub struct HandTrackingProcessor {
    session: HandLandmarkSession,
    error_count: u32,
    frame_count: u64,
    missing_streak: u32,
    detected_streak: u32,
    smoothed_landmarks: Option<Vec<Landmark3D>>,
}

impl HandTrackingProcessor {
    pub fn new(model_path: &std::path::Path) -> Result<Self> {
        let session = HandLandmarkSession::from_model_file(model_path)?;
        Ok(Self {
            session,
            error_count: 0,
            frame_count: 0,
            missing_streak: 0,
            detected_streak: 0,
            smoothed_landmarks: None,
        })
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

        let mut mark_missing = || {
            self.missing_streak = self.missing_streak.saturating_add(1);
            self.detected_streak = 0;
            if self.missing_streak >= 2 {
                self.smoothed_landmarks = None;
            }
        };

        match self.session.run_on_frame(&frame) {
            Ok(landmarks) => {
                if landmarks.len() >= LANDMARK_COUNT {
                    if is_plausible_detection(&landmarks, frame.width, frame.height) {
                        self.detected_streak = self.detected_streak.saturating_add(1);
                        self.missing_streak = 0;

                        // 連続で有効検出が取れたときだけ描画して、瞬間的な誤検出を抑える。
                        if self.detected_streak >= 2 {
                            let stable = self.smooth_landmarks(&landmarks);
                            let drawn = draw_skeleton(&mut frame, &stable);
                            if drawn == 0 && self.frame_count % 120 == 1 {
                                let (min_x, max_x, min_y, max_y) = stable.iter().fold(
                                    (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                                    |acc, lm| {
                                        (
                                            acc.0.min(lm.x),
                                            acc.1.max(lm.x),
                                            acc.2.min(lm.y),
                                            acc.3.max(lm.y),
                                        )
                                    },
                                );
                                eprintln!(
                                    "ランドマーク未描画: x=[{min_x:.3},{max_x:.3}] y=[{min_y:.3},{max_y:.3}]"
                                );
                            }
                        }
                    } else {
                        mark_missing();
                    }
                } else {
                    mark_missing();
                    if self.frame_count % 180 == 1 {
                        eprintln!("ランドマーク不足: {} / {}", landmarks.len(), LANDMARK_COUNT);
                    }
                }

                if landmarks.len() < LANDMARK_COUNT && self.frame_count % 120 == 1 {
                        let (min_x, max_x, min_y, max_y) = landmarks.iter().fold(
                            (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY),
                            |acc, lm| {
                                (
                                    acc.0.min(lm.x),
                                    acc.1.max(lm.x),
                                    acc.2.min(lm.y),
                                    acc.3.max(lm.y),
                                )
                            },
                        );
                        eprintln!(
                            "ランドマーク範囲(参考): x=[{min_x:.3},{max_x:.3}] y=[{min_y:.3},{max_y:.3}]"
                        );
                    }
                self.error_count = 0;
            }
            Err(err) => {
                mark_missing();
                self.error_count = self.error_count.saturating_add(1);
                if self.error_count % 30 == 1 {
                    eprintln!("推論エラー(継続): {err:#}");
                }
            }
        }
        Ok(frame)
    }
}

impl HandTrackingProcessor {
    fn smooth_landmarks(&mut self, current: &[Landmark3D]) -> Vec<Landmark3D> {
        const ALPHA: f32 = 0.35;

        if let Some(prev) = &self.smoothed_landmarks {
            if prev.len() == current.len() {
                let smoothed: Vec<Landmark3D> = prev
                    .iter()
                    .zip(current.iter())
                    .map(|(p, c)| Landmark3D {
                        x: p.x * (1.0 - ALPHA) + c.x * ALPHA,
                        y: p.y * (1.0 - ALPHA) + c.y * ALPHA,
                        z: p.z * (1.0 - ALPHA) + c.z * ALPHA,
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

fn is_plausible_detection(landmarks: &[Landmark3D], frame_w: usize, frame_h: usize) -> bool {
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

    let mut coarse_unique = std::collections::HashSet::new();
    for (x, y) in &points {
        coarse_unique.insert((x / 4, y / 4));
    }
    if coarse_unique.len() < 10 {
        return false;
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for (x, y) in &points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    let bbox_w = (max_x - min_x).max(0) as usize;
    let bbox_h = (max_y - min_y).max(0) as usize;

    // 小さすぎるクラスタ（画面中心に潰れた誤検出）を除外する。
    let min_w = (frame_w as f32 * 0.08) as usize;
    let min_h = (frame_h as f32 * 0.08) as usize;
    if bbox_w < min_w || bbox_h < min_h {
        return false;
    }

    let frame_diag = ((frame_w * frame_w + frame_h * frame_h) as f32).sqrt();
    let mut edge_sum = 0.0_f32;
    let mut edge_max = 0.0_f32;
    let mut edge_count = 0_u32;
    for (a, b) in HAND_CONNECTIONS {
        let Some(&(x0, y0)) = points.get(a) else {
            continue;
        };
        let Some(&(x1, y1)) = points.get(b) else {
            continue;
        };

        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        edge_sum += len;
        edge_max = edge_max.max(len);
        edge_count += 1;
    }

    if edge_count < 18 {
        return false;
    }

    let edge_mean = edge_sum / edge_count as f32;
    if edge_mean < frame_diag * 0.01 || edge_mean > frame_diag * 0.25 {
        return false;
    }

    edge_max >= frame_diag * 0.05
}

fn draw_skeleton(frame: &mut Frame, landmarks: &[Landmark3D]) -> usize {
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

    points.len()
}

fn to_frame_point(lm: Landmark3D, frame_w: usize, frame_h: usize) -> Option<(i32, i32)> {
    if !lm.x.is_finite() || !lm.y.is_finite() {
        return None;
    }

    let x = map_coord(lm.x, frame_w as f32, MODEL_INPUT_WIDTH as f32);
    let y = map_coord(lm.y, frame_h as f32, MODEL_INPUT_HEIGHT as f32);

    let px = x.round().clamp(0.0, (frame_w.saturating_sub(1)) as f32) as i32;
    let py = y.round().clamp(0.0, (frame_h.saturating_sub(1)) as f32) as i32;

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
