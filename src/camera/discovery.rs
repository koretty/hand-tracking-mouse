use anyhow::{Context, Result};
use nokhwa::{
    query,
    utils::{ApiBackend, CameraIndex},
};

#[derive(Clone, Debug)]
pub struct CameraDevice {
    pub display_name: String,
    pub index: CameraIndex,
}

pub fn list_cameras() -> Result<Vec<CameraDevice>> {
    let infos = query(ApiBackend::Auto).context("カメラ情報の取得に失敗しました")?;

    Ok(infos
        .into_iter()
        .map(|info| CameraDevice {
            display_name: info.human_name().to_string(),
            index: info.index().clone(),
        })
        .collect())
}
