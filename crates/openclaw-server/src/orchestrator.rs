use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use openclaw_core::{Result, OpenClawError, Message, Content, Role};
use openclaw_agent::{Agent, BaseAgent, AgentInfo};
use openclaw_agent::task::{TaskRequest, TaskInput, TaskType};
use openclaw_agent::task::TaskOutput;
use openclaw_channels::{Channel, ChannelManager, ChannelMessage, SendMessage};

pub struct ServiceOrchestrator {
    agent_service: AgentServiceState,
    channel_service: ChannelServiceState,
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
pub struct OrchestratorConfig {
    pub enable_agents: bool,
    pub enable_channels: bool,
    pub enable_voice: bool,
    pub default_agent: Option<String>,
    pub channel_to_agent_map: HashMap<String, String>,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enable_agents: false,
            enable_channels: false,
            enable_voice: false,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: HashMap::new(),
        }
    }
}

impl ServiceOrchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            agent_service: AgentServiceState::default(),
            channel_service: ChannelServiceState::default(),
            config,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        if self.config.enable_agents {
            self.init_default_agents().await?;
        }
        
        if self.config.enable_channels {
            self.channel_service.manager.read().await.start_all().await?;
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

    pub async fn process_message(&self, agent_id: &str, message: String, session_id: Option<String>) -> Result<String> {
        let agent = self.get_agent(agent_id).await
            .ok_or_else(|| OpenClawError::Config(format!("Agent not found: {}", agent_id)))?;

        let msg = Message::new(
            Role::User,
            vec![Content::Text { text: message }],
        );

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Message { message: msg },
        );
        
        let result = agent.process(task).await?;

        let output = match result.output {
            Some(TaskOutput::Message { message }) => {
                message.content.iter()
                    .map(|c| {
                        match c {
                            Content::Text { text } => text.clone(),
                            _ => format!("[{:?}]", c),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            Some(other) => format!("{:?}", other),
            None => result.error.unwrap_or_else(|| "No output".to_string()),
        };

        Ok(output)
    }

    pub async fn process_channel_message(&self, channel_name: &str, message: String) -> Result<ChannelMessage> {
        let agent_id = self.config.channel_to_agent_map.get(channel_name)
            .cloned()
            .or_else(|| self.config.default_agent.clone())
            .ok_or_else(|| OpenClawError::Config("No agent configured".to_string()))?;

        let response = self.process_message(&agent_id, message, Some(channel_name.to_string())).await?;

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

    pub async fn send_to_channel(&self, channel_name: &str, message: SendMessage) -> Result<ChannelMessage> {
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
            health.insert("agents".to_string(), !self.agent_service.agents.read().await.is_empty());
        }
        
        if self.config.enable_channels {
            let manager = self.channel_service.manager.read().await;
            let channel_health = manager.health_check_all().await;
            for (name, status) in channel_health {
                health.insert(format!("channel:{}", name), status);
            }
        }
        
        health
    }

    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }
}

impl Default for ServiceOrchestrator {
    fn default() -> Self {
        Self::new(OrchestratorConfig::default())
    }
}
