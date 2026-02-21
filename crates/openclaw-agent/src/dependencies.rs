//! Agent 依赖抽象层
//!
//! 通过 trait 抽象来解耦 Agent 与具体实现，允许运行时注入依赖

use std::sync::Arc;
use async_trait::async_trait;

use openclaw_core::Result;

/// AI 能力提供者 trait
#[async_trait]
pub trait AIProviderTrait: Send + Sync {
    async fn chat(&self, prompt: &str, context: &str) -> Result<String>;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// 记忆管理器 trait  
#[async_trait]
pub trait MemoryTrait: Send + Sync {
    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryItem>>;
    async fn remember(&self, content: &str, metadata: MemoryMetadata) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct MemoryItem {
    pub content: String,
    pub score: f32,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone)]
pub struct MemoryMetadata {
    pub timestamp: i64,
    pub importance: f32,
}

/// 安全管线 trait
#[async_trait]
pub trait SecurityTrait: Send + Sync {
    async fn check(&self, content: &str) -> Result<SecurityCheckResult>;
}

#[derive(Debug, Clone)]
pub struct SecurityCheckResult {
    pub is_safe: bool,
    pub violations: Vec<String>,
}

/// 工具注册表 trait
#[async_trait]
pub trait ToolRegistryTrait: Send + Sync {
    async fn execute(&self, tool_name: &str, args: serde_json::Value) -> Result<serde_json::Value>;
    async fn list_tools(&self) -> Result<Vec<String>>;
}

/// 向量存储 trait
#[async_trait]
pub trait VectorStoreTrait: Send + Sync {
    async fn upsert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()>;
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<VectorSearchResult>>;
}

#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

/// Agent 依赖容器 - 通过 Arc 包装实现共享注入
pub struct AgentDependencies {
    pub ai_provider: Option<Arc<dyn AIProviderTrait>>,
    pub memory: Option<Arc<dyn MemoryTrait>>,
    pub security: Option<Arc<dyn SecurityTrait>>,
    pub tools: Option<Arc<dyn ToolRegistryTrait>>,
    pub vector_store: Option<Arc<dyn VectorStoreTrait>>,
}

impl AgentDependencies {
    pub fn new() -> Self {
        Self {
            ai_provider: None,
            memory: None,
            security: None,
            tools: None,
            vector_store: None,
        }
    }
    
    pub fn with_ai(mut self, provider: Arc<dyn AIProviderTrait>) -> Self {
        self.ai_provider = Some(provider);
        self
    }
    
    pub fn with_memory(mut self, memory: Arc<dyn MemoryTrait>) -> Self {
        self.memory = Some(memory);
        self
    }
    
    pub fn with_security(mut self, security: Arc<dyn SecurityTrait>) -> Self {
        self.security = Some(security);
        self
    }
    
    pub fn with_tools(mut self, tools: Arc<dyn ToolRegistryTrait>) -> Self {
        self.tools = Some(tools);
        self
    }
    
    pub fn with_vector_store(mut self, store: Arc<dyn VectorStoreTrait>) -> Self {
        self.vector_store = Some(store);
        self
    }
}

impl Default for AgentDependencies {
    fn default() -> Self {
        Self::new()
    }
}
