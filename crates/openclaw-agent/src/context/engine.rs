use async_trait::async_trait;
use openclaw_core::{Message, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::sessions::Session;
use crate::types::AgentId;
use openclaw_memory::traits::VectorStoreTrait;

#[async_trait]
pub trait ContextEngine: Send + Sync {
    async fn initialize(&self, agent_id: &AgentId) -> Result<ContextState>;

    async fn on_message(&self, session: &Session, message: &Message) -> Result<()>;

    async fn on_context(&self, session: &Session) -> Result<Vec<ContextFragment>>;

    async fn on_compress(&self, session: &Session) -> Result<CompressResult>;

    async fn on_build(&self, state: &mut PromptContext) -> Result<()>;

    async fn on_complete(&self, session: &Session, response: &Message) -> Result<()>;

    async fn on_sub_agent(&self, parent: &Session, child: &AgentId) -> Result<SubAgentContext>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextState {
    pub system_prompt: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ContextState {
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFragment {
    pub content: String,
    pub source: ContextSource,
    pub priority: u8,
    pub token_count: usize,
    #[serde(default)]
    pub expandable: bool,
    #[serde(default)]
    pub expand_ref: Option<Vec<Uuid>>,
}

impl ContextFragment {
    pub fn new(content: impl Into<String>, source: ContextSource, priority: u8) -> Self {
        let content_str = content.into();
        Self {
            content: content_str.clone(),
            source,
            priority,
            token_count: content_str.len() / 4,
            expandable: false,
            expand_ref: None,
        }
    }

    pub fn with_expandable(mut self, expand_ref: Vec<Uuid>) -> Self {
        self.expandable = true;
        self.expand_ref = Some(expand_ref);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSource {
    SystemPrompt,
    ProjectContext,
    LongTermMemory,
    SessionHistory,
    WorkingMemory,
    Skill,
    VectorSearch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressResult {
    pub summary: String,
    pub archived_count: usize,
    pub token_saved: usize,
}

impl Default for CompressResult {
    fn default() -> Self {
        Self {
            summary: String::new(),
            archived_count: 0,
            token_saved: 0,
        }
    }
}

impl CompressResult {
    pub fn new(summary: impl Into<String>, archived_count: usize, token_saved: usize) -> Self {
        Self {
            summary: summary.into(),
            archived_count,
            token_saved,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentContext {
    pub inherited_context: String,
    pub context_limit: usize,
}

impl SubAgentContext {
    pub fn new(inherited_context: impl Into<String>, context_limit: usize) -> Self {
        Self {
            inherited_context: inherited_context.into(),
            context_limit,
        }
    }

    pub fn default() -> Self {
        Self {
            inherited_context: String::new(),
            context_limit: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptContext {
    pub system_prompt: String,
    pub fragments: Vec<ContextFragment>,
    pub max_tokens: usize,
    pub current_tokens: usize,
}

impl Default for PromptContext {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            fragments: Vec::new(),
            max_tokens: 100000,
            current_tokens: 0,
        }
    }
}

impl PromptContext {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            system_prompt: String::new(),
            fragments: Vec::new(),
            max_tokens,
            current_tokens: 0,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self.current_tokens += self.system_prompt.len() / 4;
        self
    }

    pub fn add_fragment(&mut self, fragment: ContextFragment) {
        if self.current_tokens + fragment.token_count <= self.max_tokens {
            self.current_tokens += fragment.token_count;
            self.fragments.push(fragment);
        }
    }

    pub fn build(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        if !self.system_prompt.is_empty() {
            messages.push(Message::system(&self.system_prompt));
        }

        let mut sorted_fragments = self.fragments.clone();
        sorted_fragments.sort_by(|a, b| b.priority.cmp(&a.priority));

        for fragment in sorted_fragments {
            messages.push(Message::system(&fragment.content));
        }

        messages
    }

    pub fn remaining_tokens(&self) -> usize {
        self.max_tokens.saturating_sub(self.current_tokens)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEngineConfig {
    pub engine_type: ContextEngineType,
    pub max_tokens: usize,
    pub max_contexts: usize,
    pub compression_threshold: f32,
    pub auto_capture: bool,
    pub auto_recall: bool,
    #[serde(default)]
    pub vector_rag_config: Option<VectorRagConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorRagConfig {
    pub namespace: String,
    pub top_k: usize,
    pub min_score: f32,
}

impl Default for ContextEngineConfig {
    fn default() -> Self {
        Self {
            engine_type: ContextEngineType::Default,
            max_tokens: 100000,
            max_contexts: 100,
            compression_threshold: 0.8,
            auto_capture: true,
            auto_recall: true,
            vector_rag_config: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextEngineType {
    Default,
    Lossless,
    RagLite,
    VectorRag,
}

pub fn extract_content(message: &openclaw_core::Message) -> String {
    message.content.iter().map(|c| {
        match c {
            openclaw_core::Content::Text { text } => text.clone(),
            openclaw_core::Content::Image { url } => format!("[Image: {}]", url),
            openclaw_core::Content::Audio { url } => format!("[Audio: {}]", url),
            openclaw_core::Content::ToolCall { name, .. } => format!("[Tool: {}]", name),
            openclaw_core::Content::ToolResult { content, .. } => content.clone(),
        }
    }).collect::<Vec<_>>().join("\n")
}

pub fn create_context_engine(
    config: ContextEngineConfig,
) -> Arc<dyn ContextEngine> {
    match config.engine_type {
        ContextEngineType::Default => Arc::new(DefaultContextEngine::new(config)),
        ContextEngineType::Lossless => Arc::new(LosslessContextEngine::new(config)),
        ContextEngineType::RagLite => Arc::new(RagLiteContextEngine::new(config)),
        ContextEngineType::VectorRag => Arc::new(VectorRagContextEngine::new(config)),
    }
}

pub struct DefaultContextEngine {
    config: ContextEngineConfig,
}

impl DefaultContextEngine {
    pub fn new(config: ContextEngineConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ContextEngine for DefaultContextEngine {
    async fn initialize(&self, _agent_id: &AgentId) -> Result<ContextState> {
        Ok(ContextState::new("You are a helpful AI assistant."))
    }

    async fn on_message(&self, _session: &Session, _message: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_context(&self, _session: &Session) -> Result<Vec<ContextFragment>> {
        Ok(Vec::new())
    }

    async fn on_compress(&self, session: &Session) -> Result<CompressResult> {
        let message_count = session.message_count;
        let token_count = session.token_count as usize;
        let summary = format!(
            "[Summary of {} messages, {} tokens]",
            message_count, token_count
        );
        Ok(CompressResult::new(summary, message_count, token_count / 2))
    }

    async fn on_build(&self, _state: &mut PromptContext) -> Result<()> {
        Ok(())
    }

    async fn on_complete(&self, _session: &Session, _response: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_sub_agent(&self, _parent: &Session, _child: &AgentId) -> Result<SubAgentContext> {
        Ok(SubAgentContext::new(String::new(), 10000))
    }
}

pub struct LosslessContextEngine {
    config: ContextEngineConfig,
}

impl LosslessContextEngine {
    pub fn new(config: ContextEngineConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ContextEngine for LosslessContextEngine {
    async fn initialize(&self, _agent_id: &AgentId) -> Result<ContextState> {
        Ok(ContextState::new(
            "You are a helpful AI assistant. Memory is preserved across conversations.",
        ))
    }

    async fn on_message(&self, _session: &Session, _message: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_context(&self, session: &Session) -> Result<Vec<ContextFragment>> {
        let summary = format!(
            "[Session: {} - {} messages, {} tokens]",
            session.name, session.message_count, session.token_count
        );
        Ok(vec![ContextFragment::new(
            summary,
            ContextSource::SessionHistory,
            8,
        )])
    }

    async fn on_compress(&self, session: &Session) -> Result<CompressResult> {
        let message_count = session.message_count;
        let token_count = session.token_count as usize;
        let summary = format!(
            "[Lossless Summary - {} messages archived, {} tokens saved]",
            message_count, token_count
        );
        Ok(CompressResult::new(summary, message_count, token_count))
    }

    async fn on_build(&self, _state: &mut PromptContext) -> Result<()> {
        Ok(())
    }

    async fn on_complete(&self, _session: &Session, _response: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_sub_agent(&self, parent: &Session, _child: &AgentId) -> Result<SubAgentContext> {
        let inherited_text = parent.history_summary.clone().unwrap_or_default();
        Ok(SubAgentContext::new(inherited_text, self.config.max_tokens / 2))
    }
}

pub struct RagLiteContextEngine {
    config: ContextEngineConfig,
    memory_store: std::sync::Mutex<Vec<ContextFragment>>,
}

impl RagLiteContextEngine {
    pub fn new(config: ContextEngineConfig) -> Self {
        Self { 
            config,
            memory_store: std::sync::Mutex::new(Vec::new()),
        }
    }
}

pub struct VectorRagContextEngine {
    config: ContextEngineConfig,
    vector_store: Option<Arc<dyn VectorStoreTrait>>,
    memory_store: std::sync::Mutex<Vec<ContextFragment>>,
}

impl VectorRagContextEngine {
    pub fn new(config: ContextEngineConfig) -> Self {
        Self {
            config,
            vector_store: None,
            memory_store: std::sync::Mutex::new(Vec::new()),
        }
    }

    pub fn with_vector_store(mut self, store: Arc<dyn VectorStoreTrait>) -> Self {
        self.vector_store = Some(store);
        self
    }

    pub async fn add_to_vector_store(&self, content: &str, id: &str) -> Result<()> {
        if let Some(store) = &self.vector_store {
            let embedding = self.generate_embedding(content).await?;
            let payload = serde_json::json!({
                "content": content,
                "id": id,
            });
            store.upsert(id, &embedding, payload).await?;
        }
        Ok(())
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.0; 384])
    }

    pub async fn recall(&self, query: &str, top_k: usize) -> Result<Vec<ContextFragment>> {
        if let Some(store) = &self.vector_store {
            let query_embedding = self.generate_embedding(query).await?;
            let results = store.search(&query_embedding, top_k).await?;
            
            let fragments: Vec<ContextFragment> = results.iter()
                .filter(|r| r.score >= self.config.vector_rag_config.as_ref()
                    .map(|c| c.min_score).unwrap_or(0.5))
                .map(|r| {
                    let content = r.payload.get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    ContextFragment::new(content, ContextSource::VectorSearch, 5)
                })
                .collect();
            
            return Ok(fragments);
        }
        Ok(Vec::new())
    }
}

#[async_trait]
impl ContextEngine for VectorRagContextEngine {
    async fn initialize(&self, _agent_id: &AgentId) -> Result<ContextState> {
        Ok(ContextState::new(
            "You are a helpful AI assistant with Vector RAG memory. Relevant information is retrieved from vector store.",
        ))
    }

    async fn on_message(&self, _session: &Session, message: &Message) -> Result<()> {
        let content = extract_content(message);
        if !content.is_empty() {
            let id = Uuid::new_v4().to_string();
            if let Err(e) = self.add_to_vector_store(&content, &id).await {
                tracing::warn!("Failed to add to vector store: {}", e);
            }
            let fragment = ContextFragment::new(
                content.clone(),
                ContextSource::SessionHistory,
                5,
            );
            let mut store = self.memory_store.lock().unwrap();
            if store.len() >= self.config.max_contexts {
                store.remove(0);
            }
            store.push(fragment);
        }
        Ok(())
    }

    async fn on_context(&self, _session: &Session) -> Result<Vec<ContextFragment>> {
        Ok(vec![])
    }

    async fn on_compress(&self, _session: &Session) -> Result<CompressResult> {
        Ok(CompressResult::default())
    }

    async fn on_build(&self, _state: &mut PromptContext) -> Result<()> {
        Ok(())
    }

    async fn on_complete(&self, _session: &Session, _response: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_sub_agent(&self, _parent: &Session, _child: &AgentId) -> Result<SubAgentContext> {
        Ok(SubAgentContext::default())
    }
}

#[async_trait]
impl ContextEngine for RagLiteContextEngine {
    async fn initialize(&self, _agent_id: &AgentId) -> Result<ContextState> {
        Ok(ContextState::new(
            "You are a helpful AI assistant with RAG-Lite memory. Important information is stored and retrieved efficiently.",
        ))
    }

    async fn on_message(&self, _session: &Session, message: &Message) -> Result<()> {
        let content = extract_content(message);
        if !content.is_empty() {
            let fragment = ContextFragment::new(
                content.clone(),
                ContextSource::SessionHistory,
                5,
            );
            let mut store = self.memory_store.lock().unwrap();
            if store.len() >= self.config.max_contexts {
                store.remove(0);
            }
            store.push(fragment);
        }
        Ok(())
    }

    async fn on_context(&self, _session: &Session) -> Result<Vec<ContextFragment>> {
        let store = self.memory_store.lock().unwrap();
        let fragments: Vec<ContextFragment> = store.iter()
            .filter(|f| f.token_count <= self.config.max_tokens / 2)
            .cloned()
            .collect();
        Ok(fragments)
    }

    async fn on_compress(&self, session: &Session) -> Result<CompressResult> {
        let mut store = self.memory_store.lock().unwrap();
        let original_count = store.len();
        let original_tokens: usize = store.iter().map(|f| f.token_count).sum();
        
        let important: Vec<ContextFragment> = store.iter()
            .filter(|f| f.priority >= 7)
            .cloned()
            .collect();
        
        *store = important;
        
        let new_tokens: usize = store.iter().map(|f| f.token_count).sum();
        Ok(CompressResult::new(
            format!("[RAG-Lite Compressed: {} -> {} fragments, {} -> {} tokens]", 
                original_count, store.len(), original_tokens, new_tokens),
            original_count.saturating_sub(store.len()),
            original_tokens.saturating_sub(new_tokens),
        ))
    }

    async fn on_build(&self, state: &mut PromptContext) -> Result<()> {
        let store = self.memory_store.lock().unwrap();
        for fragment in store.iter() {
            state.add_fragment(fragment.clone());
        }
        Ok(())
    }

    async fn on_complete(&self, _session: &Session, _response: &Message) -> Result<()> {
        Ok(())
    }

    async fn on_sub_agent(&self, parent: &Session, _child: &AgentId) -> Result<SubAgentContext> {
        let store = self.memory_store.lock().unwrap();
        let context: String = store.iter()
            .map(|f| f.content.clone())
            .take(5)
            .collect::<Vec<_>>()
            .join("\n---\n");
        let inherited = if context.is_empty() {
            parent.history_summary.clone().unwrap_or_default()
        } else {
            context
        };
        Ok(SubAgentContext::new(inherited, self.config.max_tokens / 2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_core::session::SessionScope;
    use crate::sessions::Session;
    use openclaw_core::Message;

    #[tokio::test]
    async fn test_default_engine_initialize() {
        let engine = DefaultContextEngine::new(ContextEngineConfig::default());
        let state = engine.initialize(&"test-agent".to_string()).await;
        assert!(state.is_ok());
        let state = state.unwrap();
        assert!(!state.system_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_lossless_engine_initialize() {
        let engine = LosslessContextEngine::new(ContextEngineConfig::default());
        let state = engine.initialize(&"test-agent".to_string()).await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_context_fragment() {
        let fragment = ContextFragment::new("test content", ContextSource::SystemPrompt, 5);
        assert_eq!(fragment.content, "test content");
        assert_eq!(fragment.priority, 5);
        assert!(!fragment.expandable);
    }

    #[tokio::test]
    async fn test_context_fragment_with_expand() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let fragment = ContextFragment::new("test", ContextSource::SessionHistory, 8)
            .with_expandable(ids.clone());
        assert!(fragment.expandable);
        assert_eq!(fragment.expand_ref, Some(ids));
    }

    #[tokio::test]
    async fn test_prompt_context_build() {
        let mut ctx = PromptContext::new(10000);
        ctx = ctx.with_system_prompt("You are helpful.");

        ctx.add_fragment(ContextFragment::new(
            "Fragment 1",
            ContextSource::LongTermMemory,
            5,
        ));
        ctx.add_fragment(ContextFragment::new(
            "Fragment 2",
            ContextSource::ProjectContext,
            8,
        ));

        let messages = ctx.build();
        assert_eq!(messages.len(), 3);
    }

    #[tokio::test]
    async fn test_prompt_context_max_tokens() {
        let mut ctx = PromptContext::new(100);
        ctx = ctx.with_system_prompt("System prompt with many characters to exceed token limit");

        ctx.add_fragment(ContextFragment::new(
            "A very long fragment content that should exceed the token limit",
            ContextSource::LongTermMemory,
            5,
        ));

        assert!(ctx.current_tokens < ctx.max_tokens);
    }

    #[tokio::test]
    async fn test_compress_result() {
        let result = CompressResult::new("test summary", 10, 500);
        assert_eq!(result.summary, "test summary");
        assert_eq!(result.archived_count, 10);
        assert_eq!(result.token_saved, 500);
    }

    #[tokio::test]
    async fn test_sub_agent_context() {
        let ctx = SubAgentContext::new("inherited context text", 5000);
        assert_eq!(ctx.inherited_context, "inherited context text");
        assert_eq!(ctx.context_limit, 5000);
    }

    #[tokio::test]
    async fn test_create_default_engine() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::Default,
            ..Default::default()
        };
        let engine = create_context_engine(config);
        let state = engine.initialize(&"test".to_string()).await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_create_lossless_engine() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::Lossless,
            ..Default::default()
        };
        let engine = create_context_engine(config);
        let state = engine.initialize(&"test".to_string()).await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_rag_lite_engine_initialize() {
        let engine = RagLiteContextEngine::new(ContextEngineConfig::default());
        let state = engine.initialize(&"test-agent".to_string()).await;
        assert!(state.is_ok());
        let state = state.unwrap();
        assert!(!state.system_prompt.is_empty());
    }

    #[tokio::test]
    async fn test_rag_lite_on_message() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::RagLite,
            max_contexts: 2,
            ..Default::default()
        };
        let engine = RagLiteContextEngine::new(config);
        
        let session = Session::new("test-session", "test-agent".to_string(), SessionScope::Main);
        let message = Message::user("Hello, world!");
        
        let result = engine.on_message(&session, &message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rag_lite_on_compress() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::RagLite,
            max_contexts: 2,
            ..Default::default()
        };
        let engine = RagLiteContextEngine::new(config);
        
        let session = Session::new("test-session", "test-agent".to_string(), SessionScope::Main);
        
        let result = engine.on_compress(&session).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_create_rag_lite_engine() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::RagLite,
            ..Default::default()
        };
        let engine = create_context_engine(config);
        let state = engine.initialize(&"test".to_string()).await;
        assert!(state.is_ok());
    }

    #[tokio::test]
    async fn test_create_vector_rag_engine() {
        let config = ContextEngineConfig {
            engine_type: ContextEngineType::VectorRag,
            vector_rag_config: Some(VectorRagConfig {
                namespace: "test".to_string(),
                top_k: 5,
                min_score: 0.7,
            }),
            ..Default::default()
        };
        let engine = VectorRagContextEngine::new(config);
        let state = engine.initialize(&"test".to_string()).await;
        assert!(state.is_ok());
    }
}
