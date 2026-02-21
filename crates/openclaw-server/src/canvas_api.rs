//! 画布 API 路由

use axum::{
    Json, Router,
    extract::{
        Path, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use futures::{SinkExt, StreamExt};
use openclaw_canvas::{
    CanvasId, CanvasManager, CanvasOps, CanvasState, CollabEvent, CollabManager, DrawAction,
    Element, ElementUpdate, UserColorGenerator, UserCursor, UserInfo, WsMessage,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

/// 画布 API 状态
#[derive(Clone)]
pub struct CanvasApiState {
    pub canvas_manager: Arc<CanvasManager>,
    pub collab_manager: Arc<CollabManager>,
    pub color_generator: Arc<UserColorGenerator>,
}

impl CanvasApiState {
    pub fn new() -> Self {
        Self {
            canvas_manager: Arc::new(CanvasManager::new()),
            collab_manager: Arc::new(CollabManager::new()),
            color_generator: Arc::new(UserColorGenerator::new()),
        }
    }

    pub fn with_manager(manager: Arc<CanvasManager>) -> Self {
        Self {
            canvas_manager: manager,
            collab_manager: Arc::new(CollabManager::new()),
            color_generator: Arc::new(UserColorGenerator::new()),
        }
    }
}

impl Default for CanvasApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// 创建画布 API 路由
pub fn create_canvas_router(state: CanvasApiState) -> Router {
    Router::new()
        .route("/canvas", post(create_canvas))
        .route("/canvas", get(list_canvases))
        .route("/canvas/{id}", get(get_canvas))
        .route("/canvas/{id}", delete(delete_canvas))
        .route("/canvas/{id}/elements", post(add_element))
        .route("/canvas/{id}/elements/{element_id}", put(update_element))
        .route("/canvas/{id}/elements/{element_id}", delete(delete_element))
        .route("/canvas/{id}/clear", post(clear_canvas))
        .route("/canvas/{id}/ws", get(canvas_websocket))
        .with_state(state)
}

/// 创建画布请求
#[derive(Debug, Deserialize)]
pub struct CreateCanvasRequest {
    pub name: String,
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// 创建画布响应
#[derive(Debug, Serialize)]
pub struct CreateCanvasResponse {
    pub id: CanvasId,
}

/// 创建画布
async fn create_canvas(
    State(state): State<CanvasApiState>,
    Json(req): Json<CreateCanvasRequest>,
) -> Json<CreateCanvasResponse> {
    let id = state
        .canvas_manager
        .create_canvas(
            req.name,
            req.width.unwrap_or(1920.0),
            req.height.unwrap_or(1080.0),
        )
        .await;

    Json(CreateCanvasResponse { id })
}

/// 列出画布
async fn list_canvases(
    State(state): State<CanvasApiState>,
) -> Json<Vec<openclaw_canvas::CanvasInfo>> {
    let canvases = state.canvas_manager.list_canvases().await;
    Json(canvases)
}

/// 获取画布
async fn get_canvas(
    State(state): State<CanvasApiState>,
    Path(id): Path<CanvasId>,
) -> Result<Json<CanvasState>, String> {
    state
        .canvas_manager
        .get_canvas_state(&id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

/// 删除画布
async fn delete_canvas(
    State(state): State<CanvasApiState>,
    Path(id): Path<CanvasId>,
) -> Result<Json<serde_json::Value>, String> {
    state
        .canvas_manager
        .delete_canvas(&id)
        .await
        .map(|_| Json(serde_json::json!({"success": true})))
        .map_err(|e| e.to_string())
}

/// 添加元素请求
#[derive(Debug, Deserialize)]
pub struct AddElementRequest {
    pub element: Element,
}

/// 添加元素
async fn add_element(
    State(state): State<CanvasApiState>,
    Path(canvas_id): Path<CanvasId>,
    Json(req): Json<AddElementRequest>,
) -> Result<Json<serde_json::Value>, String> {
    let canvas = state
        .canvas_manager
        .get_canvas(&canvas_id)
        .await
        .ok_or("画布不存在")?;

    let element_id = CanvasOps::add_element(&canvas, req.element)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({ "id": element_id })))
}

/// 更新元素
async fn update_element(
    State(state): State<CanvasApiState>,
    Path((canvas_id, element_id)): Path<(CanvasId, String)>,
    Json(updates): Json<ElementUpdate>,
) -> Result<Json<serde_json::Value>, String> {
    let canvas = state
        .canvas_manager
        .get_canvas(&canvas_id)
        .await
        .ok_or("画布不存在")?;

    CanvasOps::update_element(&canvas, &element_id, updates)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 删除元素
async fn delete_element(
    State(state): State<CanvasApiState>,
    Path((canvas_id, element_id)): Path<(CanvasId, String)>,
) -> Result<Json<serde_json::Value>, String> {
    let canvas = state
        .canvas_manager
        .get_canvas(&canvas_id)
        .await
        .ok_or("画布不存在")?;

    CanvasOps::delete_element(&canvas, &element_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// 清空画布
async fn clear_canvas(
    State(state): State<CanvasApiState>,
    Path(canvas_id): Path<CanvasId>,
) -> Result<Json<serde_json::Value>, String> {
    let canvas = state
        .canvas_manager
        .get_canvas(&canvas_id)
        .await
        .ok_or("画布不存在")?;

    CanvasOps::clear_canvas(&canvas)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(serde_json::json!({"success": true})))
}

/// WebSocket 连接
async fn canvas_websocket(
    State(state): State<CanvasApiState>,
    Path(canvas_id): Path<CanvasId>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_canvas_ws(socket, state, canvas_id))
}

/// 处理 WebSocket 连接
async fn handle_canvas_ws(socket: WebSocket, state: CanvasApiState, canvas_id: CanvasId) {
    let (mut tx, mut rx) = socket.split();

    // 获取或创建协作会话
    let session = state
        .collab_manager
        .get_or_create_session(canvas_id.clone())
        .await;

    // 生成用户信息
    let user_id = uuid::Uuid::new_v4().to_string();
    let user = UserInfo {
        id: user_id.clone(),
        name: format!("User-{}", &user_id[..4]),
        color: state.color_generator.next(),
        avatar_url: None,
    };

    // 加入会话
    let mut event_rx = session.join(user.clone()).await;

    info!("用户 {} 连接到画布 {}", user_id, canvas_id);

    // 发送初始同步信息
    if let Ok(canvas_state) = state.canvas_manager.get_canvas_state(&canvas_id).await {
        let users = session.get_users().await;
        let cursors = session.get_cursors().await;

        let sync_msg = WsMessage::SyncResponse {
            canvas_state,
            users,
            cursors,
        };

        if let Ok(msg) = serde_json::to_string(&sync_msg)
            && let Err(e) = tx.send(Message::Text(msg.into())).await {
                tracing::warn!("Failed to send sync response to WebSocket: {}", e);
            }
    }

    // 处理消息循环
    loop {
        tokio::select! {
            // 接收客户端消息
            msg = rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                            handle_ws_message(&state, &session, &canvas_id, &user_id, ws_msg).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
            // 接收协作事件
            event = event_rx.recv() => {
                if let Ok(event) = event
                    && let Ok(msg) = serde_json::to_string(&CollabEventWrapper { event })
                        && let Err(e) = tx.send(Message::Text(msg.into())).await {
                            tracing::warn!("Failed to send collab event to WebSocket: {}", e);
                        }
            }
        }
    }

    // 离开会话
    session.leave(&user_id).await;
    state.collab_manager.cleanup_session(&canvas_id).await;

    info!("用户 {} 断开画布 {}", user_id, canvas_id);
}

/// 处理 WebSocket 消息
async fn handle_ws_message(
    state: &CanvasApiState,
    session: &Arc<openclaw_canvas::CollabSession>,
    canvas_id: &CanvasId,
    user_id: &str,
    msg: WsMessage,
) {
    match msg {
        WsMessage::CursorMove { position, tool } => {
            let cursor = UserCursor {
                user_id: user_id.to_string(),
                position,
                color: openclaw_canvas::Color::black(),
                name: String::new(),
                tool,
            };
            session.update_cursor(cursor).await;
        }
        WsMessage::DrawAction { action } => {
            if let Some(canvas) = state.canvas_manager.get_canvas(canvas_id).await {
                // 应用绘图操作
                match &action {
                    DrawAction::AddElement { element } => {
                        if let Err(e) = CanvasOps::add_element(&canvas, element.clone()).await {
                            tracing::warn!("Failed to add canvas element: {}", e);
                        }
                    }
                    DrawAction::DeleteElement { element } => {
                        if let Err(e) = CanvasOps::delete_element(&canvas, &element.id).await {
                            tracing::warn!("Failed to delete canvas element: {}", e);
                        }
                    }
                    _ => {}
                }
            }
        }
        WsMessage::ViewportChange { viewport } => {
            // 可以广播视口变化给其他用户
            debug!("用户 {} 视口变化: {:?}", user_id, viewport);
        }
        _ => {}
    }
}

/// 协作事件包装
#[derive(Debug, Serialize)]
struct CollabEventWrapper {
    event: CollabEvent,
}
