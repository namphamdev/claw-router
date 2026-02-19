use backend::config::{
    Config, Model, Provider, ProviderType, RoutingProfile, Tier,
};
use backend::state::AppState;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Standard OpenAI-style success response with token usage.
fn openai_success_body() -> Value {
    json!({
        "id": "chatcmpl-test123",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "Hello from mock!"
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    })
}

/// Build a minimal chat completion request body.
fn chat_request(model: &str) -> Value {
    json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}]
    })
}

fn chat_request_with_extra(model: &str, extra: Value) -> Value {
    let mut base = json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hello"}]
    });
    if let (Some(base_map), Some(extra_map)) = (base.as_object_mut(), extra.as_object()) {
        for (k, v) in extra_map {
            base_map.insert(k.clone(), v.clone());
        }
    }
    base
}

/// Create a test Config with one OpenAI-compatible provider pointed at the
/// given mock endpoint.
fn make_test_config(endpoint: &str, model_id: &str) -> Config {
    Config {
        providers: vec![Provider {
            id: "mock-provider".to_string(),
            name: "Mock Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key: Some("test-key-123".to_string()),
            endpoint: Some(endpoint.to_string()),
            tier: Tier::Cheap,
            enabled: true,
            priority: 1,
            models: vec![Model {
                id: model_id.to_string(),
                name: model_id.to_string(),
                input_cost_per_1m: 1.0,
                output_cost_per_1m: 2.0,
                context_window: 128000,
                supports_vision: false,
                supports_function_calling: true,
            }],
        }],
        profiles: vec![RoutingProfile {
            name: "auto".to_string(),
            description: "test profile".to_string(),
            allowed_tiers: vec![Tier::Subscription, Tier::Cheap, Tier::Free, Tier::PayPerRequest],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }],
        active_profile: "auto".to_string(),
        scorer: None,
        cache: None,
        agentic_mode: false,
        session: None,
    }
}

/// Create an AppState with the given config (no disk persistence needed).
fn make_state(config: Config) -> AppState {
    AppState {
        config: Arc::new(RwLock::new(config)),
        config_path: PathBuf::from("/dev/null"),
        logs: Arc::new(RwLock::new(Vec::new())),
        sessions: Arc::new(RwLock::new(HashMap::new())),
    }
}

/// Build the axum Router from AppState without CORS (tests don't need it).
fn test_app(state: AppState) -> axum::Router {
    backend::app(state)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Successful chat completion through an OpenAI-compatible mock provider.
#[tokio::test]
async fn test_chat_completions_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state.clone());

    let client = reqwest::Client::new();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"][0]["message"]["content"], "Hello from mock!");
    assert_eq!(body["usage"]["prompt_tokens"], 10);
    assert_eq!(body["usage"]["completion_tokens"], 5);

    // Verify a log entry was created
    let logs = state.get_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "success");
    assert_eq!(logs[0].model, "test-model");
    assert_eq!(logs[0].provider.as_deref(), Some("Mock Provider"));
    assert_eq!(logs[0].input_tokens, Some(10));
    assert_eq!(logs[0].output_tokens, Some(5));
    assert_eq!(logs[0].cache_status.as_deref(), Some("miss"));
}

/// Request for unknown model returns 400.
#[tokio::test]
async fn test_chat_completions_no_provider_for_model() {
    let config = make_test_config("http://127.0.0.1:1", "test-model");
    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("nonexistent-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let text = resp.text().await.unwrap();
    assert!(text.contains("No provider found"));

    // Log entry records "no_provider"
    let logs = state.get_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "no_provider");
}

/// When the upstream provider returns an error, the handler returns 503.
#[tokio::test]
async fn test_chat_completions_all_providers_fail() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 503);
    let text = resp.text().await.unwrap();
    assert!(text.contains("All providers failed"));

    let logs = state.get_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "error");
    assert_eq!(
        logs[0].error_message.as_deref(),
        Some("All providers failed")
    );
    assert_eq!(logs[0].providers_tried, vec!["Mock Provider"]);
}

