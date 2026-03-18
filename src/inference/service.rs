use std::path::Path;
use anyhow::{Context, Result};
use ort::{session::{Session, builder::GraphOptimizationLevel}, value::Value};

use crate::pipeline::Frame;
use crate::inference::types::{MODEL_INPUT_WIDTH, MODEL_INPUT_HEIGHT, LANDMARK_COUNT, Landmark3D};
use crate::inference::utils::{fill_nchw_rgb_f32, parse_landmarks_from_raw, parse_landmarks_xyz_iter};

pub struct HandLandmarkSession {
    session: Session,
    logged_output_info: bool,
}

impl HandLandmarkSession {
    pub fn from_model_file(model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            return Err(anyhow::anyhow!("ONNXモデルが見つかりません: {}", model_path.display()));
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

    pub fn run_on_frame(&mut self, frame: &Frame) -> Result<Vec<Landmark3D>> {
        if frame.width == 0 || frame.height == 0 {
            anyhow::bail!("フレームサイズが不正です: {}x{}", frame.width, frame.height);
        }

        let input_data = fill_nchw_rgb_f32(frame, MODEL_INPUT_WIDTH, MODEL_INPUT_HEIGHT).context("入力データの準備に失敗しました")?;

        let input_value = Value::from_array((
            [1, 3, MODEL_INPUT_HEIGHT, MODEL_INPUT_WIDTH],
            input_data,
        ))
        .context("入力テンソル作成に失敗しました")?;

        let outputs = self.session.run(ort::inputs![input_value]).context("ONNX推論の実行に失敗しました")?;

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
