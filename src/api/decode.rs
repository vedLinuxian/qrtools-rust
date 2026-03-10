use axum::{
    body::Bytes,
    extract::{Multipart, State},
    Json,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    state::{AppState, HistoryEntry, OperationKind},
};

/// JSON body for decode-from-base64 requests.
#[derive(Debug, Deserialize)]
pub struct DecodeJsonRequest {
    /// Base64-encoded image data (raw base64 or data URI stripped)
    pub image_b64: String,
}

/// Decoded QR result.
#[derive(Debug, Serialize)]
pub struct DecodeResponse {
    pub id: String,
    /// The decoded text content
    pub text: String,
    /// Format of the barcode found
    pub format: String,
    pub timestamp: String,
}

/// POST /api/decode/json  — accepts base64-encoded image in JSON body
pub async fn decode_json(
    State(state): State<AppState>,
    Json(req): Json<DecodeJsonRequest>,
) -> Result<Json<DecodeResponse>, AppError> {
    let raw = req.image_b64.trim();
    // Strip data URI prefix if present (e.g. "data:image/png;base64,...")
    let raw = raw
        .find(";base64,")
        .map(|i| &raw[i + 9..])
        .unwrap_or(raw);

    let img_bytes = BASE64
        .decode(raw)
        .map_err(|_| AppError::InvalidInput("Invalid base64 in 'image_b64'".to_string()))?;

    decode_bytes(state, &img_bytes).await
}

/// POST /api/decode/upload  — accepts multipart/form-data upload
pub async fn decode_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<DecodeResponse>, AppError> {
    let mut file_bytes: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        AppError::InvalidInput(format!("Multipart error: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "image" {
            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::InvalidInput(format!("Failed to read upload: {}", e)))?;
            file_bytes = Some(data);
            break;
        }
    }

    let bytes = file_bytes.ok_or_else(|| {
        AppError::InvalidInput("No 'file' or 'image' field in multipart".to_string())
    })?;

    if bytes.len() > 20 * 1024 * 1024 {
        return Err(AppError::PayloadTooLarge {
            max_bytes: 20 * 1024 * 1024,
        });
    }

    decode_bytes(state, &bytes).await
}

async fn decode_bytes(state: AppState, img_bytes: &[u8]) -> Result<Json<DecodeResponse>, AppError> {
    // Load & convert to grayscale
    let img = image::load_from_memory(img_bytes)
        .map_err(|e| AppError::DecodeError(format!("Cannot load image: {}", e)))?;

    let gray = img.to_luma8();
    let (width, height) = gray.dimensions();

    // Use rxing high-level helper (None = auto-detect all barcode formats)
    let result = rxing::helpers::detect_in_luma(gray.into_raw(), width, height, None)
        .map_err(|e| AppError::DecodeError(format!("No QR code found: {:?}", e)))?;

    state.increment_ops();

    let id = Uuid::new_v4().to_string();
    let text = result.getText().to_string();
    let format = format!("{:?}", result.getBarcodeFormat());
    let timestamp = Utc::now().to_rfc3339();

    let entry = HistoryEntry {
        id: id.clone(),
        kind: OperationKind::Decode,
        summary: truncate(&text, 60),
        timestamp: Utc::now(),
        preview_png_b64: None,
    };
    state.history.insert(id.clone(), entry).await;

    Ok(Json(DecodeResponse {
        id,
        text,
        format,
        timestamp,
    }))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

