mod config;
mod state;
mod router;
mod handlers;

use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "backend=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_path = PathBuf::from("config.json");
    let state = state::AppState::new(config_path).await;

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/models", get(handlers::list_models))
        .route("/api/config", get(handlers::get_config).post(handlers::update_config))
        .route("/api/stats", get(handlers::get_stats))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
