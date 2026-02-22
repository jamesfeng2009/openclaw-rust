use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[cfg(feature = "per_session_memory")]
use std::collections::VecDeque;

use openclaw_ai::AIProvider;
use openclaw_agent::aieos::{AIEOSParser, AIEOSPromptGenerator};
use openclaw_agent::sessions::{MemorySessionStorage, SessionManager};
use openclaw_agent::task::TaskOutput;
use openclaw_agent::task::{TaskInput, TaskRequest, TaskType};
use openclaw_agent::{Agent, AgentConfig as OpenclawAgentConfig, AgentInfo, AgentType, BaseAgent};
use openclaw_canvas::CanvasManager;
use openclaw_channels::{ChannelManager, ChannelMessage, SendMessage};
use openclaw_core::{Config, Content, Message, OpenClawError, Result, Role};
#[cfg(feature = "per_session_memory")]
use openclaw_memory::MemoryConfig;
use openclaw_memory::MemoryManager;
use openclaw_security::SecurityPipeline;

use crate::agentic_rag::AgenticRAGEngine;

#[cfg(feature = "per_session_memory")]
const DEFAULT_MAX_SESSION_MEMORIES: usize = 100;

#[cfg(feature = "per_session_memory")]
struct SessionMemoryCache {
    map: HashMap<Uuid, Arc<MemoryManager>>,
    order: VecDeque<Uuid>,
    max_size: usize,
}

#[cfg(feature = "per_session_memory")]
impl SessionMemoryCache {
    fn new(max_size: usize) -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            max_size,
        }
    }

    fn get(&self, session_id: &Uuid) -> Option<Arc<MemoryManager>> {
        self.map.get(session_id).cloned()
    }

    fn insert(&mut self, session_id: Uuid, memory: Arc<MemoryManager>) {
        if self.map.contains_key(&session_id) {
            return;
        }

        if self.map.len() >= self.max_size {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            }
        }

        self.map.insert(session_id, memory);
        self.order.push_back(session_id);
    }

    fn remove(&mut self, session_id: &Uuid) -> Option<Arc<MemoryManager>> {
        if let Some(memory) = self.map.remove(session_id) {
            self.order.retain(|id| id != session_id);
            Some(memory)
        } else {
            None
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(feature = "per_session_memory")]
impl Default for SessionMemoryCache {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_SESSION_MEMORIES)
    }
}

pub struct ServiceOrchestrator {
    agent_service: AgentServiceState,
    channel_service: ChannelServiceState,
    canvas_service: CanvasServiceState,
    session_service: SessionServiceState,
    config: OrchestratorConfig,
    running: Arc<RwLock<bool>>,
    ai_provider: Arc<RwLock<Option<Arc<dyn AIProvider>>>>,
    memory_manager: Arc<RwLock<Option<Arc<MemoryManager>>>>,
    security_pipeline: Arc<RwLock<Option<Arc<SecurityPipeline>>>>,
    tool_executor: Arc<RwLock<Option<Arc<openclaw_tools::SkillRegistry>>>>,
    channel_factory: Arc<openclaw_channels::ChannelFactoryRegistry>,
    agentic_rag_engine: Arc<RwLock<Option<Arc<AgenticRAGEngine>>>>,
    #[cfg(feature = "per_session_memory")]
    session_memory_cache: Arc<tokio::sync::RwLock<SessionMemoryCache>>,
}

#[derive(Clone, Default)]
pub struct AgentServiceState {
    agents: Arc<RwLock<HashMap<String, Arc<dyn Agent>>>>,
}

#[derive(Clone, Default)]
pub struct ChannelServiceState {
    manager: Arc<RwLock<ChannelManager>>,
}

#[derive(Clone)]
pub struct CanvasServiceState {
    pub manager: Arc<CanvasManager>,
}

impl Default for CanvasServiceState {
    fn default() -> Self {
        Self {
            manager: Arc::new(CanvasManager::new()),
        }
    }
}

#[derive(Clone)]
pub struct SessionServiceState {
    manager: Arc<SessionManager>,
}

impl SessionServiceState {
    pub fn new(manager: SessionManager) -> Self {
        Self {
            manager: Arc::new(manager),
        }
    }
    
