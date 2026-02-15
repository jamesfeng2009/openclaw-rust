//! SMS 通道实现
//!
//! 支持多种 SMS 提供商：Twilio, Nexmo, etc.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// SMS 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SmsProvider {
    Twilio,
    Nexmo,
    Custom,
}

/// SMS 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsConfig {
    /// SMS 提供商
    pub provider: SmsProvider,
    /// API Key / Account SID (Twilio)
    pub api_key: Option<String>,
    /// API Secret / Auth Token (Twilio)
    pub api_secret: Option<String>,
    /// 提供商特定的配置
    pub base_url: Option<String>,
    /// 发送者号码
    pub from_number: Option<String>,
    /// 是否启用
    #[serde(default)]
    pub enabled: bool,
}

/// SMS 消息响应
#[derive(Debug, Deserialize)]
pub struct SmsResponse {
    #[serde(default)]
    pub sid: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error_code: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

/// SMS 客户端
pub struct SmsClient {
    config: SmsConfig,
    client: reqwest::Client,
    running: std::sync::RwLock<bool>,
}

impl SmsClient {
    pub fn new(config: SmsConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
            running: std::sync::RwLock::new(false),
        }
    }

    /// 发送 SMS
    pub async fn send_sms(&self, to: &str, body: &str) -> Result<SmsResponse> {
        match self.config.provider {
            SmsProvider::Twilio => self.send_twilio(to, body).await,
            SmsProvider::Nexmo => self.send_nexmo(to, body).await,
            SmsProvider::Custom => self.send_custom(to, body).await,
        }
    }

    async fn send_twilio(&self, to: &str, body: &str) -> Result<SmsResponse> {
        let account_sid = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| OpenClawError::Channel("Twilio Account SID 未配置".into()))?;
        let auth_token = self
            .config
            .api_secret
            .as_ref()
            .ok_or_else(|| OpenClawError::Channel("Twilio Auth Token 未配置".into()))?;

        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            account_sid
        );

        let body = serde_json::json!({
            "To": to,
            "From": self.config.from_number.as_deref().unwrap_or(""),
            "Body": body
        });

        let credentials = format!("{}:{}", account_sid, auth_token);
        let encoded_credentials = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            credentials,
        );

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Basic {}", encoded_credentials))
            .form(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Twilio API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Twilio API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Twilio 响应失败: {}", e)))
    }

    async fn send_nexmo(&self, to: &str, body: &str) -> Result<SmsResponse> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| OpenClawError::Channel("Nexmo API Key 未配置".into()))?;
        let api_secret = self
            .config
            .api_secret
            .as_ref()
            .ok_or_else(|| OpenClawError::Channel("Nexmo API Secret 未配置".into()))?;

        let url = self
            .config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://rest.nexmo.com/sms/json".to_string());

        let body = serde_json::json!({
            "api_key": api_key,
            "api_secret": api_secret,
            "from": self.config.from_number.as_deref().unwrap_or("OpenClaw"),
            "to": to,
            "text": body
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Nexmo API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "Nexmo API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 Nexmo 响应失败: {}", e)))
    }

    async fn send_custom(&self, to: &str, body: &str) -> Result<SmsResponse> {
        let base_url = self
            .config
            .base_url
            .as_ref()
            .ok_or_else(|| OpenClawError::Channel("自定义 SMS 提供商 URL 未配置".into()))?;

        let url = format!("{}/send", base_url);

        let request_body = serde_json::json!({
            "to": to,
            "from": self.config.from_number.as_deref().unwrap_or(""),
            "body": body,
            "api_key": self.config.api_key.as_deref().unwrap_or(""),
            "api_secret": self.config.api_secret.as_deref().unwrap_or(""),
        });

        let response = self
            .client
            .post(&url)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("SMS API 错误: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::Http(format!(
                "SMS API 错误 ({}): {}",
                status, error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析 SMS 响应失败: {}", e)))
    }
}

#[async_trait]
impl Channel for SmsClient {
    fn channel_type(&self) -> ChannelType {
        ChannelType::SMS
    }

    fn name(&self) -> &str {
        "sms"
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.api_key.is_none() {
            return Err(OpenClawError::Channel("SMS API Key 未配置".into()));
        }

        *self.running.write().unwrap() = true;
        tracing::info!("SMS 客户端已启动，提供商: {:?}", self.config.provider);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().unwrap() = false;
        tracing::info!("SMS 客户端已停止");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let response = self.send_sms(&message.chat_id, &message.content).await?;

        if let Some(error_code) = response.error_code {
            return Err(OpenClawError::Channel(format!(
                "SMS 发送失败: {}",
                response.error_message.unwrap_or_else(|| error_code)
            )));
        }

        Ok(ChannelMessage {
            id: response.sid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            channel_type: ChannelType::SMS,
            chat_id: message.chat_id,
            user_id: self
                .config
                .from_number
                .clone()
                .unwrap_or_default(),
            content: message.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.api_key.is_some())
    }
}
