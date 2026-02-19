use crate::config::Config;
use crate::router::Router;
use crate::state::AppState;
use axum::{
    extract::{State, Json},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;


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

use std::collections::HashMap;

pub async fn list_models(State(state): State<AppState>) -> impl IntoResponse {
    let config = state.get_config().await;
    let mut models = Vec::new();

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
    // Routing logic
    let config = state.get_config().await;
    let candidates = Router::route_request(&config, &request.model);

    if candidates.is_empty() {
        return (StatusCode::BAD_REQUEST, "No provider found for model").into_response();
    }

    let client = reqwest::Client::new();

    // Try each candidate
    for provider in candidates {
        // Prepare request
        let url = provider.endpoint.clone().unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());
        let api_key = provider.api_key.clone().unwrap_or_default();

        // Forward headers (except host/content-length)
        let mut forward_headers = headers.clone();
        forward_headers.remove("host");
        forward_headers.remove("content-length");
        forward_headers.insert("Authorization", format!("Bearer {}", api_key).parse().unwrap());

        // Reconstruct body
        // Ensure model name is correct for provider (might need mapping, but keeping simple for now)
        let mut body_map = serde_json::Map::new();
        body_map.insert("model".to_string(), Value::String(request.model.clone()));
        body_map.insert("messages".to_string(), Value::Array(request.messages.clone()));
        for (k, v) in &request.extra {
             body_map.insert(k.clone(), v.clone());
        }
        let body = Value::Object(body_map);

        // Send request
        let res = client.post(&url)
            .headers(forward_headers)
            .json(&body)
            .send()
            .await;

        match res {
            Ok(response) => {
                if response.status().is_success() {
                    // Success! Stream back response or return JSON
                    // For simplicity, return full JSON response
                    let body_bytes = response.bytes().await.unwrap_or_default();
                    return (StatusCode::OK, body_bytes).into_response();
                } else {
                    // Log error and continue to next provider
                    tracing::warn!("Provider {} failed: {:?}", provider.name, response.status());
                }
            }
            Err(e) => {
                tracing::warn!("Provider {} error: {:?}", provider.name, e);
            }
        }
    }

    (StatusCode::SERVICE_UNAVAILABLE, "All providers failed").into_response()
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

pub async fn get_stats(State(_state): State<AppState>) -> impl IntoResponse {
    // Placeholder for stats
    let stats = serde_json::json!({
        "requests": 120,
        "saved_cost": 4.50,
        "active_profile": "auto"
    });
    Json(stats)
}
