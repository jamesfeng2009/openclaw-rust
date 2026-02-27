//! ACP Service - Agent Collaboration Protocol Service
//!
//! Provides multi-agent collaboration, message routing, and context sharing.

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use openclaw_acp::{
    AgentRegistry, AgentInfo,
    ContextManager, Router,
    AcpEnvelope, EnvelopeType,
    AcpRequest, AcpResponse,
    CapabilityRegistry,
};
use openclaw_core::OpenClawError;

pub type AcpResult<T> = std::result::Result<T, OpenClawError>;

#[derive(Clone)]
pub struct AcpService {
    agent_registry: Arc<AgentRegistry>,
    context_manager: Arc<ContextManager>,
    router: Arc<Router>,
    capability_registry: Arc<CapabilityRegistry>,
    local_agents: Arc<RwLock<HashMap<String, LocalAgentHandle>>>,
    http_clients: Arc<RwLock<HashMap<String, reqwest::Client>>>,
}

pub struct LocalAgentHandle {
    pub id: String,
    pub name: String,
    pub capabilities: Vec<String>,
    handler: Arc<dyn Fn(AcpRequest) -> AcpResponse + Send + Sync>,
}

impl AcpService {
    pub fn new() -> Self {
        let agent_registry = Arc::new(AgentRegistry::new());
        let context_manager = Arc::new(ContextManager::new());
        let router = Arc::new(Router::new(
            Arc::clone(&agent_registry),
            Arc::clone(&context_manager),
        ));
        let capability_registry = Arc::new(CapabilityRegistry::new());

        Self {
            agent_registry,
            context_manager,
            router,
            capability_registry,
            local_agents: Arc::new(RwLock::new(HashMap::new())),
            http_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_default_agent(mut self, agent_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        self.router = Arc::new(Router::new(
            Arc::clone(&self.agent_registry),
            Arc::clone(&self.context_manager),
        ).with_default_agent(agent_id));
        self
    }

    pub async fn register_agent(&self, info: AgentInfo) {
        self.agent_registry.register(info).await;
    }

    pub async fn register_local_agent<F>(&self, id: impl Into<String>, name: impl Into<String>, capabilities: Vec<String>, handler: F) 
    where
        F: Fn(AcpRequest) -> AcpResponse + Send + Sync + 'static,
    {
        let id = id.into();
        let name = name.into();
        
        let agent_info = AgentInfo::new(id.clone(), name.clone())
            .with_capabilities(capabilities.clone())
            .with_endpoint("local");
        
        let handle = LocalAgentHandle {
            id: id.clone(),
            name,
            capabilities,
            handler: Arc::new(handler),
        };
        
        let mut agents = self.local_agents.write().await;
        agents.insert(id.clone(), handle);
        
        self.agent_registry.register(agent_info).await;
    }

    pub async fn unregister_agent(&self, agent_id: &str) -> bool {
        let mut agents = self.local_agents.write().await;
        agents.remove(agent_id);
        self.agent_registry.unregister(agent_id).await
    }

    pub async fn get_agent(&self, agent_id: &str) -> Option<AgentInfo> {
        self.agent_registry.get(agent_id).await
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        self.agent_registry.list().await
    }

    pub async fn list_online_agents(&self) -> Vec<AgentInfo> {
        self.agent_registry.list_online().await
    }

    pub async fn find_agents_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
        self.agent_registry.find_by_capability(capability).await
    }

    pub async fn route_message(&self, content: &str, conversation_id: &str) -> RouterResult {
        let result = self.router.route(content, conversation_id).await;
        
        RouterResult {
            target_agent: result.target_agent,
            cleaned_content: result.cleaned_content,
            matched_rule: result.matched_rule,
            context_id: result.context_id,
        }
    }

    pub async fn handle_message(&self, content: &str, conversation_id: &str, user_id: Option<&str>) -> AcpResult<String> {
        let route_result = self.route_message(content, conversation_id).await;
        
        if route_result.target_agent.is_empty() {
            return Err(OpenClawError::Unknown("No agent found to handle the message".to_string()));
        }

        let context_id = route_result.context_id.unwrap_or_else(|| {
            format!("{}_{}", conversation_id, uuid::Uuid::new_v4())
        });

        let request = AcpRequest {
            action: "handle_message".to_string(),
            params: serde_json::json!({
                "content": route_result.cleaned_content,
                "conversation_id": conversation_id,
                "user_id": user_id,
            }),
            callback_id: None,
            timeout: Some(60),
        };

        let response = self.call_agent(&route_result.target_agent, request, Some(&context_id)).await?;
        
        if response.success {
            Ok(response.data.map(|d| d.to_string()).unwrap_or_default())
        } else {
            Err(OpenClawError::Execution(response.error.unwrap_or_else(|| "Unknown error".to_string())))
        }
    }

    pub async fn call_agent(&self, agent_id: &str, request: AcpRequest, context_id: Option<&str>) -> AcpResult<AcpResponse> {
        if let Some(local) = self.local_agents.read().await.get(agent_id) {
            let response = (local.handler)(request);
            return Ok(response);
        }

        if let Some(agent) = self.agent_registry.get(agent_id).await {
            if let Some(endpoint) = agent.endpoint {
                if endpoint != "local" {
                    return self.call_remote_agent(&endpoint, request, context_id).await;
                }
            }
        }

        Err(OpenClawError::Unknown(format!("Agent {} not found", agent_id)))
    }

    async fn call_remote_agent(&self, endpoint: &str, request: AcpRequest, context_id: Option<&str>) -> AcpResult<AcpResponse> {
        let client = reqwest::Client::new();
        
        let envelope = AcpEnvelope {
            version: "1.0.0".to_string(),
            msg_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now(),
            envelope_type: EnvelopeType::Request,
            from: "openclaw-server".to_string(),
            to: None,
            conversation_id: context_id.map(String::from),
            payload: serde_json::to_value(&request).map_err(|e| OpenClawError::Serialization(e))?,
            extensions: Default::default(),
        };

        let response = client
            .post(endpoint)
            .json(&envelope)
            .send()
            .await
            .map_err(|e| OpenClawError::Network(e.to_string()))?;

        let envelope: AcpEnvelope = response.json().await
            .map_err(|e| OpenClawError::Serialization(serde_json::from_str::<serde_json::Value>(&e.to_string()).unwrap_err()))?;

        let acp_response: AcpResponse = serde_json::from_value(envelope.payload)
            .map_err(|e| OpenClawError::Serialization(e))?;

        Ok(acp_response)
    }

    pub async fn get_context(&self, context_id: &str) -> Option<openclaw_acp::SharedContext> {
        self.context_manager.get(context_id).await
    }

    pub async fn create_context(&self, conversation_id: impl Into<String>) -> openclaw_acp::SharedContext {
        self.context_manager.create(conversation_id.into()).await
    }

    pub async fn update_context(&self, context_id: &str, key: &str, value: serde_json::Value) {
        self.context_manager.update(context_id, key.to_string(), value, "system".to_string());
    }

    pub fn agent_registry(&self) -> Arc<AgentRegistry> {
        Arc::clone(&self.agent_registry)
    }

    pub fn context_manager(&self) -> Arc<ContextManager> {
        Arc::clone(&self.context_manager)
    }

    pub fn router(&self) -> Arc<Router> {
        Arc::clone(&self.router)
    }

    pub fn capability_registry(&self) -> Arc<CapabilityRegistry> {
        Arc::clone(&self.capability_registry)
    }
}

impl Default for AcpService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RouterResult {
    pub target_agent: String,
    pub cleaned_content: String,
    pub matched_rule: Option<String>,
    pub context_id: Option<String>,
}
