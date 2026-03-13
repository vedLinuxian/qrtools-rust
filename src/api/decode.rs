use axum::{
    Json,
    extract::{Multipart, State},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use bytes::Bytes;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::instrument;

use crate::{
    error::AppError,
    state::{AppState, HistoryEntry, OperationKind},
};

#[derive(Debug, Deserialize)]
pub struct DecodeJsonRequest {
    pub image: String,
}

#[derive(Debug, Deserialize)]
pub struct DecodeUrlRequest {
    pub url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct DecodeResponse {
    pub id: String,
    pub text: String,
    pub format: String,
    pub timestamp: chrono::DateTime<Utc>,
}

#[instrument(skip(state, req))]
pub async fn decode_json(
    State(state): State<AppState>,
    Json(req): Json<DecodeJsonRequest>,
) -> Result<Json<Value>, AppError> {
    let raw = strip_data_uri(&req.image).to_string();
    let bytes = B64.decode(&raw)
        .map_err(|e| AppError::InvalidInput(format!("base64 decode failed: {e}")))?;
    let resp = decode_bytes(&bytes)?;
    record(&state, &resp).await;
    Ok(Json(json!(resp)))
}

#[instrument(skip(state, multipart))]
pub async fn decode_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let max = state.config.max_upload_bytes;
    let mut bytes_opt: Option<Bytes> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::InvalidInput(format!("multipart error: {e}"))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if matches!(name.as_str(), "file" | "image" | "") {
            let data = field.bytes().await.map_err(|e| {
                AppError::InvalidInput(format!("field read error: {e}"))
            })?;
            if data.len() > max {
                return Err(AppError::PayloadTooLarge { max_bytes: max });
            }
            bytes_opt = Some(data);
            break;
        }
    }
    let bytes = bytes_opt.ok_or_else(|| {
        AppError::InvalidInput("no file field found in multipart body".into())
    })?;
    let resp = decode_bytes(&bytes)?;
    record(&state, &resp).await;
    Ok(Json(json!(resp)))
}

#[instrument(skip(state, req))]
pub async fn decode_url(
    State(state): State<AppState>,
    Json(req): Json<DecodeUrlRequest>,
) -> Result<Json<Value>, AppError> {
    let max = state.config.max_remote_fetch_bytes;
    if !req.url.starts_with("http://") && !req.url.starts_with("https://") {
        return Err(AppError::InvalidInput("URL must start with http:// or https://".into()));
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::RemoteFetch(e.to_string()))?;
    let response = client.get(&req.url).send().await
        .map_err(|e| AppError::RemoteFetch(format!("fetch failed: {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::RemoteFetch(format!("remote returned HTTP {}", response.status())));
    }
    let mut collected: Vec<u8> = Vec::new();
    let mut stream = response.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::RemoteFetch(e.to_string()))?;
        collected.extend_from_slice(&chunk);
        if collected.len() > max {
            return Err(AppError::PayloadTooLarge { max_bytes: max });
        }
    }
    let resp = decode_bytes(&collected)?;
    record(&state, &resp).await;
    Ok(Json(json!(resp)))
}

fn decode_bytes(bytes: &[u8]) -> Result<DecodeResponse, AppError> {
    let img = image::load_from_memory(bytes)?;
    let luma = img.to_luma8();
    let (w, h) = (luma.width(), luma.height());
    let raw: Vec<u8> = luma.into_raw();
    let result = rxing::helpers::detect_in_luma(raw, w, h, None)
        .map_err(|e| AppError::DecodeError(e.to_string()))?;
    metrics::counter!("qrtools_decode_total").increment(1);
    Ok(DecodeResponse {
        id: uuid::Uuid::new_v4().to_string(),
        text: result.getText().to_string(),
        format: result.getBarcodeFormat().to_string(),
        timestamp: Utc::now(),
    })
}

fn strip_data_uri(s: &str) -> &str {
    if let Some(pos) = s.find(";base64,") { &s[pos + 8..] } else { s }
}

async fn record(state: &AppState, resp: &DecodeResponse) {
    let entry = HistoryEntry {
        id: resp.id.clone(),
        kind: OperationKind::Decode,
        summary: truncate(&resp.text, 80),
        timestamp: resp.timestamp,
        preview_png_b64: None,
        content: Some(resp.text.clone()),
    };
    state.history.insert(resp.id.clone(), entry.clone()).await;
    state.increment_ops();
    let ts = resp.timestamp.to_rfc3339();
    let _ = sqlx::query(
        "INSERT OR REPLACE INTO history (id, kind, summary, timestamp, content)
         VALUES (?, 'decode', ?, ?, ?)",
    )
    .bind(&resp.id)
    .bind(&entry.summary)
    .bind(&ts)
    .bind(&resp.text)
    .execute(&state.db)
    .await;
    state.push_stats();
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max { s.to_string() }
    else { format!("{}…", s.chars().take(max).collect::<String>()) }
}
