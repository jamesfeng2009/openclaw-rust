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

pub struct Gateway {
    config: Config,
    orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    device_manager: Arc<DeviceManager>,
    voice_service: Arc<VoiceService>,
    memory_service: Arc<MemoryService>,
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

        Self {
            config,
            orchestrator,
            device_manager,
            voice_service,
            memory_service,
        }
    }

    /// 启动服务
    pub async fn start(&self) -> openclaw_core::Result<()> {
        self.device_manager.init().await?;

        if let Some(ref orchestrator) = *self.orchestrator.read().await {
            orchestrator.start().await?;

            if !self.config.agents.list.is_empty() {
                orchestrator.init_agents_from_config(&self.config).await?;
            }

            let (ai_provider, memory_manager) = self.inject_dependencies_to_agents(orchestrator).await?;

            if self.config.server.enable_agents {
                self.init_memory_service(memory_manager).await?;
            }
        }

        if self.config.server.enable_voice {
            self.init_voice_service().await?;
        }

        let mut app = Router::new()
            .merge(create_router(self.orchestrator.clone(), self.voice_service.clone()))
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
    ) -> openclaw_core::Result<(Arc<dyn AIProvider>, Arc<MemoryManager>)> {
        let ai_provider = self.create_ai_provider().await?;
        tracing::info!("AI Provider created: {}", self.config.ai.default_provider);

        let memory_manager = self.create_memory_manager_with_embedding().await?;
        tracing::info!("Memory Manager created with embedding provider");

        let security_pipeline = self.create_security_pipeline();
        tracing::info!("Security Pipeline created");

        orchestrator
            .inject_dependencies(ai_provider.clone(), memory_manager.clone(), security_pipeline)
            .await;

        tracing::info!("Dependencies injected to all agents");
        
        Ok((ai_provider, memory_manager))
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
        let config = openclaw_memory::types::MemoryConfig {
            working: openclaw_memory::types::WorkingMemoryConfig::default(),
            short_term: openclaw_memory::types::ShortTermMemoryConfig::default(),
            long_term: openclaw_memory::types::LongTermMemoryConfig {
                enabled: true,
                backend: "memory".to_string(),
                collection: "openclaw_memories".to_string(),
                embedding_provider: "openai".to_string(),
                embedding_model: "text-embedding-3-small".to_string(),
                custom_embedding: None,
            },
        };

        let vector_store: Arc<dyn openclaw_vector::VectorStore> = 
            Arc::new(openclaw_vector::MemoryStore::new());

        let manager = MemoryManager::new(config)
            .with_vector_store(vector_store);

        Ok(Arc::new(manager))
    }

    async fn create_memory_manager_with_embedding(&self) -> openclaw_core::Result<Arc<MemoryManager>> {
        use openclaw_memory::ai_adapter::AIProviderEmbeddingAdapter;

        let ai_provider = self.create_ai_provider().await?;
        
        let embedding_provider = AIProviderEmbeddingAdapter::new(
            ai_provider.clone(),
            "text-embedding-3-small".to_string(),
            1536,
        );

        let vector_store: Arc<dyn openclaw_vector::VectorStore> = 
            Arc::new(openclaw_vector::MemoryStore::new());

        let config = openclaw_memory::types::MemoryConfig {
            working: openclaw_memory::types::WorkingMemoryConfig::default(),
            short_term: openclaw_memory::types::ShortTermMemoryConfig::default(),
            long_term: openclaw_memory::types::LongTermMemoryConfig {
                enabled: true,
                backend: "memory".to_string(),
                collection: "openclaw_memories".to_string(),
                embedding_provider: self.config.ai.default_provider.clone(),
                embedding_model: "text-embedding-3-small".to_string(),
                custom_embedding: None,
            },
        };

        let manager = MemoryManager::new(config)
            .with_vector_store(vector_store)
            .with_embedding_provider(embedding_provider);

        Ok(Arc::new(manager))
    }

    fn create_security_pipeline(&self) -> Arc<SecurityPipeline> {
        let config = openclaw_security::pipeline::PipelineConfig::default();
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
        use openclaw_voice::{OpenAIWhisperStt, OpenAITts, SttConfig, TtsConfig};
        
        let stt_config = SttConfig::default();
        let stt: Arc<dyn openclaw_voice::SpeechToText> = Arc::new(OpenAIWhisperStt::new(stt_config));

        let tts_config = TtsConfig::default();
        let tts: Arc<dyn openclaw_voice::TextToSpeech> = Arc::new(OpenAITts::new(tts_config));

        self.voice_service.init_voice(stt, tts).await;

        tracing::info!("Voice service initialized");
        Ok(())
    }

    async fn init_memory_service(&self, memory_manager: Arc<MemoryManager>) -> openclaw_core::Result<()> {
        use openclaw_vector::MemoryStore;

        let vector_store: Arc<dyn openclaw_vector::VectorStore> = Arc::new(MemoryStore::new());

        self.memory_service.init(memory_manager, vector_store).await;

        tracing::info!("Memory service initialized (reusing agent's MemoryManager)");
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