/// Provider fallback: first provider fails, second succeeds.
#[tokio::test]
async fn test_chat_completions_provider_fallback() {
    let failing_server = MockServer::start().await;
    let succeeding_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&failing_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .mount(&succeeding_server)
        .await;

    let config = Config {
        providers: vec![
            Provider {
                id: "p1".to_string(),
                name: "Failing Provider".to_string(),
                provider_type: ProviderType::OpenAI,
                api_key: Some("key1".to_string()),
                endpoint: Some(failing_server.uri()),
                tier: Tier::Cheap,
                enabled: true,
                priority: 2, // higher priority → tried first
                models: vec![Model {
                    id: "test-model".to_string(),
                    name: "test-model".to_string(),
                    input_cost_per_1m: 1.0,
                    output_cost_per_1m: 2.0,
                    context_window: 128000,
                    supports_vision: false,
                    supports_function_calling: true,
                }],
            },
            Provider {
                id: "p2".to_string(),
                name: "Good Provider".to_string(),
                provider_type: ProviderType::OpenAI,
                api_key: Some("key2".to_string()),
                endpoint: Some(succeeding_server.uri()),
                tier: Tier::Cheap,
                enabled: true,
                priority: 1,
                models: vec![Model {
                    id: "test-model".to_string(),
                    name: "test-model".to_string(),
                    input_cost_per_1m: 1.0,
                    output_cost_per_1m: 2.0,
                    context_window: 128000,
                    supports_vision: false,
                    supports_function_calling: true,
                }],
            },
        ],
        profiles: vec![RoutingProfile {
            name: "auto".to_string(),
            description: "test".to_string(),
            allowed_tiers: vec![Tier::Cheap],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }],
        active_profile: "auto".to_string(),
        scorer: None,
        cache: None,
        agentic_mode: false,
        session: None,
    };

    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let logs = state.get_logs().await;
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].status, "success");
    assert_eq!(logs[0].provider.as_deref(), Some("Good Provider"));
    // Both providers were attempted
    assert_eq!(logs[0].providers_tried.len(), 2);
    assert!(logs[0].providers_tried.contains(&"Failing Provider".to_string()));
    assert!(logs[0].providers_tried.contains(&"Good Provider".to_string()));
}

/// Token usage is extracted and cost is estimated.
#[tokio::test]
async fn test_chat_completions_cost_estimation() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "ok"},
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 1000000,  // 1M input tokens
            "completion_tokens": 500000, // 500K output tokens
            "total_tokens": 1500000
        }
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    // cost = (1M / 1M) * 1.0 + (500K / 1M) * 2.0 = 1.0 + 1.0 = 2.0
    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    let logs = state.get_logs().await;
    assert_eq!(logs[0].input_tokens, Some(1000000));
    assert_eq!(logs[0].output_tokens, Some(500000));
    let cost = logs[0].estimated_cost.unwrap();
    assert!((cost - 2.0).abs() < 0.001, "Expected cost ~2.0 got {}", cost);
}

/// Extra request parameters (temperature, max_tokens) are forwarded.
#[tokio::test]
async fn test_chat_completions_extra_params_forwarded() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let body = chat_request_with_extra(
        "test-model",
        json!({"temperature": 0.5, "max_tokens": 100}),
    );

    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&body)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
}

