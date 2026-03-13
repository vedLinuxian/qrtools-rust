//! Health / readiness probes.

use axum::{Json, extract::State};
use chrono::Utc;
use serde_json::{json, Value};
use crate::state::AppState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// `GET /api/health` — Kubernetes liveness probe (always 200 if binary runs).
pub async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": VERSION,
        "timestamp": Utc::now(),
    }))
}

/// `GET /api/health/ready` — readiness probe with operational stats.
pub async fn ready(State(state): State<AppState>) -> Json<Value> {
    Json(json!({
        "status": "ready",
        "version": VERSION,
        "ops_total": state.total_ops(),
        "history_entries": state.history.entry_count(),
        "timestamp": Utc::now(),
    }))
}
