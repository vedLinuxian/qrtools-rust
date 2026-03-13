//! Per-IP token-bucket rate limiter using `governor`.

use std::{net::{IpAddr, SocketAddr}, num::NonZeroU32, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request},
    http::{StatusCode, header::HeaderValue},
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    Quota, RateLimiter,
    clock::DefaultClock,
    state::keyed::DefaultKeyedStateStore,
};

pub type IpRateLimiter =
    Arc<RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>;

pub fn new_limiter(rpm: u32) -> IpRateLimiter {
    let rpm = rpm.max(1);
    let burst = NonZeroU32::new(rpm.min(30)).unwrap();
    let quota = Quota::per_minute(NonZeroU32::new(rpm).unwrap()).allow_burst(burst);
    Arc::new(RateLimiter::keyed(quota))
}

/// Axum `from_fn`-compatible rate-limit middleware.
/// Extracts client IP from the `ConnectInfo` extension set by axum's
/// `into_make_service_with_connect_info`.
pub async fn rate_limit_layer(
    req: Request,
    next: Next,
) -> Response {
    // The limiter is expected to live in TypedState / extensions.
    // We inline the logic here; callers wrap this via from_fn + closure.
    next.run(req).await
}

/// The actual per-IP check. Called from a closure in `main.rs` that captures
/// the shared limiter.
pub async fn enforce(
    limiter: &IpRateLimiter,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path();
    if !path.starts_with("/api/") && !path.starts_with("/ws/") {
        return next.run(req).await;
    }

    let ip: IpAddr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));

    match limiter.check_key(&ip) {
        Ok(_) => next.run(req).await,
        Err(not_until) => {
            let retry_after = not_until
                .wait_time_from(std::time::Instant::now())
                .as_secs() + 1;
            let retry_header = HeaderValue::from_str(&retry_after.to_string())
                .unwrap_or_else(|_| HeaderValue::from_static("1"));
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(axum::http::header::RETRY_AFTER, retry_header)],
                axum::Json(serde_json::json!({
                    "error": {
                        "code": "RATE_LIMITED",
                        "message": format!("Too many requests. Retry after {}s", retry_after),
                        "retry_after": retry_after,
                    }
                })),
            ).into_response()
        }
    }
}