/// Invalid JSON body returns 422 (Unprocessable Entity).
#[tokio::test]
async fn test_chat_completions_invalid_body() {
    let config = make_test_config("http://127.0.0.1:1", "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();

    // Missing required "messages" field
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&json!({"model": "test-model"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422);
}

/// Missing model field returns 422.
#[tokio::test]
async fn test_chat_completions_missing_model() {
    let config = make_test_config("http://127.0.0.1:1", "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&json!({"messages": [{"role": "user", "content": "Hi"}]}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 422);
}

/// Disabled provider is not tried.
#[tokio::test]
async fn test_chat_completions_disabled_provider_skipped() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .expect(0) // should NOT be called
        .mount(&mock_server)
        .await;

    let mut config = make_test_config(&mock_server.uri(), "test-model");
    config.providers[0].enabled = false;

    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    // No enabled provider → no_provider → 400
    assert_eq!(resp.status(), 400);

    let logs = state.get_logs().await;
    assert_eq!(logs[0].status, "no_provider");
}

/// Multiple sequential requests each produce their own log entry.
#[tokio::test]
async fn test_chat_completions_multiple_requests_logged() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    for _ in 0..3 {
        let resp = client
            .post(format!("http://{}/v1/chat/completions", addr))
            .json(&chat_request("test-model"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    let logs = state.get_logs().await;
    assert_eq!(logs.len(), 3);
    // Each log has a unique id
    let ids: Vec<_> = logs.iter().map(|l| &l.id).collect();
    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(unique.len(), 3);
}

/// Authorization header is forwarded with Bearer token for OpenAI providers.
#[tokio::test]
async fn test_chat_completions_auth_header_forwarded() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(wiremock::matchers::header("Authorization", "Bearer test-key-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    // If the mock expectation (header match) failed, the mock would return 404
}

/// The response body from the upstream provider is faithfully passed through.
#[tokio::test]
async fn test_chat_completions_response_passthrough() {
    let mock_server = MockServer::start().await;

    let custom_response = json!({
        "id": "chatcmpl-custom",
        "object": "chat.completion",
        "created": 1234567890,
        "model": "test-model",
        "choices": [
            {
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Custom response content 123"
                },
                "finish_reason": "stop"
            },
            {
                "index": 1,
                "message": {
                    "role": "assistant",
                    "content": "Second choice"
                },
                "finish_reason": "stop"
            }
        ],
        "usage": {
            "prompt_tokens": 42,
            "completion_tokens": 17,
            "total_tokens": 59
        }
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(custom_response.clone()))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["choices"].as_array().unwrap().len(), 2);
    assert_eq!(
        body["choices"][0]["message"]["content"],
        "Custom response content 123"
    );
    assert_eq!(body["usage"]["prompt_tokens"], 42);
}

/// GET /api/logs returns the log entry created by a chat completion.
#[tokio::test]
async fn test_logs_populated_after_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();

    // Make a chat request
    client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    // Query logs endpoint
    let resp = client
        .get(format!("http://{}/api/logs", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["total"], 1);
    assert_eq!(body["logs"][0]["model"], "test-model");
    assert_eq!(body["logs"][0]["status"], "success");
}

/// GET /api/stats reflects the completed request.
#[tokio::test]
async fn test_stats_after_request() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .mount(&mock_server)
        .await;

    let config = make_test_config(&mock_server.uri(), "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();

    // Make a chat request
    client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    // Query stats
    let resp = client
        .get(format!("http://{}/api/stats", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["requests"], 1);
    assert_eq!(body["successful"], 1);
    assert_eq!(body["failed"], 0);
}

/// Tier filtering: a Subscription-only provider is not returned for a
/// Free-tier-only profile.
#[tokio::test]
async fn test_chat_completions_tier_filtering() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(openai_success_body()))
        .expect(0) // should NOT be called
        .mount(&mock_server)
        .await;

    let config = Config {
        providers: vec![Provider {
            id: "sub-only".to_string(),
            name: "Sub Only".to_string(),
            provider_type: ProviderType::OpenAI,
            api_key: Some("key".to_string()),
            endpoint: Some(mock_server.uri()),
            tier: Tier::Subscription,
            enabled: true,
            priority: 1,
            models: vec![Model {
                id: "test-model".to_string(),
                name: "test-model".to_string(),
                input_cost_per_1m: 1.0,
                output_cost_per_1m: 2.0,
                context_window: 128000,
                supports_vision: false,
                supports_function_calling: true,
            }],
        }],
        profiles: vec![RoutingProfile {
            name: "free-only".to_string(),
            description: "free".to_string(),
            allowed_tiers: vec![Tier::Free],
            model_mapping: HashMap::new(),
            agentic_model_mapping: HashMap::new(),
        }],
        active_profile: "free-only".to_string(),
        scorer: None,
        cache: None,
        agentic_mode: false,
        session: None,
    };

    let state = make_state(config);
    let app = test_app(state.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("http://{}/v1/chat/completions", addr))
        .json(&chat_request("test-model"))
        .send()
        .await
        .unwrap();

    // Subscription provider excluded by Free-only profile → no_provider
    assert_eq!(resp.status(), 400);
}

/// GET /v1/models returns provider models + virtual router/ models.
#[tokio::test]
async fn test_list_models() {
    let config = make_test_config("http://127.0.0.1:1", "test-model");
    let state = make_state(config);
    let app = test_app(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/v1/models", addr))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["object"], "list");

    let models = body["data"].as_array().unwrap();
    // Should have virtual "router/auto" + the provider model
    let ids: Vec<&str> = models.iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"router/auto"), "Missing router/auto in {:?}", ids);
    assert!(ids.contains(&"test-model"), "Missing test-model in {:?}", ids);
}
