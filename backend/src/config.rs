use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::cache::CacheConfig;
use crate::scorer::ScorerConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub enabled: bool,
    /// Session inactivity timeout in seconds.
    pub ttl_seconds: u64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    DeepSeek,
    XAI,
    CustomOpenAI, // For generic OpenAI compatible
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Subscription, // User has a paid subscription (e.g., Claude Pro)
    Cheap,        // Cheap pay-per-token (e.g., DeepSeek)
    Free,         // Free tier (e.g., Gemini Free)
    PayPerRequest, // x402 / micropayment
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub input_cost_per_1m: f64,
    pub output_cost_per_1m: f64,
    pub context_window: u32,
    pub supports_vision: bool,
    pub supports_function_calling: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub tier: Tier,
    pub enabled: bool,
    pub priority: u8, // Higher priority tries first within same tier
    pub models: Vec<Model>,
}

/// Maps a complexity tier to a specific model for a routing profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMapping {
    pub model_id: String,
    pub provider_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingProfile {
    pub name: String,
    pub description: String,
    pub allowed_tiers: Vec<Tier>,
    /// Maps complexity tier name ("simple","medium","complex","reasoning") to a specific model+provider.
    /// When set, the router will override the requested model with the mapped one.
    #[serde(default)]
    pub model_mapping: HashMap<String, ModelMapping>,
    /// Separate model mappings used when agentic mode is active (tool use, multi-step tasks).
    #[serde(default)]
    pub agentic_model_mapping: HashMap<String, ModelMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub providers: Vec<Provider>,
    pub profiles: Vec<RoutingProfile>,
    pub active_profile: String,
    #[serde(default)]
    pub scorer: Option<ScorerConfig>,
    #[serde(default)]
    pub cache: Option<CacheConfig>,
    /// Force agentic mode for all requests regardless of detection.
    #[serde(default)]
    pub agentic_mode: bool,
    /// Session persistence configuration.
    #[serde(default)]
    pub session: Option<SessionConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            providers: vec![
                Provider {
                    id: "openai".to_string(),
                    name: "OpenAI".to_string(),
                    provider_type: ProviderType::OpenAI,
                    api_key: None,
                    endpoint: Some("https://api.openai.com/v1/chat/completions".to_string()),
                    tier: Tier::Subscription,
                    enabled: true,
                    priority: 1,
                    models: vec![
                        Model {
                            id: "gpt-4-turbo".to_string(),
                            name: "GPT-4 Turbo".to_string(),
                            input_cost_per_1m: 10.0,
                            output_cost_per_1m: 30.0,
                            context_window: 128000,
                            supports_vision: true,
                            supports_function_calling: true,
                        }
                    ],
                },
                Provider {
                    id: "anthropic".to_string(),
                    name: "Anthropic".to_string(),
                    provider_type: ProviderType::Anthropic,
                    api_key: None,
                    endpoint: Some("https://api.anthropic.com/v1/messages".to_string()),
                    tier: Tier::Subscription,
                    enabled: true,
                    priority: 1,
                    models: vec![
                        Model {
                            id: "claude-3-opus".to_string(),
                            name: "Claude 3 Opus".to_string(),
                            input_cost_per_1m: 15.0,
                            output_cost_per_1m: 75.0,
                            context_window: 200000,
                            supports_vision: true,
                            supports_function_calling: true,
                        }
                    ],
                },
                Provider {
                    id: "deepseek".to_string(),
                    name: "DeepSeek".to_string(),
                    provider_type: ProviderType::DeepSeek,
                    api_key: None,
                    endpoint: Some("https://api.deepseek.com/chat/completions".to_string()),
                    tier: Tier::Cheap,
                    enabled: true,
                    priority: 1,
                    models: vec![
                        Model {
                            id: "deepseek-chat".to_string(),
                            name: "DeepSeek Chat".to_string(),
                            input_cost_per_1m: 0.14,
                            output_cost_per_1m: 0.28,
                            context_window: 128000,
                            supports_vision: false,
                            supports_function_calling: true,
                        }
                    ],
                },
            ],
            profiles: vec![
                RoutingProfile {
                    name: "auto".to_string(),
                    description: "Balanced cost and quality".to_string(),
                    allowed_tiers: vec![Tier::Subscription, Tier::Cheap, Tier::Free, Tier::PayPerRequest],
                    model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "xai/grok-code-fast-1".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "google/gemini-3-pro-preview".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "xai/grok-4-1-fast-reasoning".to_string(), provider_id: "".to_string() }),
                    ]),
                    agentic_model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "claude-haiku-4.5".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "claude-sonnet-4.6".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                    ]),
                },
                RoutingProfile {
                    name: "eco".to_string(),
                    description: "Focus on low cost".to_string(),
                    allowed_tiers: vec![Tier::Free, Tier::Cheap],
                    model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "nvidia/gpt-oss-120b".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "google/gemini-2.5-flash".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "google/gemini-2.5-flash".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "xai/grok-4-1-fast-reasoning".to_string(), provider_id: "".to_string() }),
                    ]),
                    agentic_model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "claude-haiku-4.5".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "claude-sonnet-4.6".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                    ]),
                },
                RoutingProfile {
                    name: "premium".to_string(),
                    description: "Focus on best quality".to_string(),
                    allowed_tiers: vec![Tier::Subscription, Tier::PayPerRequest],
                    model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "openai/gpt-5.2-codex".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "claude-opus-4".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "claude-sonnet-4".to_string(), provider_id: "".to_string() }),
                    ]),
                    agentic_model_mapping: HashMap::from([
                        ("simple".to_string(), ModelMapping { model_id: "claude-haiku-4.5".to_string(), provider_id: "".to_string() }),
                        ("medium".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                        ("complex".to_string(), ModelMapping { model_id: "claude-sonnet-4.6".to_string(), provider_id: "".to_string() }),
                        ("reasoning".to_string(), ModelMapping { model_id: "moonshot/kimi-k2.5".to_string(), provider_id: "".to_string() }),
                    ]),
                },
            ],
            active_profile: "auto".to_string(),
            scorer: None,
            cache: None,
            agentic_mode: false,
            session: None,
        }
    }
}
