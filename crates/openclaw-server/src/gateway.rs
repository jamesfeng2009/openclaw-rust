//! 网关服务

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_core::Config;

use crate::api::create_router;
use crate::websocket::websocket_router;
use crate::orchestrator::{ServiceOrchestrator, OrchestratorConfig};

pub struct Gateway {
    config: Config,
    orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
}

impl Gateway {
    pub fn new(config: Config) -> Self {
        let orchestrator_config = OrchestratorConfig {
            enable_agents: config.server.enable_agents,
            enable_channels: config.server.enable_channels,
            enable_voice: config.server.enable_voice,
            default_agent: Some("orchestrator".to_string()),
            channel_to_agent_map: std::collections::HashMap::new(),
        };

        let orchestrator = Arc::new(RwLock::new(
            if config.server.enable_agents || config.server.enable_channels {
                Some(ServiceOrchestrator::new(orchestrator_config))
            } else {
                None
            }
        ));

        Self { config, orchestrator }
    }

    /// 启动服务
    pub async fn start(&self) -> openclaw_core::Result<()> {
        if let Some(ref orchestrator) = *self.orchestrator.read().await {
            orchestrator.start().await?;
        }

        let app = Router::new()
            .merge(create_router())
            .merge(websocket_router())
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
            .layer(TraceLayer::new_for_http());

        let addr: SocketAddr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse()
            .expect("Invalid address");

        tracing::info!("OpenClaw Gateway starting on {}", addr);
        tracing::info!("Services enabled: agents={}, channels={}, voice={}", 
            self.config.server.enable_agents, 
            self.config.server.enable_channels, 
            self.config.server.enable_voice);

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("绑定地址失败: {}", e)))?;

        axum::serve(listener, app).await
            .map_err(|e| openclaw_core::OpenClawError::Unknown(e.to_string()))?;

        Ok(())
    }

    pub async fn get_orchestrator(&self) -> Option<Arc<RwLock<Option<ServiceOrchestrator>>>> {
        if self.config.server.enable_agents || self.config.server.enable_channels {
            Some(self.orchestrator.clone())
        } else {
            None
        }
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
