//! Discord 通道实现
//!
//! 支持 Discord Bot API

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Discord 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot Token
    pub bot_token: String,
    /// Webhook URL (可选)
    pub webhook_url: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Discord 通道
pub struct DiscordChannel {
    config: DiscordConfig,
    client: Client,
}

impl DiscordChannel {
    /// 创建新的 Discord 通道
    pub fn new(config: DiscordConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    /// 获取 API URL
    fn get_api_url(&self, endpoint: &str) -> String {
        format!("https://discord.com/api/v10/{}", endpoint)
    }

    /// 发送消息到频道
    pub async fn send_to_channel(&self, channel_id: &str, content: &str) -> Result<()> {
        let url = self.get_api_url(&format!("channels/{}/messages", channel_id));
        
        let body = json!({
            "content": content
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Discord API 错误: {}", error_text)));
        }

        Ok(())
    }

    /// 发送 Embed 消息
    pub async fn send_embed(
        &self,
        channel_id: &str,
        title: &str,
        description: &str,
        color: Option<u32>,
    ) -> Result<()> {
        let url = self.get_api_url(&format!("channels/{}/messages", channel_id));
        
        let body = json!({
            "embeds": [{
                "title": title,
                "description": description,
                "color": color.unwrap_or(0x00AE86)
            }]
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Discord API 错误: {}", error_text)));
        }

        Ok(())
    }

    /// 使用 Webhook 发送消息
    pub async fn send_webhook(&self, content: &str, username: Option<&str>) -> Result<()> {
        let webhook_url = self.config.webhook_url.as_ref().ok_or_else(|| {
            OpenClawError::Config("未配置 Webhook URL".to_string())
        })?;

        let mut body = json!({
            "content": content
        });

        if let Some(name) = username {
            body["username"] = json!(name);
        }

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord Webhook 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Discord Webhook 错误: {}", error_text)));
        }

        Ok(())
    }

    /// 打字提示
    pub async fn trigger_typing(&self, channel_id: &str) -> Result<()> {
        let url = self.get_api_url(&format!("channels/{}/typing", channel_id));
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            // 打字提示失败不影响主流程
            tracing::warn!("Discord typing trigger failed");
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Discord
    }

    fn name(&self) -> &str {
        "discord"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("Discord 通道未启用".to_string()));
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
        } else if !message.chat_id.is_empty() {
            // 使用 Bot API
            match message.message_type.as_str() {
                "embed" => {
                    let title = message.title.as_deref().unwrap_or("消息");
                    self.send_embed(&message.chat_id, title, &message.content, None).await?;
                }
                _ => {
                    self.send_to_channel(&message.chat_id, &message.content).await?;
                }
            }
        } else {
            return Err(OpenClawError::Config(
                "Discord 通道需要配置 webhook_url 或提供 chat_id".to_string()
            ));
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::Discord,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_channel_creation() {
        let config = DiscordConfig {
            bot_token: "BOT_TOKEN".to_string(),
            webhook_url: Some("https://discord.com/api/webhooks/xxx/yyy".to_string()),
            enabled: true,
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
    }
}
