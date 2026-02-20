//! HTTP API 路由

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, post},
};
use openclaw_agent::{Agent, AgentType, BaseAgent};
use openclaw_canvas::CanvasManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::browser_api::{BrowserApiState, create_browser_router};
use crate::canvas_api::{CanvasApiState, create_canvas_router};
use crate::orchestrator::ServiceOrchestrator;
use crate::voice_service::VoiceService;

pub fn create_router(
    orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    voice_service: Arc<VoiceService>,
    canvas_manager: Option<Arc<CanvasManager>>,
) -> Router {
    let state = Arc::new(RwLock::new(ApiState::new(orchestrator, voice_service)));

    let canvas_state = match canvas_manager {
        Some(manager) => CanvasApiState::with_manager(manager),
        None => CanvasApiState::new(),
    };

    Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .route("/models", get(list_models))
        .route("/stats", get(get_stats))
        .route("/voice/tts", post(tts_handler))
        .route("/voice/stt", post(stt_handler))
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
        .merge(create_canvas_router(canvas_state))
        .merge(create_browser_router(BrowserApiState::new()))
}

#[derive(Clone)]
pub struct ApiState {
    pub orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    pub presence: String,
    pub voice_service: Arc<VoiceService>,
}

impl ApiState {
    pub fn new(
        orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
        voice_service: Arc<VoiceService>,
    ) -> Self {
        Self {
            orchestrator,
            presence: "online".to_string(),
            voice_service,
        }
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new(
            Arc::new(RwLock::new(None)),
            Arc::new(VoiceService::new()),
        )
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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
    pub agent_id: Option<String>,
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

async fn chat_handler(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(request): Json<ChatRequest>,
) -> Json<ChatResponse> {
    let session_id = request.session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let agents = orchestrator.list_agents().await;
        
        let agent_id = if let Some(ref requested_agent_id) = request.agent_id {
            if agents.iter().any(|a| a.config.id == *requested_agent_id) {
                requested_agent_id.clone()
            } else {
                agents.first()
                    .map(|a| a.config.id.clone())
                    .unwrap_or_else(|| "default".to_string())
            }
        } else {
            agents.first()
                .map(|a| a.config.id.clone())
                .unwrap_or_else(|| "default".to_string())
        };
        
        let result = orchestrator.process_agent_message(&agent_id, &request.message, &session_id).await;
        
        match result {
            Ok(reply) => {
                return Json(ChatResponse {
                    reply,
                    session_id,
                    model: request.model.unwrap_or_else(|| "gpt-4".to_string()),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                    },
                });
            }
            Err(e) => {
                return Json(ChatResponse {
                    reply: format!("Error: {}", e),
                    session_id,
                    model: request.model.unwrap_or_else(|| "gpt-4".to_string()),
                    usage: TokenUsage {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                    },
                });
            }
        }
    }
    
    Json(ChatResponse {
        reply: format!("Error: No orchestrator available"),
        session_id,
        model: request.model.unwrap_or_else(|| "gpt-4".to_string()),
        usage: TokenUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
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

async fn list_models(State(state): State<Arc<RwLock<ApiState>>>) -> Json<ModelsResponse> {
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        if let Some(provider) = orchestrator.get_ai_provider().await {
            match provider.models().await {
                Ok(models) => {
                    let model_infos: Vec<ModelInfo> = models.into_iter().map(|id| ModelInfo {
                        id: id.clone(),
                        name: id,
                        provider: "dynamic".to_string(),
                    }).collect();
                    return Json(ModelsResponse { models: model_infos });
                }
                Err(e) => {
                    tracing::warn!("Failed to get models from provider: {}", e);
                }
            }
        }
    }
    
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
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let channel_names = orchestrator.list_channels().await;
        let channels = channel_names
            .into_iter()
            .map(|name| ChannelInfo {
                id: name.clone(),
                channel_type: "webchat".to_string(),
                name,
                enabled: true,
                config: None,
            })
            .collect();
        Json(channels)
    } else {
        Json(vec![])
    }
}

async fn create_channel(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<ChannelInfo>,
) -> Json<ChannelInfo> {
    let channel = ChannelInfo {
        id: uuid::Uuid::new_v4().to_string(),
        channel_type: input.channel_type.clone(),
        name: input.name.clone(),
        enabled: input.enabled,
        config: input.config,
    };
    
    let state = state.read().await;
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        if let Err(e) = orchestrator.create_channel(input.name, input.channel_type).await {
            tracing::warn!("Failed to create channel in orchestrator: {}", e);
        }
    }
    
