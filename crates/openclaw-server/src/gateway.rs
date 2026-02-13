//! 网关服务

use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use openclaw_core::Config;

use crate::api::create_router;

/// 网关服务
pub struct Gateway {
    config: Config,
}

impl Gateway {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// 启动服务
    pub async fn start(&self) -> openclaw_core::Result<()> {
        let app = Router::new()
            .merge(create_router())
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any))
            .layer(TraceLayer::new_for_http());

        let addr: SocketAddr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse()
            .expect("Invalid address");

        tracing::info!("OpenClaw Gateway starting on {}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| openclaw_core::OpenClawError::Config(format!("绑定地址失败: {}", e)))?;

        axum::serve(listener, app).await
            .map_err(|e| openclaw_core::OpenClawError::Unknown(e.to_string()))?;

        Ok(())
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
