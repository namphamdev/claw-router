use crate::config::{Config, Provider, Tier};
use crate::scorer::ComplexityTier;
use std::cmp::Ordering;

pub struct Router;

impl Router {
    /// Parse a "router/<profile>" model name. Returns Some(profile_name) if matched.
    pub fn parse_router_model(model_id: &str) -> Option<&str> {
        model_id.strip_prefix("router/")
    }

    pub fn route_request(
        config: &Config,
        model_id: &str,
        complexity: Option<ComplexityTier>,
        use_agentic: bool,
    ) -> Vec<Provider> {
        Self::route_request_with_profile(config, model_id, complexity, None, use_agentic)
    }

    pub fn route_request_with_profile(
        config: &Config,
        model_id: &str,
        complexity: Option<ComplexityTier>,
        profile_override: Option<&str>,
        use_agentic: bool,
    ) -> Vec<Provider> {
        // 1. Find the profile (override or active)
        let profile_name = profile_override.unwrap_or(&config.active_profile);
        let profile = config.profiles.iter()
            .find(|p| p.name == profile_name)
            .cloned()
            .unwrap_or_else(|| config.profiles[0].clone());

        // Select mapping source: agentic or normal
        let mapping_source = if use_agentic && !profile.agentic_model_mapping.is_empty() {
            &profile.agentic_model_mapping
        } else {
            &profile.model_mapping
        };

        // 2. If the profile has a model_mapping for this complexity tier,
        //    override the requested model_id with the mapped one.
        let effective_model_id = if let Some(c) = complexity {
            let tier_key = match c {
                ComplexityTier::Simple => "simple",
                ComplexityTier::Medium => "medium",
                ComplexityTier::Complex => "complex",
                ComplexityTier::Reasoning => "reasoning",
            };
            if let Some(mapping) = mapping_source.get(tier_key) {
                if !mapping.model_id.is_empty() {
                    mapping.model_id.as_str()
                } else {
                    model_id
                }
            } else {
                model_id
            }
        } else {
            model_id
        };

        // 3. Determine effective allowed tiers
        let effective_tiers = if let Some(c) = complexity {
            let complexity_tiers = default_provider_tiers_for_complexity(c);
            let intersection: Vec<Tier> = profile.allowed_tiers.iter()
                .filter(|t| complexity_tiers.contains(t))
                .cloned()
                .collect();
            if intersection.is_empty() {
                profile.allowed_tiers.clone()
            } else {
                intersection
            }
        } else {
            profile.allowed_tiers.clone()
        };

        // 4. Filter providers: check for the effective model id.
        //    If a model_mapping specified a provider_id, prefer that provider.
        let mapped_provider_id = complexity.and_then(|c| {
            let tier_key = match c {
                ComplexityTier::Simple => "simple",
                ComplexityTier::Medium => "medium",
                ComplexityTier::Complex => "complex",
                ComplexityTier::Reasoning => "reasoning",
            };
            mapping_source.get(tier_key)
                .filter(|m| !m.provider_id.is_empty())
                .map(|m| m.provider_id.as_str())
        });

        let mut candidates: Vec<Provider> = config.providers.iter()
            .filter(|p| p.enabled)
            .filter(|p| p.models.iter().any(|m| m.id == effective_model_id))
            .filter(|p| {
                if let Some(pid) = mapped_provider_id {
                    p.id == pid
                } else {
                    effective_tiers.contains(&p.tier)
                }
            })
            .cloned()
            .collect();

        // 5. If model_mapping redirected to a different model but no providers matched
        //    (e.g., the mapped model doesn't exist in any provider yet),
        //    fall back to the original model_id with standard tier filtering.
        if candidates.is_empty() && effective_model_id != model_id {
            candidates = config.providers.iter()
                .filter(|p| p.enabled)
                .filter(|p| p.models.iter().any(|m| m.id == model_id))
                .filter(|p| effective_tiers.contains(&p.tier))
                .cloned()
                .collect();
        }

        // 6. Sort candidates
        candidates.sort_by(|a, b| {
            let tier_a_idx = effective_tiers.iter().position(|t| t == &a.tier).unwrap_or(usize::MAX);
            let tier_b_idx = effective_tiers.iter().position(|t| t == &b.tier).unwrap_or(usize::MAX);

            match tier_a_idx.cmp(&tier_b_idx) {
                Ordering::Equal => {
                    let cost_a = a.models.iter().find(|m| m.id == effective_model_id).map(|m| m.input_cost_per_1m).unwrap_or(f64::MAX);
                    let cost_b = b.models.iter().find(|m| m.id == effective_model_id).map(|m| m.input_cost_per_1m).unwrap_or(f64::MAX);

                    match cost_a.partial_cmp(&cost_b).unwrap_or(Ordering::Equal) {
                        Ordering::Equal => b.priority.cmp(&a.priority),
                        ord => ord,
                    }
                }
                ord => ord,
            }
        });

        candidates
    }

