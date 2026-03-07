use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::squad::SquadRegistry;

#[derive(Debug, Error)]
pub enum CollaborationError {
    #[error("Collaboration error: {0}")]
    General(String),
    
    #[error("Message queue full")]
    QueueFull,
    
    #[error("Max delegation hops exceeded")]
    MaxHopsExceeded,
    
    #[error("Delegation conditions not met")]
    ConditionsNotMet,
    
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    
    #[error("Squad not found: {0}")]
    SquadNotFound(String),
}

impl From<CollaborationError> for openclaw_core::OpenClawError {
    fn from(err: CollaborationError) -> Self {
        openclaw_core::OpenClawError::Execution(err.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub message_type: MessageType,
    pub content: String,
    pub context: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Request,
    Response,
    Delegate,
    Notify,
    Broadcast,
}

impl Default for MessageType {
    fn default() -> Self {
        MessageType::Request
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub from_agent: String,
    pub to_agent: String,
    pub task_id: String,
    pub task_input: String,
    pub conditions: Vec<String>,
    pub max_hops: u32,
    pub current_hops: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResponse {
    pub task_id: String,
    pub from_agent: String,
    pub to_agent: String,
    pub status: DelegationStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DelegationStatus {
    Pending,
    Accepted,
    InProgress,
    Completed,
    Failed,
    Rejected,
    Timeout,
}

#[async_trait]
pub trait MessageHandler: Send + Sync {
    async fn handle(&self, message: AgentMessage) -> Result<Option<AgentMessage>, CollaborationError>;
}

pub struct HandlerStorage {
    handlers: Vec<Box<dyn MessageHandler>>,
}

impl HandlerStorage {
    pub fn new() -> Self {
        Self { handlers: Vec::new() }
    }
    
    pub fn push(&mut self, handler: Box<dyn MessageHandler>) {
        self.handlers.push(handler);
    }
}

pub struct AgentMessageBus {
    subscribers: Arc<RwLock<HashMap<String, HandlerStorage>>>,
    message_queue: Arc<RwLock<Vec<AgentMessage>>>,
    delegation_rules: Arc<RwLock<Vec<DelegationRule>>>,
    squad_registry: Arc<SquadRegistry>,
    max_queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRule {
    pub from_agent: String,
    pub to_agent: String,
    pub conditions: Vec<Condition>,
    pub max_hops: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: String,
}

impl AgentMessageBus {
    pub fn new(squad_registry: Arc<SquadRegistry>) -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            delegation_rules: Arc::new(RwLock::new(Vec::new())),
            squad_registry,
            max_queue_size: 1000,
        }
    }

    pub async fn subscribe(&self, agent_id: &str, handler: Box<dyn MessageHandler>) {
        let mut subs = self.subscribers.write().await;
        subs.entry(agent_id.to_string())
            .or_insert_with(HandlerStorage::new)
            .handlers
            .push(handler);
    }

    pub async fn unsubscribe(&self, agent_id: &str) {
        let mut subs = self.subscribers.write().await;
        subs.remove(agent_id);
    }

    pub async fn publish(&self, message: AgentMessage) -> Result<(), CollaborationError> {
        let mut queue = self.message_queue.write().await;
        
        if queue.len() >= self.max_queue_size {
            return Err(CollaborationError::QueueFull);
        }
        
        queue.push(message.clone());
        drop(queue);

        self.dispatch_message(message).await
    }

    async fn dispatch_message(&self, message: AgentMessage) -> Result<(), CollaborationError> {
        let subscribers = self.subscribers.read().await;
        
        if let Some(storage) = subscribers.get(&message.to_agent) {
            for handler in &storage.handlers {
                if let Err(e) = handler.handle(message.clone()).await {
                    tracing::warn!("Message handler error: {}", e);
                }
            }
        }
        
        Ok(())
    }

    pub async fn send_message(
        &self,
        from_agent: &str,
        to_agent: &str,
        message_type: MessageType,
        content: String,
        context: HashMap<String, String>,
    ) -> Result<AgentMessage, CollaborationError> {
        let message = AgentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from_agent: from_agent.to_string(),
            to_agent: to_agent.to_string(),
            message_type,
            content,
            context,
            timestamp: Utc::now(),
            correlation_id: None,
        };

        self.publish(message.clone()).await?;
        Ok(message)
    }

    pub async fn delegate_task(&self, request: DelegationRequest) -> Result<DelegationResponse, CollaborationError> {
        if request.current_hops >= request.max_hops {
            return Err(CollaborationError::MaxHopsExceeded);
        }

        let rules = self.delegation_rules.read().await;
        let matching_rule = rules.iter().find(|rule| {
            rule.from_agent == request.from_agent 
            && rule.to_agent == request.to_agent 
            && rule.enabled
        });

        if let Some(rule) = matching_rule {
            if !self.match_conditions(&rule.conditions, &request.task_input).await {
                return Err(CollaborationError::ConditionsNotMet);
            }
        }

        let message = AgentMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from_agent: request.from_agent.clone(),
            to_agent: request.to_agent.clone(),
            message_type: MessageType::Delegate,
            content: request.task_input.clone(),
            context: HashMap::new(),
            timestamp: Utc::now(),
            correlation_id: Some(request.task_id.clone()),
        };

        self.publish(message).await?;

        Ok(DelegationResponse {
            task_id: request.task_id,
            from_agent: request.from_agent,
            to_agent: request.to_agent,
            status: DelegationStatus::Accepted,
            result: None,
            error: None,
        })
    }

    pub async fn broadcast(&self, from_agent: &str, content: String) -> Result<Vec<AgentMessage>, CollaborationError> {
        let squads = self.squad_registry.list_squads().await;
        let mut messages = Vec::new();

        for squad in squads {
            for member in &squad.members {
                if member.agent_id != from_agent {
                    let msg = self.send_message(
                        from_agent,
                        &member.agent_id,
                        MessageType::Broadcast,
                        content.clone(),
                        HashMap::new(),
                    ).await?;
                    messages.push(msg);
                }
            }
        }

        Ok(messages)
    }

    pub async fn check_availability(&self, agent_id: &str) -> Result<bool, CollaborationError> {
        let squad = self.squad_registry.find_squad_by_agent(agent_id).await;
        Ok(squad.is_some())
    }

    pub async fn add_delegation_rule(&self, rule: DelegationRule) -> Result<(), CollaborationError> {
        let mut rules = self.delegation_rules.write().await;
        rules.retain(|r| !(r.from_agent == rule.from_agent && r.to_agent == rule.to_agent));
        rules.push(rule);
        Ok(())
    }

    pub async fn get_delegation_rules(&self, agent_id: &str) -> Vec<DelegationRule> {
        let rules = self.delegation_rules.read().await;
        rules.iter()
            .filter(|r| r.from_agent == agent_id && r.enabled)
            .cloned()
            .collect()
    }

    pub async fn list_pending_messages(&self) -> Vec<AgentMessage> {
        self.message_queue.read().await.clone()
    }

    pub async fn clear_processed_messages(&self) {
        let mut queue = self.message_queue.write().await;
        if queue.len() > self.max_queue_size / 2 {
            queue.clear();
        }
    }

    async fn match_conditions(&self, conditions: &[Condition], task_input: &str) -> bool {
        for condition in conditions {
            match condition.operator.as_str() {
                "equals" | "==" => {
                    if !task_input.contains(&condition.value) {
                        return false;
                    }
                }
                "contains" => {
                    if !task_input.contains(&condition.value) {
                        return false;
                    }
                }
                "starts_with" => {
                    if !task_input.starts_with(&condition.value) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }
}

impl Default for AgentMessageBus {
    fn default() -> Self {
        Self::new(Arc::new(SquadRegistry::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        received: Arc<RwLock<Vec<AgentMessage>>>,
    }

    #[async_trait]
    impl MessageHandler for TestHandler {
        async fn handle(&self, message: AgentMessage) -> Result<Option<AgentMessage>, CollaborationError> {
            let mut received = self.received.write().await;
            received.push(message);
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_message_bus_creation() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        assert!(bus.subscribers.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_subscribe_unsubscribe() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        let handler = Box::new(TestHandler {
            received: Arc::new(RwLock::new(Vec::new())),
        });

        bus.subscribe("agent_1", handler).await;
        assert!(bus.subscribers.read().await.contains_key("agent_1"));

        bus.unsubscribe("agent_1").await;
        assert!(!bus.subscribers.read().await.contains_key("agent_1"));
    }

    #[tokio::test]
    async fn test_send_message() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        let handler = Box::new(TestHandler {
            received: Arc::new(RwLock::new(Vec::new())),
        });

        bus.subscribe("agent_2", handler).await;

        let result = bus.send_message(
            "agent_1",
            "agent_2",
            MessageType::Request,
            "Hello".to_string(),
            HashMap::new(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_check_availability() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let result = bus.check_availability("unknown_agent").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_delegation_rule() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let rule = DelegationRule {
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            conditions: vec![],
            max_hops: 3,
            enabled: true,
        };

        let result = bus.add_delegation_rule(rule).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_delegation_rules() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let rule = DelegationRule {
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            conditions: vec![],
            max_hops: 3,
            enabled: true,
        };

        let _ = bus.add_delegation_rule(rule).await;
        
        let rules = bus.get_delegation_rules("agent_1").await;
        assert_eq!(rules.len(), 1);
    }

    #[tokio::test]
    async fn test_condition_matching() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let conditions = vec![
            Condition {
                field: "task_type".to_string(),
                operator: "contains".to_string(),
                value: "development".to_string(),
            },
        ];

        let result = bus.match_conditions(&conditions, "development task").await;
        assert!(result);

        let result2 = bus.match_conditions(&conditions, "marketing task").await;
        assert!(!result2);
    }

    #[tokio::test]
    async fn test_delegate_task() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let rule = DelegationRule {
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            conditions: vec![],
            max_hops: 3,
            enabled: true,
        };

        let _ = bus.add_delegation_rule(rule).await;

        let request = DelegationRequest {
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            task_id: "task_1".to_string(),
            task_input: "test task".to_string(),
            conditions: vec![],
            max_hops: 3,
            current_hops: 0,
        };

        let result = bus.delegate_task(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, DelegationStatus::Accepted);
    }

    #[tokio::test]
    async fn test_delegate_task_max_hops() {
        let bus = AgentMessageBus::new(Arc::new(SquadRegistry::new()));
        
        let request = DelegationRequest {
            from_agent: "agent_1".to_string(),
            to_agent: "agent_2".to_string(),
            task_id: "task_1".to_string(),
            task_input: "test task".to_string(),
            conditions: vec![],
            max_hops: 2,
            current_hops: 2,
        };

        let result = bus.delegate_task(request).await;
        assert!(result.is_err());
    }
}
