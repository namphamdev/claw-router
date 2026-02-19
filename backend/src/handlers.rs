use crate::cache;
use crate::config::{Config, Provider, ProviderType};
use crate::router::Router;
use crate::scorer::Scorer;
use crate::state::{AppState, RequestLog};
use axum::{
    extract::{State, Json, Query},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct ModelListResponse {
    pub object: String,
    pub data: Vec<ModelEntry>,
}

#[derive(Debug, Serialize)]
pub struct ModelEntry {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub status: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
}

pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.get_config().await;
    let mut models = Vec::new();

    // Add virtual router/<profile> models
    for profile in &config.profiles {
        models.push(ModelEntry {
            id: format!("router/{}", profile.name),
            object: "model".to_string(),
            created: 1677610602,
            owned_by: "claw-router".to_string(),
        });
    }

    for provider in config.providers {
        for model in provider.models {
            models.push(ModelEntry {
                id: model.id,
                object: "model".to_string(),
                created: 1677610602,
                owned_by: provider.name.clone(),
            });
        }
    }

    Json(ModelListResponse {
        object: "list".to_string(),
        data: models,
    })
}

pub async fn chat_completions(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> Response {
    let start = Instant::now();
    let mut log_entry = RequestLog::new(&request.model);

    // Routing logic
    let config = state.get_config().await;

    // Detect "router/<profile>" model name for per-request profile selection
    let profile_override = Router::parse_router_model(&request.model);
    if let Some(profile_name) = profile_override {
        tracing::info!(profile = %profile_name, "Per-request profile override via router/ model name");
    }

    // --- Session persistence: extract session ID ---
    let session_config = config.session.clone().unwrap_or_default();
    let session_id = if session_config.enabled {
        extract_session_id(&headers, &request)
    } else {
        None
    };
    log_entry.session_id = session_id.clone();

    // --- Session persistence: check for pinned session ---
    if let Some(ref sid) = session_id {
        if let Some(pinned) = state.get_session(sid, session_config.ttl_seconds).await {
            // Verify the pinned provider still exists and is enabled
            if let Some(provider) = config.providers.iter().find(|p| p.id == pinned.provider_id && p.enabled) {
                state.touch_session(sid).await;
                log_entry.session_pinned = Some(true);
                log_entry.effective_model = Some(pinned.model_id.clone());
                tracing::info!(
                    session_id = %sid,
                    pinned_model = %pinned.model_id,
                    pinned_provider = %pinned.provider_id,
                    "Session pinned - using cached provider+model"
                );

                // Forward directly to the pinned provider
                let client = reqwest::Client::new();
                if let Some((status, final_body)) = forward_to_provider(
                    &client, &headers, &request, provider, &pinned.model_id, &mut log_entry,
                ).await {
                    log_entry.provider = Some(provider.name.clone());
                    log_entry.status = "success".to_string();
                    log_entry.status_code = Some(status.as_u16());
                    log_entry.duration_ms = start.elapsed().as_millis() as u64;
                    log_entry.cache_status = Some("skip".to_string());
                    state.add_log(log_entry).await;
                    return (status, final_body).into_response();
                }
                // Pinned provider failed — fall through to normal routing
                tracing::warn!(session_id = %sid, "Pinned session provider failed, falling through");
            }
        }
    }

    // Check cache before making any upstream requests
    let cache_config = config.cache.clone().unwrap_or_default();
    let cache_key_str = cache::cache_key(&request.model, &request.messages, &request.extra);

    if let Some(cached_body) = cache::get(&cache_config, &cache_key_str) {
        tracing::info!(model = %request.model, key = %&cache_key_str[..12], "Cache hit");
        log_entry.provider = Some("cache".to_string());
        log_entry.status = "success".to_string();
        log_entry.status_code = Some(200);
        log_entry.duration_ms = start.elapsed().as_millis() as u64;
        log_entry.cache_status = Some("hit".to_string());
        state.add_log(log_entry).await;
        return (StatusCode::OK, cached_body).into_response();
    }

    // --- Tool detection (before scoring) ---
    let tools_present = has_tools(&request);

    // Score request complexity using the 15-dimension weighted scorer
    let scorer_config = config.scorer.clone().unwrap_or_default();
    let scoring_result = if scorer_config.enabled {
        Some(Scorer::score(&request.messages, &scorer_config))
    } else {
        None
    };
    let complexity = scoring_result.as_ref().map(|r| r.tier);

    if let Some(ref result) = scoring_result {
        log_entry.complexity_tier = Some(format!("{:?}", result.tier));
        log_entry.complexity_score = Some(result.raw_score);
        tracing::info!(
            tier = ?result.tier,
            score = format!("{:.3}", result.raw_score),
            confidence = format!("{:.3}", result.confidence),
            signals = ?result.signals,
            override_applied = ?result.override_applied,
            "Scored request complexity"
        );
    }

    // --- Agentic auto-detection ---
    let agentic_keyword_count = scoring_result.as_ref().map(|r| r.agentic_keyword_count).unwrap_or(0);
    let is_agentic = tools_present
        || config.agentic_mode
        || agentic_keyword_count >= 2;
    log_entry.agentic_mode = Some(is_agentic);

    if is_agentic {
        tracing::info!(
            tools = tools_present,
            config_force = config.agentic_mode,
            keyword_count = agentic_keyword_count,
            "Agentic mode activated"
        );
    }

    // --- Route with agentic flag ---
    let candidates = Router::route_request_with_profile(&config, &request.model, complexity, profile_override, is_agentic);
    let effective_model = Router::resolve_model_id_with_profile(&config, &request.model, complexity, profile_override, is_agentic).to_string();

    if effective_model != request.model {
        log_entry.effective_model = Some(effective_model.clone());
        tracing::info!(
            requested = %request.model,
            effective = %effective_model,
            agentic = is_agentic,
            "Model mapping applied"
        );
    }

    if candidates.is_empty() {
        log_entry.status = "no_provider".to_string();
        log_entry.error_message = Some("No provider found for model".to_string());
        log_entry.duration_ms = start.elapsed().as_millis() as u64;
        state.add_log(log_entry).await;
        return (StatusCode::BAD_REQUEST, "No provider found for model").into_response();
    }

    let client = reqwest::Client::new();

    // Try each candidate
    for provider in &candidates {
        log_entry.providers_tried.push(provider.name.clone());

        if let Some((status, final_body)) = forward_to_provider(
            &client, &headers, &request, provider, &effective_model, &mut log_entry,
        ).await {
            log_entry.provider = Some(provider.name.clone());
            log_entry.status = "success".to_string();
            log_entry.status_code = Some(status.as_u16());
            log_entry.duration_ms = start.elapsed().as_millis() as u64;
            log_entry.cache_status = Some("miss".to_string());

            // Store in cache
            cache::put(&cache_config, &cache_key_str, &request.model, &final_body);

            // Record session pin on success
            if let Some(ref sid) = session_id {
                if session_config.enabled {
                    state.set_session(
                        sid.clone(),
                        provider.id.clone(),
                        effective_model.clone(),
                    ).await;
                }
            }

            state.add_log(log_entry).await;
            return (status, final_body).into_response();
        }
    }

    log_entry.status = "error".to_string();
    log_entry.error_message = Some("All providers failed".to_string());
    log_entry.duration_ms = start.elapsed().as_millis() as u64;
    state.add_log(log_entry).await;

    (StatusCode::SERVICE_UNAVAILABLE, "All providers failed").into_response()
}

/// Extract a session ID from the request using a priority chain.
fn extract_session_id(headers: &HeaderMap, request: &ChatCompletionRequest) -> Option<String> {
    // 1. Custom header
    if let Some(val) = headers.get("x-session-id") {
        if let Ok(s) = val.to_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    // 2. Request body field
    if let Some(val) = request.extra.get("conversation_id") {
        if let Some(s) = val.as_str() {
            if !s.is_empty() {
                return Some(s.to_string());
            }
        }
    }
    // 3. Fingerprint from system message + first user message
    let mut parts = String::new();
    for msg in &request.messages {
        let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role == "system" || role == "user" {
            if let Some(content) = msg.get("content").and_then(|c| c.as_str()) {
                parts.push_str(content);
            }
            if role == "user" {
                break; // Only use first user message
            }
        }
    }
    if !parts.is_empty() {
        let hash = Sha256::digest(parts.as_bytes());
        Some(format!("fp:{:x}", hash))
    } else {
        None
    }
}

/// Check if the request contains a non-empty tools array (function calling).
fn has_tools(request: &ChatCompletionRequest) -> bool {
    request.extra.get("tools")
        .and_then(|v| v.as_array())
        .map(|arr| !arr.is_empty())
        .unwrap_or(false)
}

/// Forward a request to a single provider. Returns Some((StatusCode, body)) on success.
async fn forward_to_provider(
    client: &reqwest::Client,
    headers: &HeaderMap,
    request: &ChatCompletionRequest,
    provider: &Provider,
    effective_model: &str,
    log_entry: &mut RequestLog,
) -> Option<(StatusCode, Vec<u8>)> {
    let url = provider.endpoint.clone().unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
    let api_key = provider.api_key.clone().unwrap_or_default();
    let is_anthropic = provider.provider_type == ProviderType::Anthropic;

    // Build headers based on provider type
    let mut forward_headers = headers.clone();
    forward_headers.remove("host");
    forward_headers.remove("content-length");

    if is_anthropic {
        forward_headers.remove("authorization");
        forward_headers.insert("x-api-key", api_key.parse().unwrap());
        forward_headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
        forward_headers.insert("content-type", "application/json".parse().unwrap());
    } else {
        forward_headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());
    }

    // Build request body based on provider type
    let body: Value = if is_anthropic {
        let openai_req = build_openai_chat_request(request, effective_model);
        let anthropic_req: aidapter::anthropic::types::ChatRequest = (&openai_req).into();
        serde_json::to_value(&anthropic_req).unwrap_or_default()
    } else {
        let mut body_map = serde_json::Map::new();
        body_map.insert("model".to_string(), Value::String(effective_model.to_string()));
        body_map.insert("messages".to_string(), Value::Array(request.messages.clone()));
        for (k, v) in &request.extra {
            body_map.insert(k.clone(), v.clone());
        }
        Value::Object(body_map)
    };

    // Send request
    let res = client.post(&url)
        .headers(forward_headers)
        .json(&body)
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                let resp_status = response.status();
                let body_bytes = response.bytes().await.unwrap_or_default();

                // Convert response back to OpenAI format if Anthropic
                let final_body = if is_anthropic {
                    match serde_json::from_slice::<aidapter::anthropic::types::ChatResponse>(&body_bytes) {
                        Ok(anthropic_resp) => {
                            log_entry.input_tokens = Some(anthropic_resp.usage.input_tokens as u64);
                            log_entry.output_tokens = Some(anthropic_resp.usage.output_tokens as u64);
                            let openai_resp: aidapter::openai::types::ChatResponse = (&anthropic_resp).into();
                            serde_json::to_vec(&openai_resp).unwrap_or_else(|_| body_bytes.to_vec())
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse Anthropic response: {:?}", e);
                            body_bytes.to_vec()
                        }
                    }
                } else {
                    if let Ok(resp_json) = serde_json::from_slice::<Value>(&body_bytes) {
                        if let Some(usage) = resp_json.get("usage") {
                            log_entry.input_tokens = usage.get("prompt_tokens").and_then(|v| v.as_u64());
                            log_entry.output_tokens = usage.get("completion_tokens").and_then(|v| v.as_u64());
                        }
                    }
                    body_bytes.to_vec()
                };

                // Estimate cost
                if let (Some(input_t), Some(output_t)) = (log_entry.input_tokens, log_entry.output_tokens) {
                    if let Some(model_cfg) = provider.models.iter().find(|m| m.id == effective_model) {
                        let cost = (input_t as f64 / 1_000_000.0) * model_cfg.input_cost_per_1m
                            + (output_t as f64 / 1_000_000.0) * model_cfg.output_cost_per_1m;
                        log_entry.estimated_cost = Some(cost);
                    }
                }

                let axum_status = StatusCode::from_u16(resp_status.as_u16()).unwrap_or(StatusCode::OK);
                Some((axum_status, final_body))
            } else {
                tracing::warn!("Provider {} failed: {:?}", provider.name, response.status());
                None
            }
        }
        Err(e) => {
            tracing::warn!("Provider {} error: {:?}", provider.name, e);
            None
        }
    }
}

