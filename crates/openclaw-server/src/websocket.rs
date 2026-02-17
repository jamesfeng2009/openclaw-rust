//! WebSocket 支持

use axum::{
    Router,
    extract::State,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct WebSocketState {
    pub connections: Arc<RwLock<Vec<String>>>,
}

impl WebSocketState {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

pub fn websocket_router() -> Router {
    let state = WebSocketState::new();
    Router::new()
        .route("/ws", get(websocket_handler))
        .with_state(Arc::new(RwLock::new(state)))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<RwLock<WebSocketState>>>,
) -> Response {
    let connections = {
        let s = state.read().await;
        s.connections.clone()
    };

    ws.on_upgrade(move |socket| handle_socket(socket, connections))
}

async fn handle_socket(socket: WebSocket, connections: Arc<RwLock<Vec<String>>>) {
    let (mut sender, mut receiver) = socket.split();

    let conn_id = uuid::Uuid::new_v4().to_string();
    connections.write().await.push(conn_id.clone());

    tracing::info!("WebSocket client connected: {}", conn_id);

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::info!("Received: {}", text);

                let response = format!("Echo: {}", text);
                if sender.send(Message::Text(response.into())).await.is_err() {
                    break;
                }
            }
            Ok(Message::Binary(data)) => {
                tracing::info!("Received binary: {} bytes", data.len());
            }
            Ok(Message::Ping(data)) => {
                let _ = sender.send(Message::Pong(data)).await;
            }
            Ok(Message::Pong(_)) => {}
            Ok(Message::Close(_)) => {
                tracing::info!("Client closed connection");
                break;
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    connections.write().await.retain(|id| *id != conn_id);
    tracing::info!("WebSocket client disconnected: {}", conn_id);
}
