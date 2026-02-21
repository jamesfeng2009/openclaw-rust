//! 网关服务

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use openclaw_ai::providers::{ProviderFactory, ProviderType};
use openclaw_ai::AIProvider;
use openclaw_memory::MemoryManager;
use openclaw_security::pipeline::SecurityPipeline;
use openclaw_core::Config;

use crate::api::create_router;
use crate::device_manager::DeviceManager;
use crate::orchestrator::{OrchestratorConfig, ServiceOrchestrator};
use crate::voice_service::VoiceService;
use crate::memory_service::MemoryService;
use crate::websocket::websocket_router;
use crate::vector_store_registry::VectorStoreRegistry;

pub struct Gateway {
    config: Config,
    orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    device_manager: Arc<DeviceManager>,
    voice_service: Arc<VoiceService>,
    memory_service: Arc<MemoryService>,
    vector_store_registry: Arc<VectorStoreRegistry>,
}

impl Gateway {
    pub fn new(config: Config) -> Self {
        let orchestrator_config = OrchestratorConfig {
            enable_agents: config.server.enable_agents,
            enable_channels: config.server.enable_channels,
            enable_voice: config.server.enable_voice,
            enable_canvas: config.server.enable_canvas,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: std::collections::HashMap::new(),
            agent_to_canvas_map: std::collections::HashMap::new(),
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

        let device_manager = Arc::new(DeviceManager::new(config.clone()));

        let voice_service = Arc::new(VoiceService::new());

        let memory_service = Arc::new(MemoryService::new());

        let vector_store_registry = Arc::new(VectorStoreRegistry::new());

        Self {
            config,
            orchestrator,
            device_manager,
            voice_service,
            memory_service,
            vector_store_registry,
        }
    }

    /// 启动服务
    pub async fn start(&self) -> openclaw_core::Result<()> {
        self.vector_store_registry.register_defaults().await;

        self.device_manager.init().await?;

        if let Some(ref orchestrator) = *self.orchestrator.read().await {
            orchestrator.start().await?;

            if !self.config.agents.list.is_empty() {
                orchestrator.init_agents_from_config(&self.config).await?;
            }

            let (ai_provider, memory_manager) = self.inject_dependencies_to_agents(orchestrator).await?;

            if self.config.server.enable_agents {
                let inner_memory = memory_manager.read().await.clone();
                self.init_memory_service(inner_memory).await?;
            }
        }

        if self.config.server.enable_voice {
            self.init_voice_service().await?;
        }

        let canvas_manager = if let Some(ref orchestrator) = *self.orchestrator.read().await {
            Some(orchestrator.canvas_manager())
        } else {
            None
        };

        let mut app = Router::new()
            .merge(create_router(self.orchestrator.clone(), self.voice_service.clone(), canvas_manager))
            .merge(websocket_router())
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
            .layer(TraceLayer::new_for_http());

        let addr: SocketAddr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse()
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("Invalid address: {}", e)))?;

        tracing::info!("OpenClaw Gateway starting on {}", addr);
        tracing::info!(
            "Services enabled: agents={}, channels={}, voice={}, devices={}",
            self.config.server.enable_agents,
            self.config.server.enable_channels,
            self.config.server.enable_voice,
            self.config.devices.enabled
        );

