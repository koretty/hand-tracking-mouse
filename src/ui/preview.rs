use anyhow::{Context, Result};
use minifb::{Key, Window, WindowOptions};

use crate::pipeline::Frame;

const PREVIEW_WIDTH: usize = 640;
const PREVIEW_HEIGHT: usize = 480;

pub struct PreviewWindow {
    title: String,
    window: Option<Window>,
    pixels: Vec<u32>,
}

impl PreviewWindow {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            window: None,
            pixels: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.window.as_ref().is_none_or(|w| w.is_open() && !w.is_key_down(Key::Escape))
    }

    pub fn render_rgb(&mut self, frame: &Frame) -> Result<()> {
        self.ensure_window()?;
        self.pixels = rgb_to_u32_resized(frame, PREVIEW_WIDTH, PREVIEW_HEIGHT)?;

        let window = self.window.as_mut().context("プレビューウィンドウが初期化されていません")?;
        window
            .update_with_buffer(&self.pixels, PREVIEW_WIDTH, PREVIEW_HEIGHT)
            .context("プレビュー更新に失敗しました")
    }

    fn ensure_window(&mut self) -> Result<()> {
        if self.window.is_some() {
            return Ok(());
        }

        self.window = Some(
            Window::new(&self.title, PREVIEW_WIDTH, PREVIEW_HEIGHT, WindowOptions::default())
                .context("プレビューウィンドウ作成に失敗しました")?,
        );

        Ok(())
    }
}

fn rgb_to_u32_resized(frame: &Frame, target_width: usize, target_height: usize) -> Result<Vec<u32>> {
    if frame.width == 0 || frame.height == 0 {
        anyhow::bail!("不正なフレームサイズです: {}x{}", frame.width, frame.height);
    }

    let mut out = Vec::with_capacity(target_width * target_height);
    for y in 0..target_height {
        let src_y = y * frame.height / target_height;
        for x in 0..target_width {
            let src_x = x * frame.width / target_width;
            let idx = (src_y * frame.width + src_x) * 3;

            let r = *frame.data.get(idx).unwrap_or(&0) as u32;
            let g = *frame.data.get(idx + 1).unwrap_or(&0) as u32;
            let b = *frame.data.get(idx + 2).unwrap_or(&0) as u32;
            out.push((r << 16) | (g << 8) | b);
        }
    }

    Ok(out)
}
