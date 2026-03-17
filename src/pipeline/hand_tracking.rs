use std::path::Path;

use anyhow::Result;

use crate::inference::{HandLandmarkSession, LANDMARK_COUNT, Landmark3D, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH};

use super::{Frame, FrameProcessor};

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
    session: HandLandmarkSession,
    error_count: u32,
    frame_count: u64,
}

impl HandTrackingProcessor {
    pub fn new(model_path: &Path) -> Result<Self> {
        let session = HandLandmarkSession::from_model_file(model_path)?;
        Ok(Self {
            session,
            error_count: 0,
            frame_count: 0,
        })
    }
}

impl FrameProcessor for HandTrackingProcessor {
    fn process(&mut self, mut frame: Frame) -> Result<Frame> {
        self.frame_count = self.frame_count.saturating_add(1);
        match self.session.run_on_frame(&frame) {
            Ok(landmarks) => {
                if landmarks.len() >= LANDMARK_COUNT {
                    let drawn = draw_skeleton(&mut frame, &landmarks);
                    if drawn == 0 && self.frame_count % 120 == 1 {
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
                            "ランドマーク未描画: x=[{min_x:.3},{max_x:.3}] y=[{min_y:.3},{max_y:.3}]"
                        );
                    }
                }
                self.error_count = 0;
            }
            Err(err) => {
                self.error_count = self.error_count.saturating_add(1);
                if self.error_count % 30 == 1 {
                    eprintln!("推論エラー(継続): {err:#}");
                }
            }
        }
        Ok(frame)
    }
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
