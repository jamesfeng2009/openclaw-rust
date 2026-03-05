//! 记忆管理工厂实现 - 符合开闭原则

use async_trait::async_trait;
use openclaw_ai::AIProvider;
use openclaw_core::Result;
use openclaw_vector::VectorStore;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::ai_adapter::AIProviderEmbeddingAdapter;
use crate::bm25::Bm25Index;
use crate::compress_adapter::AIProviderCompressAdapter;
use crate::hybrid_search::{HybridSearchConfig, HybridSearchManager};
use crate::knowledge_graph::KnowledgeGraph;
use crate::manager::MemoryManager;
use crate::recall::RecallResult;
use crate::types::{MemoryConfig, MemoryItem, MemoryRetrieval};

#[async_trait]
pub trait MemoryBackend: Send + Sync {
    async fn store(&self, memory: MemoryItem) -> Result<()>;
    async fn recall(&self, query: &str) -> Result<RecallResult>;
    async fn add(&self, message: openclaw_core::Message) -> Result<()>;
    async fn retrieve(&self, query: &str, limit: usize) -> Result<MemoryRetrieval>;
}

pub struct HybridMemoryBackend {
    manager: Arc<Mutex<MemoryManager>>,
}

impl HybridMemoryBackend {
    pub fn new(manager: MemoryManager) -> Self {
        Self {
            manager: Arc::new(Mutex::new(manager)),
        }
    }
}

#[async_trait]
impl MemoryBackend for HybridMemoryBackend {
    async fn store(&self, memory: MemoryItem) -> Result<()> {
        let content = memory.content.to_text();
        let msg = openclaw_core::Message::user(content);
        let mut manager = self.manager.lock().await;
        manager.add(msg).await
    }

    async fn recall(&self, query: &str) -> Result<RecallResult> {
        let manager = self.manager.lock().await;
        manager.recall(query).await
    }

    async fn add(&self, message: openclaw_core::Message) -> Result<()> {
        let mut manager = self.manager.lock().await;
        manager.add(message).await
    }

    async fn retrieve(&self, query: &str, limit: usize) -> Result<MemoryRetrieval> {
        let manager = self.manager.lock().await;
        manager.retrieve(query, limit).await
    }
}

#[async_trait]
pub trait MemoryManagerFactory: Send + Sync {
    fn name(&self) -> &str;
    async fn create(
        &self,
        config: &MemoryConfig,
        ai_provider: Arc<dyn AIProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Result<Arc<dyn MemoryBackend>>;
}

pub struct HybridMemoryFactory;

impl HybridMemoryFactory {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(
        &self,
        config: &MemoryConfig,
        ai_provider: Arc<dyn AIProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Result<Arc<dyn MemoryBackend>> {
        let ai_provider_clone = ai_provider.clone();
        let embedding_provider = AIProviderEmbeddingAdapter::new(
            ai_provider,
            config.long_term.embedding_model.clone(),
            config.long_term.embedding_dimensions,
        );

        let hybrid_config = HybridSearchConfig {
            vector_weight: 0.5,
            keyword_weight: 0.3,
            bm25_weight: 0.2,
            knowledge_graph_weight: 0.1,
            min_score: Some(0.0),
            limit: 10,
            embedding_dimension: Some(config.long_term.embedding_dimensions),
            enable_vector: true,
            enable_bm25: config.long_term.enable_bm25,
            enable_knowledge_graph: config.long_term.enable_knowledge_graph,
        };

        let mut hybrid_search = HybridSearchManager::new(vector_store.clone(), hybrid_config.clone());

        if hybrid_config.enable_bm25 {
            if let Ok(bm25_index) = Bm25Index::new(std::path::Path::new("data/bm25")) {
                hybrid_search = hybrid_search.with_bm25(Arc::new(bm25_index));
            }
        }

        if hybrid_config.enable_knowledge_graph {
            let kg = KnowledgeGraph::new();
            hybrid_search = hybrid_search.with_knowledge_graph(Arc::new(tokio::sync::RwLock::new(kg)));
        }

        let manager = if config.short_term.compression_mode == "ai" {
            let summary_model = config
                .short_term
                .summary_model
                .clone()
                .unwrap_or_else(|| "gpt-4o-mini".to_string());
            let compress_adapter = AIProviderCompressAdapter::new(ai_provider_clone, summary_model);
            MemoryManager::new(config.clone())
                .with_vector_store(vector_store)
                .with_embedding_provider(embedding_provider)
                .with_hybrid_search(Arc::new(hybrid_search))
                .with_ai_compressor(Arc::new(compress_adapter))
        } else {
            MemoryManager::new(config.clone())
                .with_vector_store(vector_store)
                .with_embedding_provider(embedding_provider)
                .with_hybrid_search(Arc::new(hybrid_search))
        };

        Ok(Arc::new(HybridMemoryBackend::new(manager)) as Arc<dyn MemoryBackend>)
    }
}

impl Default for HybridMemoryFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryManagerFactory for HybridMemoryFactory {
    fn name(&self) -> &str {
        "hybrid"
    }

    async fn create(
        &self,
        config: &MemoryConfig,
        ai_provider: Arc<dyn AIProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Result<Arc<dyn MemoryBackend>> {
        self.create(config, ai_provider, vector_store).await
    }
}

pub async fn create_memory_backend(
    backend_type: &str,
    config: &MemoryConfig,
    ai_provider: Arc<dyn AIProvider>,
    vector_store: Arc<dyn VectorStore>,
) -> Result<Arc<dyn MemoryBackend>> {
    match backend_type {
        "hybrid" | "default" => {
            let factory = HybridMemoryFactory::new();
            factory.create(config, ai_provider, vector_store).await
        }
        _ => {
            let factory = HybridMemoryFactory::new();
            factory.create(config, ai_provider, vector_store).await
        }
    }
}
