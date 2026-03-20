use anyhow::Result;
use crate::pipeline::Frame;
use crate::inference::types::{Landmark3D, RoiRect};

pub(crate) fn parse_landmarks_xyz_iter<I>(iter: I, max_points: usize) -> Vec<Landmark3D>
where
    I: IntoIterator<Item = f32>,
{
    let mut out = Vec::with_capacity(max_points);
    let mut chunk = [0.0_f32; 3];
    let mut idx = 0;

    for v in iter {
        chunk[idx] = v;
        idx += 1;
        if idx == 3 {
            out.push(Landmark3D {
                x: chunk[0],
                y: chunk[1],
                z: chunk[2],
            });
            if out.len() >= max_points {
                break;
            }
            idx = 0;
        }
    }

    out
}

pub(crate) fn parse_landmarks_from_raw(raw: &[f32], count: usize) -> Vec<Landmark3D> {
    let need = count * 3;
    if raw.len() < need {
        return Vec::new();
    }

    let max_start = raw.len().saturating_sub(need).min(64);
    let mut best_start = 0;
    let mut best_score = i32::MIN;

    for start in 0..=max_start {
        let slice = &raw[start..start + need];
        let mut score = 0_i32;
        for chunk in slice.chunks_exact(3) {
            let x = chunk[0];
            let y = chunk[1];
            if x.is_finite() && y.is_finite() {
                score += 1;
                if (-2.0..=2.0).contains(&x) && (-2.0..=2.0).contains(&y) {
                    score += 2;
                }
                if (0.0..=1.2).contains(&x) && (0.0..=1.2).contains(&y) {
                    score += 1;
                }
            }
        }
        if score > best_score {
            best_score = score;
            best_start = start;
        }
    }

    let best = &raw[best_start..best_start + need];
    parse_landmarks_xyz_iter(best.iter().copied(), count)
}

pub(crate) fn fill_nchw_rgb_f32(frame: &Frame, out_w: usize, out_h: usize) -> Result<Vec<f32>> {
    fill_nchw_rgb_f32_bilinear_with_roi(frame, out_w, out_h, None)
}

pub(crate) fn fill_nchw_rgb_f32_bilinear_with_roi(
    frame: &Frame,
    out_w: usize,
    out_h: usize,
    roi: Option<RoiRect>,
) -> Result<Vec<f32>> {
    if frame.width == 0 || frame.height == 0 {
        anyhow::bail!("不正なフレームサイズです: {}x{}", frame.width, frame.height);
    }

    let roi = roi.unwrap_or(RoiRect {
        x: 0,
        y: 0,
        width: frame.width,
        height: frame.height,
    });

    if roi.width == 0 || roi.height == 0 {
        anyhow::bail!("不正なROIサイズです: {}x{}", roi.width, roi.height);
    }

    let x_end = roi.x.saturating_add(roi.width).min(frame.width);
    let y_end = roi.y.saturating_add(roi.height).min(frame.height);
    if x_end <= roi.x || y_end <= roi.y {
        anyhow::bail!("ROIがフレーム範囲外です: x={}, y={}, w={}, h={}", roi.x, roi.y, roi.width, roi.height);
    }

    let roi_w = x_end - roi.x;
    let roi_h = y_end - roi.y;

    let plane = out_w * out_h;
    let mut out = vec![0.0_f32; plane * 3];

    let scale_x = roi_w as f32 / out_w as f32;
    let scale_y = roi_h as f32 / out_h as f32;

    for y in 0..out_h {
        let src_y = (y as f32 + 0.5) * scale_y - 0.5;
        let y0 = src_y.floor().clamp(0.0, (roi_h.saturating_sub(1)) as f32) as usize;
        let y1 = (y0 + 1).min(roi_h.saturating_sub(1));
        let wy = (src_y - y0 as f32).clamp(0.0, 1.0);

        for x in 0..out_w {
            let src_x = (x as f32 + 0.5) * scale_x - 0.5;
            let x0 = src_x.floor().clamp(0.0, (roi_w.saturating_sub(1)) as f32) as usize;
            let x1 = (x0 + 1).min(roi_w.saturating_sub(1));
            let wx = (src_x - x0 as f32).clamp(0.0, 1.0);

            let dst = y * out_w + x;

            let p00 = get_rgb(frame, roi.x + x0, roi.y + y0);
            let p10 = get_rgb(frame, roi.x + x1, roi.y + y0);
            let p01 = get_rgb(frame, roi.x + x0, roi.y + y1);
            let p11 = get_rgb(frame, roi.x + x1, roi.y + y1);

            let top_r = p00[0] * (1.0 - wx) + p10[0] * wx;
            let top_g = p00[1] * (1.0 - wx) + p10[1] * wx;
            let top_b = p00[2] * (1.0 - wx) + p10[2] * wx;
            let bot_r = p01[0] * (1.0 - wx) + p11[0] * wx;
            let bot_g = p01[1] * (1.0 - wx) + p11[1] * wx;
            let bot_b = p01[2] * (1.0 - wx) + p11[2] * wx;

            let r = top_r * (1.0 - wy) + bot_r * wy;
            let g = top_g * (1.0 - wy) + bot_g * wy;
            let b = top_b * (1.0 - wy) + bot_b * wy;

            out[dst] = r;
            out[plane + dst] = g;
            out[plane * 2 + dst] = b;
        }
    }

    Ok(out)
}

fn get_rgb(frame: &Frame, x: usize, y: usize) -> [f32; 3] {
    let idx = (y * frame.width + x) * 3;
    let r = *frame.data.get(idx).unwrap_or(&0) as f32 / 255.0;
    let g = *frame.data.get(idx + 1).unwrap_or(&0) as f32 / 255.0;
    let b = *frame.data.get(idx + 2).unwrap_or(&0) as f32 / 255.0;
    [r, g, b]
}