    Json(channel)
}

async fn delete_channel(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        match orchestrator.delete_channel(&id).await {
            Ok(_) => return Json(serde_json::json!({ "success": true, "channel_id": id })),
            Err(e) => return Json(serde_json::json!({ "success": false, "error": format!("{}", e) })),
        }
    }
    Json(serde_json::json!({ "success": false, "error": "No orchestrator available" }))
}

async fn list_agents(State(state): State<Arc<RwLock<ApiState>>>) -> Json<Vec<AgentInfo>> {
    let state = state.read().await;
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let agent_infos = orchestrator.list_agents().await;
        let agents: Vec<AgentInfo> = agent_infos
            .into_iter()
            .map(|info| AgentInfo {
                id: info.config.id,
                name: info.config.name,
                status: format!("{:?}", info.status),
                capabilities: Some(info.config.capabilities.iter().map(|c| format!("{:?}", c)).collect()),
                error: None,
            })
            .collect();
        Json(agents)
    } else {
        Json(vec![])
    }
}

async fn get_agent(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<Option<AgentInfo>> {
    let state = state.read().await;
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let agent_infos = orchestrator.list_agents().await;
        let agent = agent_infos.into_iter().find(|info| info.config.id == id).map(|info| {
            AgentInfo {
                id: info.config.id,
                name: info.config.name,
                status: format!("{:?}", info.status),
                capabilities: Some(info.config.capabilities.iter().map(|c| format!("{:?}", c)).collect()),
                error: None,
            }
        });
        Json(agent)
    } else {
        Json(None)
    }
}

async fn create_agent(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<AgentInfo>,
) -> Json<AgentInfo> {
    let agent_id = uuid::Uuid::new_v4().to_string();
    let agent_type = input.capabilities.as_ref()
        .and_then(|c| c.first())
        .map(|s| match s.as_str() {
            "coder" => AgentType::Coder,
            "researcher" => AgentType::Researcher,
            "writer" => AgentType::Writer,
            _ => AgentType::DataAnalyst,
        })
        .unwrap_or(AgentType::DataAnalyst);
    
    let agent = BaseAgent::from_type(agent_id.clone(), input.name.clone(), agent_type);
    let mut agent_info = AgentInfo {
        id: agent_id.clone(),
        name: input.name,
        status: "idle".to_string(),
        capabilities: input.capabilities,
        error: None,
    };
    
    let state = state.read().await;
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        orchestrator.register_agent(agent_id, Arc::new(agent) as Arc<dyn Agent>).await;
    } else {
        agent_info.error = Some("No orchestrator available".to_string());
        agent_info.status = "failed".to_string();
    }
    
    Json(agent_info)
}

use openclaw_core::session::SessionScope as CoreSessionScope;

async fn list_sessions(State(state): State<Arc<RwLock<ApiState>>>) -> Json<Vec<SessionInfo>> {
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let sessions = orchestrator.list_sessions(None, None).await.unwrap_or_default();
        let session_infos: Vec<SessionInfo> = sessions.into_iter().map(|s| SessionInfo {
            id: s.id.to_string(),
            name: s.name,
            agent_id: Some(s.agent_id.to_string()),
            channel_id: s.channel_type,
            state: format!("{:?}", s.state),
        }).collect();
        return Json(session_infos);
    }
    
    Json(Vec::new())
}