    /// Given a config and complexity tier, resolve the effective model_id
    /// that should be used (after applying model_mapping).
    pub fn resolve_model_id<'a>(config: &'a Config, model_id: &'a str, complexity: Option<ComplexityTier>, use_agentic: bool) -> &'a str {
        Self::resolve_model_id_with_profile(config, model_id, complexity, None, use_agentic)
    }

    pub fn resolve_model_id_with_profile<'a>(
        config: &'a Config,
        model_id: &'a str,
        complexity: Option<ComplexityTier>,
        profile_override: Option<&str>,
        use_agentic: bool,
    ) -> &'a str {
        let profile_name = profile_override.unwrap_or(&config.active_profile);
        let profile = config.profiles.iter()
            .find(|p| p.name == profile_name);

        if let (Some(profile), Some(c)) = (profile, complexity) {
            let mapping_source = if use_agentic && !profile.agentic_model_mapping.is_empty() {
                &profile.agentic_model_mapping
            } else {
                &profile.model_mapping
            };
            let tier_key = match c {
                ComplexityTier::Simple => "simple",
                ComplexityTier::Medium => "medium",
                ComplexityTier::Complex => "complex",
                ComplexityTier::Reasoning => "reasoning",
            };
            if let Some(mapping) = mapping_source.get(tier_key) {
                if !mapping.model_id.is_empty() {
                    return &mapping.model_id;
                }
            }
        }
        model_id
    }
}

