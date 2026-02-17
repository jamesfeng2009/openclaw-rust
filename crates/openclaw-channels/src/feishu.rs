//! 飞书通道实现
//!
//! 支持飞书机器人（Webhook 和 Bot API）

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// 飞书通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID
    pub app_id: String,
    /// App Secret
    pub app_secret: String,
    /// Webhook 地址（可选，用于简单场景）
    pub webhook: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 飞书机器人
pub struct FeishuChannel {
    config: FeishuConfig,
    client: Client,
    access_token: Option<String>,
}

impl FeishuChannel {
    /// 创建新的飞书通道
    pub fn new(config: FeishuConfig) -> Self {
        let client = Client::new();
        Self {
            config,
            client,
            access_token: None,
        }
    }

    /// 获取 tenant_access_token
    pub async fn get_access_token(&mut self) -> Result<String> {
        if let Some(token) = &self.access_token {
            return Ok(token.clone());
        }

        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";

        let body = json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret
        });

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        let result: FeishuTokenResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.code != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 返回错误: {} - {}",
                result.code, result.msg
            )));
        }

        self.access_token = Some(result.tenant_access_token.clone());
        Ok(result.tenant_access_token)
    }

    /// 发送文本消息（Webhook 方式）
    pub async fn send_text_webhook(&self, content: &str) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "text",
            "content": {
                "text": content
            }
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送富文本消息
    pub async fn send_post(&self, title: &str, content: Vec<PostContent>) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "post",
            "content": {
                "post": {
                    "zh_cn": {
                        "title": title,
                        "content": content
                    }
                }
            }
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 发送交互式卡片消息
    pub async fn send_interactive(&self, card: CardContent) -> Result<()> {
        let webhook = self
            .config
            .webhook
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook 地址".to_string()))?;

        let body = json!({
            "msg_type": "interactive",
            "card": card
        });

        let response = self
            .client
            .post(webhook)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("飞书 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "飞书 API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Feishu
    }

    fn name(&self) -> &str {
        "feishu"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("飞书通道未启用".to_string()));
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let message_id = uuid::Uuid::new_v4().to_string();

        // 优先使用 Webhook
        if self.config.webhook.is_some() {
            match message.message_type.as_str() {
                "text" => {
                    self.send_text_webhook(&message.content).await?;
                }
                "post" | "markdown" => {
                    let title = message.title.as_deref().unwrap_or("消息");
                    let content = vec![PostContent {
                        tag: "text".to_string(),
                        text: Some(message.content.clone()),
                        ..Default::default()
                    }];
                    self.send_post(title, content).await?;
                }
                "interactive" | "card" => {
                    let card = CardContent {
                        elements: vec![CardContentElement {
                            tag: "div".to_string(),
                            text: Some(CardText {
                                tag: "plain_text".to_string(),
                                content: message.content.clone(),
                            }),
                            ..Default::default()
                        }],
                        ..Default::default()
                    };
                    self.send_interactive(card).await?;
                }
                _ => {
                    self.send_text_webhook(&message.content).await?;
                }
            }
        } else {
            return Err(OpenClawError::Config(
                "飞书通道需要配置 webhook 或 app_id/app_secret".to_string(),
            ));
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::Feishu,
            chat_id: "feishu".to_string(),
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

/// 飞书 Token 响应
#[derive(Debug, Deserialize)]
struct FeishuTokenResponse {
    code: i32,
    msg: String,
    tenant_access_token: String,
    expire: i32,
}

/// 富文本内容
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostContent {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// 卡片内容
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardContent {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<CardContentElement>,
}

/// 卡片内容元素
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CardContentElement {
    pub tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<CardText>,
}

/// 卡片文本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardText {
    pub tag: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feishu_channel_creation() {
        let config = FeishuConfig {
            app_id: "cli_xxx".to_string(),
            app_secret: "secret".to_string(),
            webhook: Some("https://open.feishu.cn/open-apis/bot/v2/hook/xxx".to_string()),
            enabled: true,
        };
        let channel = FeishuChannel::new(config);
        assert_eq!(channel.name(), "feishu");
    }
}
