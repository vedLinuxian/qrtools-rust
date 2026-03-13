use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::{error::AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}
fn default_limit() -> usize { 50 }

pub async fn list_history(
    State(state): State<AppState>,
    Query(q): Query<HistoryQuery>,
) -> Json<Value> {
    let limit = q.limit.min(500);
    let mut entries: Vec<_> = state.history.iter().map(|(_, v)| v).collect();
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    let total = entries.len();
    let page: Vec<_> = entries.into_iter().skip(q.offset).take(limit).collect();
    Json(json!({ "total": total, "limit": limit, "offset": q.offset, "entries": page }))
}

pub async fn get_history_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, AppError> {
    match state.history.get(&id).await {
        Some(entry) => Ok(Json(json!(entry))),
        None => Err(AppError::InvalidInput(format!("history entry '{id}' not found"))),
    }
}

pub async fn clear_history(State(state): State<AppState>) -> Json<Value> {
    state.history.invalidate_all();
    let _ = sqlx::query("DELETE FROM history").execute(&state.db).await;
    state.push_stats();
    Json(json!({ "cleared": true }))
}

pub async fn delete_history_entry(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    state.history.invalidate(&id).await;
    let _ = sqlx::query("DELETE FROM history WHERE id = ?")
        .bind(&id)
        .execute(&state.db)
        .await;
    state.push_stats();
    StatusCode::NO_CONTENT
}
