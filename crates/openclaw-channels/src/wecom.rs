//! 企业微信通道实现
//!
//! 支持企业微信群机器人（Webhook）

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// 企业微信通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    /// Webhook 地址
    pub webhook: String,
    /// 是否启用
    pub enabled: bool,
}

/// 企业微信机器人
pub struct WeComChannel {
    config: WeComConfig,
    client: Client,
}

impl WeComChannel {
    /// 创建新的企业微信通道
    pub fn new(config: WeComConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    /// 发送文本消息
    pub async fn send_text(
        &self,
        content: &str,
        mentioned_list: Option<Vec<String>>,
    ) -> Result<()> {
        let mut body = json!({
            "msgtype": "text",
            "text": {
                "content": content
            }
        });

        if let Some(list) = mentioned_list {
            body["text"]["mentioned_list"] = json!(list);
        }

        self.send_request(&body).await
    }

    /// 发送 Markdown 消息
    pub async fn send_markdown(&self, content: &str) -> Result<()> {
        let body = json!({
            "msgtype": "markdown",
            "markdown": {
                "content": content
            }
        });

        self.send_request(&body).await
    }

    /// 发送图片消息
    pub async fn send_image(&self, base64_data: &str, md5: &str) -> Result<()> {
        let body = json!({
            "msgtype": "image",
            "image": {
                "base64": base64_data,
                "md5": md5
            }
        });

        self.send_request(&body).await
    }

    /// 发送图文消息
    pub async fn send_news(&self, articles: Vec<NewsArticle>) -> Result<()> {
        let body = json!({
            "msgtype": "news",
            "news": {
                "articles": articles
            }
        });

        self.send_request(&body).await
    }

    /// 发送文件消息
    pub async fn send_file(&self, media_id: &str) -> Result<()> {
        let body = json!({
            "msgtype": "file",
            "file": {
                "media_id": media_id
            }
        });

        self.send_request(&body).await
    }

    /// 发送请求到企业微信 API
    async fn send_request(&self, body: &serde_json::Value) -> Result<()> {
        let response = self
            .client
            .post(&self.config.webhook)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("企业微信 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "企业微信 API 错误: {}",
                error_text
            )));
        }

        let result: WeComResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.errcode != 0 {
            return Err(OpenClawError::AIProvider(format!(
                "企业微信 API 返回错误: {} - {}",
                result.errcode, result.errmsg
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl Channel for WeComChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WeCom
    }

    fn name(&self) -> &str {
        "wecom"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("企业微信通道未启用".to_string()));
        }
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        let message_id = uuid::Uuid::new_v4().to_string();

        // 根据消息类型发送
        match message.message_type.as_str() {
            "text" => {
                self.send_text(&message.content, message.mentioned_list)
                    .await?;
            }
            "markdown" => {
                self.send_markdown(&message.content).await?;
            }
            "image" => {
                let base64 = message
                    .base64
                    .as_deref()
                    .ok_or_else(|| OpenClawError::Config("图片消息需要 base64 字段".to_string()))?;
                let md5 = message
                    .md5
                    .as_deref()
                    .ok_or_else(|| OpenClawError::Config("图片消息需要 md5 字段".to_string()))?;
                self.send_image(base64, md5).await?;
            }
            "news" => {
                let articles = message.articles.ok_or_else(|| {
                    OpenClawError::Config("图文消息需要 articles 字段".to_string())
                })?;
                self.send_news(articles).await?;
            }
            "file" => {
                let media_id = message.media_id.as_deref().ok_or_else(|| {
                    OpenClawError::Config("文件消息需要 media_id 字段".to_string())
                })?;
                self.send_file(media_id).await?;
            }
            _ => {
                // 默认作为文本消息发送
                self.send_text(&message.content, message.mentioned_list)
                    .await?;
            }
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::WeCom,
            chat_id: "wecom".to_string(),
            user_id: "bot".to_string(),
            content: message.content.clone(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // 发送测试消息（不会真正发送）
        Ok(self.config.enabled)
    }
}

/// 企业微信 API 响应
#[derive(Debug, Deserialize)]
struct WeComResponse {
    errcode: i32,
    errmsg: String,
}

/// 图文消息文章
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsArticle {
    /// 标题
    pub title: String,
    /// 描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// URL
    pub url: String,
    /// 图片 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub picurl: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wecom_channel_creation() {
        let config = WeComConfig {
            webhook: "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key=test".to_string(),
            enabled: true,
        };
        let channel = WeComChannel::new(config);
        assert_eq!(channel.name(), "wecom");
    }
}
