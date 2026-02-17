use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_agent::task::{TaskInput, TaskRequest, TaskType};
use openclaw_agent::{Agent, AgentInfo, BaseAgent};
use openclaw_core::{Content, Message, OpenClawError, Result, Role};

#[derive(Clone)]
pub struct AgentService {
    agents: Arc<RwLock<HashMap<String, Arc<dyn Agent>>>>,
}

impl AgentService {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_agent(&self, id: String, agent: Arc<dyn Agent>) {
        let mut agents = self.agents.write().await;
        agents.insert(id, agent);
    }

    pub async fn get_agent(&self, id: &str) -> Option<Arc<dyn Agent>> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().map(|a| a.info()).collect()
    }

    pub async fn remove_agent(&self, id: &str) {
        let mut agents = self.agents.write().await;
        agents.remove(id);
    }

    pub async fn process_message(
        &self,
        agent_id: &str,
        message: String,
        _session_id: Option<String>,
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

    pub async fn create_default_agents(&self) -> Result<()> {
        let orchestrator = Arc::new(BaseAgent::orchestrator()) as Arc<dyn Agent>;
        self.register_agent("orchestrator".to_string(), orchestrator)
            .await;

        let researcher = Arc::new(BaseAgent::researcher()) as Arc<dyn Agent>;
        self.register_agent("researcher".to_string(), researcher)
            .await;

        let coder = Arc::new(BaseAgent::coder()) as Arc<dyn Agent>;
        self.register_agent("coder".to_string(), coder).await;

        let writer = Arc::new(BaseAgent::writer()) as Arc<dyn Agent>;
        self.register_agent("writer".to_string(), writer).await;

        Ok(())
    }
}

impl Default for AgentService {
    fn default() -> Self {
        Self::new()
    }
}

pub use openclaw_agent::task::TaskOutput;
