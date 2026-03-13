use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("QR encode failed: {0}")]
    EncodeError(String),
    #[error("QR decode failed: {0}")]
    DecodeError(String),
    #[error("Image processing error: {0}")]
    ImageError(#[from] image::ImageError),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Payload too large (max {max_bytes} bytes)")]
    PayloadTooLarge { max_bytes: usize },
    #[error("Unsupported media type: {0}")]
    UnsupportedMediaType(String),
    #[error("Rate limit exceeded — retry after {retry_after}s")]
    RateLimited { retry_after: u64 },
    #[error("Unauthorized — provide a valid X-API-Key header")]
    Unauthorized,
    #[error("Remote fetch failed: {0}")]
    RemoteFetch(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Archive error: {0}")]
    Archive(String),
    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::EncodeError(m)   => (StatusCode::UNPROCESSABLE_ENTITY, "ENCODE_ERROR", m.clone()),
            AppError::DecodeError(m)   => (StatusCode::UNPROCESSABLE_ENTITY, "DECODE_ERROR", m.clone()),
            AppError::ImageError(e)    => (StatusCode::UNPROCESSABLE_ENTITY, "IMAGE_ERROR", e.to_string()),
            AppError::InvalidInput(m)  => (StatusCode::BAD_REQUEST, "INVALID_INPUT", m.clone()),
            AppError::PayloadTooLarge { max_bytes } => (
                StatusCode::PAYLOAD_TOO_LARGE, "PAYLOAD_TOO_LARGE",
                format!("Payload exceeds maximum size of {} bytes", max_bytes)),
            AppError::UnsupportedMediaType(m) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE, "UNSUPPORTED_MEDIA_TYPE", m.clone()),
            AppError::RateLimited { retry_after } => (
                StatusCode::TOO_MANY_REQUESTS, "RATE_LIMITED",
                format!("Rate limit exceeded — retry after {}s", retry_after)),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED",
                "Unauthorized — provide a valid X-API-Key header".to_string()),
            AppError::RemoteFetch(m) => (StatusCode::BAD_GATEWAY, "REMOTE_FETCH_ERROR", m.clone()),
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Database error".to_string())
            }
            AppError::Archive(m) => (StatusCode::INTERNAL_SERVER_ERROR, "ARCHIVE_ERROR", m.clone()),
            AppError::Internal(e) => {
                tracing::error!(error = %e, "Internal server error");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred".to_string())
            }
        };
        let body = Json(json!({ "error": { "code": code, "message": message } }));
        (status, body).into_response()
    }
}
