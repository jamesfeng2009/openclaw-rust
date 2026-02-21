//! Memory 抽象 Trait 层
//!
//! 定义 memory 的核心 trait，实现与具体实现的解耦

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::Result;

/// 向量嵌入 trait - 抽象具体的嵌入实现
#[async_trait]
pub trait EmbeddingProviderTrait: Send + Sync {
    fn name(&self) -> &str;
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// 向量存储 trait - 抽象具体的向量存储实现
#[async_trait]
pub trait VectorStoreTrait: Send + Sync {
    async fn upsert(&self, id: &str, vector: &[f32], payload: serde_json::Value) -> Result<()>;
    async fn search(&self, query: &[f32], limit: usize) -> Result<Vec<VectorSearchResult>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn clear(&self) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f32,
    pub payload: serde_json::Value,
}

/// 文本搜索 trait - 抽象 BM25 等文本搜索实现
#[async_trait]
pub trait TextSearchTrait: Send + Sync {
    async fn index(&self, id: &str, text: &str) -> Result<()>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<TextSearchResult>>;
    async fn delete(&self, id: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSearchResult {
    pub id: String,
    pub score: f32,
    pub text: String,
}

/// 知识图谱 trait - 抽象 KG 实现
#[async_trait]
pub trait KnowledgeGraphTrait: Send + Sync {
    async fn add_triple(&self, subject: &str, predicate: &str, object: &str) -> Result<()>;
    async fn query(&self, subject: &str, predicate: Option<&str>) -> Result<Vec<KnowledgeTriple>>;
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<KnowledgeTriple>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// 统一的搜索提供者 - 可选的组件
pub struct SearchProviders {
    pub vector: Option<Arc<dyn VectorStoreTrait>>,
    pub text: Option<Arc<dyn TextSearchTrait>>,
    pub knowledge_graph: Option<Arc<dyn KnowledgeGraphTrait>>,
}

impl SearchProviders {
    pub fn new() -> Self {
        Self {
            vector: None,
            text: None,
            knowledge_graph: None,
        }
    }
    
    pub fn with_vector(mut self, store: Arc<dyn VectorStoreTrait>) -> Self {
        self.vector = Some(store);
        self
    }
    
    pub fn with_text(mut self, search: Arc<dyn TextSearchTrait>) -> Self {
        self.text = Some(search);
        self
    }
    
    pub fn with_knowledge_graph(mut self, kg: Arc<dyn KnowledgeGraphTrait>) -> Self {
        self.knowledge_graph = Some(kg);
        self
    }
    
    pub fn has_vector(&self) -> bool {
        self.vector.is_some()
    }
    
    pub fn has_text(&self) -> bool {
        self.text.is_some()
    }
    
    pub fn has_knowledge_graph(&self) -> bool {
        self.knowledge_graph.is_some()
    }
}

impl Default for SearchProviders {
    fn default() -> Self {
        Self::new()
    }
}
