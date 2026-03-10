use std::env;

/// Runtime configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct Config {
    /// Host address to bind the server
    pub host: String,
    /// Port to bind the server
    pub port: u16,
    /// Maximum upload size in bytes (default 10 MB)
    pub max_upload_bytes: usize,
    /// Number of history entries kept in cache
    pub history_capacity: u64,
    /// CORS allowed origins (comma-separated, "*" for all)
    pub cors_origins: Vec<String>,
    /// Log level (e.g. "info", "debug", "warn")
    pub log_level: String,
    /// Enable JSON structured logging
    pub json_logging: bool,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let max_upload_bytes: usize = env::var("MAX_UPLOAD_BYTES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10 * 1024 * 1024); // 10 MB
        let history_capacity: u64 = env::var("HISTORY_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(500);
        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "*".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let log_level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
        let json_logging = env::var("JSON_LOGGING")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        Self {
            host,
            port,
            max_upload_bytes,
            history_capacity,
            cors_origins,
            log_level,
            json_logging,
        }
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
