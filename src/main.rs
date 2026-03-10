mod api;
mod config;
mod error;
mod state;

use std::{net::SocketAddr, str::FromStr, time::Duration};

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
    routing::{delete, get, post},
};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{config::Config, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env();

    // --- Logging setup ---
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cfg.log_level));

    if cfg.json_logging {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().pretty())
            .init();
    }

    let state = AppState::new(cfg.clone());

    // --- CORS ---
    let cors = if cfg.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
    } else {
        let origins: Vec<HeaderValue> = cfg
            .cors_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
    };

    // --- API router ---
    let api_router = Router::new()
        // Health
        .route("/health", get(api::health::health))
        .route("/health/ready", get(api::health::ready))
        // Encode
        .route("/encode", post(api::encode::encode))
        // Decode
        .route("/decode/json", post(api::decode::decode_json))
        .route("/decode/upload", post(api::decode::decode_upload))
        // History
        .route("/history", get(api::history::list_history))
        .route("/history", delete(api::history::clear_history))
        .with_state(state.clone());

    // --- Root router ---
    let app = Router::new()
        .nest("/api", api_router)
        // Serve static frontend
        .nest_service("/", ServeDir::new("static").append_index_html_on_directories(true))
        // Layers are applied outermost-last (TraceLayer wraps everything)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(cfg.max_upload_bytes))
        .layer(
            TimeoutLayer::with_status_code(
                StatusCode::REQUEST_TIMEOUT,
                Duration::from_secs(30),
            )
        );

    let addr = SocketAddr::from_str(&cfg.bind_addr())?;

    tracing::info!("qrtools-web listening on http://{}", addr);
    tracing::info!("Dashboard  -> http://{}/", addr);
    tracing::info!("API Health -> http://{}/api/health", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
