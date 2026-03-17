use std::path::Path;

use anyhow::{Context, Result};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Value,
};

use crate::pipeline::Frame;

pub const MODEL_INPUT_WIDTH: usize = 256;
pub const MODEL_INPUT_HEIGHT: usize = 256;
pub const LANDMARK_COUNT: usize = 21;

#[derive(Debug, Clone, Copy)]
pub struct Landmark3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct HandLandmarkSession {
    session: Session,
    logged_output_info: bool,
}

impl HandLandmarkSession {
    pub fn from_model_file(model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            anyhow::bail!("ONNXモデルが見つかりません: {}", model_path.display());
        }

        let session = Session::builder()
            .context("ONNXセッションビルダー作成に失敗しました")?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .context("ONNX最適化レベル設定に失敗しました")?
            .with_inter_threads(1)
            .context("ONNXスレッド設定に失敗しました")?
            .commit_from_file(model_path)
            .with_context(|| format!("ONNXモデルのロードに失敗しました: {}", model_path.display()))?;

        Ok(Self {
            session,
            logged_output_info: false,
        })
    }

    pub fn run_dummy(&mut self, input_shape: (usize, usize, usize, usize)) -> Result<Vec<Landmark3D>> {
        let (n, c, h, w) = input_shape;
        let element_count = n
            .checked_mul(c)
            .and_then(|v| v.checked_mul(h))
            .and_then(|v| v.checked_mul(w))
            .context("入力shapeが大きすぎます")?;
        let input_data = vec![0.0_f32; element_count];

        let input_value =
            Value::from_array(([n, c, h, w], input_data)).context("入力テンソル作成に失敗しました")?;
        let outputs = self
            .session
            .run(ort::inputs![input_value])
            .context("ONNX推論の実行に失敗しました")?;

        let tensor = outputs[0]
            .try_extract_array::<f32>()
            .context("ONNX出力テンソル(f32)の取り出しに失敗しました")?;
        Ok(parse_landmarks_xyz_iter(tensor.iter().copied(), LANDMARK_COUNT))
    }

    pub fn run_on_frame(&mut self, frame: &Frame) -> Result<Vec<Landmark3D>> {
        if frame.width == 0 || frame.height == 0 {
            anyhow::bail!("フレームサイズが不正です: {}x{}", frame.width, frame.height);
        }

        let input_data = fill_nchw_rgb_f32(frame, MODEL_INPUT_WIDTH, MODEL_INPUT_HEIGHT)?;

        let input_value = Value::from_array((
            [1, 3, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH],
            input_data,
        ))
        .context("入力テンソル作成に失敗しました")?;

        let outputs = self
            .session
            .run(ort::inputs![input_value])
            .context("ONNX推論の実行に失敗しました")?;

        if !self.logged_output_info {
            eprintln!("ONNX出力情報:");
            for (name, value) in outputs.iter() {
                match value.try_extract_array::<f32>() {
                    Ok(arr) => eprintln!("  - {name}: f32 elements={}", arr.len()),
                    Err(_) => eprintln!("  - {name}: non-f32 or non-tensor"),
                }
            }
            self.logged_output_info = true;
        }

        let mut best_raw: Option<Vec<f32>> = None;
        for (_, value) in outputs.iter() {
            let Ok(arr) = value.try_extract_array::<f32>() else {
                continue;
            };

            let raw: Vec<f32> = arr.iter().copied().collect();
            if raw.len() < LANDMARK_COUNT * 3 {
                continue;
            }

            let should_replace = best_raw
                .as_ref()
                .is_none_or(|current| raw.len() > current.len());
            if should_replace {
                best_raw = Some(raw);
            }
        }

        let best_raw = best_raw.context("ランドマーク候補となるf32出力テンソルが見つかりません")?;
        Ok(parse_landmarks_from_raw(&best_raw, LANDMARK_COUNT))
    }
}

pub fn parse_landmarks_xyz(flat: &[f32]) -> Vec<Landmark3D> {
    flat.chunks_exact(3)
        .map(|chunk| Landmark3D {
            x: chunk[0],
            y: chunk[1],
            z: chunk[2],
        })
        .collect()
}

fn parse_landmarks_xyz_iter<I>(iter: I, max_points: usize) -> Vec<Landmark3D>
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

fn parse_landmarks_from_raw(raw: &[f32], count: usize) -> Vec<Landmark3D> {
    let need = count * 3;
    if raw.len() < need {
        return Vec::new();
    }

    // モデルによっては先頭にスコア等が入ることがあるため、先頭64要素以内で最も妥当な開始位置を探す。
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

fn fill_nchw_rgb_f32(frame: &Frame, out_w: usize, out_h: usize) -> Result<Vec<f32>> {
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
