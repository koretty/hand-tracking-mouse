use std::fs;

use anyhow::{Context, Result};

use super::r#struct::{AppConfig, ConfigStore};
use super::utils::resolve_config_path;

impl ConfigStore {
    pub fn new(app_name: &str) -> Result<Self> {
        let path = resolve_config_path(app_name)?;
        Ok(Self { path })
    }

    pub fn load(&self) -> Result<AppConfig> {
        if !self.path.exists() {
            return Ok(AppConfig::default());
        }

        let text = fs::read_to_string(&self.path)
            .with_context(|| format!("設定ファイルの読み込みに失敗しました: {}", self.path.display()))?;

        toml::from_str::<AppConfig>(&text)
            .with_context(|| format!("設定ファイルの解析に失敗しました: {}", self.path.display()))
    }

    pub fn save(&self, config: &AppConfig) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("設定ディレクトリの作成に失敗しました: {}", parent.display()))?;
        }

        let text = toml::to_string_pretty(config).context("設定データの生成に失敗しました")?;
        fs::write(&self.path, text)
            .with_context(|| format!("設定ファイルの保存に失敗しました: {}", self.path.display()))
    }
}