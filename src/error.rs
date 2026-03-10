use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// Application-level errors with HTTP response mapping.
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

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::EncodeError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "ENCODE_ERROR", msg.clone())
            }
            AppError::DecodeError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "DECODE_ERROR", msg.clone())
            }
            AppError::ImageError(e) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "IMAGE_ERROR",
                e.to_string(),
            ),
            AppError::InvalidInput(msg) => {
                (StatusCode::BAD_REQUEST, "INVALID_INPUT", msg.clone())
            }
            AppError::PayloadTooLarge { max_bytes } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                format!("Payload exceeds maximum size of {} bytes", max_bytes),
            ),
            AppError::UnsupportedMediaType(msg) => (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "UNSUPPORTED_MEDIA_TYPE",
                msg.clone(),
            ),
            AppError::Internal(e) => {
                tracing::error!(error = %e, "Internal server error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
