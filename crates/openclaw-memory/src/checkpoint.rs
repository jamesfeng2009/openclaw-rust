use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub agent_id: String,
    pub internal_state: HashMap<String, serde_json::Value>,
    pub tool_history: Vec<ToolCall>,
    pub message_history: Vec<Message>,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl AgentState {
    pub fn new(agent_id: String) -> Self {
        let now = Utc::now();
        Self {
            agent_id,
            internal_state: HashMap::new(),
            tool_history: Vec::new(),
            message_history: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn update_state(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.internal_state.insert(key.into(), value);
        self.updated_at = Utc::now();
    }

    pub fn add_message(&mut self, role: MessageRole, content: impl Into<String>) {
        let message = Message {
            id: uuid::Uuid::new_v4().to_string(),
            role,
            content: content.into(),
            timestamp: Utc::now(),
        };
        self.message_history.push(message);
        self.updated_at = Utc::now();
    }

    pub fn add_tool_call(&mut self, tool_name: impl Into<String>, input: serde_json::Value) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let tool_call = ToolCall {
            id: id.clone(),
            tool_name: tool_name.into(),
            input,
            output: None,
            timestamp: Utc::now(),
            duration_ms: 0,
        };
        self.tool_history.push(tool_call);
        self.updated_at = Utc::now();
        id
    }

    pub fn complete_tool_call(&mut self, call_id: &str, output: serde_json::Value, duration_ms: u64) {
        if let Some(tool_call) = self.tool_history.iter_mut().find(|t| t.id == call_id) {
            tool_call.output = Some(output);
            tool_call.duration_ms = duration_ms;
        }
        self.updated_at = Utc::now();
    }

    pub fn get_recent_messages(&self, count: usize) -> Vec<&Message> {
        self.message_history.iter().rev().take(count).collect()
    }

    pub fn get_recent_tool_calls(&self, count: usize) -> Vec<&ToolCall> {
        self.tool_history.iter().rev().take(count).collect()
    }

    pub fn replay_messages(&self, from_index: usize) -> Vec<&Message> {
        self.message_history.iter().skip(from_index).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub agent_id: String,
    pub state: AgentState,
    pub session_id: String,
    pub sequence_number: u64,
    pub created_at: DateTime<Utc>,
}

impl Checkpoint {
    pub fn new(agent_id: String, session_id: String, state: AgentState, sequence_number: u64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            agent_id,
            state,
            session_id,
            sequence_number,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_state_creation() {
        let state = AgentState::new("agent-1".to_string());
        assert_eq!(state.agent_id, "agent-1");
        assert!(state.internal_state.is_empty());
        assert!(state.tool_history.is_empty());
        assert!(state.message_history.is_empty());
    }

    #[test]
    fn test_update_state() {
        let mut state = AgentState::new("agent-1".to_string());
        state.update_state("key1", serde_json::json!("value1"));
        
        assert_eq!(state.internal_state.get("key1"), Some(&serde_json::json!("value1")));
    }

    #[test]
    fn test_add_message() {
        let mut state = AgentState::new("agent-1".to_string());
        state.add_message(MessageRole::User, "Hello");
        
        assert_eq!(state.message_history.len(), 1);
        assert_eq!(state.message_history[0].content, "Hello");
    }

    #[test]
    fn test_tool_call_lifecycle() {
        let mut state = AgentState::new("agent-1".to_string());
        
        let call_id = state.add_tool_call("search", serde_json::json!({"query": "test"}));
        assert_eq!(state.tool_history.len(), 1);
        
        state.complete_tool_call(&call_id, serde_json::json!({"results": []}), 100);
        
        let tool_call = &state.tool_history[0];
        assert!(tool_call.output.is_some());
        assert_eq!(tool_call.duration_ms, 100);
    }

    #[test]
    fn test_recent_messages() {
        let mut state = AgentState::new("agent-1".to_string());
        
        for i in 0..10 {
            state.add_message(MessageRole::User, format!("Message {}", i));
        }
        
        let recent = state.get_recent_messages(3);
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn test_checkpoint_creation() {
        let state = AgentState::new("agent-1".to_string());
        let checkpoint = Checkpoint::new("agent-1".to_string(), "session-1".to_string(), state, 1);
        
        assert_eq!(checkpoint.agent_id, "agent-1");
        assert_eq!(checkpoint.session_id, "session-1");
        assert_eq!(checkpoint.sequence_number, 1);
    }

    #[test]
    fn test_replay_from_index() {
        let mut state = AgentState::new("agent-1".to_string());
        
        for i in 0..5 {
            state.add_message(MessageRole::User, format!("Message {}", i));
        }
        
        let replayed = state.replay_messages(2);
        assert_eq!(replayed.len(), 3);
    }
}
