use anyhow::{Context, Result};
use crate::pipeline::Frame;
use crate::inference::types::Landmark3D;

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
    let plane = out_w * out_h;
    let mut out = vec![0.0_f32; plane * 3];

    for y in 0..out_h {
        let src_y = y * frame.height / out_h;
        for x in 0..out_w {
            let src_x = x * frame.width / out_w;
            let src_idx = (src_y * frame.width + src_x) * 3;
            let dst = y * out_w + x;

            let r = *frame.data.get(src_idx).unwrap_or(&0) as f32 / 255.0;
            let g = *frame.data.get(src_idx + 1).unwrap_or(&0) as f32 / 255.0;
            let b = *frame.data.get(src_idx + 2).unwrap_or(&0) as f32 / 255.0;

            out[dst] = r;
            out[plane + dst] = g;
            out[plane * 2 + dst] = b;
        }
    }

    Ok(out)
}
