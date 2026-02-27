//! 服务工厂模块
//!
//! 集中管理所有服务的创建逻辑，将 Gateway 从工厂职责中解放

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_ai::AIProvider;
use openclaw_core::{Config, Result};
use openclaw_device::factory::DeviceManagerFactory;
use openclaw_device::UnifiedDeviceManager;
use openclaw_memory::factory::{create_memory_backend, MemoryBackend};
use openclaw_memory::MemoryManager;
use openclaw_security::pipeline::SecurityPipeline;
use openclaw_sandbox::SandboxManager;
use openclaw_tools::{ToolRegistry, register_builtin_tools};

use crate::app_context::AppContext;
use crate::orchestrator::OrchestratorConfig;
use crate::orchestrator::ServiceOrchestrator;
use crate::voice_service::VoiceService;
use crate::server_config::AcpConfig;
use crate::acp_service::AcpService;

#[async_trait]
pub trait ServiceFactory: Send + Sync {
    async fn create_ai_provider(&self) -> Result<Arc<dyn AIProvider>>;
    async fn create_memory_backend(&self) -> Result<Arc<dyn MemoryBackend>>;
    fn create_security_pipeline(&self) -> Arc<SecurityPipeline>;
    fn create_tool_registry(&self) -> Arc<ToolRegistry>;
    async fn create_voice_providers(
        &self,
    ) -> Result<(
        Arc<dyn openclaw_voice::SpeechToText>,
        Arc<dyn openclaw_voice::TextToSpeech>,
    )>;
    async fn create_unified_device_manager(&self) -> Result<Arc<UnifiedDeviceManager>>;
    async fn create_app_context(&self, config: Config) -> Result<Arc<AppContext>>;
    async fn create_agentic_rag_engine(
        &self,
        ai_provider: Arc<dyn AIProvider>,
        memory_backend: Option<Arc<dyn MemoryBackend>>,
    ) -> Result<Arc<crate::agentic_rag::AgenticRAGEngine>>;
    async fn create_acp_service(&self, acp_config: &AcpConfig) -> Result<Option<Arc<AcpService>>>;
}

/// 默认服务工厂实现
pub struct DefaultServiceFactory {
    config: Arc<super::config_adapter::ConfigAdapter>,
    vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
    device_manager: Option<Arc<super::device_manager::DeviceManager>>,
}

