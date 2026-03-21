use anyhow::{Context, Result};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{RequestedFormat, RequestedFormatType},
    Camera,
};

use crate::pipeline::Frame;

use super::config::DEFAULT_CAPTURE_MODE;
use super::r#struct::{CameraDevice, CameraSession};

impl CameraSession {
    pub fn open(device: CameraDevice) -> Result<Self> {
        let _ = DEFAULT_CAPTURE_MODE;
        let requested = RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(device.index, requested).context("カメラの初期化に失敗しました")?;
        camera.open_stream().context("カメラストリームを開始できませんでした")?;
        Ok(Self { camera })
    }

    pub fn capture_frame(&mut self) -> Result<Frame> {
        let frame_buffer = self
            .camera
            .frame()
            .context("カメラフレームの読み取りに失敗しました")?;
        let image = frame_buffer
            .decode_image::<RgbFormat>()
            .context("フレームのRGB変換に失敗しました")?;

        Ok(Frame {
            width: image.width() as usize,
            height: image.height() as usize,
            data: image.into_raw(),
        })
    }
}