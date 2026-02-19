use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingProfile {
    pub name: String,
    pub description: String,
    pub allowed_tiers: Vec<Tier>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub providers: Vec<Provider>,
    pub profiles: Vec<RoutingProfile>,
    pub active_profile: String,
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
                },
                RoutingProfile {
                    name: "eco".to_string(),
                    description: "Focus on low cost".to_string(),
                    allowed_tiers: vec![Tier::Free, Tier::Cheap],
                },
                RoutingProfile {
                    name: "premium".to_string(),
                    description: "Focus on best quality".to_string(),
                    allowed_tiers: vec![Tier::Subscription, Tier::PayPerRequest],
                },
            ],
            active_profile: "auto".to_string(),
        }
    }
}
