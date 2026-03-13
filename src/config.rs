use std::env;

/// Application configuration — loaded from environment variables or `.env`.
#[derive(Debug, Clone)]
pub struct Config {
    // ── Server ────────────────────────────────────────────────────────────────
    pub host: String,
    pub port: u16,
    // ── Limits ────────────────────────────────────────────────────────────────
    pub max_upload_bytes: usize,
    // ── Rate limiting (requests per minute, per IP) ───────────────────────────
    pub rate_limit_rpm: u32,
    // ── History / cache ───────────────────────────────────────────────────────
    pub history_capacity: u64,
    // ── Persistence ───────────────────────────────────────────────────────────
    /// SQLite database path; `:memory:` for ephemeral mode.
    pub db_path: String,
    // ── Observability ─────────────────────────────────────────────────────────
    pub log_level: String,
    pub json_logging: bool,
    pub metrics_enabled: bool,
    // ── CORS ──────────────────────────────────────────────────────────────────
    pub cors_origins: Vec<String>,
    // ── Auth (optional API-key header) ────────────────────────────────────────
    /// If set, every API request must carry `X-API-Key: <value>`.
    pub api_key: Option<String>,
    // ── HTTP client ───────────────────────────────────────────────────────────
    pub max_remote_fetch_bytes: usize,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
        let max_upload_bytes: usize = env::var("MAX_UPLOAD_BYTES")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(10 * 1024 * 1024);
        let rate_limit_rpm: u32 = env::var("RATE_LIMIT_RPM")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(120);
        let history_capacity: u64 = env::var("HISTORY_CAPACITY")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(500);
        let db_path = env::var("DB_PATH").unwrap_or_else(|_| "qrtools.db".to_string());
        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',').map(|s| s.trim().to_string()).collect();
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let json_logging = env::var("JSON_LOGGING")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1").unwrap_or(false);
        let metrics_enabled = env::var("METRICS_ENABLED")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1").unwrap_or(true);
        let api_key = env::var("API_KEY").ok().filter(|k| !k.is_empty());
        let max_remote_fetch_bytes: usize = env::var("MAX_REMOTE_FETCH_BYTES")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(20 * 1024 * 1024);

        Self {
            host, port, max_upload_bytes, rate_limit_rpm, history_capacity,
            db_path, log_level, json_logging, metrics_enabled, cors_origins,
            api_key, max_remote_fetch_bytes,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
