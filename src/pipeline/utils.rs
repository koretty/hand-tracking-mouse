use anyhow::Result;

use crate::preferences::PipelineConfig;
use crate::inference::{Landmark3D, RoiRect, LANDMARK_COUNT, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH};

use super::config::{
    BORDER_MARGIN_RATIO, CENTER_JUMP_MAX_DIST, CENTER_STUCK_MAX_DIST, CENTER_STUCK_RANGE_MAX,
    CENTER_STUCK_RANGE_MIN, HAND_CONNECTIONS, INDEX_MCP_LANDMARK, MIDDLE_MCP_LANDMARK,
    MIDDLE_TIP_LANDMARK, MIN_VALID_SEGMENTS, PALM_WIDTH_MIN_DIAG_RATIO, PINKY_MCP_LANDMARK,
    THUMB_TIP_LANDMARK, WRIST_LANDMARK, WRIST_TO_MIDDLE_MIN_DIAG_RATIO,
};
use super::r#struct::{ClickGesture, Frame, WorkerState};

pub fn remap_landmarks_to_full_frame(
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

pub fn build_next_roi(
    landmarks: &[Landmark3D],
    frame_w: usize,
    frame_h: usize,
    config: &PipelineConfig,
) -> Option<RoiRect> {
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
    let roi_w = (bbox_w * config.roi_expand_ratio).max(frame_w as f32 * 0.2);
    let roi_h = (bbox_h * config.roi_expand_ratio).max(frame_h as f32 * 0.2);

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

pub(super) fn is_valid_hand_detection(
    landmarks: &[Landmark3D],
    frame_w: usize,
    frame_h: usize,
    state: &mut WorkerState,
    config: &PipelineConfig,
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
    let border_x = (frame_w as f32 * BORDER_MARGIN_RATIO) as i32;
    let border_y = (frame_h as f32 * BORDER_MARGIN_RATIO) as i32;

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
        config.min_bbox_ratio_track
    } else {
        config.min_bbox_ratio_scan
    };

    if bbox_w < (frame_w as f32 * min_bbox) as usize || bbox_h < (frame_h as f32 * min_bbox) as usize {
        return false;
    }

    if bbox_w > (frame_w as f32 * config.max_bbox_ratio) as usize
        || bbox_h > (frame_h as f32 * config.max_bbox_ratio) as usize
    {
        return false;
    }

    if near_border as usize > LANDMARK_COUNT * 3 / 4 {
        return false;
    }

    let frame_diag = ((frame_w * frame_w + frame_h * frame_h) as f32).sqrt().max(1.0);
    let palm_width = point_distance(&points, INDEX_MCP_LANDMARK, PINKY_MCP_LANDMARK);
    let wrist_to_middle = point_distance(&points, WRIST_LANDMARK, MIDDLE_MCP_LANDMARK);

    if palm_width < frame_diag * PALM_WIDTH_MIN_DIAG_RATIO
        || wrist_to_middle < frame_diag * WRIST_TO_MIDDLE_MIN_DIAG_RATIO
    {
        return false;
    }

    let palm_area = triangle_area(
        points[WRIST_LANDMARK],
        points[INDEX_MCP_LANDMARK],
        points[PINKY_MCP_LANDMARK],
    );
    if palm_area < (frame_w * frame_h) as f32 * config.min_palm_area_ratio {
        return false;
    }

    let mut valid_segments = 0_u32;
    for (a, b) in HAND_CONNECTIONS {
        let seg = point_distance(&points, a, b);
        if seg >= frame_diag * config.min_segment_ratio && seg <= frame_diag * config.max_segment_ratio {
            valid_segments += 1;
        }
    }
    if valid_segments < MIN_VALID_SEGMENTS {
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

        if dist > CENTER_JUMP_MAX_DIST {
            state.prev_center = Some(center);
            state.center_stuck_count = 0;
            return false;
        }

        if dist < CENTER_STUCK_MAX_DIST
            && (CENTER_STUCK_RANGE_MIN..=CENTER_STUCK_RANGE_MAX).contains(&center.0)
            && (CENTER_STUCK_RANGE_MIN..=CENTER_STUCK_RANGE_MAX).contains(&center.1)
        {
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

pub fn draw_skeleton(frame: &mut Frame, landmarks: &[Landmark3D]) {
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

pub fn to_frame_point(lm: Landmark3D, frame_w: usize, frame_h: usize) -> Option<(i32, i32)> {
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

pub(super) fn detect_click_gesture(
    landmarks: &[Landmark3D],
    active_gesture: Option<ClickGesture>,
    config: &PipelineConfig,
) -> Option<ClickGesture> {
    if landmarks.len() <= MIDDLE_TIP_LANDMARK {
        return None;
    }

    let index_tip_landmark = config.index_finger_tip.min(landmarks.len().saturating_sub(1));
    let Some(hand_scale) = estimate_hand_scale(landmarks) else {
        return None;
    };

    let Some(thumb_index_dist) = landmark_distance(landmarks, THUMB_TIP_LANDMARK, index_tip_landmark) else {
        return None;
    };
    let Some(thumb_middle_dist) = landmark_distance(landmarks, THUMB_TIP_LANDMARK, MIDDLE_TIP_LANDMARK) else {
        return None;
    };

    let left_ratio = thumb_index_dist / hand_scale;
    let right_ratio = thumb_middle_dist / hand_scale;
    let press = config.click_pinch_press_ratio;
    let release = config.click_pinch_release_ratio.max(press + 0.01);

    let left_pressed = left_ratio <= press && right_ratio > release;
    let right_pressed = right_ratio <= press && left_ratio > release;

    match active_gesture {
        Some(ClickGesture::Left) => {
            if left_ratio <= release && right_ratio > press {
                Some(ClickGesture::Left)
            } else if right_pressed {
                Some(ClickGesture::Right)
            } else {
                None
            }
        }
        Some(ClickGesture::Right) => {
            if right_ratio <= release && left_ratio > press {
                Some(ClickGesture::Right)
            } else if left_pressed {
                Some(ClickGesture::Left)
            } else {
                None
            }
        }
        None => {
            if left_pressed {
                Some(ClickGesture::Left)
            } else if right_pressed {
                Some(ClickGesture::Right)
            } else {
                None
            }
        }
    }
}

fn estimate_hand_scale(landmarks: &[Landmark3D]) -> Option<f32> {
    let palm_width = landmark_distance(landmarks, INDEX_MCP_LANDMARK, PINKY_MCP_LANDMARK)?;
    let wrist_to_middle = landmark_distance(landmarks, WRIST_LANDMARK, MIDDLE_MCP_LANDMARK)?;
    Some(palm_width.max(wrist_to_middle).max(0.0001))
}

fn landmark_distance(landmarks: &[Landmark3D], a: usize, b: usize) -> Option<f32> {
    let a = landmarks.get(a)?;
    let b = landmarks.get(b)?;
    if !a.x.is_finite() || !a.y.is_finite() || !b.x.is_finite() || !b.y.is_finite() {
        return None;
    }

    let dx = b.x - a.x;
    let dy = b.y - a.y;
    Some((dx * dx + dy * dy).sqrt())
}

pub fn draw_dot_rgb(frame: &mut Frame, center: (i32, i32), radius: i32, color: [u8; 3]) {
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

#[cfg(target_os = "windows")]
pub(super) fn trigger_mouse_click(gesture: ClickGesture) -> Result<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        mouse_event, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_RIGHTDOWN,
        MOUSEEVENTF_RIGHTUP,
    };

    let (down, up) = match gesture {
        ClickGesture::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP),
        ClickGesture::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP),
    };

    unsafe {
        mouse_event(down, 0, 0, 0, 0);
        mouse_event(up, 0, 0, 0, 0);
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(super) fn trigger_mouse_click(_gesture: ClickGesture) -> Result<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn move_cursor_normalized(x: f32, y: f32) -> Result<()> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SetCursorPos, SM_CXSCREEN, SM_CYSCREEN,
    };

    let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
    let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
    if screen_w <= 0 || screen_h <= 0 {
        anyhow::bail!("画面サイズの取得に失敗しました");
    }

    let px = (x.clamp(0.0, 1.0) * (screen_w.saturating_sub(1)) as f32).round() as i32;
    let py = (y.clamp(0.0, 1.0) * (screen_h.saturating_sub(1)) as f32).round() as i32;

    let ok = unsafe { SetCursorPos(px, py) };
    if ok == 0 {
        anyhow::bail!("SetCursorPos が失敗しました");
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn move_cursor_normalized(_x: f32, _y: f32) -> Result<()> {
    Ok(())
}