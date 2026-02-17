//! WebChat 通道实现
//!
//! WebChat 是一个通用的 Web 即时通讯通道，支持自定义 Webhook

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// WebChat 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatConfig {
    /// Webhook URL (用于接收消息)
    pub webhook_url: Option<String>,
    /// Webhook 密钥
    pub webhook_secret: Option<String>,
    /// 自定义服务器 URL
    pub server_url: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// WebChat 消息格式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: i64,
    pub message_type: String,
}

/// WebChat 客户端
pub struct WebChatClient {
    config: WebChatConfig,
    client: reqwest::Client,
    message_sender: tokio::sync::mpsc::Sender<ChannelMessage>,
    running: std::sync::RwLock<bool>,
}

impl WebChatClient {
    pub fn new(config: WebChatConfig) -> Self {
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        Self {
            client: reqwest::Client::new(),
            config,
            message_sender: tx,
            running: std::sync::RwLock::new(false),
        }
    }

    /// 发送消息
    pub async fn send_message(&self, to: &str, content: &str) -> Result<ChannelMessage> {
        let message = ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel_type: ChannelType::WebChat,
            chat_id: to.to_string(),
            user_id: "self".to_string(),
            content: content.to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        };

        Ok(message)
    }

    /// 处理接收到的 Webhook 消息
    pub async fn handle_webhook(&self, payload: serde_json::Value) -> Result<ChannelMessage> {
        let msg: WebChatMessage =
            serde_json::from_value(payload).map_err(|e| OpenClawError::Serialization(e))?;

        Ok(ChannelMessage {
            id: msg.id,
            channel_type: ChannelType::WebChat,
            chat_id: msg.to,
            user_id: msg.from,
            content: msg.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    /// 验证 Webhook 签名
    pub fn verify_signature(&self, payload: &str, signature: &str) -> bool {
        if let Some(secret) = &self.config.webhook_secret {
            use hmac::{Hmac, Mac};
            type HmacSha256 = Hmac<sha2::Sha256>;

            let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
            mac.update(payload.as_bytes());
            let result = mac.finalize();
            let expected = hex::encode(result.into_bytes());
            signature == expected
        } else {
            true
        }
    }
}

#[async_trait]
impl Channel for WebChatClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WebChat
    }

    fn name(&self) -> &str {
        "webchat"
    }

    async fn start(&mut self) -> Result<()> {
        *self.running.write().unwrap() = true;
        tracing::info!("WebChat 客户端已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("WebChat 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        self.send_message(&message.chat_id, &message.content).await
    }

    fn messages(&self) -> Option<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>> {
        None
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}
