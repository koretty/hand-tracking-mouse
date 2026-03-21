use anyhow::Result;

use crate::camera::CameraDevice;
use crate::preferences::AppConfig;
use crate::ui::choose_camera;

pub fn select_camera(cameras: &[CameraDevice], config: &mut AppConfig) -> Result<CameraDevice> {
    if let Some(saved_name) = &config.preferred_camera_name {
        if let Some(found) = cameras.iter().find(|c| &c.display_name == saved_name) {
            println!("保存済みカメラを使用します: {}", found.display_name);
            return Ok(found.clone());
        }
        println!("保存済みカメラ '{}' が見つからないため、再選択してください。", saved_name);
    }

    let selected = choose_camera(cameras)?;
    config.preferred_camera_name = Some(selected.display_name.clone());
    Ok(selected)
}