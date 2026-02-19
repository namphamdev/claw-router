use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub ttl_seconds: u64,
    pub cache_dir: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_seconds: 3600, // 1 hour
            cache_dir: "cache".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    pub cached_at: u64, // unix timestamp
    pub model: String,
    pub response_body: Vec<u8>,
}

/// Build a deterministic cache key from the request content.
pub fn cache_key(model: &str, messages: &[serde_json::Value], extra: &std::collections::HashMap<String, serde_json::Value>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model.as_bytes());

    // Hash messages
    let messages_str = serde_json::to_string(messages).unwrap_or_default();
    hasher.update(messages_str.as_bytes());

    // Hash deterministic subset of extra params that affect output
    let mut keys: Vec<&String> = extra.keys()
        .filter(|k| matches!(k.as_str(), "temperature" | "top_p" | "max_tokens" | "max_completion_tokens" | "tools" | "tool_choice" | "stop" | "response_format" | "seed"))
        .collect();
    keys.sort();
    for k in keys {
        hasher.update(k.as_bytes());
        let v = serde_json::to_string(&extra[k]).unwrap_or_default();
        hasher.update(v.as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn entry_path(cache_dir: &Path, key: &str) -> PathBuf {
    // Use first 2 chars as subdirectory for filesystem friendliness
    let sub = &key[..2.min(key.len())];
    cache_dir.join(sub).join(format!("{}.json", key))
}

/// Look up a cached response. Returns the raw response bytes if found and not expired.
pub fn get(config: &CacheConfig, key: &str) -> Option<Vec<u8>> {
    if !config.enabled {
        return None;
    }

    let path = entry_path(Path::new(&config.cache_dir), key);
    let data = fs::read(&path).ok()?;
    let entry: CacheEntry = serde_json::from_slice(&data).ok()?;

    // Check TTL
    let age = now_unix().saturating_sub(entry.cached_at);
    if age > config.ttl_seconds {
        // Expired — remove file
        let _ = fs::remove_file(&path);
        return None;
    }

    Some(entry.response_body)
}

/// Store a response in the cache.
pub fn put(config: &CacheConfig, key: &str, model: &str, response_body: &[u8]) {
    if !config.enabled {
        return;
    }

    let path = entry_path(Path::new(&config.cache_dir), key);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let entry = CacheEntry {
        cached_at: now_unix(),
        model: model.to_string(),
        response_body: response_body.to_vec(),
    };

    if let Ok(data) = serde_json::to_vec_pretty(&entry) {
        let _ = fs::write(&path, data);
    }
}
