//! 服务工厂模块
//!
//! 集中管理所有服务的创建逻辑，将 Gateway 从工厂职责中解放

use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use openclaw_core::{Config, Result};
use openclaw_ai::AIProvider;
use openclaw_device::UnifiedDeviceManager;
use openclaw_memory::MemoryManager;
use openclaw_security::pipeline::SecurityPipeline;
use openclaw_tools::ToolRegistry;

use crate::app_context::AppContext;
use crate::orchestrator::OrchestratorConfig;
use crate::orchestrator::ServiceOrchestrator;
use crate::voice_service::VoiceService;

#[async_trait]
pub trait ServiceFactory: Send + Sync {
    async fn create_ai_provider(&self) -> Result<Arc<dyn AIProvider>>;
    async fn create_memory_manager(&self) -> Result<Arc<MemoryManager>>;
    fn create_security_pipeline(&self) -> Arc<SecurityPipeline>;
    fn create_tool_registry(&self) -> Arc<ToolRegistry>;
    async fn create_voice_providers(&self) -> Result<(Arc<dyn openclaw_voice::SpeechToText>, Arc<dyn openclaw_voice::TextToSpeech>)>;
    async fn create_app_context(&self, config: Config) -> Result<Arc<AppContext>>;
    async fn create_agentic_rag_engine(
        &self,
        ai_provider: Arc<dyn AIProvider>,
        memory_manager: Option<Arc<MemoryManager>>,
    ) -> Result<Arc<crate::agentic_rag::AgenticRAGEngine>>;
}

/// 默认服务工厂实现
pub struct DefaultServiceFactory {
    config: Arc<super::config_adapter::ConfigAdapter>,
    vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
    device_manager: Arc<super::device_manager::DeviceManager>,
    unified_device_manager: Arc<UnifiedDeviceManager>,
}

impl DefaultServiceFactory {
    pub fn new(
        config: Arc<super::config_adapter::ConfigAdapter>,
        vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
        device_manager: Arc<super::device_manager::DeviceManager>,
    ) -> Self {
        let registry = device_manager.registry().clone();
        let unified_device_manager = Arc::new(UnifiedDeviceManager::new(registry));
        
        Self {
            config,
            vector_store_registry,
            device_manager,
            unified_device_manager,
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
            .map_err(openclaw_core::OpenClawError::AIProvider)?;
        Ok(provider)
    }
    
    async fn create_memory_manager(&self) -> Result<Arc<MemoryManager>> {
        use openclaw_memory::ai_adapter::AIProviderEmbeddingAdapter;
        use openclaw_memory::hybrid_search::{HybridSearchConfig, HybridSearchManager};
        
        
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
        
        if hybrid_config.enable_bm25
            && let Ok(bm25_index) = openclaw_memory::bm25::Bm25Index::new(std::path::Path::new("data/bm25")) {
                hybrid_search = hybrid_search.with_bm25(Arc::new(bm25_index));
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
        
        
        let config = self.config.security();
        Arc::new(SecurityPipeline::new(config))
    }
    
    fn create_tool_registry(&self) -> Arc<ToolRegistry> {
        use crate::hardware_tools::CameraTool;
        
        let mut registry = ToolRegistry::new();
        
        let capabilities = self.device_manager.get_capabilities();
        
        if capabilities.sensors.contains(&openclaw_device::SensorType::Camera) {
            let camera_manager = Arc::new(openclaw_device::CameraManager::new());
            let camera_tool = Arc::new(CameraTool::new(Some(camera_manager), capabilities.clone()));
            registry.register("hardware_camera".to_string(), camera_tool);
            tracing::info!("Hardware camera tool registered");
        }
        
        if capabilities.sensors.contains(&openclaw_device::SensorType::Microphone) {
            tracing::info!("Microphone available - microphone tool can be added");
        }
        
        tracing::info!("Tool registry created with hardware tools based on device capabilities");
        
        Arc::new(registry)
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
    
    async fn create_app_context(&self, config: Config) -> Result<Arc<AppContext>> {
        let memory_config = self.config.memory();
        
        let orchestrator_config = OrchestratorConfig {
            enable_agents: config.server.enable_agents,
            enable_channels: config.server.enable_channels,
            enable_voice: config.server.enable_voice,
            enable_canvas: config.server.enable_canvas,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: std::collections::HashMap::new(),
            agent_to_canvas_map: std::collections::HashMap::new(),
            #[cfg(feature = "per_session_memory")]
            enable_per_session_memory: false,
            #[cfg(feature = "per_session_memory")]
            memory_config: Some(memory_config),
            #[cfg(feature = "per_session_memory")]
            max_session_memories: 100,
        };

        let orchestrator = Arc::new(RwLock::new(
            if config.server.enable_agents
                || config.server.enable_channels
                || config.server.enable_canvas
            {
                Some(ServiceOrchestrator::new(orchestrator_config))
            } else {
                None
            },
        ));

        let ai_provider = self.create_ai_provider().await?;
        let memory_manager = Some(self.create_memory_manager().await?);
        let security_pipeline = self.create_security_pipeline();
        let tool_registry = self.create_tool_registry();
        let voice_service = Arc::new(VoiceService::new());

        let context = AppContext::new(
            config,
            ai_provider,
            memory_manager,
            security_pipeline,
            tool_registry,
            orchestrator,
            self.device_manager.clone(),
            self.unified_device_manager.clone(),
            voice_service,
            self.vector_store_registry.clone(),
        );

        Ok(Arc::new(context))
    }

    async fn create_agentic_rag_engine(
        &self,
        ai_provider: Arc<dyn openclaw_ai::AIProvider>,
        memory_manager: Option<Arc<MemoryManager>>,
    ) -> Result<Arc<crate::agentic_rag::AgenticRAGEngine>> {
        use crate::agentic_rag::{AgenticRAGConfig, AgenticRAGEngine};

        let config = AgenticRAGConfig::default();
        
        let engine = AgenticRAGEngine::new(
            config,
            ai_provider,
            memory_manager,
            None,
            None,
        ).await?;

        Ok(Arc::new(engine))
    }
}
