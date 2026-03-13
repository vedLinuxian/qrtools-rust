use axum::{Json, extract::State};
use chrono::Utc;
use serde_json::{json, Value};
use tracing::instrument;

use crate::{
    error::AppError,
    services::qr_engine::{self, EncodeOptions},
    state::{AppState, HistoryEntry, OperationKind},
};

#[instrument(skip(state, opts), fields(data_len = opts.data.len()))]
pub async fn encode(
    State(state): State<AppState>,
    Json(opts): Json<EncodeOptions>,
) -> Result<Json<Value>, AppError> {
    let result = tokio::task::spawn_blocking({
        let opts = opts.clone();
        move || qr_engine::encode(&opts)
    })
    .await
    .map_err(|e| AppError::EncodeError(e.to_string()))??;

    metrics::counter!("qrtools_encode_total").increment(1);
    state.increment_ops();

    let entry = HistoryEntry {
        id: result.id.clone(),
        kind: OperationKind::Encode,
        summary: truncate(&opts.data, 80),
        timestamp: Utc::now(),
        preview_png_b64: Some(result.thumbnail_b64.clone()),
        content: Some(opts.data.clone()),
    };
    state.history.insert(result.id.clone(), entry.clone()).await;

    let ts = entry.timestamp.to_rfc3339();
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO history (id, kind, summary, timestamp, preview_png_b64, content)
         VALUES (?, 'encode', ?, ?, ?, ?)",
    )
    .bind(&result.id)
    .bind(&entry.summary)
    .bind(&ts)
    .bind(&entry.preview_png_b64)
    .bind(&entry.content)
    .execute(&state.db)
    .await;

    state.push_stats();

    Ok(Json(json!({
        "id":        result.id,
        "mime_type": result.mime_type,
        "data":      result.data,
        "width":     result.width,
        "height":    result.height,
        "thumbnail": result.thumbnail_b64,
        "timestamp": Utc::now(),
    })))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() }
    else { format!("{}…", s.chars().take(max).collect::<String>()) }
}
