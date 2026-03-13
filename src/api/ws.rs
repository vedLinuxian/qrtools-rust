//! `GET /ws/stats` — WebSocket endpoint that pushes real-time server stats.
//!
//! Each connected client receives a JSON `ServerStats` message whenever any
//! encode/decode/batch operation completes.

use axum::{
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};

use crate::state::AppState;

pub async fn ws_stats(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let mut rx = state.stats_tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    // Send current snapshot immediately upon connection
    let initial = crate::state::ServerStats {
        ops_total: state.total_ops(),
        history_entries: state.history.entry_count(),
        timestamp: chrono::Utc::now(),
    };
    if let Ok(payload) = serde_json::to_string(&initial) {
        let _ = sender.send(Message::Text(payload.into())).await;
    }

    loop {
        tokio::select! {
            // Forward stats broadcast to the WebSocket client
            result = rx.recv() => {
                match result {
                    Ok(stats) => {
                        if let Ok(payload) = serde_json::to_string(&stats) {
                            if sender.send(Message::Text(payload.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break, // channel lagged / closed
                }
            }
            // Handle pings and close frames from the client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        let _ = sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
