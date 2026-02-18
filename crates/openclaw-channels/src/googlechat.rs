//! Google Chat 通道实现
//!
//! Google Chat 是 Google Workspace 的消息应用
//! 文档: https://developers.google.com/chat

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Google Chat 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleChatConfig {
    /// Bot 令牌
    pub bot_token: Option<String>,
    /// 应用 ID
    pub application_credentials: Option<String>,
    /// Webhook URL (用于接收消息)
    pub webhook_url: Option<String>,
    /// Webhook 密钥
    pub webhook_secret: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// Google Chat 消息格式
#[derive(Debug, Deserialize)]
pub struct GoogleChatMessage {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub sender: Option<GoogleChatSender>,
    #[serde(default)]
    pub space: Option<GoogleChatSpace>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub create_time: Option<String>,
}

/// Google Chat 发送者
#[derive(Debug, Deserialize)]
pub struct GoogleChatSender {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub type_field: Option<String>,
}

/// Google Chat 空间
#[derive(Debug, Deserialize)]
pub struct GoogleChatSpace {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub type_field: Option<String>,
}

/// Google Chat 消息请求
#[derive(Debug, Serialize)]
pub struct GoogleChatSendRequest {
    pub text: Option<String>,
    pub cards: Option<Vec<serde_json::Value>>,
}

/// Google Chat 消息响应
#[derive(Debug, Deserialize)]
pub struct GoogleChatSendResponse {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
}

/// Google Chat 客户端
pub struct GoogleChatClient {
    config: GoogleChatConfig,
    client: reqwest::Client,
    running: std::sync::RwLock<bool>,
}

impl GoogleChatClient {
    pub fn new(config: GoogleChatConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            running: std::sync::RwLock::new(false),
        }
    }

    /// 发送文本消息
    pub async fn send_message(&self, space: &str, text: &str) -> Result<GoogleChatSendResponse> {
        let url = format!(
            "https://chat.googleapis.com/v1/{}/messages?key={}&token={}",
            space,
            self.config.bot_token.as_deref().unwrap_or(""),
            ""
        );

        let body = GoogleChatSendRequest {
            text: Some(text.to_string()),
            cards: None,
        };

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Google Chat API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Google Chat API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Google Chat 响应失败: {}", e)))
    }

    /// 发送卡片消息
    pub async fn send_card(
        &self,
        space: &str,
        cards: Vec<serde_json::Value>,
    ) -> Result<GoogleChatSendResponse> {
        let url = format!("https://chat.googleapis.com/v1/{}/messages", space);

        let body = GoogleChatSendRequest {
            text: None,
            cards: Some(cards),
        };

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Google Chat API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Google Chat API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Google Chat 响应失败: {}", e)))
    }

    /// 处理 Webhook 消息
    pub async fn handle_webhook(&self, payload: serde_json::Value) -> Result<ChannelMessage> {
        let msg: GoogleChatMessage =
            serde_json::from_value(payload).map_err(OpenClawError::Serialization)?;

        let space = msg.space.as_ref();
        let sender = msg.sender.as_ref();

        Ok(ChannelMessage {
            id: msg.name.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            channel_type: ChannelType::GoogleChat,
            chat_id: space.and_then(|s| s.name.clone()).unwrap_or_default(),
            user_id: sender.and_then(|s| s.name.clone()).unwrap_or_default(),
            content: msg.message.unwrap_or_default(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    /// 验证 Webhook 签名
    pub fn verify_signature(&self, _payload: &str, _signature: &str) -> bool {
        self.config.webhook_secret.is_some()
    }
}

#[async_trait]
impl Channel for GoogleChatClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::GoogleChat
    }

    fn name(&self) -> &str {
        "googlechat"
    }

    async fn start(&mut self) -> Result<()> {
        *self.running.write().unwrap() = true;
        tracing::info!("Google Chat 客户端已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("Google Chat 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self
            .send_message(&message.chat_id, &message.content)
            .await?;

        Ok(ChannelMessage {
            id: response
                .name
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            channel_type: ChannelType::GoogleChat,
            chat_id: message.chat_id,
            user_id: "bot".to_string(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.bot_token.is_some())
    }
}
