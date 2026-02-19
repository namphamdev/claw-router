use crate::config::Config;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub provider_id: String,
    pub model_id: String,
    pub last_active: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub model: String,
    pub effective_model: Option<String>,
    pub provider: Option<String>,
    pub status: String,        // "success", "error", "no_provider"
    pub status_code: Option<u16>,
    pub duration_ms: u64,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub complexity_tier: Option<String>,
    pub complexity_score: Option<f64>,
    pub error_message: Option<String>,
    pub providers_tried: Vec<String>,
    pub cache_status: Option<String>,
    pub agentic_mode: Option<bool>,
    pub session_id: Option<String>,
    pub session_pinned: Option<bool>,
}

impl RequestLog {
    pub fn new(model: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            model: model.to_string(),
            effective_model: None,
            provider: None,
            status: "pending".to_string(),
            status_code: None,
            duration_ms: 0,
            input_tokens: None,
            output_tokens: None,
            estimated_cost: None,
            complexity_tier: None,
            complexity_score: None,
            error_message: None,
            providers_tried: Vec::new(),
            cache_status: None,
            agentic_mode: None,
            session_id: None,
            session_pinned: None,
        }
    }
}

const MAX_LOGS: usize = 1000;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub config_path: PathBuf,
    pub logs: Arc<RwLock<Vec<RequestLog>>>,
    pub sessions: Arc<RwLock<HashMap<String, SessionEntry>>>,
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
            logs: Arc::new(RwLock::new(Vec::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn add_log(&self, log: RequestLog) {
        let mut logs = self.logs.write().await;
        logs.push(log);
        // Keep only the most recent logs
        if logs.len() > MAX_LOGS {
            let drain_count = logs.len() - MAX_LOGS;
            logs.drain(0..drain_count);
        }
    }

    pub async fn get_logs(&self) -> Vec<RequestLog> {
        let logs = self.logs.read().await;
        logs.clone()
    }

    /// Look up a session. Returns None if expired or not found.
    pub async fn get_session(&self, session_id: &str, ttl_seconds: u64) -> Option<SessionEntry> {
        let sessions = self.sessions.read().await;
        if let Some(entry) = sessions.get(session_id) {
            let age = Utc::now().signed_duration_since(entry.last_active).num_seconds() as u64;
            if age <= ttl_seconds {
                return Some(entry.clone());
            }
        }
        None
    }

    /// Record or update a session pin.
    pub async fn set_session(&self, session_id: String, provider_id: String, model_id: String) {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, SessionEntry {
            provider_id,
            model_id,
            last_active: Utc::now(),
        });
    }

    /// Touch a session to refresh its last_active timestamp.
    pub async fn touch_session(&self, session_id: &str) {
        let mut sessions = self.sessions.write().await;
        if let Some(entry) = sessions.get_mut(session_id) {
            entry.last_active = Utc::now();
        }
    }

    /// Remove expired sessions.
    pub async fn cleanup_sessions(&self, ttl_seconds: u64) {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        sessions.retain(|_, entry| {
            now.signed_duration_since(entry.last_active).num_seconds() as u64 <= ttl_seconds
        });
    }

    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
}
