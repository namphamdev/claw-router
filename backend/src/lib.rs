pub mod cache;
pub mod config;
pub mod handlers;
pub mod router;
pub mod scorer;
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};

/// Build the application router with the given state.
pub fn app(state: state::AppState) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/models", get(handlers::list_models))
        .route(
            "/api/config",
            get(handlers::get_config).post(handlers::update_config),
        )
        .route("/api/stats", get(handlers::get_stats))
        .route("/api/logs", get(handlers::get_logs))
        .with_state(state)
}
