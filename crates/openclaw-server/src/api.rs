//! HTTP API 路由

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent_service::AgentService;
use crate::browser_api::{BrowserApiState, create_browser_router};
use crate::canvas_api::{CanvasApiState, create_canvas_router};

pub fn create_router() -> Router {
    let state = Arc::new(RwLock::new(ApiState::new()));

    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .route("/models", get(list_models))
        .route("/stats", get(get_stats))
        .route("/api/channels", get(list_channels).post(create_channel))
        .route("/api/channels/:id", delete(delete_channel))
        .route("/api/agents", get(list_agents).post(create_agent))
        .route("/api/agents/:id", get(get_agent))
        .route("/api/sessions", get(list_sessions).post(create_session))
        .route("/api/sessions/:id", get(get_session))
        .route("/api/sessions/:id/close", post(close_session))
        .route("/api/agent/message", post(send_agent_message))
        .route("/api/presence", get(get_presence).post(set_presence))
        .with_state(state)
        .merge(create_canvas_router(CanvasApiState::new()))
        .merge(create_browser_router(BrowserApiState::new()))
}

#[derive(Clone)]
pub struct ApiState {
    pub channels: Vec<ChannelInfo>,
    pub agents: Vec<AgentInfo>,
    pub sessions: Vec<SessionInfo>,
    pub presence: String,
    pub agent_service: AgentService,
}

impl ApiState {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            agents: Vec::new(),
            sessions: Vec::new(),
            presence: "online".to_string(),
            agent_service: AgentService::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChannelInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub name: String,
    pub enabled: bool,
    pub config: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub agent_id: Option<String>,
    pub channel_id: Option<String>,
    pub state: String,
}

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

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
}

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

async fn chat_handler(Json(request): Json<ChatRequest>) -> Json<ChatResponse> {
    Json(ChatResponse {
        reply: format!("收到消息: {}", request.message),
        session_id: request
            .session_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        model: request.model.unwrap_or_else(|| "gpt-4".to_string()),
        usage: TokenUsage {
            prompt_tokens: 10,
            completion_tokens: 5,
        },
    })
}

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

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub sessions: usize,
    pub messages: usize,
    pub tokens_used: usize,
}

async fn get_stats() -> Json<StatsResponse> {
    Json(StatsResponse {
        sessions: 0,
        messages: 0,
        tokens_used: 0,
    })
}

async fn list_channels(State(state): State<Arc<RwLock<ApiState>>>) -> Json<Vec<ChannelInfo>> {
    let state = state.read().await;
    Json(state.channels.clone())
}

async fn create_channel(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<ChannelInfo>,
) -> Json<ChannelInfo> {
    let channel = ChannelInfo {
        id: uuid::Uuid::new_v4().to_string(),
        channel_type: input.channel_type,
        name: input.name,
        enabled: input.enabled,
        config: input.config,
    };
    let mut state = state.write().await;
    state.channels.push(channel.clone());
    Json(channel)
}

async fn delete_channel(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    state.channels.retain(|c| c.id != id);
    Json(serde_json::json!({ "success": true }))
}

async fn list_agents(State(state): State<Arc<RwLock<ApiState>>>) -> Json<Vec<AgentInfo>> {
    let state = state.read().await;
    Json(state.agents.clone())
}

async fn get_agent(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<Option<AgentInfo>> {
    let state = state.read().await;
    Json(state.agents.iter().find(|a| a.id == id).cloned())
}

async fn create_agent(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<AgentInfo>,
) -> Json<AgentInfo> {
    let agent = AgentInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name,
        status: "idle".to_string(),
        capabilities: input.capabilities,
    };
    let mut state = state.write().await;
    state.agents.push(agent.clone());
    Json(agent)
}

async fn list_sessions(State(state): State<Arc<RwLock<ApiState>>>) -> Json<Vec<SessionInfo>> {
    let state = state.read().await;
    Json(state.sessions.clone())
}

async fn get_session(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<Option<SessionInfo>> {
    let state = state.read().await;
    Json(state.sessions.iter().find(|s| s.id == id).cloned())
}

#[derive(Debug, Deserialize)]
pub struct CreateSessionRequest {
    pub name: Option<String>,
    pub agent_id: Option<String>,
    pub channel_id: Option<String>,
}

async fn create_session(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<CreateSessionRequest>,
) -> Json<SessionInfo> {
    let session = SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name.unwrap_or_else(|| "新会话".to_string()),
        agent_id: input.agent_id,
        channel_id: input.channel_id,
        state: "active".to_string(),
    };
    let mut state = state.write().await;
    state.sessions.push(session.clone());
    Json(session)
}

async fn close_session(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    if let Some(session) = state.sessions.iter_mut().find(|s| s.id == id) {
        session.state = "closed".to_string();
    }
    Json(serde_json::json!({ "success": true }))
}

#[derive(Debug, Deserialize)]
pub struct AgentMessageRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentMessageResponse {
    pub message: String,
    pub session_id: String,
}

async fn send_agent_message(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<AgentMessageRequest>,
) -> Json<AgentMessageResponse> {
    let session_id = input
        .session_id
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    Json(AgentMessageResponse {
        message: format!("Agent response to: {}", input.message),
        session_id,
    })
}

async fn get_presence(State(state): State<Arc<RwLock<ApiState>>>) -> Json<serde_json::Value> {
    let state = state.read().await;
    Json(serde_json::json!({ "status": state.presence }))
}

async fn set_presence(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let mut state = state.write().await;
    if let Some(status) = input.get("status").and_then(|v| v.as_str()) {
        state.presence = status.to_string();
    }
    Json(serde_json::json!({ "success": true }))
}
