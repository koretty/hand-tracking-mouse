use anyhow::Context;
use nokhwa::{query, utils::ApiBackend};

use super::r#struct::CameraDevice;

pub fn list_cameras() -> anyhow::Result<Vec<CameraDevice>> {
    let infos = query(ApiBackend::Auto).context("カメラ情報の取得に失敗しました")?;

    Ok(infos
        .into_iter()
        .map(|info| CameraDevice {
            display_name: info.human_name().to_string(),
            index: info.index().clone(),
        })
        .collect())
}