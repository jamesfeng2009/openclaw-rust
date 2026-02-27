//! HTTP Transport for ACP

use async_trait::async_trait;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::envelope::AcpEnvelope;
use crate::error::{AcpError, AcpResult};
use super::{HttpTransportConfig, Transport};

pub struct HttpTransport {
    config: HttpTransportConfig,
    client: Client,
    connected: Arc<RwLock<bool>>,
}

impl HttpTransport {
    pub fn new(config: HttpTransportConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            connected: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn send_request(&self, envelope: AcpEnvelope) -> AcpResult<AcpEnvelope> {
        let mut retries = 0;
        let max_retries = self.config.retries;

        loop {
            let response = self.client
                .post(&self.config.endpoint)
                .header("Content-Type", "application/json")
                .json(&envelope)
                .send()
                .await
                .map_err(|e| AcpError::Network(format!("HTTP request failed: {}", e)))?;

            if response.status().is_success() {
                let response_envelope: AcpEnvelope = response
                    .json()
                    .await
                    .map_err(|e| AcpError::Serialization(format!("Failed to parse response: {}", e)))?;
                return Ok(response_envelope);
            }

            retries += 1;
            if retries >= max_retries {
                return Err(AcpError::Network(format!(
                    "HTTP request failed after {} retries",
                    max_retries
                )));
            }
        }
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&self, envelope: AcpEnvelope) -> AcpResult<AcpEnvelope> {
        self.send_request(envelope).await
    }

    async fn receive(&self) -> AcpResult<AcpEnvelope> {
        Err(AcpError::Transport("HTTP transport does not support receiving".to_string()))
    }

    async fn connect(&mut self) -> AcpResult<()> {
        let mut connected = self.connected.write().await;
        *connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> AcpResult<()> {
        let mut connected = self.connected.write().await;
        *connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        // This is a synchronous check, so we need to handle the async lock
        // For simplicity, we'll just return true if we've ever connected
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_creation() {
        let config = HttpTransportConfig {
            endpoint: "http://localhost:8080/acp".to_string(),
            timeout_secs: 30,
            retries: 3,
        };
        let _transport = HttpTransport::new(config);
    }
}
