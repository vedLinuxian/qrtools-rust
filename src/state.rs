use std::sync::Arc;
use chrono::{DateTime, Utc};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::sync::broadcast;
use crate::config::Config;

/// Shared application state, cloned cheaply into every request handler.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    /// Fast, in-memory LRU cache for recent operations.
    pub history: Cache<String, HistoryEntry>,
    /// SQLite connection pool for durable storage.
    pub db: SqlitePool,
    /// Atomic total operation counter.
    pub ops_counter: Arc<std::sync::atomic::AtomicU64>,
    /// Broadcast channel for WebSocket real-time stats pushes.
    pub stats_tx: broadcast::Sender<ServerStats>,
}

impl AppState {
    pub fn new(config: Config, db: SqlitePool) -> Self {
        let history = Cache::builder()
            .max_capacity(config.history_capacity)
            .build();
        let (stats_tx, _) = broadcast::channel(256);
        Self {
            config: Arc::new(config),
            history,
            db,
            ops_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            stats_tx,
        }
    }

    pub fn increment_ops(&self) {
        self.ops_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn total_ops(&self) -> u64 {
        self.ops_counter.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Broadcast a snapshot to all connected WebSocket subscribers.
    pub fn push_stats(&self) {
        let stats = ServerStats {
            ops_total: self.total_ops(),
            history_entries: self.history.entry_count(),
            timestamp: Utc::now(),
        };
        // Ignore error if there are no subscribers.
        let _ = self.stats_tx.send(stats);
    }
}

// ── Data models ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub kind: OperationKind,
    pub summary: String,
    pub timestamp: DateTime<Utc>,
    pub preview_png_b64: Option<String>,
    /// The decoded / encoded text (persisted to DB only)
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OperationKind {
    Encode,
    Decode,
    Batch,
}

/// Lightweight snapshot pushed over WebSocket once per operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub ops_total: u64,
    pub history_entries: u64,
    pub timestamp: DateTime<Utc>,
}
