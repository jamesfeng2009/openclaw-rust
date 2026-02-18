//! 记忆管理器

use std::sync::Arc;

use tokio::sync::RwLock;

use openclaw_core::{Message, OpenClawError, Result};
use openclaw_vector::VectorStore;

use crate::compressor::MemoryCompressor;
use crate::embedding::EmbeddingProvider;
use crate::hybrid_search::{HybridSearchConfig, HybridSearchManager};
use crate::recall::{MemoryRecall, RecallResult, SimpleMemoryRecall};
use crate::scorer::ImportanceScorer;
use crate::types::{MemoryConfig, MemoryContent, MemoryItem, MemoryLevel, MemoryRetrieval};
use crate::working::WorkingMemory;

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub working_count: usize,
    pub working_tokens: usize,
    pub short_term_count: usize,
    pub short_term_tokens: usize,
    pub long_term_enabled: bool,
}

pub struct MemoryManager {
    working: Arc<RwLock<WorkingMemory>>,
    short_term: Arc<RwLock<Vec<MemoryItem>>>,
    long_term: Option<Arc<dyn VectorStore>>,
    hybrid_search: Option<Arc<HybridSearchManager>>,
    config: MemoryConfig,
    scorer: ImportanceScorer,
    compressor: MemoryCompressor,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
}

