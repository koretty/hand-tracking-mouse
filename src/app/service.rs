use anyhow::{Context, Result};
use std::path::Path;
use std::io::{self, Write};

use crate::app::types::FpsCounter;
use crate::camera::{CameraDevice, CameraSession, list_cameras};
use crate::config::{AppConfig, ConfigStore};
use crate::pipeline::{FrameProcessor, HandTrackingProcessor, NoopProcessor};
use crate::ui::{PreviewWindow, choose_camera};

pub fn run() -> Result<()> {
    let config_store = ConfigStore::new("HandTrackingMouse").context("configパスの作成に失敗しました")?;
    let mut config = config_store.load().context("configの読み込みに失敗しました")?;

    let cameras = list_cameras().context("利用可能なカメラの列挙に失敗しました")?;
    if cameras.is_empty() {
        return Err(anyhow::anyhow!("利用可能なカメラが見つかりませんでした"));
    }
    let selected_camera = select_camera(&cameras, &mut config).context("カメラの選択に失敗しました")?;
    let mut session = CameraSession::open(selected_camera).context("カメラの起動に失敗しました")?;
    config_store.save(&config)?;

    let model_path = Path::new(&config.model_path);
    let mut processor: Box<dyn FrameProcessor> = match HandTrackingProcessor::new(model_path) {
        Ok(p) => {
            println!("ONNXモデルをロードしました");
            Box::new(p)
        },
        Err(e) => {
            println!("推論エンジンを起動できませんでした: {}", e);
            Box::new(NoopProcessor)
        }
    };

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
        println!("保存済みカメラ '{}' が見つからないため、再選択してください。",saved_name);
    }

    let selected = choose_camera(cameras)?;
    config.preferred_camera_name = Some(selected.display_name.clone());
    Ok(selected)
}
