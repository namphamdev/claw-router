use crate::config::{Config, Provider};
use std::cmp::Ordering;

pub struct Router;

impl Router {
    pub fn route_request(
        config: &Config,
        model_id: &str,
    ) -> Vec<Provider> {
        // 1. Find the active profile
        let profile = config.profiles.iter()
            .find(|p| p.name == config.active_profile)
            .cloned() // Clone to avoid borrow issues
            .unwrap_or_else(|| config.profiles[0].clone());

        // 2. Filter providers
        let mut candidates: Vec<Provider> = config.providers.iter()
            .filter(|p| p.enabled)
            .filter(|p| p.models.iter().any(|m| m.id == model_id))
            .filter(|p| profile.allowed_tiers.contains(&p.tier))
            .cloned()
            .collect();

        // 3. Sort candidates
        // Sorting logic:
        // - Priority 1: Tier order in profile.allowed_tiers (if ordered list matters)
        // - Priority 2: Cost (cheaper first)
        // - Priority 3: Provider Priority (higher first)

        candidates.sort_by(|a, b| {
            let tier_a_idx = profile.allowed_tiers.iter().position(|t| t == &a.tier).unwrap_or(usize::MAX);
            let tier_b_idx = profile.allowed_tiers.iter().position(|t| t == &b.tier).unwrap_or(usize::MAX);

            match tier_a_idx.cmp(&tier_b_idx) {
                Ordering::Equal => {
                    // Same tier preference, check cost
                    let cost_a = a.models.iter().find(|m| m.id == model_id).map(|m| m.input_cost_per_1m).unwrap_or(f64::MAX);
                    let cost_b = b.models.iter().find(|m| m.id == model_id).map(|m| m.input_cost_per_1m).unwrap_or(f64::MAX);

                    match cost_a.partial_cmp(&cost_b).unwrap_or(Ordering::Equal) {
                        Ordering::Equal => {
                            // Same cost, check priority (descending)
                            b.priority.cmp(&a.priority)
                        }
                        ord => ord,
                    }
                }
                ord => ord,
            }
        });

        candidates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderType, Tier, Model, RoutingProfile};

    #[test]
    fn test_routing_logic() {
        let providers = vec![
            Provider {
                id: "p1".to_string(),
                name: "Expensive".to_string(),
                provider_type: ProviderType::OpenAI,
                api_key: None,
                endpoint: None,
                tier: Tier::Subscription,
                enabled: true,
                priority: 1,
                models: vec![
                    Model {
                        id: "gpt-4".to_string(),
                        name: "gpt-4".to_string(),
                        input_cost_per_1m: 30.0,
                        output_cost_per_1m: 60.0,
                        context_window: 8192,
                        supports_vision: false,
                        supports_function_calling: true,
                    }
                ],
            },
            Provider {
                id: "p2".to_string(),
                name: "Cheap".to_string(),
                provider_type: ProviderType::OpenAI,
                api_key: None,
                endpoint: None,
                tier: Tier::Cheap,
                enabled: true,
                priority: 1,
                models: vec![
                    Model {
                        id: "gpt-4".to_string(),
                        name: "gpt-4".to_string(),
                        input_cost_per_1m: 5.0,
                        output_cost_per_1m: 10.0,
                        context_window: 8192,
                        supports_vision: false,
                        supports_function_calling: true,
                    }
                ],
            },
        ];

        let profiles = vec![
            RoutingProfile {
                name: "auto".to_string(),
                description: "auto".to_string(),
                allowed_tiers: vec![Tier::Subscription, Tier::Cheap],
            },
            RoutingProfile {
                name: "eco".to_string(),
                description: "eco".to_string(),
                allowed_tiers: vec![Tier::Cheap],
            },
        ];

        let config = Config {
            providers,
            profiles,
            active_profile: "auto".to_string(),
        };

        // Auto: Subscription (p1) then Cheap (p2)
        let candidates = Router::route_request(&config, "gpt-4");
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "p1");
        assert_eq!(candidates[1].id, "p2");

        // Eco: Cheap (p2) only
        let mut config_eco = config.clone();
        config_eco.active_profile = "eco".to_string();

        let candidates_eco = Router::route_request(&config_eco, "gpt-4");
        assert_eq!(candidates_eco.len(), 1);
        assert_eq!(candidates_eco[0].id, "p2");
    }
}
