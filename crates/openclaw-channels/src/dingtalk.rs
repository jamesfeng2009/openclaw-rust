//! 钉钉通道实现
//!
//! 支持钉钉群机器人（Webhook）和企业内部机器人

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// 钉钉通道配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// Webhook 地址
    pub webhook: String,
    /// 加签密钥（可选）
    pub secret: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 钉钉机器人
pub struct DingTalkChannel {
    config: DingTalkConfig,
    client: Client,
}

impl DingTalkChannel {
    /// 创建新的钉钉通道
    pub fn new(config: DingTalkConfig) -> Self {
        let client = Client::new();
        Self { config, client }
    }

    /// 发送文本消息
    pub async fn send_text(&self, content: &str, at_mobiles: Option<Vec<String>>) -> Result<()> {
        let mut body = json!({
            "msgtype": "text",
            "text": {
                "content": content
            }
        });

        if let Some(mobiles) = at_mobiles {
            body["at"] = json!({
                "atMobiles": mobiles,
                "isAtAll": false
            });
        }

        self.send_request(&body).await
    }

    /// 发送 Markdown 消息
    pub async fn send_markdown(&self, title: &str, text: &str) -> Result<()> {
        let body = json!({
            "msgtype": "markdown",
            "markdown": {
                "title": title,
                "text": text
            }
        });

        self.send_request(&body).await
    }

    /// 发送链接消息
    pub async fn send_link(
        &self,
        title: &str,
        text: &str,
        message_url: &str,
        pic_url: Option<&str>,
    ) -> Result<()> {
        let mut body = json!({
            "msgtype": "link",
            "link": {
                "title": title,
                "text": text,
                "messageUrl": message_url
            }
        });

        if let Some(pic) = pic_url {
            body["link"]["picUrl"] = json!(pic);
        }

        self.send_request(&body).await
    }

    /// 发送 ActionCard 消息
    pub async fn send_action_card(
        &self,
        title: &str,
        text: &str,
        buttons: Vec<(&str, &str)>,
    ) -> Result<()> {
        let body = if buttons.len() == 1 {
            json!({
                "msgtype": "actionCard",
                "actionCard": {
                    "title": title,
                    "text": text,
                    "singleTitle": buttons[0].0,
                    "singleURL": buttons[0].1
                }
            })
        } else {
            json!({
                "msgtype": "actionCard",
                "actionCard": {
                    "title": title,
                    "text": text,
                    "btnOrientation": "0",
                    "btns": buttons.iter().map(|(title, url)| {
                        json!({
                            "title": title,
                            "actionURL": url
                        })
                    }).collect::<Vec<_>>()
                }
            })
        };

        self.send_request(&body).await
    }

    /// 发送请求到钉钉 API
    async fn send_request(&self, body: &serde_json::Value) -> Result<()> {
        let url = if let Some(secret) = &self.config.secret {
            // 使用加签
            let timestamp = chrono::Utc::now().timestamp_millis();
            let sign = self.sign(secret, timestamp)?;
            format!("{}&timestamp={}&sign={}", self.config.webhook, timestamp, sign)
        } else {
            self.config.webhook.clone()
        };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("钉钉 API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("钉钉 API 错误: {}", error_text)));
        }

        let result: DingTalkResponse = response.json().await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if result.errcode != 0 {
            return Err(OpenClawError::AIProvider(
                format!("钉钉 API 返回错误: {} - {}", result.errcode, result.errmsg)
            ));
        }

        Ok(())
    }

    /// 生成签名
    fn sign(&self, secret: &str, timestamp: i64) -> Result<String> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        use base64::{Engine as _, engine::general_purpose};

        let string_to_sign = format!("{}\n{}", timestamp, secret);
        
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|e| OpenClawError::Config(format!("HMAC 初始化失败: {}", e)))?;
        
        mac.update(string_to_sign.as_bytes());
        let result = mac.finalize();
        
        let signature = general_purpose::STANDARD.encode(result.into_bytes());
        Ok(urlencoding::encode(&signature).to_string())
    }
}

#[async_trait]
impl Channel for DingTalkChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::DingTalk
    }

    fn name(&self) -> &str {
        "dingtalk"
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("钉钉通道未启用".to_string()));
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
                self.send_text(&message.content, message.at_mobiles).await?;
            }
            "markdown" => {
                let title = message.title.as_deref().unwrap_or("消息");
                self.send_markdown(title, &message.content).await?;
            }
            "link" => {
                let title = message.title.as_deref().unwrap_or("链接");
                let url = message.url.as_deref().ok_or_else(|| {
                    OpenClawError::Config("链接消息需要 url 字段".to_string())
                })?;
                self.send_link(title, &message.content, url, None).await?;
            }
            _ => {
                // 默认作为文本消息发送
                self.send_text(&message.content, message.at_mobiles).await?;
            }
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::DingTalk,
            chat_id: "dingtalk".to_string(),
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

/// 钉钉 API 响应
#[derive(Debug, Deserialize)]
struct DingTalkResponse {
    errcode: i32,
    errmsg: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dingtalk_channel_creation() {
        let config = DingTalkConfig {
            webhook: "https://oapi.dingtalk.com/robot/send?access_token=test".to_string(),
            secret: None,
            enabled: true,
        };
        let channel = DingTalkChannel::new(config);
        assert_eq!(channel.name(), "dingtalk");
    }
}