    pub fn with_default() -> Self {
        let storage = Arc::new(MemorySessionStorage::new());
        let manager = SessionManager::new(storage);
        Self::new(manager)
    }
}

#[derive(Clone)]
pub struct OrchestratorConfig {
    pub enable_agents: bool,
    pub enable_channels: bool,
    pub enable_voice: bool,
    pub enable_canvas: bool,
    pub default_agent: Option<String>,
    pub channel_to_agent_map: HashMap<String, String>,
    pub agent_to_canvas_map: HashMap<String, String>,
    #[cfg(feature = "per_session_memory")]
    pub enable_per_session_memory: bool,
    #[cfg(feature = "per_session_memory")]
    pub memory_config: Option<MemoryConfig>,
    #[cfg(feature = "per_session_memory")]
    pub max_session_memories: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enable_agents: false,
            enable_channels: false,
            enable_voice: false,
            enable_canvas: false,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: HashMap::new(),
            agent_to_canvas_map: HashMap::new(),
            #[cfg(feature = "per_session_memory")]
            enable_per_session_memory: false,
            #[cfg(feature = "per_session_memory")]
            memory_config: None,
            #[cfg(feature = "per_session_memory")]
            max_session_memories: DEFAULT_MAX_SESSION_MEMORIES,
        }
    }
}

