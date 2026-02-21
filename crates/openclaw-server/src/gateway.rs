//! 网关服务

use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use openclaw_core::Config;

use crate::api::create_router;
use crate::app_context::AppContext;
use crate::config_adapter::ConfigAdapter;
use crate::service_factory::{DefaultServiceFactory, ServiceFactory};
use crate::websocket::websocket_router;

pub struct Gateway {
    config: Config,
    context: Arc<AppContext>,
}

impl Gateway {
    pub async fn new(config: Config) -> Self {
        let config_for_adapter = config.clone();
        let config_for_device = config.clone();
        
        let config_adapter = ConfigAdapter::from_ref(&config_for_adapter);
        let vector_store_registry = Arc::new(crate::vector_store_registry::VectorStoreRegistry::new());
        let device_manager = Arc::new(crate::device_manager::DeviceManager::new(config_for_device));

        let factory = DefaultServiceFactory::new(
            Arc::new(config_adapter),
            vector_store_registry,
            device_manager,
        );

        let context = factory.create_app_context(config.clone())
            .await
            .expect("Failed to create app context");

        Self { config, context }
    }

    pub async fn start(&self) -> openclaw_core::Result<()> {
        self.context.vector_store_registry.register_defaults().await;

        self.context.device_manager.init().await?;

        if let Some(ref orchestrator) = *self.context.orchestrator.read().await {
            orchestrator.start().await?;

            if !self.config.agents.list.is_empty() {
                orchestrator.init_agents_from_config(&self.config).await?;
            }

            orchestrator
                .inject_dependencies_with_tool_registry(
                    self.context.ai_provider.clone(),
                    self.context.memory_manager.clone(),
                    self.context.security_pipeline.clone(),
                    self.context.tool_registry.clone(),
                )
                .await;

            tracing::info!("Dependencies injected to all agents");
        }

        if self.config.server.enable_voice {
            self.init_voice_service().await?;
        }

        let canvas_manager = (*self.context.orchestrator.read().await).as_ref().map(|orchestrator| orchestrator.canvas_manager());

        let app = Router::new()
            .merge(create_router(self.context.clone(), canvas_manager))
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

        self.context.voice_service.init_voice(stt, tts).await;

        tracing::info!("Voice service initialized with STT: {}, TTS: {}", voice_config.stt_provider, voice_config.tts_provider);
        Ok(())
    }

    pub fn context(&self) -> Arc<AppContext> {
        self.context.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_core::Config;

    #[test]
    fn test_gateway_config_fields() {
        let config = Config::default();
        assert!(!config.server.enable_agents);
    }

    #[test]
    fn test_gateway_config_with_agents() {
        let mut config = Config::default();
        config.server.enable_agents = true;
        assert!(config.server.enable_agents);
    }

    #[tokio::test]
    async fn test_gateway_new_is_async() {
        let config = Config::default();
        let gateway = Gateway::new(config).await;
        assert!(!gateway.config.server.enable_agents);
    }

    #[tokio::test]
    async fn test_gateway_context_available() {
        let config = Config::default();
        let gateway = Gateway::new(config).await;
        let ctx = gateway.context();
        assert!(!ctx.config.server.enable_agents);
    }
}