pub async fn get_logs(
    State(state): State<AppState>,
    Query(params): Query<LogsQuery>,
) -> impl IntoResponse {
    let all_logs = state.get_logs().await;
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    // Filter
    let filtered: Vec<&RequestLog> = all_logs.iter()
        .rev() // newest first
        .filter(|log| {
            if let Some(ref s) = params.status {
                if &log.status != s { return false; }
            }
            if let Some(ref m) = params.model {
                if !log.model.contains(m.as_str()) { return false; }
            }
            if let Some(ref p) = params.provider {
                if let Some(ref lp) = log.provider {
                    if !lp.contains(p.as_str()) { return false; }
                } else {
                    return false;
                }
            }
            true
        })
        .collect();

    let total = filtered.len();
    let page: Vec<&RequestLog> = filtered.into_iter().skip(offset).take(limit).collect();

    Json(serde_json::json!({
        "logs": page,
        "total": total,
        "limit": limit,
        "offset": offset,
    }))
}

pub async fn get_config(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.get_config().await;
    Json(config)
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(new_config): Json<Config>,
) -> impl IntoResponse {
    match state.update_config(new_config).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

pub async fn get_stats(State(state): State<AppState>) -> impl IntoResponse {
    let logs = state.get_logs().await;
    let config = state.get_config().await;

    let total_requests = logs.len();
    let successful = logs.iter().filter(|l| l.status == "success").count();
    let failed = logs.iter().filter(|l| l.status == "error").count();
    let total_cost: f64 = logs.iter().filter_map(|l| l.estimated_cost).sum();
    let avg_duration: f64 = if total_requests > 0 {
        logs.iter().map(|l| l.duration_ms as f64).sum::<f64>() / total_requests as f64
    } else {
        0.0
    };

    // Agentic & session stats
    let agentic_count = logs.iter().filter(|l| l.agentic_mode == Some(true)).count();
    let session_pinned_count = logs.iter().filter(|l| l.session_pinned == Some(true)).count();
    let active_sessions = state.session_count().await;

    // Provider breakdown
    let mut providers: HashMap<String, serde_json::Value> = HashMap::new();
    for log in &logs {
        let name = log.provider.clone().unwrap_or_else(|| "unknown".to_string());
        let entry = providers.entry(name).or_insert_with(|| serde_json::json!({
            "requests": 0_i64,
            "successful": 0_i64,
            "failed": 0_i64,
            "total_cost": 0.0_f64,
            "total_duration_ms": 0.0_f64,
        }));
        if let Some(obj) = entry.as_object_mut() {
            *obj.get_mut("requests").unwrap() = serde_json::json!(obj["requests"].as_i64().unwrap_or(0) + 1);
            if log.status == "success" {
                *obj.get_mut("successful").unwrap() = serde_json::json!(obj["successful"].as_i64().unwrap_or(0) + 1);
            } else {
                *obj.get_mut("failed").unwrap() = serde_json::json!(obj["failed"].as_i64().unwrap_or(0) + 1);
            }
            let prev_cost = obj["total_cost"].as_f64().unwrap_or(0.0);
            *obj.get_mut("total_cost").unwrap() = serde_json::json!(prev_cost + log.estimated_cost.unwrap_or(0.0));
            let prev_dur = obj["total_duration_ms"].as_f64().unwrap_or(0.0);
            *obj.get_mut("total_duration_ms").unwrap() = serde_json::json!(prev_dur + log.duration_ms as f64);
        }
    }
    // Compute avg_duration_ms per provider
    let providers_out: HashMap<String, serde_json::Value> = providers.into_iter().map(|(k, v)| {
        let reqs = v["requests"].as_i64().unwrap_or(1).max(1) as f64;
        let total_dur = v["total_duration_ms"].as_f64().unwrap_or(0.0);
        (k, serde_json::json!({
            "requests": v["requests"],
            "successful": v["successful"],
            "failed": v["failed"],
            "total_cost": v["total_cost"],
            "avg_duration_ms": ((total_dur / reqs) * 100.0).round() / 100.0,
        }))
    }).collect();

    // Model breakdown
    let mut models: HashMap<String, serde_json::Value> = HashMap::new();
    for log in &logs {
        let entry = models.entry(log.model.clone()).or_insert_with(|| serde_json::json!({
            "requests": 0_i64,
            "total_cost": 0.0_f64,
        }));
        if let Some(obj) = entry.as_object_mut() {
            *obj.get_mut("requests").unwrap() = serde_json::json!(obj["requests"].as_i64().unwrap_or(0) + 1);
            let prev_cost = obj["total_cost"].as_f64().unwrap_or(0.0);
            *obj.get_mut("total_cost").unwrap() = serde_json::json!(prev_cost + log.estimated_cost.unwrap_or(0.0));
        }
    }

    // Complexity tier breakdown
    let mut complexity_tiers: HashMap<String, i64> = HashMap::new();
    for log in &logs {
        if let Some(ref tier) = log.complexity_tier {
            *complexity_tiers.entry(tier.clone()).or_insert(0) += 1;
        }
    }

    // Recent requests (last 10)
    let recent: Vec<&RequestLog> = logs.iter().rev().take(10).collect();

    Json(serde_json::json!({
        "requests": total_requests,
        "successful": successful,
        "failed": failed,
        "total_cost": (total_cost * 10000.0).round() / 10000.0,
        "avg_duration_ms": (avg_duration * 100.0).round() / 100.0,
        "active_profile": config.active_profile,
        "providers": providers_out,
        "models": models,
        "complexity_tiers": complexity_tiers,
        "recent_requests": recent,
        "agentic_count": agentic_count,
        "session_pinned_count": session_pinned_count,
        "active_sessions": active_sessions,
    }))
}

/// Convert our incoming ChatCompletionRequest (raw JSON values) into an
/// aidapter OpenAI ChatRequest so we can use aidapter's From conversions.
fn build_openai_chat_request(
    request: &ChatCompletionRequest,
    effective_model: &str,
) -> aidapter::openai::types::ChatRequest {
    // Deserialize messages from raw JSON values into aidapter OpenAI messages
    let messages: Vec<aidapter::openai::types::Message> = request
        .messages
        .iter()
        .filter_map(|v| serde_json::from_value(v.clone()).ok())
        .collect();

    let temperature = request.extra.get("temperature").and_then(|v| v.as_f64()).map(|v| v as f32);
    let top_p = request.extra.get("top_p").and_then(|v| v.as_f64()).map(|v| v as f32);
    let max_tokens = request.extra.get("max_tokens").and_then(|v| v.as_u64()).map(|v| v as u32);
    let max_completion_tokens = request.extra.get("max_completion_tokens").and_then(|v| v.as_u64()).map(|v| v as u32);
    let stream = request.extra.get("stream").and_then(|v| v.as_bool());

    let tools: Option<Vec<aidapter::openai::types::Tool>> = request
        .extra
        .get("tools")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let tool_choice: Option<aidapter::openai::types::ToolChoice> = request
        .extra
        .get("tool_choice")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let stop: Option<aidapter::openai::types::Stop> = request
        .extra
        .get("stop")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    aidapter::openai::types::ChatRequest {
        model: effective_model.to_string(),
        messages,
        tools,
        tool_choice,
        parallel_tool_calls: None,
        temperature,
        top_p,
        max_completion_tokens,
        max_tokens,
        n: None,
        modalities: None,
        audio: None,
        response_format: None,
        prediction: None,
        verbosity: None,
        stop,
        logprobs: None,
        top_logprobs: None,
        logit_bias: None,
        frequency_penalty: None,
        presence_penalty: None,
        reasoning_effort: None,
        stream,
        stream_options: None,
        service_tier: None,
        store: None,
        prompt_cache_key: None,
        prompt_cache_retention: None,
        metadata: None,
        safety_identifier: None,
        user: None,
        seed: None,
        function_call: None,
        functions: None,
        web_search_options: None,
    }
}
