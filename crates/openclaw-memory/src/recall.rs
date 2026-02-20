//! 简化版 Memory Recall 工具
//!
//! 作为 AI 工具自动调用，根据上下文自动检索相关记忆

use async_trait::async_trait;
use openclaw_core::{Message, Result};
use openclaw_vector::SearchQuery;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::embedding::EmbeddingProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallConfig {
    pub max_items: usize,
    pub min_similarity: f32,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            max_items: 10,
            min_similarity: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallResult {
    pub items: Vec<RecallItem>,
    pub query: String,
    pub total_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallItem {
    pub id: String,
    pub content: String,
    pub source: String,
    pub similarity: f32,
    pub memory_level: String,
}

#[async_trait]
pub trait MemoryRecall: Send + Sync {
    async fn recall(&self, query: &str, context: Option<&[Message]>) -> Result<RecallResult>;
}

pub struct SimpleMemoryRecall {
    embedding: Arc<dyn EmbeddingProvider>,
    vector_store: Arc<dyn openclaw_vector::VectorStore>,
    config: RecallConfig,
}

impl SimpleMemoryRecall {
    pub fn new(embedding: Arc<dyn EmbeddingProvider>, vector_store: Arc<dyn openclaw_vector::VectorStore>) -> Self {
        Self {
            embedding,
            vector_store,
            config: RecallConfig::default(),
        }
    }

    pub fn with_config(mut self, config: RecallConfig) -> Self {
        self.config = config;
        self
    }
}

#[async_trait]
impl MemoryRecall for SimpleMemoryRecall {
    async fn recall(&self, query: &str, _context: Option<&[Message]>) -> Result<RecallResult> {
        let query_embedding = self.embedding.embed(query).await?;

        let search_query = SearchQuery {
            vector: query_embedding,
            limit: self.config.max_items,
            filter: None,
            min_score: Some(self.config.min_similarity),
        };

        let results = self.vector_store.search(search_query).await?;

        let items: Vec<RecallItem> = results
            .into_iter()
            .filter(|r| r.score >= self.config.min_similarity)
            .map(|r| RecallItem {
                id: r.id.clone(),
                content: r.payload.get("content").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                source: r.payload.get("source").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                similarity: r.score,
                memory_level: r.payload.get("level").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            })
            .collect();

        let total_found = items.len();

        Ok(RecallResult {
            items,
            query: query.to_string(),
            total_found,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_vector::{VectorStore, MemoryStore};
    use crate::embedding::{Embedding, Embeddings};
    use openclaw_vector::VectorItem;

    struct MockEmbeddingProvider;

    #[async_trait]
    impl EmbeddingProvider for MockEmbeddingProvider {
        fn name(&self) -> &str { "mock" }
        fn model(&self) -> &str { "mock-model" }
        fn dimensions(&self) -> usize { 3 }
        
        async fn embed(&self, text: &str) -> Result<Embedding> {
            Ok(vec![0.1, 0.2, 0.3])
        }
        
        async fn embed_batch(&self, texts: &[String]) -> Result<Embeddings> {
            Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3]).collect())
        }
    }

    #[tokio::test]
    async fn test_recall_with_empty_store() {
        let embedding = Arc::new(MockEmbeddingProvider);
        let vector_store: Arc<dyn VectorStore> = Arc::new(MemoryStore::new());
        
        let recall = SimpleMemoryRecall::new(embedding, vector_store);
        let result = recall.recall("test query", None).await;
        
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.items.len(), 0);
        assert_eq!(result.query, "test query");
    }

    #[tokio::test]
    async fn test_recall_with_config() {
        let embedding = Arc::new(MockEmbeddingProvider);
        let vector_store: Arc<dyn VectorStore> = Arc::new(MemoryStore::new());
        
        let config = RecallConfig {
            max_items: 5,
            min_similarity: 0.9,
        };
        
        let recall = SimpleMemoryRecall::new(embedding, vector_store).with_config(config);
        let result = recall.recall("test", None).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_recall_with_items() {
        let embedding = Arc::new(MockEmbeddingProvider);
        let vector_store: Arc<dyn VectorStore> = Arc::new(MemoryStore::new());
        
        vector_store.upsert(VectorItem {
            id: "test1".to_string(),
            vector: vec![0.1, 0.2, 0.3],
            payload: vec![
                ("content".to_string(), "Hello world".to_string()),
                ("source".to_string(), "test".to_string()),
                ("level".to_string(), "short_term".to_string()),
            ].into_iter().collect(),
            created_at: chrono::Utc::now(),
        }).await.unwrap();
        
        let recall = SimpleMemoryRecall::new(embedding, vector_store);
        let result = recall.recall("hello", None).await;
        
        assert!(result.is_ok());
    }
}
