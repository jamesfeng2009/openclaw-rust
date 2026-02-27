//! WebSocket Transport for ACP

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::envelope::AcpEnvelope;
use crate::error::{AcpError, AcpResult};
use super::Transport;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsTransportConfig {
    pub endpoint: String,
    pub heartbeat_interval_secs: u64,
    pub reconnect_attempts: u32,
    pub reconnect_delay_ms: u64,
}

impl Default for WsTransportConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            heartbeat_interval_secs: 30,
            reconnect_attempts: 5,
            reconnect_delay_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsMessage {
    #[serde(rename = "envelope")]
    Envelope(AcpEnvelope),
    #[serde(rename = "heartbeat")]
    Heartbeat { timestamp: i64 },
    #[serde(rename = "connect_ack")]
    ConnectAck { session_id: String },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

pub struct WebSocketTransport {
    config: WsTransportConfig,
    session_id: Option<String>,
    connected: Arc<RwLock<bool>>,
    sender: Arc<RwLock<Option<mpsc::Sender<WsMessage>>>>,
    receiver: Arc<RwLock<Option<mpsc::Receiver<WsMessage>>>>,
}

impl WebSocketTransport {
    pub fn new(config: WsTransportConfig) -> Self {
        Self {
            config,
            session_id: None,
            connected: Arc::new(RwLock::new(false)),
            sender: Arc::new(RwLock::new(None)),
            receiver: Arc::new(RwLock::new(None)),
        }
    }

    async fn do_connect(&mut self) -> AcpResult<()> {
        let (ws_stream, _) = connect_async(&self.config.endpoint)
            .await
            .map_err(|e| AcpError::Transport(format!("WebSocket connect failed: {}", e)))?;

        let (mut write, mut read) = ws_stream.split();

        let (tx, mut rx) = mpsc::channel::<WsMessage>(100);
        let (internal_tx, mut internal_rx) = mpsc::channel::<WsMessage>(100);

        {
            let mut sender = self.sender.write().await;
            *sender = Some(tx);
        }

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg = rx.recv() => {
                        if let Some(msg) = msg {
                            let json = serde_json::to_string(&msg)
                                .map_err(|e| AcpError::Serialization(e.to_string()));
                            if let Ok(json) = json {
                                let _ = write.send(Message::Text(json.into())).await;
                            }
                        } else {
                            break;
                        }
                    }
                    msg = read.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                                    let _ = internal_tx.send(ws_msg).await;
                                }
                            }
                            Some(Ok(Message::Close(_))) | None => {
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        {
            let mut receiver = self.receiver.write().await;
            *receiver = Some(internal_rx);
        }

        {
            let mut connected = self.connected.write().await;
            *connected = true;
        }

        Ok(())
    }

    async fn connect_with_retry(&mut self) -> AcpResult<()> {
        let mut attempts = 0;

        loop {
            match self.do_connect().await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.config.reconnect_attempts {
                        return Err(e);
                    }

                    let delay = self.config.reconnect_delay_ms * 2u64.pow(attempts - 1);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                }
            }
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&self, _envelope: AcpEnvelope) -> AcpResult<AcpEnvelope> {
        let sender = self.sender.read().await;
        let sender = sender.as_ref()
            .ok_or_else(|| AcpError::Transport("Not connected".to_string()))?;

        let msg = WsMessage::Envelope(_envelope);
        sender.send(msg).await
            .map_err(|e| AcpError::Transport(e.to_string()))?;

        Err(AcpError::Transport("Use receive() to get response".to_string()))
    }

    async fn receive(&self) -> AcpResult<AcpEnvelope> {
        let mut receiver = self.receiver.write().await;
        if let Some(ref mut rx) = *receiver {
            if let Some(msg) = rx.recv().await {
                return match msg {
                    WsMessage::Envelope(envelope) => Ok(envelope),
                    WsMessage::Heartbeat { .. } => self.receive().await,
                    WsMessage::Error { code, message } =>
                        Err(AcpError::Transport(format!("{}: {}", code, message))),
                    WsMessage::ConnectAck { session_id } => {
                        tracing::debug!("Received ConnectAck: {}", session_id);
                        self.receive().await
                    }
                };
            }
        }
        Err(AcpError::Transport("Connection closed".to_string()))
    }

    async fn connect(&mut self) -> AcpResult<()> {
        self.connect_with_retry().await
    }

    async fn disconnect(&mut self) -> AcpResult<()> {
        {
            let mut sender = self.sender.write().await;
            *sender = None;
        }
        {
            let mut receiver = self.receiver.write().await;
            *receiver = None;
        }
        {
            let mut connected = self.connected.write().await;
            *connected = false;
        }
        self.session_id = None;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        false
    }
}
