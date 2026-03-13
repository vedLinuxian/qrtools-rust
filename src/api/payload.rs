//! `POST /api/payload/build` — generate a well-formed QR payload string from
//! structured data (WiFi, vCard, SMS, email, geo, phone, calendar, Bitcoin …)

use axum::{Json, extract::State};
use serde_json::{json, Value};

use crate::{
    error::AppError,
    services::{
        payload_builder::{self, PayloadRequest},
        qr_engine::{self, EncodeOptions, OutputFormat},
    },
    state::AppState,
};

/// Build a payload string only — clients can use it with `/api/encode`.
pub async fn build_payload(
    Json(req): Json<PayloadRequest>,
) -> Result<Json<Value>, AppError> {
    let text = payload_builder::build(&req)?;
    Ok(Json(json!({ "payload": text })))
}

/// Build payload and immediately encode it into a QR code image.
pub async fn build_and_encode(
    State(state): State<AppState>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    // Top-level: { "payload": <PayloadRequest>, "options": <EncodeOptions> }
    let payload_req: PayloadRequest = serde_json::from_value(
        req.get("payload")
            .cloned()
            .ok_or_else(|| AppError::InvalidInput("missing 'payload' field".into()))?,
    )
    .map_err(|e| AppError::InvalidInput(format!("invalid payload: {e}")))?;

    let mut opts: EncodeOptions = req
        .get("options")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(|| EncodeOptions {
            data: String::new(),
            format: OutputFormat::Png,
            ec_level: "M".into(),
            module_size: 10,
            quiet_zone: 4,
            foreground: "#000000".into(),
            background: "#ffffff".into(),
            module_style: Default::default(),
            gradient_color: None,
            logo_base64: None,
            logo_ratio: 0.22,
            badge_label: None,
            jpeg_quality: 85,
        });

    opts.data = payload_builder::build(&payload_req)?;

    let result = tokio::task::spawn_blocking({
        let opts = opts.clone();
        move || qr_engine::encode(&opts)
    })
    .await
    .map_err(|e| AppError::EncodeError(e.to_string()))??;

    metrics::counter!("qrtools_encode_total").increment(1);
    state.increment_ops();
    state.push_stats();

    Ok(Json(json!({
        "payload": opts.data,
        "id":        result.id,
        "mime_type": result.mime_type,
        "data":      result.data,
        "width":     result.width,
        "height":    result.height,
        "thumbnail": result.thumbnail_b64,
    })))
}
