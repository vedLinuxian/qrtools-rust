use std::sync::Arc;

use chrono::{DateTime, Utc};
use moka::future::Cache;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Shared application state injected into every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// LRU cache of recent encode/decode operations
    pub history: Cache<String, HistoryEntry>,
    /// Prometheus-compatible request counter (operations total)
    pub ops_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let history = Cache::builder()
            .max_capacity(config.history_capacity)
            .build();

        Self {
            config: Arc::new(config),
            history,
            ops_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn increment_ops(&self) {
        self.ops_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn total_ops(&self) -> u64 {
        self.ops_counter
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// A history record for one encode or decode operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub kind: OperationKind,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
    /// Base64-encoded PNG preview (if applicable)
    pub preview_png_b64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Encode,
    Decode,
}
