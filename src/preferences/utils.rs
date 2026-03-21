use std::path::PathBuf;

use anyhow::{Context, Result};

use super::config::CONFIG_FILE_NAME;

pub fn resolve_config_path(app_name: &str) -> Result<PathBuf> {
    let base_dir = dirs::config_dir().context("設定ディレクトリを解決できませんでした")?;
    Ok(base_dir.join(app_name).join(CONFIG_FILE_NAME))
}