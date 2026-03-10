use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::state::AppState;

/// GET /api/history — returns recent encode/decode operations
pub async fn list_history(State(state): State<AppState>) -> Json<Value> {
    let mut entries: Vec<_> = state
        .history
        .iter()
        .map(|(_k, v)| v)
        .collect();

    // Sort newest-first
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Limit to 100 most recent
    entries.truncate(100);

    Json(json!({
        "total": entries.len(),
        "entries": entries,
    }))
}

/// DELETE /api/history — clear all history cache entries
pub async fn clear_history(State(state): State<AppState>) -> Json<Value> {
    state.history.invalidate_all();
    Json(json!({ "cleared": true }))
}
