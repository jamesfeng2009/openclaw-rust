//! 检索执行器

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{Message, Result};

use super::config::{ExecutorConfig, SourceType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub id: String,
    pub content: String,
    pub source: SourceType,
    pub relevance_score: f32,
    pub metadata: HashMap<String, String>,
}

#[async_trait]
pub trait RetrievalExecutor: Send + Sync {
    async fn execute(
        &self,
        query: &str,
        config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>>;

    async fn execute_with_context(
        &self,
        query: &str,
        context: &[Message],
        config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>>;

    fn source_type(&self) -> SourceType;
}

pub struct MemoryRetrievalExecutor {
    memory_manager: Arc<openclaw_memory::MemoryManager>,
    source_type: SourceType,
}

impl MemoryRetrievalExecutor {
    pub fn new(memory_manager: Arc<openclaw_memory::MemoryManager>) -> Self {
        Self {
            memory_manager,
            source_type: SourceType::Memory,
        }
    }
}

#[async_trait]
impl RetrievalExecutor for MemoryRetrievalExecutor {
    async fn execute(
        &self,
        query: &str,
        _config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>> {
        let result = self.memory_manager.recall(query).await?;

        Ok(result
            .items
            .into_iter()
            .map(|item| RetrievalResult {
                id: item.id.to_string(),
                content: item.content.clone(),
                source: SourceType::Memory,
                relevance_score: item.similarity,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("memory_level".to_string(), item.memory_level);
                    m.insert("source".to_string(), item.source);
                    m
                },
            })
            .collect())
    }

    async fn execute_with_context(
        &self,
        query: &str,
        _context: &[Message],
        config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>> {
        self.execute(query, config).await
    }

    fn source_type(&self) -> SourceType {
        self.source_type.clone()
    }
}

pub struct VectorDBRetrievalExecutor {
    vector_store: Arc<dyn openclaw_vector::VectorStore>,
    embedding_provider: Arc<dyn openclaw_memory::embedding::EmbeddingProvider>,
    source_type: SourceType,
    collection: Option<String>,
}

impl VectorDBRetrievalExecutor {
    pub fn new(
        vector_store: Arc<dyn openclaw_vector::VectorStore>,
        embedding_provider: Arc<dyn openclaw_memory::embedding::EmbeddingProvider>,
    ) -> Self {
        Self {
            vector_store,
            embedding_provider,
            source_type: SourceType::VectorDB,
            collection: None,
        }
    }

    pub fn with_collection(mut self, collection: String) -> Self {
        self.collection = Some(collection);
        self
    }
}

#[async_trait]
impl RetrievalExecutor for VectorDBRetrievalExecutor {
    async fn execute(
        &self,
        query: &str,
        config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>> {
        let query_embedding = self.embedding_provider.embed(query).await?;

        let search_result = self
            .vector_store
            .search(openclaw_vector::SearchQuery {
                vector: query_embedding,
                limit: config.max_results_per_source,
                filter: None,
                min_score: Some(0.0),
            })
            .await?;

        Ok(search_result
            .into_iter()
            .map(|item| {
                let metadata: HashMap<String, String> = item
                    .payload
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter_map(|(k, v)| {
                                v.as_str().map(|s| (k.clone(), s.to_string()))
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                RetrievalResult {
                    id: item.id.clone(),
                    content: item
                        .payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    source: SourceType::VectorDB,
                    relevance_score: item.score,
                    metadata,
                }
            })
            .collect())
    }

    async fn execute_with_context(
        &self,
        query: &str,
        _context: &[Message],
        config: &ExecutorConfig,
    ) -> Result<Vec<RetrievalResult>> {
        self.execute(query, config).await
    }

    fn source_type(&self) -> SourceType {
        self.source_type.clone()
    }
}

pub struct MultiSourceRetrievalExecutor {
    executors: HashMap<SourceType, Box<dyn RetrievalExecutor>>,
}

impl MultiSourceRetrievalExecutor {
    pub fn new() -> Self {
        Self {
            executors: HashMap::new(),
        }
    }

    pub fn add_executor(mut self, executor: Box<dyn RetrievalExecutor>) -> Self {
        let source_type = executor.source_type();
        self.executors.insert(source_type, executor);
        self
    }

    pub async fn execute_all(
        &self,
        query: &str,
        sources: &[SourceType],
        config: &ExecutorConfig,
        parallel: bool,
    ) -> Result<Vec<RetrievalResult>> {
        if parallel {
            let mut futures = Vec::new();
            for source in sources {
                if let Some(executor) = self.executors.get(source) {
                    let q = query.to_string();
                    let c = config.clone();
                    futures.push(async move {
                        executor.execute(&q, &c).await
                    });
                }
            }

            let results = futures::future::join_all(futures)
                .await
                .into_iter()
                .filter_map(|r| r.ok())
                .flatten()
                .collect();

            Ok(results)
        } else {
            let mut all_results = Vec::new();
            for source in sources {
                if let Some(executor) = self.executors.get(source) {
                    match executor.execute(query, config).await {
                        Ok(results) => all_results.extend(results),
                        Err(e) => tracing::warn!("Executor failed for {:?}: {}", source, e),
                    }
                }
            }
            Ok(all_results)
        }
    }

    pub fn get_executor(&self, source: &SourceType) -> Option<&Box<dyn RetrievalExecutor>> {
        self.executors.get(source)
    }
}

impl Default for MultiSourceRetrievalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_rag::config::SourceType;

    #[test]
    fn test_retrieval_result_creation() {
        let result = RetrievalResult {
            id: "test-1".to_string(),
            content: "Test content".to_string(),
            source: SourceType::Memory,
            relevance_score: 0.95,
            metadata: HashMap::new(),
        };

        assert_eq!(result.id, "test-1");
        assert_eq!(result.source, SourceType::Memory);
    }

    #[test]
    fn test_multi_source_executor_new() {
        let executor = MultiSourceRetrievalExecutor::new();
        assert!(executor.get_executor(&SourceType::Memory).is_none());
    }

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_results_per_source, 10);
        assert!(config.enable_parallel);
    }
}
