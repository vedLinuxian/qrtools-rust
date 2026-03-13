//! `POST /api/batch/encode`  — encode multiple items concurrently, return results.
//! `POST /api/batch/export`  — encode multiple items and return a ZIP archive.

use axum::{
    Json,
    extract::State,
    http::header,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::Utc;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Write;
use tracing::instrument;

use crate::{
    error::AppError,
    services::qr_engine::{self, EncodeOptions},
    state::{AppState, HistoryEntry, OperationKind},
};

const MAX_BATCH: usize = 200;

// ── Request / response ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct BatchEncodeRequest {
    pub items: Vec<BatchItem>,
    /// Shared encode options applied to every item (overridden per-item).
    #[serde(default)]
    pub defaults: BatchDefaults,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct BatchDefaults {
    pub format:        Option<crate::services::qr_engine::OutputFormat>,
    pub ec_level:      Option<String>,
    pub module_size:   Option<u32>,
    pub quiet_zone:    Option<u32>,
    pub foreground:    Option<String>,
    pub background:    Option<String>,
    pub module_style:  Option<crate::services::qr_engine::ModuleStyle>,
    pub badge_label:   Option<String>,
    pub jpeg_quality:  Option<u8>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchItem {
    pub data: String,
    /// Per-item overrides — if absent, `defaults` value is used.
    pub format: Option<crate::services::qr_engine::OutputFormat>,
    pub ec_level: Option<String>,
    pub module_size: Option<u32>,
    pub foreground: Option<String>,
    pub background: Option<String>,
    pub module_style: Option<crate::services::qr_engine::ModuleStyle>,
    pub badge_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchResultItem {
    pub index: usize,
    pub data: String,
    pub ok: bool,
    pub id: Option<String>,
    pub mime_type: Option<String>,
    pub image: Option<String>,
    pub thumbnail: Option<String>,
    pub error: Option<String>,
}

// ── Batch encode → JSON ───────────────────────────────────────────────────────

#[instrument(skip(state, req), fields(items = req.items.len()))]
pub async fn batch_encode(
    State(state): State<AppState>,
    Json(req): Json<BatchEncodeRequest>,
) -> Result<Json<Value>, AppError> {
    if req.items.is_empty() {
        return Err(AppError::InvalidInput("items must not be empty".into()));
    }
    if req.items.len() > MAX_BATCH {
        return Err(AppError::InvalidInput(format!(
            "batch size {n} exceeds maximum of {MAX_BATCH}",
            n = req.items.len()
        )));
    }

    let defaults = req.defaults.clone();
    let futures: Vec<_> = req
        .items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let opts = merge_opts(item, &defaults);
            tokio::task::spawn_blocking(move || {
                let result = qr_engine::encode(&opts);
                (idx, opts.data.clone(), result)
            })
        })
        .collect();

    let joined = join_all(futures).await;

    let mut results: Vec<BatchResultItem> = Vec::with_capacity(joined.len());
    for handle in joined {
        match handle {
            Ok((idx, data, Ok(r))) => {
                results.push(BatchResultItem {
                    index: idx,
                    data,
                    ok: true,
                    id: Some(r.id),
                    mime_type: Some(r.mime_type),
                    image: Some(r.data),
                    thumbnail: Some(r.thumbnail_b64),
                    error: None,
                });
            }
            Ok((idx, data, Err(e))) => {
                results.push(BatchResultItem {
                    index: idx,
                    data,
                    ok: false,
                    id: None,
                    mime_type: None,
                    image: None,
                    thumbnail: None,
                    error: Some(e.to_string()),
                });
            }
            Err(e) => {
                results.push(BatchResultItem {
                    index: 0,
                    data: String::new(),
                    ok: false,
                    id: None,
                    mime_type: None,
                    image: None,
                    thumbnail: None,
                    error: Some(format!("task panic: {e}")),
                });
            }
        }
    }

    let success = results.iter().filter(|r| r.ok).count();
    metrics::counter!("qrtools_batch_total").increment(success as u64);
    state.ops_counter.fetch_add(success as u64, std::sync::atomic::Ordering::Relaxed);

    // Record batch operation in history
    let batch_id = uuid::Uuid::new_v4().to_string();
    let summary = format!("Batch {success}/{} encoded", results.len());
    let first_thumb = results.iter().find(|r| r.ok).and_then(|r| r.thumbnail.clone());
    let entry = HistoryEntry {
        id: batch_id.clone(),
        kind: OperationKind::Batch,
        summary: summary.clone(),
        timestamp: Utc::now(),
        preview_png_b64: first_thumb,
        content: None,
    };
    state.history.insert(batch_id.clone(), entry).await;
    state.push_stats();

    Ok(Json(json!({
        "batch_id": batch_id,
        "total": results.len(),
        "success": success,
        "failed": results.len() - success,
        "items": results,
        "timestamp": Utc::now(),
    })))
}

// ── Batch export → ZIP ────────────────────────────────────────────────────────

#[instrument(skip(state, req), fields(items = req.items.len()))]
pub async fn batch_export(
    State(state): State<AppState>,
    Json(req): Json<BatchEncodeRequest>,
) -> Result<Response, AppError> {
    if req.items.is_empty() {
        return Err(AppError::InvalidInput("items must not be empty".into()));
    }
    if req.items.len() > MAX_BATCH {
        return Err(AppError::InvalidInput(format!(
            "batch size {} exceeds maximum of {MAX_BATCH}",
            req.items.len()
        )));
    }

    let defaults = req.defaults.clone();
    let futures: Vec<_> = req
        .items
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            let opts = merge_opts(item, &defaults);
            tokio::task::spawn_blocking(move || {
                let result = qr_engine::encode(&opts);
                (idx, result)
            })
        })
        .collect();

    let joined = join_all(futures).await;

    // Build ZIP in memory
    let zip_bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
        let mut buf = Vec::new();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        for handle in joined {
            let (idx, result) = handle.map_err(|e| AppError::Archive(e.to_string()))?;
            match result {
                Ok(r) => {
                    // Decode the base64 back to raw bytes for the ZIP entry
                    let ext = match r.mime_type.as_str() {
                        "image/jpeg" => "jpg",
                        "image/webp" => "webp",
                        "image/svg+xml" => "svg",
                        _ => "png",
                    };
                    let filename = format!("qr_{:04}.{}", idx, ext);
                    zip.start_file(&filename, options)
                        .map_err(|e| AppError::Archive(e.to_string()))?;

                    let raw = if ext == "svg" {
                        r.data.into_bytes()
                    } else {
                        B64.decode(&r.data)
                            .map_err(|e| AppError::Archive(e.to_string()))?
                    };
                    zip.write_all(&raw)
                        .map_err(|e| AppError::Archive(e.to_string()))?;
                }
                Err(e) => {
                    let filename = format!("qr_{:04}.err.txt", idx);
                    zip.start_file(&filename, options)
                        .map_err(|e| AppError::Archive(e.to_string()))?;
                    zip.write_all(e.to_string().as_bytes())
                        .map_err(|e2| AppError::Archive(e2.to_string()))?;
                }
            }
        }

        zip.finish().map_err(|e| AppError::Archive(e.to_string()))?;
        Ok(buf)
    })
    .await
    .map_err(|e| AppError::Archive(e.to_string()))??;

    metrics::counter!("qrtools_export_total").increment(1);
    state.increment_ops();
    state.push_stats();

    let response = (
        [
            (header::CONTENT_TYPE, "application/zip"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"qrcodes.zip\"",
            ),
        ],
        zip_bytes,
    )
        .into_response();

    Ok(response)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn merge_opts(item: BatchItem, defaults: &BatchDefaults) -> EncodeOptions {
    use crate::services::qr_engine::OutputFormat;
    EncodeOptions {
        data: item.data,
        format: item.format.or_else(|| defaults.format.clone()).unwrap_or(OutputFormat::Png),
        ec_level: item.ec_level.or_else(|| defaults.ec_level.clone()).unwrap_or_else(|| "M".into()),
        module_size: item.module_size.or(defaults.module_size).unwrap_or(10),
        quiet_zone: defaults.quiet_zone.unwrap_or(4),
        foreground: item.foreground.or_else(|| defaults.foreground.clone()).unwrap_or_else(|| "#000000".into()),
        background: item.background.or_else(|| defaults.background.clone()).unwrap_or_else(|| "#ffffff".into()),
        module_style: item.module_style.or_else(|| defaults.module_style.clone()).unwrap_or_default(),
        gradient_color: None,
        logo_base64: None,
        logo_ratio: 0.22,
        badge_label: item.badge_label.or_else(|| defaults.badge_label.clone()),
        jpeg_quality: defaults.jpeg_quality.unwrap_or(85u8),
    }
}
