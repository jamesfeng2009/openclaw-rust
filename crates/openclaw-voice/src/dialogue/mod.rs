//! 对话上下文管理模块

pub mod intent;
pub mod slots;

pub use intent::{IntentRecognizer, KeywordIntentRecognizer, IntentRecognitionResult, RuleBasedIntentRecognizer, IntentRule};
pub use slots::{Slot, SlotManager};

use std::collections::HashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq)]
pub enum Speaker {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub id: String,
    pub speaker: Speaker,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub intent: Option<Intent>,
    pub entities: Vec<Entity>,
}

impl ConversationTurn {
    pub fn new(speaker: Speaker, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            speaker,
            content,
            timestamp: Utc::now(),
            intent: None,
            entities: Vec::new(),
        }
    }

    pub fn with_intent(mut self, intent: Intent) -> Self {
        self.intent = Some(intent);
        self
    }

    pub fn with_entities(mut self, entities: Vec<Entity>) -> Self {
        self.entities = entities;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Intent {
    pub name: String,
    pub confidence: f32,
    pub slots: HashMap<String, SlotValue>,
}

impl Intent {
    pub fn new(name: String, confidence: f32) -> Self {
        Self {
            name,
            confidence,
            slots: HashMap::new(),
        }
    }

    pub fn with_slot(mut self, key: String, value: SlotValue) -> Self {
        self.slots.insert(key, value);
        self
    }
}

#[derive(Debug, Clone)]
pub enum SlotValue {
    String(String),
    Number(f64),
    Boolean(bool),
    DateTime(DateTime<Utc>),
}

impl SlotValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            SlotValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            SlotValue::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SlotValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl std::fmt::Display for SlotValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotValue::String(s) => write!(f, "{}", s),
            SlotValue::Number(n) => write!(f, "{}", n),
            SlotValue::Boolean(b) => write!(f, "{}", b),
            SlotValue::DateTime(dt) => write!(f, "{}", dt),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub value: String,
    pub entity_type: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DialogueState {
    Idle,
    Listening,
    Processing,
    Speaking,
    WaitingForResponse,
}

pub struct DialogueContext {
    pub conversation_id: String,
    turns: Arc<RwLock<Vec<ConversationTurn>>>,
    current_state: Arc<RwLock<DialogueState>>,
    topic_stack: Arc<RwLock<Vec<String>>>,
    slots: Arc<RwLock<HashMap<String, SlotValue>>>,
    metadata: Arc<RwLock<HashMap<String, String>>>,
    created_at: DateTime<Utc>,
    updated_at: Arc<RwLock<DateTime<Utc>>>,
}

impl Clone for DialogueContext {
    fn clone(&self) -> Self {
        Self {
            conversation_id: self.conversation_id.clone(),
            turns: self.turns.clone(),
            current_state: self.current_state.clone(),
            topic_stack: self.topic_stack.clone(),
            slots: self.slots.clone(),
            metadata: self.metadata.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at.clone(),
        }
    }
}

impl DialogueContext {
    pub fn new(conversation_id: String) -> Self {
        Self {
            conversation_id,
            turns: Arc::new(RwLock::new(Vec::new())),
            current_state: Arc::new(RwLock::new(DialogueState::Idle)),
            topic_stack: Arc::new(RwLock::new(Vec::new())),
            slots: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            created_at: Utc::now(),
            updated_at: Arc::new(RwLock::new(Utc::now())),
        }
    }

    pub async fn add_turn(&self, speaker: Speaker, content: String) -> ConversationTurn {
        let turn = ConversationTurn::new(speaker, content);
        let mut turns = self.turns.write().await;
        turns.push(turn.clone());
        
        let mut updated = self.updated_at.write().await;
        *updated = Utc::now();
        
        turn
    }

    pub async fn add_user_turn(&self, content: String) -> ConversationTurn {
        self.add_turn(Speaker::User, content).await
    }

    pub async fn add_assistant_turn(&self, content: String) -> ConversationTurn {
        self.add_turn(Speaker::Assistant, content).await
    }

    pub async fn get_turns(&self) -> Vec<ConversationTurn> {
        self.turns.read().await.clone()
    }

    pub async fn get_last_turn(&self) -> Option<ConversationTurn> {
        let turns = self.turns.read().await;
        turns.last().cloned()
    }

    pub async fn get_conversation_history(&self) -> String {
        let turns = self.turns.read().await;
        turns
            .iter()
            .map(|t| match t.speaker {
                Speaker::User => format!("User: {}", t.content),
                Speaker::Assistant => format!("Assistant: {}", t.content),
                Speaker::System => format!("System: {}", t.content),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub async fn set_state(&self, state: DialogueState) {
        let mut current = self.current_state.write().await;
        *current = state;
    }

    pub async fn get_state(&self) -> DialogueState {
        self.current_state.read().await.clone()
    }

    pub async fn push_topic(&self, topic: String) {
        let mut topics = self.topic_stack.write().await;
        topics.push(topic);
    }

    pub async fn pop_topic(&self) -> Option<String> {
        let mut topics = self.topic_stack.write().await;
        topics.pop()
    }

    pub async fn get_current_topic(&self) -> Option<String> {
        let topics = self.topic_stack.read().await;
        topics.last().cloned()
    }

    pub async fn set_slot(&self, key: String, value: SlotValue) {
        let mut slots = self.slots.write().await;
        slots.insert(key, value);
    }

    pub async fn get_slot(&self, key: &str) -> Option<SlotValue> {
        let slots = self.slots.read().await;
        slots.get(key).cloned()
    }

    pub async fn clear_slots(&self) {
        let mut slots = self.slots.write().await;
        slots.clear();
    }

    pub async fn set_metadata(&self, key: String, value: String) {
        let mut metadata = self.metadata.write().await;
        metadata.insert(key, value);
    }

    pub async fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self.metadata.read().await;
        metadata.get(key).cloned()
    }

    pub async fn turn_count(&self) -> usize {
        self.turns.read().await.len()
    }

    pub async fn clear(&self) {
        let mut turns = self.turns.write().await;
        turns.clear();
        
        let mut topics = self.topic_stack.write().await;
        topics.clear();
        
        let mut slots = self.slots.write().await;
        slots.clear();
        
        let mut metadata = self.metadata.write().await;
        metadata.clear();
        
        let mut state = self.current_state.write().await;
        *state = DialogueState::Idle;
    }
}

pub struct DialogueContextManager {
    contexts: Arc<RwLock<HashMap<String, DialogueContext>>>,
    max_contexts: usize,
}

impl Clone for DialogueContextManager {
    fn clone(&self) -> Self {
        Self {
            contexts: self.contexts.clone(),
            max_contexts: self.max_contexts,
        }
    }
}

impl DialogueContextManager {
    pub fn new(max_contexts: usize) -> Self {
        Self {
            contexts: Arc::new(RwLock::new(HashMap::new())),
            max_contexts,
        }
    }

    pub async fn create_context(&self, conversation_id: String) -> DialogueContext {
        let context = DialogueContext::new(conversation_id.clone());
        
        let mut contexts = self.contexts.write().await;
        if contexts.len() >= self.max_contexts {
            if let Some(oldest) = contexts.keys().next().cloned() {
                contexts.remove(&oldest);
            }
        }
        
        contexts.insert(conversation_id, context.clone());
        context
    }

    pub async fn get_context(&self, conversation_id: &str) -> Option<DialogueContext> {
        let contexts = self.contexts.read().await;
        contexts.get(conversation_id).cloned()
    }

    pub async fn remove_context(&self, conversation_id: &str) -> bool {
        let mut contexts = self.contexts.write().await;
        contexts.remove(conversation_id).is_some()
    }

    pub async fn list_contexts(&self) -> Vec<String> {
        let contexts = self.contexts.read().await;
        contexts.keys().cloned().collect()
    }

    pub async fn context_count(&self) -> usize {
        let contexts = self.contexts.read().await;
        contexts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_turn_creation() {
        let turn = ConversationTurn::new(Speaker::User, "Hello".to_string());
        assert_eq!(turn.speaker, Speaker::User);
        assert_eq!(turn.content, "Hello");
        assert!(turn.intent.is_none());
    }

    #[test]
    fn test_conversation_turn_with_intent() {
        let intent = Intent::new("greeting".to_string(), 0.9);
        let turn = ConversationTurn::new(Speaker::User, "Hello".to_string())
            .with_intent(intent);
        assert!(turn.intent.is_some());
    }

    #[test]
    fn test_intent_with_slots() {
        let intent = Intent::new("weather_query".to_string(), 0.85)
            .with_slot("location".to_string(), SlotValue::String("Beijing".to_string()));
        
        assert_eq!(intent.slots.get("location").unwrap().as_string(), Some(&"Beijing".to_string()));
    }

    #[test]
    fn test_slot_value_display() {
        assert_eq!(SlotValue::String("test".to_string()).to_string(), "test");
        assert_eq!(SlotValue::Number(42.0).to_string(), "42");
        assert_eq!(SlotValue::Boolean(true).to_string(), "true");
    }

    #[tokio::test]
    async fn test_dialogue_context_new() {
        let context = DialogueContext::new("test-conversation".to_string());
        assert_eq!(context.conversation_id, "test-conversation");
        assert_eq!(context.get_state().await, DialogueState::Idle);
    }

    #[tokio::test]
    async fn test_dialogue_context_add_turn() {
        let context = DialogueContext::new("test".to_string());
        context.add_user_turn("Hello".to_string()).await;
        
        let turns = context.get_turns().await;
        assert_eq!(turns.len(), 1);
        assert_eq!(turns[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_dialogue_context_slots() {
        let context = DialogueContext::new("test".to_string());
        context.set_slot("name".to_string(), SlotValue::String("Alice".to_string())).await;
        
        let slot = context.get_slot("name").await;
        assert!(slot.is_some());
        assert_eq!(slot.unwrap().as_string(), Some(&"Alice".to_string()));
    }

    #[tokio::test]
    async fn test_dialogue_context_manager() {
        let manager = DialogueContextManager::new(10);
        let context = manager.create_context("conv-1".to_string()).await;
        
        assert_eq!(manager.context_count().await, 1);
        
        let retrieved = manager.get_context("conv-1").await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_dialogue_context_manager_max_contexts() {
        let manager = DialogueContextManager::new(2);
        
        manager.create_context("conv-1".to_string()).await;
        manager.create_context("conv-2".to_string()).await;
        manager.create_context("conv-3".to_string()).await;
        
        assert_eq!(manager.context_count().await, 2);
    }
}
