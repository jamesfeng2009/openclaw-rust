//! HTTP API 路由

use axum::{
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::browser_api::{create_browser_router, BrowserApiState};
use crate::canvas_api::{create_canvas_router, CanvasApiState};

/// 创建完整 API 路由
pub fn create_router() -> Router {
    Router::new()
        // 基础 API
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .route("/models", get(list_models))
        .route("/stats", get(get_stats))
        // 画布 API
        .merge(create_canvas_router(CanvasApiState::new()))
        // 浏览器控制 API
        .merge(create_browser_router(BrowserApiState::new()))
}

/// 健康检查
async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// 聊天请求
#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

/// 聊天响应
#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub session_id: String,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Serialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
}

/// 聊天处理器
async fn chat_handler(Json(request): Json<ChatRequest>) -> Json<ChatResponse> {
    // TODO: 实际的聊天逻辑
    Json(ChatResponse {
        reply: format!("收到消息: {}", request.message),
        session_id: request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        model: request.model.unwrap_or_else(|| "gpt-4".to_string()),
        usage: TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
        },
    })
}

/// 模型列表响应
#[derive(Debug, Serialize)]
pub struct ModelsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
}

/// 列出模型
async fn list_models() -> Json<ModelsResponse> {
    Json(ModelsResponse {
        models: vec![
            ModelInfo {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: "openai".to_string(),
            },
            ModelInfo {
                id: "claude-3-opus".to_string(),
                name: "Claude 3 Opus".to_string(),
                provider: "anthropic".to_string(),
            },
        ],
    })
}

/// 统计信息响应
#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub sessions: usize,
    pub messages: usize,
    pub tokens_used: usize,
}

/// 获取统计信息
async fn get_stats() -> Json<StatsResponse> {
    // TODO: 实际的统计信息
    Json(StatsResponse {
        sessions: 0,
        messages: 0,
        tokens_used: 0,
    })
}