impl ServiceOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let storage = Arc::new(MemorySessionStorage::new());
        let session_manager = SessionManager::new(storage);
        
        Self {
            agent_service: AgentServiceState::default(),
            channel_service: ChannelServiceState::default(),
            canvas_service: CanvasServiceState::default(),
            session_service: SessionServiceState::new(session_manager),
            config: config.clone(),
            running: Arc::new(RwLock::new(false)),
            ai_provider: Arc::new(RwLock::new(None)),
            memory_manager: Arc::new(RwLock::new(None)),
            security_pipeline: Arc::new(RwLock::new(None)),
            tool_executor: Arc::new(RwLock::new(None)),
            channel_factory: Arc::new(openclaw_channels::ChannelFactoryRegistry::new()),
            agentic_rag_engine: Arc::new(RwLock::new(None)),
            #[cfg(feature = "per_session_memory")]
            session_memory_cache: Arc::new(tokio::sync::RwLock::new(
                SessionMemoryCache::new(config.max_session_memories)
            )),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if self.config.enable_agents {
            self.init_default_agents().await?;
        }

        if self.config.enable_channels {
            self.channel_service
                .manager
                .read()
                .await
                .start_all()
                .await?;
        }

        *self.running.write().await = true;
        tracing::info!("ServiceOrchestrator started");
        Ok(())
    }

    pub async fn stop(&self) -> Result<()> {
        *self.running.write().await = false;

        if self.config.enable_channels {
            self.channel_service.manager.read().await.stop_all().await?;
        }

        tracing::info!("ServiceOrchestrator stopped");
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }

    async fn init_default_agents(&self) -> Result<()> {
        let mut agents = self.agent_service.agents.write().await;

        let orchestrator = Arc::new(BaseAgent::orchestrator()) as Arc<dyn Agent>;
        agents.insert("orchestrator".to_string(), orchestrator);

        let researcher = Arc::new(BaseAgent::researcher()) as Arc<dyn Agent>;
        agents.insert("researcher".to_string(), researcher);

        let coder = Arc::new(BaseAgent::coder()) as Arc<dyn Agent>;
        agents.insert("coder".to_string(), coder);

        let writer = Arc::new(BaseAgent::writer()) as Arc<dyn Agent>;
        agents.insert("writer".to_string(), writer);

        tracing::info!("Default agents initialized");
        Ok(())
    }

    pub async fn register_agent(&self, id: String, agent: Arc<dyn Agent>) {
        let (should_inject, agent_to_inject) = {
            let mut agents = self.agent_service.agents.write().await;
            
            let should_inject = {
                let provider = self.ai_provider.read().await;
                let memory = self.memory_manager.read().await;
                let pipeline = self.security_pipeline.read().await;
                provider.is_some() && memory.is_some() && pipeline.is_some()
            };
            
            let agent_to_inject = if should_inject {
                Some(agent.clone())
            } else {
                None
            };
            
            agents.insert(id.clone(), agent);
            
            (should_inject, agent_to_inject)
        };
        
        if should_inject
            && let Some(agent_to_inject) = agent_to_inject {
                let ai_provider = self.ai_provider.clone();
                let memory_manager = self.memory_manager.clone();
                let security_pipeline = self.security_pipeline.clone();
                tokio::spawn(async move {
                    let ai = {
                        let p = ai_provider.read().await;
                        p.clone()
                    };
                    let mem = {
                        let m = memory_manager.read().await;
                        m.clone()
                    };
                    let sec = {
                        let s = security_pipeline.read().await;
                        s.clone()
                    };
                    if let (Some(ai), Some(sec)) = (ai, sec)
                        && let Some(m) = mem {
                            agent_to_inject.inject_dependencies(ai, Some(m), sec, None).await;
                        }
                });
            }
    }

    pub async fn inject_dependencies(
        &self,
        ai_provider: Arc<dyn AIProvider>,
        memory_manager: Option<Arc<MemoryManager>>,
        security_pipeline: Arc<SecurityPipeline>,
        tool_registry: Option<Arc<openclaw_tools::ToolRegistry>>,
    ) {
        {
            let mut provider = self.ai_provider.write().await;
            *provider = Some(ai_provider.clone());
        }
        {
            let mut memory = self.memory_manager.write().await;
            *memory = memory_manager.clone();
        }
        {
            let mut pipeline = self.security_pipeline.write().await;
            *pipeline = Some(security_pipeline.clone());
        }
        
        let agents: Vec<Arc<dyn Agent>> = {
            let agents = self.agent_service.agents.read().await;
            agents.values().cloned().collect()
        };

        let mem_lock = memory_manager.clone();
        for agent in agents {
            agent.inject_dependencies(
                ai_provider.clone(),
                mem_lock.clone(),
                security_pipeline.clone(),
                tool_registry.clone(),
            ).await;
        }

        tracing::info!("Dependencies injected to all agents");
    }

    pub async fn get_agent(&self, id: &str) -> Option<Arc<dyn Agent>> {
        let agents = self.agent_service.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agent_service.agents.read().await;
        agents.values().map(|a| a.info()).collect()
    }

    pub async fn get_ai_provider(&self) -> Option<Arc<dyn AIProvider>> {
        let provider = self.ai_provider.read().await;
        provider.clone()
    }

    pub async fn get_agentic_rag_engine(&self) -> Option<Arc<AgenticRAGEngine>> {
        let engine = self.agentic_rag_engine.read().await;
        engine.clone()
    }

    pub async fn set_agentic_rag_engine(&self, engine: Arc<AgenticRAGEngine>) {
        let mut e = self.agentic_rag_engine.write().await;
        *e = Some(engine);
    }

    pub async fn list_sessions(
        &self,
        agent_id: Option<&str>,
        state: Option<openclaw_agent::sessions::SessionState>,
    ) -> openclaw_agent::Result<Vec<openclaw_agent::sessions::Session>> {
        let agent_id: Option<openclaw_agent::types::AgentId> = agent_id.map(|s| s.to_string());
        self.session_service.manager.list_sessions(agent_id, state).await
    }

    pub async fn create_session(
        &self,
        name: String,
        agent_id: String,
        scope: openclaw_core::session::SessionScope,
        channel_type: Option<String>,
    ) -> openclaw_agent::Result<openclaw_agent::sessions::Session> {
        let agent_id_owned = agent_id.clone();
        self.session_service.manager
            .create_session(name, agent_id_owned, scope, channel_type, None)
            .await
    }

    pub async fn close_session(&self, session_id: &str) -> openclaw_agent::Result<()> {
        let uuid = uuid::Uuid::parse_str(session_id)
            .map_err(|_| openclaw_agent::OpenClawError::Config(format!("Invalid session ID: {}", session_id)))?;
        
        #[cfg(feature = "per_session_memory")]
        {
            self.cleanup_session_memory(&uuid).await;
        }
        
        self.session_service.manager.close_session(&uuid).await
    }

    #[cfg(feature = "per_session_memory")]
    pub async fn get_session_memory(&self, session_id: &Uuid) -> Option<Arc<MemoryManager>> {
        if !self.config.enable_per_session_memory {
            return None;
        }
        
        let mut cache = self.session_memory_cache.write().await;
        
        if let Some(memory) = cache.get(session_id) {
            return Some(memory);
        }
        
        let config = self.config.memory_config.clone()
            .unwrap_or_else(|| MemoryConfig::default());
        
        let session_memory = Arc::new(MemoryManager::new(config));
        
        cache.insert(*session_id, session_memory.clone());
        
        tracing::debug!("Created new session memory for session: {}", session_id);
        Some(session_memory)
    }

    #[cfg(feature = "per_session_memory")]
    async fn cleanup_session_memory(&self, session_id: &Uuid) {
        let mut cache = self.session_memory_cache.write().await;
        
        if let Some(memory) = cache.remove(session_id) {
            tracing::debug!("Cleaning up session memory for session: {}", session_id);
        }
    }

    pub async fn get_session(&self, session_id: &str) -> openclaw_agent::Result<Option<openclaw_agent::sessions::Session>> {
        let uuid = uuid::Uuid::parse_str(session_id)
            .map_err(|_| openclaw_agent::OpenClawError::Config(format!("Invalid session ID: {}", session_id)))?;
        self.session_service.manager.get_session(&uuid).await
    }

    pub async fn process_agent_message(
        &self,
        agent_id: &str,
        message: &str,
        session_id: &str,
    ) -> Result<String> {
        self.process_message(agent_id, message.to_string(), Some(session_id.to_string())).await
    }

    pub async fn process_message(
        &self,
        agent_id: &str,
        message: String,
        session_id: Option<String>,
    ) -> Result<String> {
        let agent = self
            .get_agent(agent_id)
            .await
            .ok_or_else(|| OpenClawError::Config(format!("Agent not found: {}", agent_id)))?;

        let msg = Message::new(Role::User, vec![Content::Text { text: message }]);

        let task = TaskRequest::new(TaskType::Conversation, TaskInput::Message { message: msg })
            .with_session_id(session_id.clone().unwrap_or_else(|| format!("agent-{}", agent_id)));

        #[cfg(feature = "per_session_memory")]
        {
            if self.config.enable_per_session_memory {
                if let Some(ref sid) = session_id {
                    if let Ok(uuid) = uuid::Uuid::parse_str(sid) {
                        if let Some(session_memory) = self.get_session_memory(&uuid).await {
                            agent.inject_dependencies(
                                self.ai_provider.read().await.clone().unwrap(),
                                Some(session_memory),
                                self.security_pipeline.read().await.clone().unwrap(),
                                None,
                            ).await;
                        }
                    }
                }
            }
        }

        let result = agent.process(task).await?;

        let output = match result.output {
            Some(TaskOutput::Message { message }) => message
                .content
                .iter()
                .map(|c| match c {
                    Content::Text { text } => text.clone(),
                    _ => format!("[{:?}]", c),
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Some(other) => format!("{:?}", other),
            None => result.error.unwrap_or_else(|| "No output".to_string()),
        };

        Ok(output)
    }

    pub async fn process_channel_message(
        &self,
        channel_name: &str,
        message: String,
    ) -> Result<ChannelMessage> {
        let agent_id = self
            .config
            .channel_to_agent_map
            .get(channel_name)
            .cloned()
            .or_else(|| self.config.default_agent.clone())
            .ok_or_else(|| OpenClawError::Config("No agent configured".to_string()))?;

        let response = self
            .process_message(&agent_id, message, Some(channel_name.to_string()))
            .await?;

        let channel_msg = ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel_type: openclaw_channels::ChannelType::WebChat,
            chat_id: channel_name.to_string(),
            user_id: "agent".to_string(),
            content: response,
            timestamp: chrono::Utc::now(),
            metadata: None,
        };

        Ok(channel_msg)
    }

    pub async fn send_to_channel(
        &self,
        channel_name: &str,
        message: SendMessage,
    ) -> Result<ChannelMessage> {
        let manager = self.channel_service.manager.read().await;
        manager.send_to_channel(channel_name, message).await
    }

    pub async fn broadcast(&self, message: SendMessage) -> Result<Vec<ChannelMessage>> {
        let manager = self.channel_service.manager.read().await;
        manager.broadcast(message).await
    }

    pub async fn list_channels(&self) -> Vec<String> {
        let manager = self.channel_service.manager.read().await;
        manager.list_channels().await
    }

    pub async fn create_channel(&self, name: String, channel_type: String) -> openclaw_core::Result<()> {
        let config = serde_json::json!({
            "enabled": true
        });
        
        let channel = self.channel_factory.create(&channel_type, config).await?;
        
        let manager = self.channel_service.manager.read().await;
        manager.register_channel(name, channel).await;
        Ok(())
    }

    pub async fn delete_channel(&self, name: &str) -> openclaw_core::Result<()> {
        let manager = self.channel_service.manager.read().await;
        manager.unregister_channel(name).await;
        Ok(())
    }

    pub async fn health_check(&self) -> HashMap<String, bool> {
        let mut health = HashMap::new();

        if self.config.enable_agents {
            health.insert(
                "agents".to_string(),
                !self.agent_service.agents.read().await.is_empty(),
            );
        }

        if self.config.enable_channels {
            let manager = self.channel_service.manager.read().await;
            let channel_health = manager.health_check_all().await;
            for (name, status) in channel_health {
                health.insert(format!("channel:{}", name), status);
            }
        }

        if self.config.enable_canvas {
            health.insert("canvas".to_string(), true);
        }

        health
    }

    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    pub fn canvas_manager(&self) -> Arc<CanvasManager> {
        self.canvas_service.manager.clone()
    }

    pub fn canvas_service(&self) -> &CanvasServiceState {
        &self.canvas_service
    }

    pub async fn create_canvas(&self, name: String, width: f64, height: f64) -> Result<String> {
        let canvas_id = self
            .canvas_service
            .manager
            .create_canvas(name, width, height)
            .await;
        Ok(canvas_id)
    }

    pub async fn agent_generate_to_canvas(
        &self,
        agent_id: &str,
        prompt: &str,
        canvas_name: Option<String>,
    ) -> Result<String> {
        if !self.config.enable_canvas {
            return Err(OpenClawError::Config(
                "Canvas service not enabled".to_string(),
            ));
        }

        let canvas_name = canvas_name.unwrap_or_else(|| format!("canvas_{}", agent_id));

        let canvas_id = self
            .create_canvas(canvas_name.clone(), 1920.0, 1080.0)
            .await?;

        let response = self
            .process_message(agent_id, prompt.to_string(), Some(canvas_id.clone()))
            .await?;

        tracing::info!(
            "Agent {} generated content for canvas {}: {}",
            agent_id,
            canvas_id,
            response
        );

        Ok(canvas_id)
    }

    pub async fn init_agents_from_config(&self, config: &Config) -> Result<()> {
        let agents_config = &config.agents;

        for agent_cfg in &agents_config.list {
            let mut openclaw_cfg = OpenclawAgentConfig::new(
                agent_cfg.id.clone(),
                agent_cfg.id.clone(),
                AgentType::Custom(agent_cfg.id.clone()),
            );

            if let Some(aieos_path) = &agent_cfg.aieos_path
                && aieos_path.exists()
            {
                match AIEOSParser::from_file(aieos_path) {
                    Ok(aieos) => {
                        let system_prompt = AIEOSPromptGenerator::generate_system_prompt(&aieos);
                        openclaw_cfg = openclaw_cfg.with_system_prompt(system_prompt);
                        tracing::info!(
                            "Loaded AIEOS for agent {} from {:?}",
                            agent_cfg.id,
                            aieos_path
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load AIEOS for agent {}: {}", agent_cfg.id, e);
                    }
                }
            }

            let agent = Arc::new(BaseAgent::new(openclaw_cfg)) as Arc<dyn Agent>;
            self.register_agent(agent_cfg.id.clone(), agent).await;
        }

        tracing::info!(
            "Initialized {} agents from config",
            agents_config.list.len()
        );
        Ok(())
    }
}

impl Default for ServiceOrchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_agent::Agent;
    use openclaw_core::session::SessionScope;

    #[cfg(feature = "per_session_memory")]
    fn make_per_session_config(max_memories: usize) -> OrchestratorConfig {
        OrchestratorConfig {
            enable_per_session_memory: true,
            memory_config: Some(MemoryConfig::default()),
            max_session_memories: max_memories,
            enable_agents: false,
            enable_channels: false,
            enable_voice: false,
            enable_canvas: false,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: std::collections::HashMap::new(),
            agent_to_canvas_map: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert!(!config.enable_agents);
        assert!(!config.enable_channels);
        assert!(!config.enable_voice);
        assert!(!config.enable_canvas);
        
        #[cfg(feature = "per_session_memory")]
        {
            assert!(!config.enable_per_session_memory);
            assert!(config.memory_config.is_none());
            assert_eq!(config.max_session_memories, 100);
        }
    }

    #[test]
    fn test_orchestrator_config_with_per_session() {
        #[cfg(feature = "per_session_memory")]
        {
            let config = OrchestratorConfig {
                enable_per_session_memory: true,
                memory_config: Some(MemoryConfig::default()),
                max_session_memories: 50,
                ..Default::default()
            };
            
            assert!(config.enable_per_session_memory);
            assert!(config.memory_config.is_some());
            assert_eq!(config.max_session_memories, 50);
        }
    }

    #[tokio::test]
    async fn test_orchestrator_inject_dependencies() {
        use openclaw_agent::Agent;
        use openclaw_memory::MemoryManager;
        use openclaw_security::SecurityPipeline;
        use openclaw_testing::ai::MockAiProvider;
        use openclaw_tools::ToolRegistry;
        
        let orchestrator = ServiceOrchestrator::new(OrchestratorConfig::default());
        
        let ai_provider: Arc<dyn AIProvider> = Arc::new(MockAiProvider::new());
        let memory_manager: Option<Arc<MemoryManager>> = None;
        let security_pipeline = Arc::new(SecurityPipeline::default());
        let tool_registry: Option<Arc<ToolRegistry>> = None;
        
        orchestrator.inject_dependencies(
            ai_provider,
            memory_manager,
            security_pipeline,
            tool_registry,
        ).await;
        
        let provider = orchestrator.get_ai_provider().await;
        assert!(provider.is_some());
    }

    #[test]
    fn test_agent_service_state_default() {
        let state = AgentServiceState::default();
        let agents = state.agents.blocking_read();
        assert!(agents.is_empty());
    }

    #[test]
    fn test_channel_service_state_default() {
        let state = ChannelServiceState::default();
        let _ = state.manager;
    }

    #[test]
    fn test_canvas_service_state_default() {
        let state = CanvasServiceState::default();
        let _ = state.manager;
    }

    #[tokio::test]
    async fn test_service_orchestrator_new() {
        let orchestrator = ServiceOrchestrator::new(OrchestratorConfig::default());
        let running = orchestrator.running.read().await;
        assert!(!*running);
    }

    #[cfg(feature = "per_session_memory")]
    mod per_session_memory_tests {
        use super::*;
        use uuid::Uuid;

        #[test]
        fn test_session_memory_cache_new() {
            let cache = SessionMemoryCache::new(10);
            assert_eq!(cache.len(), 0);
        }

        #[test]
        fn test_session_memory_cache_insert_and_get() {
            let mut cache = SessionMemoryCache::new(10);
            let session_id = Uuid::new_v4();
            let memory = Arc::new(MemoryManager::default());
            
            cache.insert(session_id, memory.clone());
            assert_eq!(cache.len(), 1);
            
            let retrieved = cache.get(&session_id);
            assert!(retrieved.is_some());
            assert!(Arc::ptr_eq(&retrieved.unwrap(), &memory));
        }

        #[test]
        fn test_session_memory_cache_remove() {
            let mut cache = SessionMemoryCache::new(10);
            let session_id = Uuid::new_v4();
            let memory = Arc::new(MemoryManager::default());
            
            cache.insert(session_id, memory);
            assert_eq!(cache.len(), 1);
            
            let removed = cache.remove(&session_id);
            assert!(removed.is_some());
            assert_eq!(cache.len(), 0);
            
            let retrieved = cache.get(&session_id);
            assert!(retrieved.is_none());
        }

        #[test]
        fn test_session_memory_cache_lru_eviction() {
            let mut cache = SessionMemoryCache::new(2);
            
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();
            let id3 = Uuid::new_v4();
            
            cache.insert(id1, Arc::new(MemoryManager::default()));
            cache.insert(id2, Arc::new(MemoryManager::default()));
            assert_eq!(cache.len(), 2);
            
            cache.insert(id3, Arc::new(MemoryManager::default()));
            assert_eq!(cache.len(), 2);
            
            assert!(cache.get(&id1).is_none());
            assert!(cache.get(&id2).is_some());
            assert!(cache.get(&id3).is_some());
        }

        #[test]
        fn test_session_memory_cache_no_duplicate_insert() {
            let mut cache = SessionMemoryCache::new(10);
            let session_id = Uuid::new_v4();
            let memory1 = Arc::new(MemoryManager::default());
            let memory2 = Arc::new(MemoryManager::default());
            
            cache.insert(session_id, memory1);
            cache.insert(session_id, memory2);
            
            assert_eq!(cache.len(), 1);
        }

        #[tokio::test]
        async fn test_orchestrator_get_session_memory() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            let memory = orchestrator.get_session_memory(&session_id).await;
            
            assert!(memory.is_some());
        }

        #[tokio::test]
        async fn test_orchestrator_get_session_memory_cached() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            
            let memory1 = orchestrator.get_session_memory(&session_id).await;
            let memory2 = orchestrator.get_session_memory(&session_id).await;
            
            assert!(memory1.is_some());
            assert!(memory2.is_some());
        }

        #[tokio::test]
        async fn test_orchestrator_cleanup_session_memory() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            
            let _ = orchestrator.get_session_memory(&session_id).await;
            
            orchestrator.cleanup_session_memory(&session_id).await;
            
            let cache = orchestrator.session_memory_cache.read().await;
            assert!(cache.get(&session_id).is_none());
        }

        #[tokio::test]
        async fn test_orchestrator_close_session_cleans_memory() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            
            let _ = orchestrator.get_session_memory(&session_id).await;
            
            orchestrator.cleanup_session_memory(&session_id).await;
            
            let cache = orchestrator.session_memory_cache.read().await;
            assert!(cache.get(&session_id).is_none());
        }

        #[tokio::test]
        async fn test_per_session_memory_isolation() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id1 = Uuid::new_v4();
            let session_id2 = Uuid::new_v4();
            
            let memory1 = orchestrator.get_session_memory(&session_id1).await;
            let memory2 = orchestrator.get_session_memory(&session_id2).await;
            
            assert!(memory1.is_some());
            assert!(memory2.is_some());
            
            let mem1_ptr = Arc::as_ptr(&memory1.unwrap());
            let mem2_ptr = Arc::as_ptr(&memory2.unwrap());
            assert_ne!(mem1_ptr, mem2_ptr, "Each session should have its own MemoryManager");
        }

        #[tokio::test]
        async fn test_multiple_sessions_with_lru_eviction() {
            let config = make_per_session_config(2);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id1 = Uuid::new_v4();
            
            let _ = orchestrator.get_session_memory(&session_id1).await;
            
            let cache = orchestrator.session_memory_cache.read().await;
            assert_eq!(cache.len(), 1);
        }

        #[tokio::test]
        async fn test_session_memory_persists_across_get_calls() {
            let config = make_per_session_config(10);
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            
            let memory1 = orchestrator.get_session_memory(&session_id).await;
            assert!(memory1.is_some());
            
            let memory2 = orchestrator.get_session_memory(&session_id).await;
            assert!(memory2.is_some());
            
            let mem1_ptr = Arc::as_ptr(&memory1.unwrap());
            let mem2_ptr = Arc::as_ptr(&memory2.unwrap());
            assert_eq!(mem1_ptr, mem2_ptr, "Same session should return same MemoryManager");
        }

        #[tokio::test]
        async fn test_disabled_per_session_memory_uses_global() {
            let config = make_per_session_config(10);
            let mut config = config;
            config.enable_per_session_memory = false;
            let orchestrator = ServiceOrchestrator::new(config);
            
            let session_id = Uuid::new_v4();
            
            let memory = orchestrator.get_session_memory(&session_id).await;
            assert!(memory.is_none(), "No session memory should be created when disabled");
        }
    }
}
