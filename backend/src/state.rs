use crate::config::Config;
use anyhow::Result;
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub config_path: PathBuf,
}

impl AppState {
    pub async fn new(path: PathBuf) -> Self {
        let config = if path.exists() {
            let content = fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            config_path: path,
        }
    }

    pub async fn save(&self) -> Result<()> {
        let config = self.config.read().await;
        let content = serde_json::to_string_pretty(&*config)?;
        fs::write(&self.config_path, content)?;
        Ok(())
    }

    pub async fn get_config(&self) -> Config {
        let config = self.config.read().await;
        config.clone()
    }

    pub async fn update_config(&self, new_config: Config) -> Result<()> {
        {
            let mut config = self.config.write().await;
            *config = new_config;
        }
        self.save().await
    }
}
