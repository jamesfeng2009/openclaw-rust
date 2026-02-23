//! 网关服务

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use openclaw_core::Result;
use crate::server_config::ServerConfig;

use crate::adapters::{AIProviderAdapter, SecurityPipelineAdapter, ToolRegistryAdapter};
use crate::api::create_router;
use crate::app_context::AppContext;
use crate::config_adapter::ConfigAdapter;
use crate::service_factory::{DefaultServiceFactory, ServiceFactory};
use crate::websocket::websocket_router;

pub struct Gateway {
    config: ServerConfig,
    context: Arc<AppContext>,
    factory: Arc<DefaultServiceFactory>,
}

impl Gateway {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let config_for_adapter = config.core.clone();
        let config_for_device = config.devices.clone();

        let config_adapter = ConfigAdapter::from_ref(&config_for_adapter);
        let vector_store_registry =
            Arc::new(crate::vector_store_registry::VectorStoreRegistry::new());
        let device_manager = Arc::new(crate::device_manager::DeviceManager::new(config_for_device));

        let factory = DefaultServiceFactory::new(
            Arc::new(config_adapter),
            vector_store_registry,
            device_manager,
        );

        let context = factory.create_app_context(config.core.clone()).await?;

        Ok(Self {
            config,
            context,
            factory: Arc::new(factory),
        })
    }

    pub async fn start(&self) -> openclaw_core::Result<()> {
        let enabled_backends = self.config.vector.backends.clone();
        self.context
            .vector_store_registry
            .register_defaults(enabled_backends)
            .await;

        self.context.device_manager.init().await?;

        if let Some(ref orchestrator) = *self.context.orchestrator.read().await {
            orchestrator.start().await?;

            if !self.config.agents.list.is_empty() {
                orchestrator.init_agents_from_config(&self.config).await?;
            }

            let ai_port = Arc::new(AIProviderAdapter::new(
                self.context.ai_provider.clone(),
                "default",
            )) as Arc<dyn openclaw_agent::ports::AIPort>;
            let security_port = Arc::new(SecurityPipelineAdapter::new(
                self.context.security_pipeline.clone(),
            )) as Arc<dyn openclaw_agent::ports::SecurityPort>;
            let tool_port = Arc::new(ToolRegistryAdapter::new(self.context.tool_registry.clone()))
                as Arc<dyn openclaw_agent::ports::ToolPort>;

            let memory_port = self.context.memory_manager.as_ref().map(|m| {
                Arc::new(crate::ports::adapters::MemoryPortAdapter::new(Arc::clone(m)))
                    as Arc<dyn openclaw_agent::ports::MemoryPort>
            });

            orchestrator
                .inject_ports(Some(ai_port), memory_port, Some(security_port), Some(tool_port))
                .await;

            if self.config.server.enable_agentic_rag {
                let agentic_rag_engine = self
                    .factory
                    .create_agentic_rag_engine(
                        self.context.ai_provider.clone(),
                        self.context.memory_manager.clone(),
                    )
                    .await?;
                orchestrator
                    .set_agentic_rag_engine(agentic_rag_engine)
                    .await;
                tracing::info!("Agentic RAG engine initialized");
            }

            tracing::info!("Dependencies injected to all agents");
        }

        if self.config.server.enable_voice {
            self.init_voice_service().await?;
        }

        let canvas_manager = (*self.context.orchestrator.read().await)
            .as_ref()
            .map(|orchestrator| orchestrator.canvas_manager());

        let browser_config = self
            .config
            .browser
            .as_ref()
            .map(|v| serde_json::from_value::<openclaw_browser::BrowserConfig>(v.clone()).ok())
            .flatten();

        let app = Router::new()
            .merge(create_router(
                self.context.clone(),
                canvas_manager,
                browser_config,
            ))
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
            self.config.channels.enabled,
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

    async fn init_voice_service(&self) -> openclaw_core::Result<()> {
        let (stt, tts) = self.factory.create_voice_providers().await?;

        self.context.voice_service.init_voice(stt, tts).await;

        let voice_config = self.config.voice.clone().unwrap_or_default();
        tracing::info!(
            "Voice service initialized with STT: {}, TTS: {}",
            voice_config.stt_provider,
            voice_config.tts_provider
        );
        Ok(())
    }

    pub fn context(&self) -> Arc<AppContext> {
        self.context.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_config::ServerConfig;

    #[test]
    fn test_gateway_config_fields() {
        let config = ServerConfig::default();
        assert!(!config.core.server.enable_agents);
    }

    #[test]
    fn test_gateway_config_with_agents() {
        let mut config = ServerConfig::default();
        config.core.server.enable_agents = true;
        assert!(config.core.server.enable_agents);
    }

    #[tokio::test]
    async fn test_gateway_new_is_async() {
        let config = ServerConfig::default();
        let gateway = Gateway::new(config).await.unwrap();
        assert!(!gateway.config.core.server.enable_agents);
    }

    #[tokio::test]
    async fn test_gateway_context_available() {
        let config = ServerConfig::default();
        let gateway = Gateway::new(config).await.unwrap();
        let ctx = gateway.context();
        assert!(!ctx.config.server.enable_agents);
    }
}