impl DefaultServiceFactory {
    pub fn new(
        config: Arc<super::config_adapter::ConfigAdapter>,
        vector_store_registry: Arc<super::vector_store_registry::VectorStoreRegistry>,
        device_manager: Option<Arc<super::device_manager::DeviceManager>>,
    ) -> Self {
        Self {
            config,
            vector_store_registry,
            device_manager,
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

        let provider_type = ProviderType::from_str(&core_config.name).ok_or_else(|| {
            openclaw_core::OpenClawError::AIProvider(format!(
                "Unknown AI provider: {}",
                core_config.name
            ))
        })?;

        let provider = ProviderFactory::create(provider_type, ai_config)
            .map_err(openclaw_core::OpenClawError::AIProvider)?;
        Ok(provider)
    }

    async fn create_memory_backend(&self) -> Result<Arc<dyn MemoryBackend>> {
        let ai_provider = self.create_ai_provider().await?;
        let memory_config = self.config.memory();

        let vector_store = self
            .vector_store_registry
            .create(&memory_config.long_term.backend)
            .await
            .unwrap_or_else(|| {
                Arc::new(openclaw_vector::MemoryStore::new())
                    as Arc<dyn openclaw_vector::VectorStore>
            });

        let backend_type = "hybrid";
        let backend = create_memory_backend(
            backend_type,
            &memory_config,
            ai_provider,
            vector_store,
        )
        .await?;

        Ok(backend)
    }

    fn create_security_pipeline(&self) -> Arc<SecurityPipeline> {
        let config = self.config.security();
        Arc::new(SecurityPipeline::new(config))
    }

    fn create_tool_registry(&self) -> Arc<ToolRegistry> {
        use crate::hardware_tools::CameraTool;

        let mut registry = ToolRegistry::new();

        register_builtin_tools(&mut registry);

        if let Some(ref device_manager) = self.device_manager {
            let capabilities = device_manager.get_capabilities();

            if capabilities
                .sensors
                .contains(&openclaw_device::SensorType::Camera)
            {
                let camera_manager = Arc::new(openclaw_device::CameraManager::new());
                let camera_tool = Arc::new(CameraTool::new(Some(camera_manager), capabilities.clone()));
                registry.register("hardware_camera".to_string(), camera_tool);
                tracing::info!("Hardware camera tool registered");
            }

            if capabilities
                .sensors
                .contains(&openclaw_device::SensorType::Microphone)
            {
                tracing::info!("Microphone available - microphone tool can be added");
            }

            tracing::info!("Tool registry created with hardware tools based on device capabilities");
        }

        Arc::new(registry)
    }

    async fn create_voice_providers(
        &self,
    ) -> Result<(
        Arc<dyn openclaw_voice::SpeechToText>,
        Arc<dyn openclaw_voice::TextToSpeech>,
    )> {
        use openclaw_voice::{
            SttConfig, SttProvider, TtsConfig, TtsProvider, create_stt, create_tts,
        };

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

        let stt: Arc<dyn openclaw_voice::SpeechToText> =
            create_stt(stt_provider, stt_config).into();
        let tts: Arc<dyn openclaw_voice::TextToSpeech> =
            create_tts(tts_provider, tts_config).into();

        Ok((stt, tts))
    }

    async fn create_unified_device_manager(&self) -> Result<Arc<UnifiedDeviceManager>> {
        if let Some(device_manager) = &self.device_manager {
            let registry = device_manager.registry().clone();
            let unified = UnifiedDeviceManager::new(registry);
            Ok(Arc::new(unified))
        } else {
            openclaw_device::factory::init_default_factory();
            let config = self.config.device();
            let factory = openclaw_device::factory::get_factory("default")
                .ok_or_else(|| openclaw_core::OpenClawError::Config("Device factory not found".to_string()))?;
            factory.create(&config).await.map_err(|e| openclaw_core::OpenClawError::Config(e.to_string()))
        }
    }

    async fn create_app_context(&self, config: Config) -> Result<Arc<AppContext>> {
        let memory_config = self.config.memory();
        let channel_to_agent_map = config.channels.channel_to_agent_map.clone();

        let orchestrator_config = OrchestratorConfig {
            enable_agents: config.server.enable_agents,
            enable_channels: config.channels.enabled,
            enable_voice: config.server.enable_voice,
            enable_canvas: config.server.enable_canvas,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map,
            agent_to_canvas_map: std::collections::HashMap::new(),
            #[cfg(feature = "per_session_memory")]
            enable_per_session_memory: false,
            #[cfg(feature = "per_session_memory")]
            memory_config: Some(memory_config),
            #[cfg(feature = "per_session_memory")]
            max_session_memories: 100,
        };

        let orchestrator = Arc::new(RwLock::new(
            if config.server.enable_agents || config.channels.enabled || config.server.enable_canvas
            {
                Some(ServiceOrchestrator::new(orchestrator_config))
            } else {
                None
            },
        ));

        let ai_provider = self.create_ai_provider().await?;
        let memory_backend = Some(self.create_memory_backend().await?);
        let security_pipeline = self.create_security_pipeline();
        let tool_registry = self.create_tool_registry();
        let voice_service = Arc::new(VoiceService::new());

        let unified_device_manager = match self.create_unified_device_manager().await {
            Ok(manager) => Some(manager),
            Err(e) => {
                tracing::warn!("Failed to create unified device manager: {}", e);
                None
            }
        };

        let sandbox_manager = if self.config.sandbox().enabled {
            let sandbox = Arc::new(SandboxManager::new());
            Some(sandbox)
        } else {
            None
        };

        let context = AppContext::new(
            config,
            ai_provider,
            memory_backend,
            security_pipeline,
            tool_registry,
            sandbox_manager,
            orchestrator,
            self.device_manager.clone(),
            unified_device_manager,
            voice_service,
            self.vector_store_registry.clone(),
        );

        Ok(Arc::new(context))
    }

    async fn create_agentic_rag_engine(
        &self,
        ai_provider: Arc<dyn openclaw_ai::AIProvider>,
        _memory_backend: Option<Arc<dyn MemoryBackend>>,
    ) -> Result<Arc<crate::agentic_rag::AgenticRAGEngine>> {
        use crate::agentic_rag::{AgenticRAGConfig, AgenticRAGEngine};

        let config = AgenticRAGConfig::default();

        let engine = AgenticRAGEngine::new(config, ai_provider, None, None, None).await?;

        Ok(Arc::new(engine))
    }

    async fn create_acp_service(&self, acp_config: &AcpConfig) -> Result<Option<Arc<AcpService>>> {
        if !acp_config.enabled {
            return Ok(None);
        }

        let acp = if let Some(default_agent) = &acp_config.default_agent {
            AcpService::new().with_default_agent(default_agent.clone())
        } else {
            AcpService::new()
        };

        for agent_config in &acp_config.agents {
            let info = openclaw_acp::AgentInfo::new(
                agent_config.id.clone(),
                agent_config.name.clone(),
            )
            .with_endpoint(agent_config.endpoint.clone().unwrap_or_else(|| "local".to_string()))
            .with_capabilities(agent_config.capabilities.clone());
            
            acp.register_agent(info).await;
        }

        for rule in &acp_config.router.rules {
            let mut router = (*acp.router()).clone();
            if let Err(e) = router.add_rule(&rule.pattern, &rule.target, rule.priority) {
                tracing::warn!("Failed to add route rule: {}", e);
            }
        }

        Ok(Some(Arc::new(acp)))
    }
}
