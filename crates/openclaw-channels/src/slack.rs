//! Slack 通道实现
//!
//! 支持 Slack Webhook 和 Bot API

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Slack 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot Token
    pub bot_token: Option<String>,
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// App Token (可选)
    pub app_token: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Slack 通道
pub struct SlackChannel {
    config: SlackConfig,
    client: Client,
}

impl SlackChannel {
    /// 创建新的 Slack 通道
    pub fn new(config: SlackConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    /// 获取 API URL
    fn get_api_url(&self, method: &str) -> String {
        format!("https://slack.com/api/{}", method)
    }

    /// 使用 Webhook 发送消息
    pub async fn send_webhook(&self, text: &str, blocks: Option<Vec<SlackBlock>>) -> Result<()> {
        let webhook_url = self.config.webhook_url.as_ref().ok_or_else(|| {
            OpenClawError::Config("未配置 Webhook URL".to_string())
        })?;

        let mut body = json!({
            "text": text
        });

        if let Some(b) = blocks {
            body["blocks"] = json!(b);
        }

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Slack Webhook 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Slack Webhook 错误: {}", error_text)));
        }

        Ok(())
    }

    /// 发送消息到频道
    pub async fn send_to_channel(
        &self,
        channel: &str,
        text: &str,
        blocks: Option<Vec<SlackBlock>>,
    ) -> Result<()> {
        let bot_token = self.config.bot_token.as_ref().ok_or_else(|| {
            OpenClawError::Config("未配置 Bot Token".to_string())
        })?;

        let url = self.get_api_url("chat.postMessage");

        let mut body = json!({
            "channel": channel,
            "text": text
        });

        if let Some(b) = blocks {
            body["blocks"] = json!(b);
        }

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Slack API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Slack API 错误: {}", error_text)));
        }

        let result: SlackResponse = response.json().await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if !result.ok {
            return Err(OpenClawError::AIProvider(
                format!("Slack API 返回错误: {:?}", result.error)
            ));
        }

        Ok(())
    }

    /// 发送临时消息（只有用户可见）
    pub async fn send_ephemeral(&self, channel: &str, user: &str, text: &str) -> Result<()> {
        let bot_token = self.config.bot_token.as_ref().ok_or_else(|| {
            OpenClawError::Config("未配置 Bot Token".to_string())
        })?;

        let url = self.get_api_url("chat.postEphemeral");

        let body = json!({
            "channel": channel,
            "user": user,
            "text": text
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Slack API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Slack API 错误: {}", error_text)));
        }

        Ok(())
    }

    /// 添加反应表情
    pub async fn add_reaction(&self, channel: &str, timestamp: &str, emoji: &str) -> Result<()> {
        let bot_token = self.config.bot_token.as_ref().ok_or_else(|| {
            OpenClawError::Config("未配置 Bot Token".to_string())
        })?;

        let url = self.get_api_url("reactions.add");

        let body = json!({
            "channel": channel,
            "timestamp": timestamp,
            "name": emoji
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Slack API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            // 反应失败不影响主流程
            tracing::warn!("Slack reaction add failed");
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Slack
    }

    fn name(&self) -> &str {
        "slack"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("Slack 通道未启用".to_string()));
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let message_id = uuid::Uuid::new_v4().to_string();

        // 优先使用 Webhook
        if self.config.webhook_url.is_some() {
            self.send_webhook(&message.content, None).await?;
        } else if self.config.bot_token.is_some() && !message.chat_id.is_empty() {
            // 使用 Bot API
            match message.message_type.as_str() {
                "blocks" => {
                    let blocks = vec![SlackBlock {
                        r#type: "section".to_string(),
                        text: Some(SlackText {
                            r#type: "mrkdwn".to_string(),
                            text: message.content.clone(),
                            emoji: None,
                        }),
                        ..Default::default()
                    }];
                    self.send_to_channel(&message.chat_id, &message.content, Some(blocks)).await?;
                }
                _ => {
                    self.send_to_channel(&message.chat_id, &message.content, None).await?;
                }
            }
        } else {
            return Err(OpenClawError::Config(
                "Slack 通道需要配置 webhook_url 或 bot_token + chat_id".to_string()
            ));
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::Slack,
            chat_id: message.chat_id,
            user_id: "bot".to_string(),
            content: message.content.clone(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.enabled)
    }
}

/// Slack API 响应
#[derive(Debug, Deserialize)]
struct SlackResponse {
    ok: bool,
    error: Option<String>,
}

/// Slack Block
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SlackBlock {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<SlackText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_id: Option<String>,
}

/// Slack 文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackText {
    #[serde(rename = "type")]
    pub r#type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_channel_creation() {
        let config = SlackConfig {
            bot_token: Some("xoxb-xxx".to_string()),
            webhook_url: Some("https://hooks.slack.com/services/xxx".to_string()),
            app_token: None,
            enabled: true,
        };
        let channel = SlackChannel::new(config);
        assert_eq!(channel.name(), "slack");
    }
}