/// Map a complexity tier to eligible provider tiers.
fn default_provider_tiers_for_complexity(complexity: ComplexityTier) -> Vec<Tier> {
    match complexity {
        ComplexityTier::Simple => vec![Tier::Free, Tier::Cheap],
        ComplexityTier::Medium => vec![Tier::Cheap, Tier::Free, Tier::PayPerRequest],
        ComplexityTier::Complex => vec![Tier::Subscription, Tier::Cheap, Tier::PayPerRequest],
        ComplexityTier::Reasoning => vec![Tier::Subscription, Tier::PayPerRequest],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ProviderType, Model, RoutingProfile};
    use std::collections::HashMap;

    fn make_provider(id: &str, name: &str, tier: Tier, cost: f64, priority: u8) -> Provider {
        Provider {
            id: id.to_string(),
            name: name.to_string(),
            provider_type: ProviderType::OpenAI,
            api_key: None,
            endpoint: None,
            tier,
            enabled: true,
            priority,
            models: vec![
                Model {
                    id: "gpt-4".to_string(),
                    name: "gpt-4".to_string(),
                    input_cost_per_1m: cost,
                    output_cost_per_1m: cost * 2.0,
                    context_window: 8192,
                    supports_vision: false,
                    supports_function_calling: true,
                }
            ],
        }
    }

    fn make_config(providers: Vec<Provider>, profiles: Vec<RoutingProfile>, active: &str) -> Config {
        Config {
            providers,
            profiles,
            active_profile: active.to_string(),
            scorer: None,
            cache: None,
            agentic_mode: false,
            session: None,
        }
    }

    fn make_profile(name: &str, desc: &str, tiers: Vec<Tier>) -> RoutingProfile {
        RoutingProfile {
            name: name.to_string(),
            description: desc.to_string(),
            allowed_tiers: tiers,
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }
    }

    #[test]
    fn test_routing_logic_no_complexity() {
        let providers = vec![
            make_provider("p1", "Expensive", Tier::Subscription, 30.0, 1),
            make_provider("p2", "Cheap", Tier::Cheap, 5.0, 1),
        ];

        let profiles = vec![
            make_profile("auto", "auto", vec![Tier::Subscription, Tier::Cheap]),
            make_profile("eco", "eco", vec![Tier::Cheap]),
        ];

        let config = make_config(providers, profiles, "auto");

        // Auto without complexity: Subscription (p1) then Cheap (p2)
        let candidates = Router::route_request(&config, "gpt-4", None, false);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "p1");
        assert_eq!(candidates[1].id, "p2");

        // Eco without complexity: Cheap (p2) only
        let mut config_eco = config.clone();
        config_eco.active_profile = "eco".to_string();
        let candidates_eco = Router::route_request(&config_eco, "gpt-4", None, false);
        assert_eq!(candidates_eco.len(), 1);
        assert_eq!(candidates_eco[0].id, "p2");
    }

    #[test]
    fn test_simple_complexity_routes_to_cheap() {
        let providers = vec![
            make_provider("sub", "Subscription", Tier::Subscription, 30.0, 1),
            make_provider("cheap", "Cheap", Tier::Cheap, 5.0, 1),
            make_provider("free", "Free", Tier::Free, 0.0, 1),
        ];

        let profiles = vec![RoutingProfile {
            name: "auto".to_string(),
            description: "auto".to_string(),
            allowed_tiers: vec![Tier::Subscription, Tier::Cheap, Tier::Free],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }];

        let config = make_config(providers, profiles, "auto");

        // Simple complexity → eligible: Free, Cheap
        // Intersection with auto (Sub, Cheap, Free) preserves profile order → Cheap, Free
        let candidates = Router::route_request(&config, "gpt-4", Some(ComplexityTier::Simple), false);
        assert_eq!(candidates.len(), 2);
        // Cheap first (profile order: Sub, Cheap, Free; after intersection: Cheap, Free)
        assert_eq!(candidates[0].id, "cheap");
        assert_eq!(candidates[1].id, "free");
    }

    #[test]
    fn test_reasoning_complexity_routes_to_subscription() {
        let providers = vec![
            make_provider("sub", "Subscription", Tier::Subscription, 30.0, 1),
            make_provider("cheap", "Cheap", Tier::Cheap, 5.0, 1),
            make_provider("free", "Free", Tier::Free, 0.0, 1),
        ];

        let profiles = vec![RoutingProfile {
            name: "auto".to_string(),
            description: "auto".to_string(),
            allowed_tiers: vec![Tier::Subscription, Tier::Cheap, Tier::Free],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }];

        let config = make_config(providers, profiles, "auto");

        // Reasoning complexity → eligible: Subscription, PayPerRequest
        // Intersection with auto → Subscription only
        let candidates = Router::route_request(&config, "gpt-4", Some(ComplexityTier::Reasoning), false);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "sub");
    }

    #[test]
    fn test_complexity_with_eco_profile_fallback() {
        let providers = vec![
            make_provider("sub", "Subscription", Tier::Subscription, 30.0, 1),
            make_provider("cheap", "Cheap", Tier::Cheap, 5.0, 1),
        ];

        let profiles = vec![RoutingProfile {
            name: "eco".to_string(),
            description: "eco".to_string(),
            allowed_tiers: vec![Tier::Cheap],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }];

        let config = make_config(providers, profiles, "eco");

        // Reasoning complexity → eligible: Subscription, PayPerRequest
        // Intersection with eco (Cheap only) → EMPTY
        // Fallback to full eco tiers → Cheap
        let candidates = Router::route_request(&config, "gpt-4", Some(ComplexityTier::Reasoning), false);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "cheap");
    }

    #[test]
    fn test_complex_complexity() {
        let providers = vec![
            make_provider("sub", "Subscription", Tier::Subscription, 30.0, 1),
            make_provider("cheap", "Cheap", Tier::Cheap, 5.0, 1),
            make_provider("free", "Free", Tier::Free, 0.0, 1),
        ];

        let profiles = vec![RoutingProfile {
            name: "auto".to_string(),
            description: "auto".to_string(),
            allowed_tiers: vec![Tier::Subscription, Tier::Cheap, Tier::Free],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }];

        let config = make_config(providers, profiles, "auto");

        // Complex complexity → eligible: Subscription, Cheap, PayPerRequest
        // Intersection with auto → Subscription, Cheap
        let candidates = Router::route_request(&config, "gpt-4", Some(ComplexityTier::Complex), false);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].id, "sub");
        assert_eq!(candidates[1].id, "cheap");
    }
}
