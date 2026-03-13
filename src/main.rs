mod api;
mod config;
mod error;
mod middleware;
mod services;
mod state;

use std::{net::SocketAddr, str::FromStr, time::Duration};

use axum::{
    Router,
    http::{HeaderValue, Method, StatusCode, header},
    middleware as axum_mw,
    routing::{delete, get, post},
};
use metrics_exporter_prometheus::PrometheusBuilder;
use sqlx::sqlite::SqlitePoolOptions;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{
    api::metrics_handler::{PROMETHEUS_HANDLE, metrics},
    config::Config,
    middleware::rate_limit::{IpRateLimiter, enforce, new_limiter},
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env();

    // ── Logging ────────────────────────────────────────────────────────────────
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cfg.log_level));
    if cfg.json_logging {
        tracing_subscriber::registry().with(filter).with(fmt::layer().json()).init();
    } else {
        tracing_subscriber::registry().with(filter).with(fmt::layer().pretty()).init();
    }

    // ── Prometheus metrics ─────────────────────────────────────────────────────
    if cfg.metrics_enabled {
        let handle = PrometheusBuilder::new().install_recorder()?;
        PROMETHEUS_HANDLE.set(handle).ok();
        tracing::info!("Prometheus metrics enabled at GET /metrics");
    }

    // ── SQLite ─────────────────────────────────────────────────────────────────
    let db_url = if cfg.db_path == ":memory:" {
        "sqlite::memory:".to_string()
    } else {
        format!("sqlite://{}?mode=rwc", cfg.db_path)
    };
    let db = SqlitePoolOptions::new().max_connections(5).connect(&db_url).await?;
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS history (
            id              TEXT PRIMARY KEY,
            kind            TEXT NOT NULL,
            summary         TEXT NOT NULL,
            timestamp       TEXT NOT NULL,
            preview_png_b64 TEXT,
            content         TEXT
        )",
    )
    .execute(&db)
    .await?;
    tracing::info!("SQLite at '{}'", cfg.db_path);

    // ── App state ──────────────────────────────────────────────────────────────
    let state = AppState::new(cfg.clone(), db);

    // ── CORS ───────────────────────────────────────────────────────────────────
    let cors = if cfg.cors_origins.iter().any(|o| o == "*") {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::ACCEPT,
                           "x-api-key".parse::<axum::http::HeaderName>().unwrap()])
    } else {
        let origins: Vec<HeaderValue> = cfg.cors_origins.iter()
            .filter_map(|o| o.parse().ok()).collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
            .allow_headers([header::CONTENT_TYPE, header::ACCEPT,
                           "x-api-key".parse::<axum::http::HeaderName>().unwrap()])
    };

    // ── Rate limiter ───────────────────────────────────────────────────────────
    let limiter: IpRateLimiter = new_limiter(cfg.rate_limit_rpm);

    // ── Per-API optional auth ──────────────────────────────────────────────────
    let api_key_opt = cfg.api_key.clone();

    // ── API router ─────────────────────────────────────────────────────────────
    let api_router = Router::new()
        .route("/health",           get(api::health::health))
        .route("/health/ready",     get(api::health::ready))
        .route("/encode",           post(api::encode::encode))
        .route("/decode/json",      post(api::decode::decode_json))
        .route("/decode/upload",    post(api::decode::decode_upload))
        .route("/decode/url",       post(api::decode::decode_url))
        .route("/batch/encode",     post(api::batch::batch_encode))
        .route("/batch/export",     post(api::batch::batch_export))
        .route("/payload/build",    post(api::payload::build_payload))
        .route("/payload/encode",   post(api::payload::build_and_encode))
        .route("/history",          get(api::history::list_history)
                                   .delete(api::history::clear_history))
        .route("/history/{id}",      get(api::history::get_history_entry)
                                   .delete(api::history::delete_history_entry))
        .with_state(state.clone());

    // Rate-limit middleware wrapping entire API
    let lim = limiter.clone();
    let api_router = api_router.layer(axum_mw::from_fn(
        move |req, next| {
            let l = lim.clone();
            async move { enforce(&l, req, next).await }
        },
    ));

    // Optional API-key auth
    let api_router = if let Some(key) = api_key_opt {
        api_router.layer(axum_mw::from_fn(move |req, next| {
            let k = key.clone();
            async move { crate::middleware::auth::require_api_key(k, req, next).await }
        }))
    } else {
        api_router
    };

    // ── WebSocket ──────────────────────────────────────────────────────────────
    let ws_router = Router::new()
        .route("/stats", get(api::ws::ws_stats))
        .with_state(state.clone());

    // ── Full app ───────────────────────────────────────────────────────────────
    let app = Router::new()
        .nest("/api",     api_router)
        .nest("/ws",      ws_router)
        .route("/metrics", get(metrics).with_state(state.clone()))
        .fallback_service(ServeDir::new("static").append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(cfg.max_upload_bytes))
        .layer(TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_secs(30)));

    let addr = SocketAddr::from_str(&cfg.bind_addr())?;
    tracing::info!("qrtools v{} → http://{}", env!("CARGO_PKG_VERSION"), addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    tracing::info!("qrtools shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c    => tracing::info!("received Ctrl-C"),
        _ = terminate => tracing::info!("received SIGTERM"),
    }
}
