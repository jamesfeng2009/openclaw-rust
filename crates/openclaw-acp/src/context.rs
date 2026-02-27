//! ACP Context Management
//!
//! Provides shared context for multi-agent collaboration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Context entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
    pub updated_by: String,
}

/// Shared Context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedContext {
    pub context_id: String,
    pub conversation_id: String,
    pub entries: HashMap<String, ContextEntry>,
    pub agent_outputs: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SharedContext {
    pub fn new(conversation_id: String) -> Self {
        let now = Utc::now();
        Self {
            context_id: uuid::Uuid::new_v4().to_string(),
            conversation_id,
            entries: HashMap::new(),
            agent_outputs: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn set(&mut self, key: String, value: serde_json::Value, updated_by: String) {
        let entry = ContextEntry {
            key: key.clone(),
            value,
            updated_at: Utc::now(),
            updated_by,
        };
        self.entries.insert(key, entry);
        self.updated_at = Utc::now();
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.entries.get(key).map(|e| &e.value)
    }

    pub fn set_agent_output(&mut self, agent_id: String, output: serde_json::Value) {
        self.agent_outputs.insert(agent_id, output);
        self.updated_at = Utc::now();
    }

    pub fn get_agent_output(&self, agent_id: &str) -> Option<&serde_json::Value> {
        self.agent_outputs.get(agent_id)
    }
}

/// Context Manager
pub struct ContextManager {
    contexts: Arc<RwLock<HashMap<String, SharedContext>>>,
    max_contexts: usize,
}

impl ContextManager {
    pub fn new() -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            max_contexts: 1000,
        }
    }

    pub fn with_max_contexts(max: usize) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            max_contexts: max,
        }
    }

    pub async fn create(&self, conversation_id: String) -> SharedContext {
        let context = SharedContext::new(conversation_id);
        let mut contexts = self.contexts.write().await;
        contexts.insert(context.context_id.clone(), context.clone());
        context
    }

    pub async fn get(&self, context_id: &str) -> Option<SharedContext> {
        let contexts = self.contexts.read().await;
        contexts.get(context_id).cloned()
    }

    pub async fn get_or_create(&self, conversation_id: String) -> SharedContext {
        let contexts = self.contexts.read().await;
        for context in contexts.values() {
            if context.conversation_id == conversation_id {
                return context.clone();
            }
        }
        drop(contexts);
        self.create(conversation_id).await
    }

    pub async fn update(&self, context_id: &str, key: String, value: serde_json::Value, updated_by: String) -> Option<SharedContext> {
        let mut contexts = self.contexts.write().await;
        if let Some(context) = contexts.get_mut(context_id) {
            context.set(key, value, updated_by);
            return Some(context.clone());
        }
        None
    }

    pub async fn set_agent_output(&self, context_id: &str, agent_id: String, output: serde_json::Value) -> Option<SharedContext> {
        let mut contexts = self.contexts.write().await;
        if let Some(context) = contexts.get_mut(context_id) {
            context.set_agent_output(agent_id, output);
            return Some(context.clone());
        }
        None
    }

    pub async fn delete(&self, context_id: &str) -> bool {
        let mut contexts = self.contexts.write().await;
        contexts.remove(context_id).is_some()
    }

    pub async fn cleanup_expired(&self, max_age_seconds: i64) {
        let mut contexts = self.contexts.write().await;
        let now = Utc::now();
        contexts.retain(|_, context| {
            let age = (now - context.updated_at).num_seconds();
            age < max_age_seconds
        });
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_creation() {
        let manager = ContextManager::new();
        let context = manager.create("conv-123".to_string()).await;
        assert_eq!(context.conversation_id, "conv-123");
    }

    #[tokio::test]
    async fn test_context_set_get() {
        let manager = ContextManager::new();
        let context = manager.create("conv-123".to_string()).await;
        manager.update(&context.context_id, "key1".to_string(), serde_json::json!("value1"), "agent1".to_string()).await;
        
        let updated = manager.get(&context.context_id).await.unwrap();
        assert_eq!(updated.get("key1").unwrap(), "value1");
    }
}
