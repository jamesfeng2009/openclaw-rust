//! BlueBubbles 通道实现
//!
//! BlueBubbles 是一个 macOS 应用，提供 REST API 来发送 iMessage 消息
//! 文档: https://bluebubblesapp.com/

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// BlueBubbles 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueBubblesConfig {
    /// BlueBubbles 服务器地址 (例如 http://localhost:1234)
    pub server_url: String,
    /// API 密钥
    pub api_key: Option<String>,
    /// 默认发送者 ID
    pub default_sender: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// BlueBubbles 消息响应
#[derive(Debug, Deserialize)]
pub struct BlueBubblesMessageResponse {
    #[serde(default)]
    pub result: Option<BlueBubblesMessageResult>,
    #[serde(default)]
    pub error: Option<BlueBubblesError>,
}

#[derive(Debug, Deserialize)]
pub struct BlueBubblesMessageResult {
    #[serde(rename = "guid")]
    pub guid: Option<String>,
    #[serde(rename = "id")]
    pub id: Option<String>,
    #[serde(rename = "text")]
    pub text: Option<String>,
    #[serde(rename = "handleId")]
    pub handle_id: Option<String>,
    #[serde(rename = "dateCreated")]
    pub date_created: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct BlueBubblesError {
    pub message: Option<String>,
    pub code: Option<i32>,
}

/// BlueBubbles 聊天响应
#[derive(Debug, Deserialize)]
pub struct BlueBubblesChatsResponse {
    #[serde(default)]
    pub chats: Option<Vec<BlueBubblesChat>>,
}

#[derive(Debug, Deserialize)]
pub struct BlueBubblesChat {
    #[serde(rename = "guid")]
    pub guid: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "chatName")]
    pub chat_name: Option<String>,
    #[serde(rename = "isGroup")]
    pub is_group: Option<bool>,
}

/// BlueBubbles 客户端
pub struct BlueBubblesClient {
    config: BlueBubblesConfig,
    client: reqwest::Client,
    running: std::sync::RwLock<bool>,
}

impl BlueBubblesClient {
    pub fn new(config: BlueBubblesConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            running: std::sync::RwLock::new(false),
            config,
        }
    }

    fn get_api_url(&self, endpoint: &str) -> String {
        let base = self.config.server_url.trim_end_matches('/');
        format!("{}{}", base, endpoint)
    }

    /// 发送 iMessage 文本消息
    pub async fn send_message(
        &self,
        recipient: &str,
        text: &str,
    ) -> Result<BlueBubblesMessageResponse> {
        let url = self.get_api_url("/api/v1/message/text");

        let mut body = serde_json::json!({
            "address": recipient,
            "text": text,
        });

        if let Some(sender) = &self.config.default_sender {
            body["from"] = serde_json::json!(sender);
        }

        let mut request = self.client.post(&url).json(&body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("BlueBubbles API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "BlueBubbles API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 BlueBubbles 响应失败: {}", e)))
    }

    /// 发送 iMessage 图片消息
    pub async fn send_attachment(
        &self,
        recipient: &str,
        attachment_path: &str,
        mime_type: Option<&str>,
    ) -> Result<BlueBubblesMessageResponse> {
        let url = self.get_api_url("/api/v1/message/attachment");

        let mut body = serde_json::json!({
            "address": recipient,
            "attachmentPath": attachment_path,
        });

        if let Some(mime) = mime_type {
            body["mimeType"] = serde_json::json!(mime);
        }

        if let Some(sender) = &self.config.default_sender {
            body["from"] = serde_json::json!(sender);
        }

        let mut request = self.client.post(&url).json(&body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("BlueBubbles API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "BlueBubbles API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 BlueBubbles 响应失败: {}", e)))
    }

    /// 获取聊天列表
    pub async fn get_chats(&self) -> Result<Vec<BlueBubblesChat>> {
        let url = self.get_api_url("/api/v1/chat");

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("BlueBubbles API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "BlueBubbles API 错误 ({}): {}",
                status, error_text
            )));
        }

        let result: BlueBubblesChatsResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 BlueBubbles 响应失败: {}", e)))?;

        Ok(result.chats.unwrap_or_default())
    }

    /// 获取服务器状态
    pub async fn get_server_info(&self) -> Result<serde_json::Value> {
        let url = self.get_api_url("/api/v1/server/info");

        let mut request = self.client.get(&url);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("BlueBubbles API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "BlueBubbles API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 BlueBubbles 响应失败: {}", e)))
    }

    /// 标记消息为已读
    pub async fn mark_read(&self, chat_guid: &str) -> Result<()> {
        let url = self.get_api_url("/api/v1/chat/read");

        let body = serde_json::json!({
            "guid": chat_guid,
            "read": true,
        });

        let mut request = self.client.put(&url).json(&body);

        if let Some(api_key) = &self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("BlueBubbles API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "BlueBubbles API 错误 ({}): {}",
                status, error_text
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for BlueBubblesClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::IMessage
    }

    fn name(&self) -> &str {
        "bluebubbles"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.server_url.is_empty() {
            return Err(OpenClawError::Channel(
                "BlueBubbles server URL 未配置".into(),
            ));
        }

        *self.running.write().unwrap() = true;
        tracing::info!(
            "BlueBubbles 客户端已启动，服务器: {}",
            self.config.server_url
        );
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("BlueBubbles 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self
            .send_message(&message.chat_id, &message.content)
            .await?;

        if let Some(error) = response.error {
            return Err(OpenClawError::Channel(
                error.message.unwrap_or_else(|| "发送消息失败".into()),
            ));
        }

        let msg_result = response
            .result
            .ok_or_else(|| OpenClawError::Channel("解析消息响应失败".into()))?;

        Ok(ChannelMessage {
            id: msg_result.guid.unwrap_or_else(|| message.chat_id.clone()),
            channel_type: ChannelType::IMessage,
            chat_id: message.chat_id,
            user_id: self
                .config
                .default_sender
                .clone()
                .unwrap_or_default(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        match self.get_server_info().await {
            Ok(info) => {
                tracing::debug!("BlueBubbles 服务器信息: {:?}", info);
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }
}
