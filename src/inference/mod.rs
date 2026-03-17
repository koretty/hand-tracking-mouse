use std::path::Path;

use anyhow::{Context, Result};
use ort::{
    session::{Session, builder::GraphOptimizationLevel},
    value::Value,
};

#[derive(Debug, Clone, Copy)]
pub struct Landmark3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub struct HandLandmarkSession {
    session: Session,
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

        Ok(Self { session })
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
        Ok(parse_landmarks_xyz_iter(tensor.iter().copied()))
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

fn parse_landmarks_xyz_iter<I>(iter: I) -> Vec<Landmark3D>
where
    I: IntoIterator<Item = f32>,
{
    let mut out = Vec::new();
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
            idx = 0;
        }
    }

    out
}