async fn get_session(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<Option<SessionInfo>> {
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        if let Ok(sessions) = orchestrator.list_sessions(None, None).await {
            if let Some(s) = sessions.into_iter().find(|s| s.id.to_string() == id) {
                return Json(Some(SessionInfo {
                    id: s.id.to_string(),
                    name: s.name,
                    agent_id: Some(s.agent_id.to_string()),
                    channel_id: s.channel_type,
                    state: format!("{:?}", s.state),
                }));
            }
        }
    }
    
    Json(None)
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
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        let agent_id = input.agent_id.unwrap_or_else(|| "default".to_string());
        let name = input.name.unwrap_or_else(|| "新会话".to_string());
        
        match orchestrator.create_session(
            name.clone(),
            agent_id.clone(),
            openclaw_core::session::SessionScope::Main,
            input.channel_id.clone(),
        ).await {
            Ok(session) => {
                return Json(SessionInfo {
                    id: session.id.to_string(),
                    name: session.name,
                    agent_id: Some(session.agent_id.to_string()),
                    channel_id: session.channel_type,
                    state: format!("{:?}", session.state),
                });
            }
            Err(e) => {
                return Json(SessionInfo {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    agent_id: Some(agent_id),
                    channel_id: input.channel_id,
                    state: format!("Error: {}", e),
                });
            }
        }
    }
    
    Json(SessionInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: input.name.unwrap_or_else(|| "新会话".to_string()),
        agent_id: input.agent_id,
        channel_id: input.channel_id,
        state: "active".to_string(),
    })
}

async fn close_session(
    State(state): State<Arc<RwLock<ApiState>>>,
    Path(id): Path<String>,
) -> Json<serde_json::Value> {
    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        match orchestrator.close_session(&id).await {
            Ok(_) => Json(serde_json::json!({ "success": true, "session_id": id })),
            Err(e) => Json(serde_json::json!({ "success": false, "error": format!("{}", e) })),
        }
    } else {
        Json(serde_json::json!({ "success": false, "error": "No orchestrator available" }))
    }
}

#[derive(Debug, Deserialize)]
pub struct AgentMessageRequest {
    pub agent_id: String,
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

    let state = state.read().await;
    
    if let Some(ref orchestrator) = *state.orchestrator.read().await {
        match orchestrator
            .process_agent_message(&input.agent_id, &input.message, &session_id)
            .await
        {
            Ok(response) => {
                Json(AgentMessageResponse {
                    message: response,
                    session_id,
                })
            }
            Err(e) => {
                Json(AgentMessageResponse {
                    message: format!("Error: {}", e),
                    session_id,
                })
            }
        }
    } else {
        Json(AgentMessageResponse {
            message: "Orchestrator not available".to_string(),
            session_id,
        })
    }
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

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice: Option<String>,
    pub model: Option<String>,
}

async fn tts_handler(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<TtsRequest>,
) -> Json<serde_json::Value> {
    let voice_service = &state.read().await.voice_service;
    
    match voice_service.speak(&input.text).await {
        Some(Ok(_audio_data)) => {
            Json(serde_json::json!({
                "success": true,
                "message": "TTS audio generated (base64 encoding not available)",
                "format": "mp3"
            }))
        }
        Some(Err(e)) => Json(serde_json::json!({
            "success": false,
            "error": format!("TTS error: {}", e)
        })),
        None => Json(serde_json::json!({
            "success": false,
            "error": "Voice service not initialized"
        })),
    }
}

#[derive(Debug, Deserialize)]
pub struct SttRequest {
    pub audio: String,
    pub language: Option<String>,
    pub model: Option<String>,
}

async fn stt_handler(
    State(state): State<Arc<RwLock<ApiState>>>,
    Json(input): Json<SttRequest>,
) -> Json<serde_json::Value> {
    let voice_service = &state.read().await.voice_service;
    
    if input.audio.is_empty() {
        return Json(serde_json::json!({
            "success": false,
            "error": "Empty audio data"
        }));
    }
    
    Json(serde_json::json!({
        "success": true,
        "text": "STT processing requires base64 crate - audio placeholder"
    }))
}
