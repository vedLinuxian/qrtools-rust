//! Optional API-key authentication middleware.
//!
//! If `Config::api_key` is `Some(key)`, every request to `/api/*` must carry
//! `X-API-Key: <key>`.  Health and static paths are exempt.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub async fn require_api_key(
    expected_key: String,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    // Exempt health checks and static assets
    if path.starts_with("/api/health") || !path.starts_with("/api/") {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if provided != expected_key {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": { "code": "UNAUTHORIZED", "message": "Invalid or missing X-API-Key" } })),
        ).into_response();
    }

    next.run(req).await
}
