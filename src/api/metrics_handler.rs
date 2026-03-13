//! `GET /metrics` — Prometheus-compatible text metrics endpoint.
//!
//! When `METRICS_ENABLED=false` this returns 404.

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use metrics_exporter_prometheus::PrometheusHandle;

use crate::state::AppState;

pub async fn metrics(State(state): State<AppState>) -> Response {
    if !state.config.metrics_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    // The handle is stored in state; we read it from the once_cell global.
    match PROMETHEUS_HANDLE.get() {
        Some(handle) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")],
            handle.render(),
        )
            .into_response(),
        None => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

use once_cell::sync::OnceCell;
pub static PROMETHEUS_HANDLE: OnceCell<PrometheusHandle> = OnceCell::new();
