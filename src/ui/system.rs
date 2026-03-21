use anyhow::{Context, Result};
use minifb::{Key, Window, WindowOptions};

use crate::pipeline::Frame;

use super::config::{PREVIEW_HEIGHT, PREVIEW_WIDTH};
use super::r#struct::PreviewWindow;
use super::utils::rgb_to_u32_resized;

impl PreviewWindow {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            window: None,
            pixels: Vec::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.window
            .as_ref()
            .is_none_or(|w| w.is_open() && !w.is_key_down(Key::Escape))
    }

    pub fn render_rgb(&mut self, frame: &Frame) -> Result<()> {
        self.ensure_window()?;
        self.pixels = rgb_to_u32_resized(frame, PREVIEW_WIDTH, PREVIEW_HEIGHT)?;

        let window = self
            .window
            .as_mut()
            .context("プレビューウィンドウが初期化されていません")?;
        window
            .update_with_buffer(&self.pixels, PREVIEW_WIDTH, PREVIEW_HEIGHT)
            .context("プレビュー更新に失敗しました")
    }

    fn ensure_window(&mut self) -> Result<()> {
        if self.window.is_some() {
            return Ok(());
        }

        self.window = Some(
            Window::new(
                &self.title,
                PREVIEW_WIDTH,
                PREVIEW_HEIGHT,
                WindowOptions::default(),
            )
            .context("プレビューウィンドウ作成に失敗しました")?,
        );

        Ok(())
    }
}