        if self.config.devices.enabled {
            tracing::info!(
                "Custom devices configured: {}",
                self.config.devices.custom_devices.len()
            );
            tracing::info!("Plugins configured: {}", self.config.devices.plugins.len());
        }

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("绑定地址失败: {}", e)))?;

        axum::serve(listener, app)
            .await
            .map_err(|e| openclaw_core::OpenClawError::Unknown(e.to_string()))?;

        Ok(())
    }

    async fn inject_dependencies_to_agents(
        &self,
        orchestrator: &ServiceOrchestrator,
    ) -> openclaw_core::Result<(Arc<dyn AIProvider>, Arc<tokio::sync::RwLock<Arc<MemoryManager>>>)> {
        let ai_provider = self.create_ai_provider().await?;
        tracing::info!("AI Provider created: {}", self.config.ai.default_provider);

        let memory_manager = self.create_memory_manager_with_embedding().await?;
        tracing::info!("Memory Manager created with embedding provider");

        let security_pipeline = self.create_security_pipeline();
        tracing::info!("Security Pipeline created");

        let tool_executor = self.create_tool_executor();
        tracing::info!("Tool Executor created");

        let memory_lock: Arc<tokio::sync::RwLock<Arc<MemoryManager>>> = Arc::new(tokio::sync::RwLock::new(memory_manager));
        let memory_for_orchestrator = Some(memory_lock.read().await.clone());
        orchestrator
            .inject_dependencies_with_tools(ai_provider.clone(), memory_for_orchestrator, security_pipeline, tool_executor)
            .await;

        tracing::info!("Dependencies injected to all agents");
        
        Ok((ai_provider, memory_lock))
    }

    fn create_tool_executor(&self) -> Arc<openclaw_tools::SkillRegistry> {
        let registry = openclaw_tools::SkillRegistry::new();
        Arc::new(registry)
    }

    async fn create_ai_provider(&self) -> openclaw_core::Result<Arc<dyn AIProvider>> {
        let provider_type = ProviderType::from_str(&self.config.ai.default_provider)
            .ok_or_else(|| {
                openclaw_core::OpenClawError::Config(format!(
                    "Unknown AI provider: {}",
                    self.config.ai.default_provider
                ))
            })?;

        let provider_config = self
            .config
            .ai
            .providers
            .iter()
            .find(|p| p.name == self.config.ai.default_provider);

        let (api_key, base_url) = if let Some(config) = provider_config {
            let api_key = config.api_key.clone().ok_or_else(|| {
                openclaw_core::OpenClawError::Config(format!(
                    "No API key provided for provider: {}",
                    self.config.ai.default_provider
                ))
            })?;
            (api_key, config.base_url.clone())
        } else {
            return Err(openclaw_core::OpenClawError::Config(format!(
                "No configuration found for provider: {}",
                self.config.ai.default_provider
            )));
        };

        let mut cfg = openclaw_ai::providers::ProviderConfig::new(
            &self.config.ai.default_provider,
            api_key,
        );
        if let Some(url) = base_url {
            cfg = cfg.with_base_url(url);
        }

        ProviderFactory::create(provider_type, cfg)
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("AI provider error: {}", e)))
    }

    async fn create_memory_manager(&self) -> openclaw_core::Result<Arc<MemoryManager>> {
        use openclaw_memory::types::{
            MemoryConfig as MemoryConfigType,
            WorkingMemoryConfig as WorkingMemoryConfigType,
            ShortTermMemoryConfig as ShortTermMemoryConfigType,
            LongTermMemoryConfig as LongTermMemoryConfigType,
            CustomEmbeddingConfig as CustomEmbeddingConfigType,
        };
        
        let custom_embedding: Option<CustomEmbeddingConfigType> = self.config.memory.long_term
            .custom_embedding
            .as_ref()
            .map(|c| CustomEmbeddingConfigType {
                base_url: c.base_url.clone(),
                api_key: c.api_key.clone(),
                model: c.model.clone(),
            });

        let config = MemoryConfigType {
            working: WorkingMemoryConfigType {
                max_messages: self.config.memory.working.max_messages,
                max_tokens: self.config.memory.working.max_tokens,
            },
            short_term: ShortTermMemoryConfigType {
                compress_after: self.config.memory.short_term.compress_after,
                max_summaries: self.config.memory.short_term.max_summaries,
            },
            long_term: LongTermMemoryConfigType {
                enabled: self.config.memory.long_term.enabled,
                backend: self.config.memory.long_term.backend.clone(),
                collection: self.config.memory.long_term.collection.clone(),
                embedding_provider: self.config.memory.long_term.embedding_provider.clone(),
                embedding_model: self.config.memory.long_term.embedding_model.clone(),
                embedding_dimensions: self.config.memory.long_term.embedding_dimensions,
                chunk_size: self.config.memory.long_term.chunk_size,
                overlap: self.config.memory.long_term.overlap,
                enable_bm25: self.config.memory.long_term.enable_bm25,
                enable_knowledge_graph: self.config.memory.long_term.enable_knowledge_graph,
                custom_embedding,
            },
        };

        let vector_store = self.vector_store_registry
            .create(&self.config.memory.long_term.backend)
            .await
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Vector store backend '{}' not found, falling back to memory store",
                    self.config.memory.long_term.backend
                );
                Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn openclaw_vector::VectorStore>
            });

        let manager = MemoryManager::new(config)
            .with_vector_store(vector_store);

        Ok(Arc::new(manager))
    }

    async fn create_memory_manager_with_embedding(&self) -> openclaw_core::Result<Arc<MemoryManager>> {
        use openclaw_memory::ai_adapter::AIProviderEmbeddingAdapter;
        use openclaw_memory::hybrid_search::{HybridSearchConfig, HybridSearchManager};
        use openclaw_memory::types::{
            MemoryConfig as MemoryConfigType,
            WorkingMemoryConfig as WorkingMemoryConfigType,
            ShortTermMemoryConfig as ShortTermMemoryConfigType,
            LongTermMemoryConfig as LongTermMemoryConfigType,
            CustomEmbeddingConfig as CustomEmbeddingConfigType,
        };

        let ai_provider = self.create_ai_provider().await?;
        
        let embedding_provider = AIProviderEmbeddingAdapter::new(
            ai_provider.clone(),
            self.config.memory.long_term.embedding_model.clone(),
            1536,
        );

        let vector_store = self.vector_store_registry
            .create(&self.config.memory.long_term.backend)
            .await
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Vector store backend '{}' not found, falling back to memory store",
                    self.config.memory.long_term.backend
                );
                Arc::new(openclaw_vector::MemoryStore::new()) as Arc<dyn openclaw_vector::VectorStore>
            });

        let custom_embedding: Option<CustomEmbeddingConfigType> = self.config.memory.long_term
            .custom_embedding
            .as_ref()
            .map(|c| CustomEmbeddingConfigType {
                base_url: c.base_url.clone(),
                api_key: c.api_key.clone(),
                model: c.model.clone(),
            });

        let config = MemoryConfigType {
            working: WorkingMemoryConfigType {
                max_messages: self.config.memory.working.max_messages,
                max_tokens: self.config.memory.working.max_tokens,
            },
            short_term: ShortTermMemoryConfigType {
                compress_after: self.config.memory.short_term.compress_after,
                max_summaries: self.config.memory.short_term.max_summaries,
            },
            long_term: LongTermMemoryConfigType {
                enabled: self.config.memory.long_term.enabled,
                backend: self.config.memory.long_term.backend.clone(),
                collection: self.config.memory.long_term.collection.clone(),
                embedding_provider: self.config.memory.long_term.embedding_provider.clone(),
                embedding_model: self.config.memory.long_term.embedding_model.clone(),
                embedding_dimensions: self.config.memory.long_term.embedding_dimensions,
                chunk_size: self.config.memory.long_term.chunk_size,
                overlap: self.config.memory.long_term.overlap,
                enable_bm25: self.config.memory.long_term.enable_bm25,
                enable_knowledge_graph: self.config.memory.long_term.enable_knowledge_graph,
                custom_embedding,
            },
        };

        let hybrid_config = HybridSearchConfig {
            vector_weight: 0.5,
            keyword_weight: 0.3,
            bm25_weight: 0.2,
            knowledge_graph_weight: 0.1,
            min_score: Some(0.0),
            limit: 10,
            embedding_dimension: Some(1536),
            enable_vector: true,
            enable_bm25: false,
            enable_knowledge_graph: false,
        };

        let mut hybrid_search = HybridSearchManager::new(vector_store.clone(), hybrid_config.clone());

        if hybrid_config.enable_bm25 {
            if let Ok(bm25_index) = openclaw_memory::bm25::Bm25Index::new(std::path::Path::new("data/bm25")) {
                hybrid_search = hybrid_search.with_bm25(Arc::new(bm25_index));
            } else {
                tracing::warn!("Failed to create BM25 index, skipping");
            }
        }

        if hybrid_config.enable_knowledge_graph {
            let kg = openclaw_memory::knowledge_graph::KnowledgeGraph::new();
            hybrid_search = hybrid_search.with_knowledge_graph(Arc::new(RwLock::new(kg)));
        }

        let manager = MemoryManager::new(config)
            .with_vector_store(vector_store)
            .with_embedding_provider(embedding_provider)
            .with_hybrid_search(Arc::new(hybrid_search));

        Ok(Arc::new(manager))
    }

    fn create_security_pipeline(&self) -> Arc<SecurityPipeline> {
        use openclaw_security::pipeline::PipelineConfig;
        
        let config = PipelineConfig {
            enable_input_filter: self.config.security.enable_input_filter,
            enable_classifier: self.config.security.enable_classifier,
            enable_output_validation: self.config.security.enable_output_validation,
            enable_audit: self.config.security.enable_audit,
            enable_self_healer: self.config.security.enable_self_healer,
            classifier_strict_mode: self.config.security.classifier_strict_mode,
            stuck_timeout: self.config.security.stuck_timeout,
        };
        Arc::new(SecurityPipeline::new(config))
    }

    pub async fn get_orchestrator(&self) -> Option<Arc<RwLock<Option<ServiceOrchestrator>>>> {
        if self.config.server.enable_agents
            || self.config.server.enable_channels
            || self.config.server.enable_canvas
        {
            Some(self.orchestrator.clone())
        } else {
            None
        }
    }

    async fn init_voice_service(&self) -> openclaw_core::Result<()> {
        use openclaw_voice::{create_stt, create_tts, SttConfig, TtsConfig, SttProvider, TtsProvider};
        
        let voice_config = self.config.voice.clone().unwrap_or_default();
        
        let stt_provider = match voice_config.stt_provider.as_str() {
            "openai" => SttProvider::OpenAI,
            "local" => SttProvider::LocalWhisper,
            "azure" => SttProvider::Azure,
            "google" => SttProvider::Google,
            _ => SttProvider::OpenAI,
        };
        
        let tts_provider = match voice_config.tts_provider.as_str() {
            "openai" => TtsProvider::OpenAI,
            "edge" => TtsProvider::Edge,
            "azure" => TtsProvider::Azure,
            "google" => TtsProvider::Google,
            "elevenlabs" => TtsProvider::ElevenLabs,
            _ => TtsProvider::OpenAI,
        };
        
        let mut stt_config = SttConfig::default();
        stt_config.openai_api_key = voice_config.api_key.clone();
        
        let mut tts_config = TtsConfig::default();
        tts_config.openai_api_key = voice_config.api_key.clone();
        
        let stt = create_stt(stt_provider, stt_config);
        let tts = create_tts(tts_provider, tts_config);
        
        let stt: Arc<dyn openclaw_voice::SpeechToText> = stt.into();
        let tts: Arc<dyn openclaw_voice::TextToSpeech> = tts.into();

        self.voice_service.init_voice(stt, tts).await;

        tracing::info!("Voice service initialized with STT: {}, TTS: {}", voice_config.stt_provider, voice_config.tts_provider);
        Ok(())
    }

    async fn init_memory_service(&self, memory_manager: Arc<MemoryManager>) -> openclaw_core::Result<()> {
        let vector_store = memory_manager.get_vector_store()
            .ok_or_else(|| openclaw_core::OpenClawError::Config("MemoryManager has no VectorStore configured".to_string()))?;

        self.memory_service.init(memory_manager, vector_store).await;

        tracing::info!("Memory service initialized (reusing agent's MemoryManager and VectorStore)");
        Ok(())
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_core::Config;

    #[test]
    fn test_gateway_default() {
        let gateway = Gateway::default();
        assert!(!gateway.config.server.enable_agents);
    }

    #[test]
    fn test_gateway_with_agents_enabled() {
        let mut config = Config::default();
        config.server.enable_agents = true;
        
        let gateway = Gateway::new(config);
        assert!(gateway.config.server.enable_agents);
    }

    #[tokio::test]
    async fn test_gateway_orchestrator_initialization() {
        let mut config = Config::default();
        config.server.enable_agents = true;
        
        let gateway = Gateway::new(config);
        let orchestrator = gateway.orchestrator.read().await;
        assert!(orchestrator.is_some());
    }
}
