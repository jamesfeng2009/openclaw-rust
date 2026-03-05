//! 检索执行器

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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

pub struct QueryCache {
    cache: Arc<tokio::sync::RwLock<LruCache<String, (Vec<RetrievalResult>, Instant)>>>,
    ttl: Duration,
}

impl QueryCache {
    pub fn new(capacity: usize, ttl_seconds: u64) -> Self {
        let cache = LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap_or(std::num::NonZeroUsize::new(100).unwrap()));
        Self {
            cache: Arc::new(tokio::sync::RwLock::new(cache)),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub async fn get(&self, query: &str) -> Option<Vec<RetrievalResult>> {
        let query_str = query.to_string();
        let mut cache = self.cache.write().await;
        if let Some((results, timestamp)) = cache.get(&query_str) {
            if timestamp.elapsed() < self.ttl {
                return Some(results.clone());
            }
        }
        None
    }

    pub async fn put(&self, query: String, results: Vec<RetrievalResult>) {
        let mut cache = self.cache.write().await;
        cache.put(query, (results, Instant::now()));
    }

    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
}

#[async_trait]
pub trait RetrievalExecutor: Send + Sync {
    async fn execute(&self, query: &str, config: &ExecutorConfig) -> Result<Vec<RetrievalResult>>;

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
    async fn execute(&self, query: &str, _config: &ExecutorConfig) -> Result<Vec<RetrievalResult>> {
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

pub struct WebRetrievalExecutor {
    client: reqwest::Client,
    source_type: SourceType,
    search_api_url: Option<String>,
    api_key: Option<String>,
}

impl WebRetrievalExecutor {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
            source_type: SourceType::Web,
            search_api_url: None,
            api_key: None,
        }
    }

    pub fn with_search_api(mut self, url: String, api_key: String) -> Self {
        self.search_api_url = Some(url);
        self.api_key = Some(api_key);
        self
    }
}

impl Default for WebRetrievalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RetrievalExecutor for WebRetrievalExecutor {
    async fn execute(&self, query: &str, config: &ExecutorConfig) -> Result<Vec<RetrievalResult>> {
        if let (Some(url), Some(key)) = (&self.search_api_url, &self.api_key) {
            let results = self.search_with_api(url, key, query, config.max_results_per_source).await?;
            return Ok(results);
        }

        Ok(vec![RetrievalResult {
            id: "web-placeholder".to_string(),
            content: format!("Web search for: {}", query),
            source: SourceType::Web,
            relevance_score: 0.5,
            metadata: {
                let mut m = HashMap::new();
                m.insert("note".to_string(), "Configure search_api_url and api_key for actual search".to_string());
                m
            },
        }])
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

impl WebRetrievalExecutor {
    async fn search_with_api(
        &self,
        url: &str,
        api_key: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<RetrievalResult>> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .query(&[("q", query), ("limit", &limit.to_string())])
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Network(e.to_string()))?;

        let results: Vec<serde_json::Value> = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Parse(e.to_string()))?;

        Ok(results
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let snippet = item.get("snippet").and_then(|v| v.as_str()).unwrap_or("");
                let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("");

                RetrievalResult {
                    id: format!("web-{}", i),
                    content: format!("{}\n\n{}", title, snippet),
                    source: SourceType::Web,
                    relevance_score: 1.0 - (i as f32 * 0.1),
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("url".to_string(), url.to_string());
                        m
                    },
                }
            })
            .collect())
    }
}

pub struct FileRetrievalExecutor {
    base_path: Option<String>,
    source_type: SourceType,
}

impl FileRetrievalExecutor {
    pub fn new() -> Self {
        Self {
            base_path: None,
            source_type: SourceType::File,
        }
    }

    pub fn with_base_path(mut self, path: String) -> Self {
        self.base_path = Some(path);
        self
    }
}