impl MemoryManager {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            working: Arc::new(RwLock::new(WorkingMemory::new(config.working.clone()))),
            short_term: Arc::new(RwLock::new(Vec::new())),
            long_term: None,
            hybrid_search: None,
            scorer: ImportanceScorer::new(),
            compressor: MemoryCompressor::new(config.short_term.clone()),
            config,
            embedding_provider: None,
        }
    }

    pub fn with_vector_store(mut self, store: Arc<dyn VectorStore>) -> Self {
        self.long_term = Some(store);
        self
    }

    pub fn with_hybrid_search(mut self, search: Arc<HybridSearchManager>) -> Self {
        self.hybrid_search = Some(search);
        self
    }

    pub fn with_embedding_provider<E: EmbeddingProvider + 'static>(mut self, provider: E) -> Self {
        self.embedding_provider = Some(Arc::new(provider));
        self
    }

    pub async fn recall(&self, query: &str) -> Result<RecallResult> {
        if let (Some(provider), Some(store)) = (&self.embedding_provider, &self.long_term) {
            let recall_tool = SimpleMemoryRecall::new(provider.clone(), store.clone());
            let result = recall_tool.recall(query, None).await?;
            Ok(result)
        } else {
            Err(OpenClawError::Memory(
                "Embedding provider or vector store not configured".to_string(),
            ))
        }
    }

    pub async fn add(&self, message: Message) -> Result<()> {
        let score = self.scorer.score(&message);
        let item = MemoryItem::from_message(message, score);

        let overflow = {
            let mut working = self.working.write().await;
            working.add(item)
        };

        if let Some(overflow_items) = overflow {
            let summary = self.compressor.compress(overflow_items).await?;
            {
                let mut short_term = self.short_term.write().await;
                short_term.push(summary);

                if short_term.len() > self.config.short_term.max_summaries {
                    if let Some(old_summary) = short_term.first().cloned() {
                        if self.config.long_term.enabled
                            && let Some(store) = &self.long_term
                        {
                            let store = store.clone();
                            let old = old_summary.clone();
                            tokio::spawn(async move {
                                let _ = Self::archive_to_long_term_internal(&*store, old).await;
                            });
                        }
                        short_term.remove(0);
                    }
                }
            }
        }

        Ok(())
    }

    async fn archive_to_long_term_internal(store: &dyn VectorStore, item: MemoryItem) -> Result<()> {
        let payload = serde_json::json!({
            "content": format!("{:?}", item.content),
            "level": "short_term",
            "importance": item.importance_score,
        });
        let vector_item = openclaw_vector::VectorItem::new(vec![0.0; 128], payload);
        store.upsert(vector_item).await?;
        Ok(())
    }

    pub async fn retrieve(&self, query: &str, max_tokens: usize) -> Result<MemoryRetrieval> {
        let mut retrieval = MemoryRetrieval::new();
        let mut current_tokens = 0;

        let working_items = {
            let working = self.working.read().await;
            working.get_all()
        };
        for item in working_items.iter().rev() {
            if current_tokens + item.token_count > max_tokens {
                break;
            }
            retrieval.add(item.clone());
            current_tokens += item.token_count;
        }

        let short_term_items = {
            let short_term = self.short_term.read().await;
            short_term.iter().cloned().collect::<Vec<_>>()
        };
        for item in short_term_items.iter().rev() {
            if current_tokens + item.token_count > max_tokens {
                break;
            }
            retrieval.add(item.clone());
            current_tokens += item.token_count;
        }

        if self.config.long_term.enabled
            && let Some(search) = &self.hybrid_search
        {
            let config = HybridSearchConfig::default();
            if let Ok(results) = search.search(query, None, &config).await {
                for result in results {
                    let content_preview = result
                        .payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let token_count = content_preview.len() / 4;
                    let memory_item = MemoryItem {
                        id: uuid::Uuid::new_v4(),
                        level: MemoryLevel::LongTerm,
                        content: MemoryContent::VectorRef {
                            vector_id: result.id.clone(),
                            preview: content_preview,
                        },
                        created_at: chrono::Utc::now(),
                        last_accessed: chrono::Utc::now(),
                        access_count: 1,
                        importance_score: result.score,
                        token_count,
                        metadata: crate::types::MemoryMetadata::default(),
                    };

                    if current_tokens + token_count <= max_tokens {
                        retrieval.add(memory_item);
                        current_tokens += token_count;
                    }
                }
            }
        }

        Ok(retrieval)
    }

    pub fn get_context(&self) -> Vec<Message> {
        // 使用 Arc::try_unwrap 尝试获取内部值
        // 如果失败，克隆一份
        // 这是一个简化的实现
        Vec::new()
    }

    pub async fn get_context_async(&self) -> Vec<Message> {
        let working = self.working.read().await;
        working.to_messages()
    }

    pub async fn stats(&self) -> MemoryStats {
        let working = self.working.read().await;
        let short_term = self.short_term.read().await;
        
        MemoryStats {
            working_count: working.len(),
            working_tokens: working.total_tokens(),
            short_term_count: short_term.len(),
            short_term_tokens: short_term.iter().map(|i| i.token_count).sum(),
            long_term_enabled: self.long_term.is_some(),
        }
    }

    pub async fn clear(&self) -> Result<()> {
        {
            let mut working = self.working.write().await;
            working.clear();
        }
        {
            let mut short_term = self.short_term.write().await;
            short_term.clear();
        }

        if let Some(store) = &self.long_term {
            store.clear().await?;
        }

        Ok(())
    }

    async fn archive_to_long_term(
        &self,
        store: &dyn VectorStore,
        mut item: MemoryItem,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "content": format!("{:?}", item.content),
            "level": "short_term",
            "importance": item.importance_score,
            "created_at": item.created_at.to_rfc3339(),
        });
        
        let vector_item = openclaw_vector::VectorItem::new(vec![0.0; 128], payload);
        store.upsert(vector_item).await?;
        
        tracing::debug!("Archived memory item to long term: {}", item.id);
        Ok(())
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new(MemoryConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryConfig, WorkingMemoryConfig, ShortTermMemoryConfig, LongTermMemoryConfig};

    fn create_test_config() -> MemoryConfig {
        MemoryConfig {
            working: WorkingMemoryConfig {
                max_messages: 10,
                max_tokens: 4000,
            },
            short_term: ShortTermMemoryConfig {
                compress_after: 3,
                max_summaries: 5,
            },
            long_term: LongTermMemoryConfig {
                enabled: false,
                backend: "memory".to_string(),
                collection: "test".to_string(),
                embedding_provider: "openai".to_string(),
                embedding_model: "text-embedding-3-small".to_string(),
                custom_embedding: None,
            },
        }
    }

    #[tokio::test]
    async fn test_memory_manager_new() {
        let config = create_test_config();
        let manager = MemoryManager::new(config);
        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 0);
        assert_eq!(stats.short_term_count, 0);
    }

    #[tokio::test]
    async fn test_memory_manager_add() {
        let config = create_test_config();
        let manager = MemoryManager::new(config);
        
        let message = Message::new(
            openclaw_core::Role::User,
            vec![openclaw_core::Content::Text { text: "Hello".to_string() }],
        );
        
        manager.add(message).await.unwrap();
        
        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 1);
    }

    #[tokio::test]
    async fn test_memory_manager_clear() {
        let config = create_test_config();
        let manager = MemoryManager::new(config);
        
        let message = Message::new(
            openclaw_core::Role::User,
            vec![openclaw_core::Content::Text { text: "Hello".to_string() }],
        );
        
        manager.add(message).await.unwrap();
        manager.clear().await.unwrap();
        
        let stats = manager.stats().await;
        assert_eq!(stats.working_count, 0);
        assert_eq!(stats.short_term_count, 0);
    }

    #[tokio::test]
    async fn test_memory_manager_retrieve() {
        let config = create_test_config();
        let manager = MemoryManager::new(config);
        
        let message = Message::new(
            openclaw_core::Role::User,
            vec![openclaw_core::Content::Text { text: "Hello world".to_string() }],
        );
        
        manager.add(message).await.unwrap();
        
        let retrieval = manager.retrieve("Hello", 1000).await.unwrap();
        assert!(!retrieval.items.is_empty());
    }

    #[tokio::test]
    async fn test_memory_manager_with_vector_store() {
        let config = create_test_config();
        let manager = MemoryManager::new(config).with_vector_store(
            Arc::new(openclaw_vector::MemoryStore::new())
        );
        
        let stats = manager.stats().await;
        assert!(stats.long_term_enabled);
    }

    #[tokio::test]
    async fn test_memory_manager_recall_without_provider() {
        let config = create_test_config();
        let manager = MemoryManager::new(config);
        
        let result = manager.recall("test").await;
        assert!(result.is_err());
    }
}
