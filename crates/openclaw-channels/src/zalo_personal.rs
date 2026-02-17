//! Zalo Personal 通道实现
//!
//! Zalo Personal 是 Zalo 的个人 API，适用于个人开发者
//! 与 Zalo Official Account (OA) API 不同
//! 文档: https://zalosdk.github.io/

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Zalo Personal 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloPersonalConfig {
    /// Zalo Personal API Key
    pub api_key: Option<String>,
    /// Zalo Personal Secret Key
    pub secret_key: Option<String>,
    /// Zalo User ID (自己的 ID)
    pub user_id: Option<String>,
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// Webhook Secret
    pub webhook_secret: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// Zalo Personal 消息响应
#[derive(Debug, Deserialize)]
pub struct ZaloPersonalMessageResponse {
    #[serde(default)]
    pub error: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<ZaloPersonalMessageData>,
}

#[derive(Debug, Deserialize)]
pub struct ZaloPersonalMessageData {
    #[serde(default)]
    pub msg_id: Option<String>,
}

/// Zalo Personal 用户信息
#[derive(Debug, Deserialize)]
pub struct ZaloPersonalUserInfo {
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
}

/// Zalo Personal 客户端
pub struct ZaloPersonalClient {
    config: ZaloPersonalConfig,
    client: reqwest::Client,
    running: std::sync::RwLock<bool>,
}

impl ZaloPersonalClient {
    pub fn new(config: ZaloPersonalConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            running: std::sync::RwLock::new(false),
        }
    }

    fn get_api_url(&self, endpoint: &str) -> String {
        format!("https://api.zalopersonals.com{}", endpoint)
    }

    /// 发送文本消息
    pub async fn send_text(
        &self,
        user_id: &str,
        text: &str,
    ) -> Result<ZaloPersonalMessageResponse> {
        let url = self.get_api_url("/v2/message/text");

        let body = serde_json::json!({
            "user_id": user_id,
            "message": {
                "text": text
            }
        });

        self.send_request(&url, body).await
    }

    /// 发送图片消息
    pub async fn send_image(
        &self,
        user_id: &str,
        image_url: &str,
        caption: Option<&str>,
    ) -> Result<ZaloPersonalMessageResponse> {
        let url = self.get_api_url("/v2/message/image");

        let mut message = serde_json::json!({
            "url": image_url
        });

        if let Some(cap) = caption {
            message["caption"] = serde_json::json!(cap);
        }

        let body = serde_json::json!({
            "user_id": user_id,
            "message": message
        });

        self.send_request(&url, body).await
    }

    /// 发送文件消息
    pub async fn send_file(
        &self,
        user_id: &str,
        file_url: &str,
        file_name: Option<&str>,
    ) -> Result<ZaloPersonalMessageResponse> {
        let url = self.get_api_url("/v2/message/file");

        let mut message = serde_json::json!({
            "url": file_url
        });

        if let Some(name) = file_name {
            message["file_name"] = serde_json::json!(name);
        }

        let body = serde_json::json!({
            "user_id": user_id,
            "message": message
        });

        self.send_request(&url, body).await
    }

    /// 获取用户信息
    pub async fn get_user_info(&self, user_id: &str) -> Result<ZaloPersonalUserInfo> {
        let url = self.get_api_url("/v2/user/getprofile");

        let body = serde_json::json!({
            "user_id": user_id
        });

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Zalo Personal API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Zalo Personal API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Zalo Personal 响应失败: {}", e)))
    }

    /// 创建请求
    async fn send_request(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<ZaloPersonalMessageResponse> {
        let response = self
            .client
            .post(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Zalo Personal API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Zalo Personal API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Zalo Personal 响应失败: {}", e)))
    }

    /// 处理 Webhook 消息
    pub async fn handle_webhook(&self, payload: serde_json::Value) -> Result<ChannelMessage> {
        let sender = payload
            .get("sender")
            .and_then(|s| s.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or_default();

        let content = payload
            .get("message")
            .and_then(|m| m.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or_default();

        let msg_id = payload
            .get("mid")
            .and_then(|m| m.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(ChannelMessage {
            id: msg_id,
            channel_type: ChannelType::ZaloPersonal,
            chat_id: sender.to_string(),
            user_id: sender.to_string(),
            content: content.to_string(),
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
impl Channel for ZaloPersonalClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::ZaloPersonal
    }

    fn name(&self) -> &str {
        "zalopersonal"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.api_key.is_none() {
            return Err(OpenClawError::Channel(
                "Zalo Personal API Key 未配置".into(),
            ));
        }

        *self.running.write().unwrap() = true;
        tracing::info!("Zalo Personal 客户端已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("Zalo Personal 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self.send_text(&message.chat_id, &message.content).await?;

        if let Some(error) = response.error {
            if error != 0 {
                return Err(OpenClawError::Channel(format!(
                    "Zalo Personal 发送失败: {}",
                    response.message.unwrap_or_default()
                )));
            }
        }

        let msg_id = response
            .data
            .and_then(|d| d.msg_id)
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        Ok(ChannelMessage {
            id: msg_id,
            channel_type: ChannelType::ZaloPersonal,
            chat_id: message.chat_id,
            user_id: self.config.user_id.clone().unwrap_or_default(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.api_key.is_some())
    }
}