impl Default for FileRetrievalExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RetrievalExecutor for FileRetrievalExecutor {
    async fn execute(&self, query: &str, config: &ExecutorConfig) -> Result<Vec<RetrievalResult>> {
        let base = self.base_path.as_ref().ok_or_else(|| {
            openclaw_core::OpenClawError::Config("FileRetrievalExecutor: base_path not set".to_string())
        })?;

        let search_query = query.to_lowercase();
        let mut results = Vec::new();

        let mut entries = tokio::fs::read_dir(base).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Ok(file_type) = entry.file_type().await {
                if file_type.is_file() {
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        if content.to_lowercase().contains(&search_query) {
                            results.push(RetrievalResult {
                                id: entry.path().to_string_lossy().to_string(),
                                content: content.chars().take(1000).collect(),
                                source: SourceType::File,
                                relevance_score: 0.8,
                                metadata: {
                                    let mut m = HashMap::new();
                                    m.insert("filename".to_string(), entry.file_name().to_string_lossy().to_string());
                                    m
                                },
                            });
                        }
                    }
                }
            }
        }

        results.truncate(config.max_results_per_source);
        Ok(results)
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
    async fn execute(&self, query: &str, config: &ExecutorConfig) -> Result<Vec<RetrievalResult>> {
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
                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FusionStrategy {
    RRF,
    ScoreWeighted,
    SimpleConcatenation,
}

pub struct HybridFusionExecutor {
    executors: Vec<Box<dyn RetrievalExecutor>>,
    strategy: FusionStrategy,
    weights: HashMap<SourceType, f32>,
}

impl HybridFusionExecutor {
    pub fn new(strategy: FusionStrategy) -> Self {
        Self {
            executors: Vec::new(),
            strategy,
            weights: HashMap::new(),
        }
    }

    pub fn add_executor(mut self, executor: Box<dyn RetrievalExecutor>) -> Self {
        let source_type = executor.source_type();
        let weight = match source_type {
            SourceType::VectorDB => 0.4,
            SourceType::Memory => 0.3,
            SourceType::Web => 0.2,
            SourceType::File => 0.1,
            SourceType::API => 0.0,
        };
        self.weights.insert(source_type, weight);
        self.executors.push(executor);
        self
    }

    pub async fn execute(&self, query: &str, config: &ExecutorConfig) -> Result<Vec<RetrievalResult>> {
        let mut all_results = Vec::new();

        for executor in &self.executors {
            match executor.execute(query, config).await {
                Ok(results) => all_results.extend(results),
                Err(e) => tracing::warn!("Executor failed: {}", e),
            }
        }

        match self.strategy {
            FusionStrategy::RRF => self.rrf_fusion(all_results, config.max_results_per_source),
            FusionStrategy::ScoreWeighted => self.score_weighted_fusion(all_results, config.max_results_per_source),
            FusionStrategy::SimpleConcatenation => {
                all_results.truncate(config.max_results_per_source);
                Ok(all_results)
            }
        }
    }

    fn rrf_fusion(&self, mut results: Vec<RetrievalResult>, limit: usize) -> Result<Vec<RetrievalResult>> {
        let mut rrf_scores: HashMap<String, f32> = HashMap::new();
        let mut content_map: HashMap<String, RetrievalResult> = HashMap::new();

        for result in results {
            let id = result.id.clone();
            let rrf_score = 1.0 / (60.0 + result.relevance_score * 100.0);
            *rrf_scores.entry(id.clone()).or_insert(0.0) += rrf_score;
            content_map.insert(id, result);
        }

        let mut fused: Vec<(String, f32)> = rrf_scores.into_iter().collect();
        fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let final_results: Vec<RetrievalResult> = fused
            .into_iter()
            .take(limit)
            .filter_map(|(id, score)| {
                let mut result = content_map.remove(&id)?;
                result.relevance_score = score;
                Some(result)
            })
            .collect();

        Ok(final_results)
    }

    fn score_weighted_fusion(&self, mut results: Vec<RetrievalResult>, limit: usize) -> Result<Vec<RetrievalResult>> {
        for result in &mut results {
            let weight = self.weights.get(&result.source).copied().unwrap_or(0.5);
            result.relevance_score *= weight;
        }

        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        Ok(results)
    }
}

pub trait Reranker: Send + Sync {
    async fn rerank(&self, query: &str, results: &mut Vec<RetrievalResult>) -> Result<()>;
}

pub struct LlmReranker {
    llm: Arc<dyn openclaw_ai::AIProvider>,
}

impl LlmReranker {
    pub fn new(llm: Arc<dyn openclaw_ai::AIProvider>) -> Self {
        Self { llm }
    }
}

impl Reranker for LlmReranker {
    async fn rerank(&self, query: &str, results: &mut Vec<RetrievalResult>) -> Result<()> {
        if results.is_empty() {
            return Ok(());
        }

        let prompt = Message::system(format!(
            "Given the query: \"{}\"\n\nRank the following results by relevance (1 = most relevant):\n\n{}\n\nProvide rankings as a JSON array of indices in order of relevance.",
            query,
            results
                .iter()
                .enumerate()
                .map(|(i, r)| format!("{}. {}", i + 1, r.content.chars().take(200).collect::<String>()))
                .collect::<Vec<_>>()
                .join("\n")
        ));

        let request = openclaw_ai::ChatRequest::new("default", vec![prompt]);
        let response = self.llm.chat(request).await?;

        let rankings_text = response.message.text_content().unwrap_or("[]");

        if let Ok(indices) = serde_json::from_str::<Vec<usize>>(rankings_text) {
            let mut reranked = Vec::with_capacity(results.len());
            for idx in indices {
                if idx < results.len() {
                    reranked.push(results.remove(idx));
                }
            }
            reranked.append(results);
            *results = reranked;
        }

        Ok(())
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
                    futures.push(async move { executor.execute(&q, &c).await });
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

    #[tokio::test]
    async fn test_query_cache_put_and_get() {
        let cache = QueryCache::new(10, 60);
        
        let results = vec![
            RetrievalResult {
                id: "1".to_string(),
                content: "test".to_string(),
                source: SourceType::Memory,
                relevance_score: 0.9,
                metadata: HashMap::new(),
            }
        ];
        
        cache.put("test query".to_string(), results.clone()).await;
        
        let cached = cache.get("test query").await;
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_query_cache_miss() {
        let cache = QueryCache::new(10, 60);
        
        let cached = cache.get("nonexistent").await;
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn test_query_cache_clear() {
        let cache = QueryCache::new(10, 60);
        
        let results = vec![
            RetrievalResult {
                id: "1".to_string(),
                content: "test".to_string(),
                source: SourceType::Memory,
                relevance_score: 0.9,
                metadata: HashMap::new(),
            }
        ];
        
        cache.put("test query".to_string(), results).await;
        cache.clear().await;
        
        let cached = cache.get("test query").await;
        assert!(cached.is_none());
    }

    #[test]
    fn test_fusion_strategy_variants() {
        assert_eq!(FusionStrategy::RRF, FusionStrategy::RRF);
        assert_eq!(FusionStrategy::ScoreWeighted, FusionStrategy::ScoreWeighted);
        assert_ne!(FusionStrategy::RRF, FusionStrategy::ScoreWeighted);
    }

    #[test]
    fn test_web_retrieval_executor_new() {
        let executor = WebRetrievalExecutor::new();
        assert_eq!(executor.source_type(), SourceType::Web);
    }

    #[test]
    fn test_file_retrieval_executor_new() {
        let executor = FileRetrievalExecutor::new();
        assert_eq!(executor.source_type(), SourceType::File);
    }

    #[test]
    fn test_file_retrieval_executor_with_base_path() {
        let executor = FileRetrievalExecutor::new()
            .with_base_path("/tmp".to_string());
        
        let base = executor.base_path.as_ref();
        assert!(base.is_some());
        assert_eq!(base.unwrap(), "/tmp");
    }
}
