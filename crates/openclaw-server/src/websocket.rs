//! WebSocket 支持 (简化版)

use axum::{
    extract::ws::{WebSocketUpgrade},
    response::Response,
    routing::get,
    Router,
};

/// WebSocket 路由
pub fn websocket_router() -> Router {
    Router::new().route("/ws", get(websocket_handler))
}

/// WebSocket 处理器
async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    // TODO: 实现完整的 WebSocket 处理
    ws.on_upgrade(|_socket| async move {
        tracing::info!("WebSocket connection established");
        // 暂时不处理消息
    })
}
