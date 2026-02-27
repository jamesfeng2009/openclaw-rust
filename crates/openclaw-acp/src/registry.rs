//! ACP Agent Registry
//!
//! Provides agent registration and discovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub endpoint: Option<String>,
    pub capabilities: Vec<String>,
    pub status: AgentStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Online,
    Offline,
    Busy,
    Unknown,
}

impl AgentInfo {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            endpoint: None,
            capabilities: Vec::new(),
            status: AgentStatus::Offline,
            metadata: HashMap::new(),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_capabilities(mut self, caps: Vec<String>) -> Self {
        self.capabilities = caps;
        self
    }

    pub fn online(&mut self) {
        self.status = AgentStatus::Online;
    }

    pub fn offline(&mut self) {
        self.status = AgentStatus::Offline;
    }
}

/// Agent Registry
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, agent: AgentInfo) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id.clone(), agent);
    }

    pub async fn unregister(&self, agent_id: &str) -> bool {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id).is_some()
    }

    pub async fn get(&self, agent_id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    pub async fn get_by_name(&self, name: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().find(|a| a.name == name).cloned()
    }

    pub async fn list(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    pub async fn list_online(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.status == AgentStatus::Online)
            .cloned()
            .collect()
    }

    pub async fn find_by_capability(&self, capability: &str) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.capabilities.iter().any(|c| c == capability))
            .cloned()
            .collect()
    }

    pub async fn update_status(&self, agent_id: &str, status: AgentStatus) -> bool {
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(agent_id) {
            agent.status = status;
            return true;
        }
        false
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry() {
        let registry = AgentRegistry::new();
        
        let agent = AgentInfo::new("agent1", "Agent One")
            .with_endpoint("http://localhost:8080")
            .with_capabilities(vec!["code".to_string(), "research".to_string()]);
        
        registry.register(agent).await;
        
        let found = registry.get("agent1").await;
        assert!(found.is_some());
    }
}
