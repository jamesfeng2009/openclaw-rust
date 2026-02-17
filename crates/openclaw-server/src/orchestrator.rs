use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_agent::aieos::{AIEOSParser, AIEOSPromptGenerator};
use openclaw_agent::task::TaskOutput;
use openclaw_agent::task::{TaskInput, TaskRequest, TaskType};
use openclaw_agent::{Agent, AgentConfig as OpenclawAgentConfig, AgentInfo, AgentType, BaseAgent};
use openclaw_canvas::CanvasManager;
use openclaw_channels::{ChannelManager, ChannelMessage, SendMessage};
use openclaw_core::{Config, Content, Message, OpenClawError, Result, Role};

pub struct ServiceOrchestrator {
    agent_service: AgentServiceState,
    channel_service: ChannelServiceState,
    canvas_service: CanvasServiceState,
    config: OrchestratorConfig,
    running: Arc<RwLock<bool>>,
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
    manager: Arc<CanvasManager>,
}

impl Default for CanvasServiceState {
    fn default() -> Self {
        Self {
            manager: Arc::new(CanvasManager::new()),
        }
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
        }
    }
}

impl ServiceOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            agent_service: AgentServiceState::default(),
            channel_service: ChannelServiceState::default(),
            canvas_service: CanvasServiceState::default(),
            config,
            running: Arc::new(RwLock::new(false)),
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
        let mut agents = self.agent_service.agents.write().await;
        agents.insert(id, agent);
    }

    pub async fn get_agent(&self, id: &str) -> Option<Arc<dyn Agent>> {
        let agents = self.agent_service.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agent_service.agents.read().await;
        agents.values().map(|a| a.info()).collect()
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

        let task = TaskRequest::new(TaskType::Conversation, TaskInput::Message { message: msg });

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

            if let Some(aieos_path) = &agent_cfg.aieos_path {
                if aieos_path.exists() {
                    match AIEOSParser::from_file(aieos_path) {
                        Ok(aieos) => {
                            let system_prompt =
                                AIEOSPromptGenerator::generate_system_prompt(&aieos);
                            openclaw_cfg = openclaw_cfg.with_system_prompt(system_prompt);
                            tracing::info!(
                                "Loaded AIEOS for agent {} from {:?}",
                                agent_cfg.id,
                                aieos_path
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to load AIEOS for agent {}: {}",
                                agent_cfg.id,
                                e
                            );
                        }
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

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert!(!config.enable_agents);
        assert!(!config.enable_channels);
        assert!(!config.enable_voice);
        assert!(!config.enable_canvas);
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
}
