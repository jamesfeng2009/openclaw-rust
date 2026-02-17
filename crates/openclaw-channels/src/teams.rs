//! Microsoft Teams 通道实现
//!
//! 支持 Teams Webhook 和 Bot Framework

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Teams 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// Bot ID (可选)
    pub bot_id: Option<String>,
    /// Bot Password (可选)
    pub bot_password: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Teams 通道
pub struct TeamsChannel {
    config: TeamsConfig,
    client: Client,
}

impl TeamsChannel {
    /// 创建新的 Teams 通道
    pub fn new(config: TeamsConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    /// 发送消息卡片
    pub async fn send_message_card(
        &self,
        title: &str,
        text: &str,
        theme_color: Option<&str>,
    ) -> Result<()> {
        let webhook_url = self
            .config
            .webhook_url
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook URL".to_string()))?;

        let mut body = json!({
            "@type": "MessageCard",
            "@context": "http://schema.org/extensions",
            "title": title,
            "text": text
        });

        if let Some(color) = theme_color {
            body["themeColor"] = json!(color);
        }

        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Teams API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Teams API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送自适应卡片
    pub async fn send_adaptive_card(&self, card: AdaptiveCard) -> Result<()> {
        let webhook_url = self
            .config
            .webhook_url
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook URL".to_string()))?;

        let body = json!({
            "type": "message",
            "attachments": [{
                "contentType": "application/vnd.microsoft.card.adaptive",
                "contentUrl": null,
                "content": card
            }]
        });

        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Teams API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Teams API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送简单文本消息
    pub async fn send_text(&self, text: &str) -> Result<()> {
        self.send_message_card("消息", text, None).await
    }

    /// 发送带操作的卡片
    pub async fn send_action_card(
        &self,
        title: &str,
        text: &str,
        actions: Vec<CardAction>,
    ) -> Result<()> {
        let webhook_url = self
            .config
            .webhook_url
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook URL".to_string()))?;

        let body = json!({
            "@type": "MessageCard",
            "@context": "http://schema.org/extensions",
            "title": title,
            "text": text,
            "potentialAction": actions
        });

        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Teams API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Teams API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for TeamsChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Teams
    }

    fn name(&self) -> &str {
        "teams"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("Teams 通道未启用".to_string()));
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let message_id = uuid::Uuid::new_v4().to_string();

        if self.config.webhook_url.is_none() {
            return Err(OpenClawError::Config(
                "Teams 通道需要配置 webhook_url".to_string(),
            ));
        }

        match message.message_type.as_str() {
            "adaptive" | "card" => {
                let card = AdaptiveCard {
                    body: vec![AdaptiveCardBody {
                        r#type: "TextBlock".to_string(),
                        text: Some(message.content.clone()),
                        ..Default::default()
                    }],
                    ..Default::default()
                };
                self.send_adaptive_card(card).await?;
            }
            "action" => {
                let title = message.title.as_deref().unwrap_or("消息");
                let actions = vec![CardAction {
                    at_type: "OpenUri".to_string(),
                    name: "查看详情".to_string(),
                    targets: vec![json!({"os": "default", "uri": "https://example.com"})],
                }];
                self.send_action_card(title, &message.content, actions)
                    .await?;
            }
            _ => {
                let title = message.title.as_deref().unwrap_or("消息");
                self.send_message_card(title, &message.content, None)
                    .await?;
            }
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::Teams,
            chat_id: "teams".to_string(),
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

/// 自适应卡片
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdaptiveCard {
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(rename = "$schema")]
    pub schema: String,
    pub version: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub body: Vec<AdaptiveCardBody>,
}

/// 自适应卡片主体
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdaptiveCardBody {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
}

/// 卡片操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardAction {
    #[serde(rename = "@type")]
    pub at_type: String,
    pub name: String,
    pub targets: Vec<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teams_channel_creation() {
        let config = TeamsConfig {
            webhook_url: Some("https://outlook.office.com/webhook/xxx".to_string()),
            bot_id: None,
            bot_password: None,
            enabled: true,
        };
        let channel = TeamsChannel::new(config);
        assert_eq!(channel.name(), "teams");
    }
}
