//! 服务工厂模块
//!
//! 集中管理所有服务的创建逻辑，将 Gateway 从工厂职责中解放

use std::sync::Arc;
use async_trait::async_trait;

use openclaw_core::Result;
use openclaw_ai::AIProvider;
use openclaw_memory::MemoryManager;
use openclaw_security::pipeline::SecurityPipeline;
use openclaw_tools::SkillRegistry;

#[async_trait]
pub trait ServiceFactory: Send + Sync {
    async fn create_ai_provider(&self) -> Result<Arc<dyn AIProvider>>;
    async fn create_memory_manager(&self) -> Result<Arc<MemoryManager>>;
    fn create_security_pipeline(&self) -> Arc<SecurityPipeline>;
    fn create_tool_executor(&self) -> Arc<SkillRegistry>;
    async fn create_voice_providers(&self) -> Result<(Arc<dyn openclaw_voice::SpeechToText>, Arc<dyn openclaw_voice::TextToSpeech>)>;
}

/// 默认服务工厂实现
pub struct DefaultServiceFactory {
    config: Arc<super::config_adapter::ConfigAdapter>,
    vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
}

impl DefaultServiceFactory {
    pub fn new(
        config: Arc<super::config_adapter::ConfigAdapter>,
        vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
    ) -> Self {
        Self {
            config,
            vector_store_registry,
        }
    }
}

#[async_trait]
impl ServiceFactory for DefaultServiceFactory {
    async fn create_ai_provider(&self) -> Result<Arc<dyn AIProvider>> {
        use openclaw_ai::providers::{ProviderConfig, ProviderFactory, ProviderType};
        
        let core_config = self.config.ai_provider();
        
        let ai_config = ProviderConfig {
            name: core_config.name.clone(),
            api_key: core_config.api_key.clone(),
            base_url: core_config.base_url.clone(),
            default_model: core_config.default_model.clone(),
            timeout: None,
            headers: std::collections::HashMap::new(),
            organization: None,
        };
        
        let provider_type = match core_config.name.as_str() {
            "openai" => ProviderType::OpenAI,
            "anthropic" => ProviderType::Anthropic,
            "google" | "gemini" => ProviderType::Gemini,
            "deepseek" => ProviderType::DeepSeek,
            "ollama" => ProviderType::Ollama,
            _ => ProviderType::OpenAI,
        };
        
        let provider = ProviderFactory::create(provider_type, ai_config)
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e))?;
        Ok(provider)
    }
    
    async fn create_memory_manager(&self) -> Result<Arc<MemoryManager>> {
        use openclaw_memory::ai_adapter::AIProviderEmbeddingAdapter;
        use openclaw_memory::hybrid_search::{HybridSearchConfig, HybridSearchManager};
        use openclaw_memory::types::*;
        
        let ai_provider = self.create_ai_provider().await?;
        let memory_config = self.config.memory();
        
        let embedding_provider = AIProviderEmbeddingAdapter::new(
            ai_provider.clone(),
            memory_config.long_term.embedding_model.clone(),
            memory_config.long_term.embedding_dimensions,
        );
        
        let vector_store = self.vector_store_registry
            .create(&memory_config.long_term.backend)
            .await
            .unwrap_or_else(|| {
                Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn openclaw_vector::VectorStore>
            });
        
        let hybrid_config = HybridSearchConfig {
            vector_weight: 0.5,
            keyword_weight: 0.3,
            bm25_weight: 0.2,
            knowledge_graph_weight: 0.1,
            min_score: Some(0.0),
            limit: 10,
            embedding_dimension: Some(memory_config.long_term.embedding_dimensions),
            enable_vector: true,
            enable_bm25: memory_config.long_term.enable_bm25,
            enable_knowledge_graph: memory_config.long_term.enable_knowledge_graph,
        };
        
        let mut hybrid_search = HybridSearchManager::new(vector_store.clone(), hybrid_config.clone());
        
        if hybrid_config.enable_bm25 {
            if let Ok(bm25_index) = openclaw_memory::bm25::Bm25Index::new(std::path::Path::new("data/bm25")) {
                hybrid_search = hybrid_search.with_bm25(Arc::new(bm25_index));
            }
        }
        
        if hybrid_config.enable_knowledge_graph {
            let kg = openclaw_memory::knowledge_graph::KnowledgeGraph::new();
            hybrid_search = hybrid_search.with_knowledge_graph(Arc::new(tokio::sync::RwLock::new(kg)));
        }
        
        let manager = MemoryManager::new(memory_config)
            .with_vector_store(vector_store)
            .with_embedding_provider(embedding_provider)
            .with_hybrid_search(Arc::new(hybrid_search));
        
        Ok(Arc::new(manager))
    }
    
    fn create_security_pipeline(&self) -> Arc<SecurityPipeline> {
        use openclaw_security::pipeline::PipelineConfig;
        
        let config = self.config.security();
        Arc::new(SecurityPipeline::new(config))
    }
    
    fn create_tool_executor(&self) -> Arc<SkillRegistry> {
        Arc::new(SkillRegistry::new())
    }
    
    async fn create_voice_providers(&self) -> Result<(Arc<dyn openclaw_voice::SpeechToText>, Arc<dyn openclaw_voice::TextToSpeech>)> {
        use openclaw_voice::{create_stt, create_tts, SttConfig, TtsConfig, SttProvider, TtsProvider};
        
        let voice_config = self.config.voice();
        
        let stt_provider = match voice_config.stt_provider.as_str() {
            "openai" => SttProvider::OpenAI,
            "google" => SttProvider::Google,
            "local_whisper" => SttProvider::LocalWhisper,
            "azure" => SttProvider::Azure,
            _ => SttProvider::OpenAI,
        };
        
        let tts_provider = match voice_config.tts_provider.as_str() {
            "openai" => TtsProvider::OpenAI,
            "google" => TtsProvider::Google,
            "elevenlabs" => TtsProvider::ElevenLabs,
            "azure" => TtsProvider::Azure,
            "edge" => TtsProvider::Edge,
            _ => TtsProvider::OpenAI,
        };
        
        let mut stt_config = SttConfig::default();
        stt_config.openai_api_key = voice_config.api_key.clone();
        
        let mut tts_config = TtsConfig::default();
        tts_config.openai_api_key = voice_config.api_key.clone();
        
        let stt: Arc<dyn openclaw_voice::SpeechToText> = create_stt(stt_provider, stt_config).into();
        let tts: Arc<dyn openclaw_voice::TextToSpeech> = create_tts(tts_provider, tts_config).into();
        
        Ok((stt, tts))
    }
}
