use axum::{extract::State, Json};
use chrono::Utc;
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /api/health — liveness probe
pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /api/health/ready — readiness probe with stats
pub async fn ready(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ready",
        "timestamp": Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION"),
        "ops_total": state.total_ops(),
        "history_entries": state.history.entry_count(),
    }))
}
