use anyhow::{Context, Result};
use std::path::Path;
use std::io::{self, Write};

mod fps;
use fps::FpsCounter;

use crate::camera::{CameraDevice, CameraSession, list_cameras};
use crate::config::{AppConfig, ConfigStore};
use crate::inference::HandLandmarkSession;
use crate::pipeline::{FrameProcessor, NoopProcessor};
use crate::ui::{PreviewWindow, choose_camera};

pub fn run() -> Result<()> {
    let mut config_store = ConfigStore::new("HandTrackingMouse")?;
    let mut config = config_store.load()?;

    // MVPでは起動時に1回だけ推論を回してモデルロードとI/Oを検証する。
    let model_path = Path::new("models/hand_landmark.onnx");
    if model_path.exists() {
        let mut onnx_session = HandLandmarkSession::from_model_file(model_path)
            .context("ONNXモデルセッションの作成に失敗しました")?;
        let landmarks = onnx_session
            .run_dummy((1, 3, 256, 256))
            .context("ONNX推論(ダミー入力)に失敗しました")?;
        println!("ONNX warm-up完了: {} landmarks", landmarks.len());
    } else {
        println!(
            "ONNXモデルが未配置のため推論をスキップしました: {}",
            model_path.display()
        );
    }

    let cameras = list_cameras().context("利用可能なカメラの列挙に失敗しました")?;
    if cameras.is_empty() {
        anyhow::bail!("利用可能なカメラが見つかりませんでした");
    }

    let selected_camera = select_camera(&cameras, &mut config)?;
    config_store.save(&config)?;

    let mut session = CameraSession::open(selected_camera)
        .context("選択したカメラを開けませんでした")?;

    let mut processor = NoopProcessor;
    let mut preview = PreviewWindow::new("HandTrackingMouse - Camera Preview");
    let mut fps = FpsCounter::new();

    while preview.is_open() {
        let frame = session.capture_frame().context("カメラフレームの取得に失敗しました")?;
        let frame = processor.process(frame)?;
        preview.render_rgb(&frame)?;

        fps.tick();
        print!("\rFPS: {:>5.1}", fps.current_fps());
        io::stdout().flush().context("FPS表示のフラッシュに失敗しました")?;
    }

    println!();

    Ok(())
}

fn select_camera(cameras: &[CameraDevice], config: &mut AppConfig) -> Result<CameraDevice> {
    if let Some(saved_name) = &config.preferred_camera_name {
        if let Some(found) = cameras.iter().find(|c| &c.display_name == saved_name) {
            println!("保存済みカメラを使用します: {}", found.display_name);
            return Ok(found.clone());
        }

        println!(
            "保存済みカメラ '{}' が見つからないため、再選択してください。",
            saved_name
        );
    }

    let selected = choose_camera(cameras)?;
    config.preferred_camera_name = Some(selected.display_name.clone());
    Ok(selected)
}
