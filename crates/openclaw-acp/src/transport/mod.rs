//! ACP Transport Layer
//!
//! Provides HTTP and WebSocket transport for ACP messages.

pub mod http;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::envelope::AcpEnvelope;
use crate::error::AcpResult;

pub use http::HttpTransport;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, envelope: AcpEnvelope) -> AcpResult<AcpEnvelope>;
    async fn receive(&self) -> AcpResult<AcpEnvelope>;
    async fn connect(&mut self) -> AcpResult<()>;
    async fn disconnect(&mut self) -> AcpResult<()>;
    fn is_connected(&self) -> bool;
}

/// HTTP Transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpTransportConfig {
    pub endpoint: String,
    pub timeout_secs: u64,
    pub retries: u32,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            timeout_secs: 30,
            retries: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpTransportConfig::default();
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.retries, 3);
    }
}
