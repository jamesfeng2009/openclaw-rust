//! Zalo 通道实现
//!
//! Zalo 是越南的即时通讯应用
//! 文档: https://developers.zalo.me/

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Zalo 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaloConfig {
    /// Zalo API Key
    pub api_key: Option<String>,
    /// Zalo Secret Key
    pub secret_key: Option<String>,
    /// Zalo App ID
    pub app_id: Option<String>,
    /// Access Token (需要手动获取或通过 OAuth)
    pub access_token: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// Zalo 用户信息
#[derive(Debug, Deserialize)]
pub struct ZaloUserInfo {
    #[serde(rename = "user_id")]
    pub user_id: Option<String>,
    #[serde(rename = "display_name")]
    pub display_name: Option<String>,
    #[serde(rename = "picture")]
    pub picture: Option<String>,
}

/// Zalo 消息响应
#[derive(Debug, Deserialize)]
pub struct ZaloMessageResponse {
    #[serde(default)]
    pub error: Option<i32>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Zalo 客户端
pub struct ZaloClient {
    config: ZaloConfig,
    client: reqwest::Client,
    running: std::sync::RwLock<bool>,
}

impl ZaloClient {
    pub fn new(config: ZaloConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            running: std::sync::RwLock::new(false),
        }
    }

    fn get_api_url(&self, endpoint: &str) -> String {
        format!("https://openapi.zalo.me{}", endpoint)
    }

    /// 发送文本消息
    pub async fn send_text(&self, user_id: &str, text: &str) -> Result<ZaloMessageResponse> {
        let url = self.get_api_url("/v3.0/oa/message/text");

        let body = serde_json::json!({
            "recipient": {
                "user_id": user_id
            },
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
    ) -> Result<ZaloMessageResponse> {
        let url = self.get_api_url("/v3.0/oa/message/image");

        let mut body = serde_json::json!({
            "recipient": {
                "user_id": user_id
            },
            "message": {
                "attachment": {
                    "type": "image",
                    "payload": {
                        "url": image_url
                    }
                }
            }
        });

        if let Some(cap) = caption {
            body["message"]["attachment"]["payload"]["caption"] = serde_json::json!(cap);
        }

        self.send_request(&url, body).await
    }

    /// 发送文件消息
    pub async fn send_file(&self, user_id: &str, file_url: &str) -> Result<ZaloMessageResponse> {
        let url = self.get_api_url("/v3.0/oa/message/file");

        let body = serde_json::json!({
            "recipient": {
                "user_id": user_id
            },
            "message": {
                "attachment": {
                    "type": "file",
                    "payload": {
                        "url": file_url
                    }
                }
            }
        });

        self.send_request(&url, body).await
    }

    /// 获取用户信息
    pub async fn get_user_info(&self, user_id: &str) -> Result<ZaloUserInfo> {
        let url = format!(
            "{}?user_id={}",
            self.get_api_url("/v3.0/getprofile"),
            user_id
        );

        let body = serde_json::json!({
            "user_id": user_id
        });

        let response = self
            .client
            .post(&url)
            .header(
                "Access-Token",
                self.config.access_token.as_deref().unwrap_or(""),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Zalo API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Zalo API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Zalo 响应失败: {}", e)))
    }

    /// 创建 OA (Official Account) 消息
    async fn send_request(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<ZaloMessageResponse> {
        let response = self
            .client
            .post(url)
            .header(
                "Access-Token",
                self.config.access_token.as_deref().unwrap_or(""),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Zalo API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Zalo API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Zalo 响应失败: {}", e)))
    }
}

#[async_trait]
impl Channel for ZaloClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WebChat
    }

    fn name(&self) -> &str {
        "zalo"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.access_token.is_none() {
            return Err(OpenClawError::Channel("Zalo access token 未配置".into()));
        }

        *self.running.write().unwrap() = true;
        tracing::info!("Zalo 客户端已启动");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("Zalo 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self.send_text(&message.chat_id, &message.content).await?;

        if let Some(error) = response.error {
            if error != 0 {
                return Err(OpenClawError::Channel(format!(
                    "Zalo 发送失败: {}",
                    response.message.unwrap_or_default()
                )));
            }
        }

        Ok(ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel_type: ChannelType::WebChat,
            chat_id: message.chat_id,
            user_id: "self".to_string(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        if self.config.access_token.is_none() {
            return Ok(false);
        }

        Ok(true)
    }
}
