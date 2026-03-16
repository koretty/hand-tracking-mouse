use std::io::{self, Write};

use anyhow::{Context, Result};

use crate::camera::CameraDevice;

pub fn choose_camera(cameras: &[CameraDevice]) -> Result<CameraDevice> {
    println!("利用可能なカメラ一覧:");
    for (i, camera) in cameras.iter().enumerate() {
        println!("  [{}] {}", i + 1, camera.display_name);
    }

    loop {
        print!("使用するカメラ番号を入力してください: ");
        io::stdout().flush().context("標準出力のフラッシュに失敗しました")?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).context("入力の読み取りに失敗しました")?;

        let trimmed = input.trim();
        let Ok(num) = trimmed.parse::<usize>() else {
            println!("数値を入力してください。\n");
            continue;
        };

        if num == 0 || num > cameras.len() {
            println!("範囲外です。1-{} の番号を入力してください。\n", cameras.len());
            continue;
        }

        return Ok(cameras[num - 1].clone());
    }
}
