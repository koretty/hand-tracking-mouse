use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::camera::{list_cameras, CameraSession};
use crate::preferences::ConfigStore;
use crate::pipeline::{FrameProcessor, HandTrackingProcessor, NoopProcessor};
use crate::ui::PreviewWindow;

use super::config::{APP_NAME, PREVIEW_TITLE};
use super::r#struct::FpsCounter;
use super::utils::select_camera;

pub fn run() -> Result<()> {
    let config_store = ConfigStore::new(APP_NAME).context("configパスの作成に失敗しました")?;
    let mut config = config_store.load().context("configの読み込みに失敗しました")?;

    let cameras = list_cameras().context("利用可能なカメラの列挙に失敗しました")?;
    if cameras.is_empty() {
        return Err(anyhow::anyhow!("利用可能なカメラが見つかりませんでした"));
    }
    let selected_camera = select_camera(&cameras, &mut config).context("カメラの選択に失敗しました")?;
    let mut session = CameraSession::open(selected_camera).context("カメラの起動に失敗しました")?;
    config_store.save(&config)?;

    let model_path = Path::new(&config.model_path);
    let mut processor: Box<dyn FrameProcessor> =
        match HandTrackingProcessor::new(model_path, config.pipeline.clone()) {
            Ok(p) => {
                println!("ONNXモデルをロードしました");
                Box::new(p)
            }
            Err(e) => {
                println!("推論エンジンを起動できませんでした: {}", e);
                Box::new(NoopProcessor)
            }
        };

    let mut preview = PreviewWindow::new(PREVIEW_TITLE);
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