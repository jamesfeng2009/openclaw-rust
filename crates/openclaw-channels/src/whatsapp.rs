//! WhatsApp Cloud API 通道实现
//!
//! 使用 Meta WhatsApp Business API 发送消息

use crate::{Channel, ChannelMessage, ChannelType, SendMessage};
use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// WhatsApp Cloud API 配置
#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    /// WhatsApp Business Account ID (WABA ID)
    pub business_account_id: String,
    /// Phone Number ID
    pub phone_number_id: String,
    /// Access Token (永久或临时)
    pub access_token: String,
    /// Webhook Verify Token (用于接收消息验证)
    pub verify_token: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

impl Default for WhatsAppConfig {
    fn default() -> Self {
        Self {
            business_account_id: String::new(),
            phone_number_id: String::new(),
            access_token: String::new(),
            verify_token: None,
            enabled: false,
        }
    }
}

/// WhatsApp 通道
pub struct WhatsAppChannel {
    config: WhatsAppConfig,
    client: Client,
}

impl WhatsAppChannel {
    const API_BASE: &'static str = "https://graph.facebook.com/v18.0";

    /// 创建新的 WhatsApp 通道
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// 获取 API URL
    fn get_api_url(&self) -> String {
        format!(
            "{}/{}/messages",
            Self::API_BASE,
            self.config.phone_number_id
        )
    }

    /// 发送文本消息
    pub async fn send_text(&self, to: &str, text: &str) -> Result<()> {
        let url = self.get_api_url();

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "text",
            "text": {
                "preview_url": false,
                "body": text
            }
        });

        self.send_request(&url, &body).await
    }

    /// 发送模板消息
    pub async fn send_template(
        &self,
        to: &str,
        template_name: &str,
        language_code: &str,
        components: Option<Vec<serde_json::Value>>,
    ) -> Result<()> {
        let url = self.get_api_url();

        let mut template = json!({
            "name": template_name,
            "language": {
                "code": language_code
            }
        });

        if let Some(comp) = components {
            template["components"] = json!(comp);
        }

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "template",
            "template": template
        });

        self.send_request(&url, &body).await
    }

    /// 发送图片消息
    pub async fn send_image(&self, to: &str, image_url: &str, caption: Option<&str>) -> Result<()> {
        let url = self.get_api_url();

        let mut image = json!({
            "link": image_url
        });

        if let Some(cap) = caption {
            image["caption"] = json!(cap);
        }

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "image",
            "image": image
        });

        self.send_request(&url, &body).await
    }

    /// 发送文档消息
    pub async fn send_document(
        &self,
        to: &str,
        document_url: &str,
        filename: &str,
        caption: Option<&str>,
    ) -> Result<()> {
        let url = self.get_api_url();

        let mut document = json!({
            "link": document_url,
            "filename": filename
        });

        if let Some(cap) = caption {
            document["caption"] = json!(cap);
        }

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "document",
            "document": document
        });

        self.send_request(&url, &body).await
    }

    /// 发送交互式按钮消息
    pub async fn send_interactive_buttons(
        &self,
        to: &str,
        body_text: &str,
        buttons: Vec<WhatsAppButton>,
    ) -> Result<()> {
        let url = self.get_api_url();

        let button_rows: Vec<serde_json::Value> = buttons
            .into_iter()
            .enumerate()
            .map(|(i, btn)| {
                json!({
                    "type": "reply",
                    "reply": {
                        "id": btn.id.unwrap_or_else(|| format!("btn_{}", i)),
                        "title": btn.title
                    }
                })
            })
            .collect();

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "interactive",
            "interactive": {
                "type": "button",
                "body": {
                    "text": body_text
                },
                "action": {
                    "buttons": button_rows
                }
            }
        });

        self.send_request(&url, &body).await
    }

    /// 发送列表消息
    pub async fn send_interactive_list(
        &self,
        to: &str,
        body_text: &str,
        button_text: &str,
        sections: Vec<WhatsAppListSection>,
    ) -> Result<()> {
        let url = self.get_api_url();

        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": to,
            "type": "interactive",
            "interactive": {
                "type": "list",
                "body": {
                    "text": body_text
                },
                "action": {
                    "button": button_text,
                    "sections": sections.iter().map(|s| json!({
                        "title": s.title,
                        "rows": s.rows.iter().map(|r| json!({
                            "id": r.id,
                            "title": r.title,
                            "description": r.description
                        })).collect::<Vec<_>>()
                    })).collect::<Vec<_>>()
                }
            }
        });

        self.send_request(&url, &body).await
    }

    /// 发送请求
    async fn send_request(&self, url: &str, body: &serde_json::Value) -> Result<()> {
        let response = self
            .client
            .post(url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.access_token),
            )
            .header("Content-Type", "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("WhatsApp API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "WhatsApp API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 标记消息为已读
    pub async fn mark_as_read(&self, message_id: &str) -> Result<()> {
        let url = self.get_api_url();

        let body = json!({
            "messaging_product": "whatsapp",
            "status": "read",
            "message_id": message_id
        });

        self.send_request(&url, &body).await
    }
}

/// WhatsApp 按钮
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppButton {
    pub title: String,
    pub id: Option<String>,
}

/// WhatsApp 列表行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppListRow {
    pub id: String,
    pub title: String,
    pub description: String,
}

/// WhatsApp 列表区块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppListSection {
    pub title: String,
    pub rows: Vec<WhatsAppListRow>,
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::WhatsApp
    }

    fn name(&self) -> &str {
        "WhatsApp"
    }

    async fn start(&mut self) -> Result<()> {
        // WhatsApp Cloud API 是无状态的，不需要启动
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        // WhatsApp Cloud API 是无状态的，不需要停止
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("WhatsApp 通道未启用".to_string()));
        }

        let message_id = uuid::Uuid::new_v4().to_string();
        let to = if message.chat_id.is_empty() {
            return Err(OpenClawError::Config(
                "WhatsApp 需要提供接收者号码 (chat_id)".to_string(),
            ));
        } else {
            &message.chat_id
        };

        match message.message_type.as_str() {
            "text" => {
                self.send_text(to, &message.content).await?;
            }
            "image" => {
                let url = message.content.clone();
                let caption = message.title.as_deref();
                self.send_image(to, &url, caption).await?;
            }
            "document" => {
                let parts: Vec<&str> = message.content.splitn(2, '|').collect();
                let doc_url = parts.get(0).unwrap_or(&"");
                let filename = parts.get(1).unwrap_or(&"document.pdf");
                self.send_document(to, doc_url, filename, message.title.as_deref())
                    .await?;
            }
            "template" => {
                // 格式: template_name|language_code
                let parts: Vec<&str> = message.content.splitn(2, '|').collect();
                let template_name = parts.get(0).unwrap_or(&"hello_world");
                let language_code = parts.get(1).unwrap_or(&"en_US");
                self.send_template(to, template_name, language_code, None)
                    .await?;
            }
            "interactive" | "buttons" => {
                // 简单按钮，content 格式: "body_text|button1,button2,button3"
                let parts: Vec<&str> = message.content.splitn(2, '|').collect();
                let body_text = parts.get(0).unwrap_or(&"");
                let button_titles: Vec<&str> = parts
                    .get(1)
                    .unwrap_or(&"")
                    .split(',')
                    .map(|s| s.trim())
                    .collect();

                let buttons: Vec<WhatsAppButton> = button_titles
                    .iter()
                    .enumerate()
                    .map(|(i, t)| WhatsAppButton {
                        title: t.to_string(),
                        id: Some(format!("btn_{}", i)),
                    })
                    .collect();

                self.send_interactive_buttons(to, body_text, buttons)
                    .await?;
            }
            _ => {
                // 默认发送文本
                self.send_text(to, &message.content).await?;
            }
        }

        Ok(ChannelMessage {
            id: message_id,
            channel_type: ChannelType::WhatsApp,
            chat_id: message.chat_id,
            user_id: self.config.phone_number_id.clone(),
            content: message.content.clone(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // 简单检查配置是否有效
        Ok(self.config.enabled
            && !self.config.phone_number_id.is_empty()
            && !self.config.access_token.is_empty())
    }
}

/// WhatsApp Webhook 消息（用于接收）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppWebhookMessage {
    pub entry: Vec<WhatsAppEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppEntry {
    pub id: String,
    pub changes: Vec<WhatsAppChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppChange {
    pub field: String,
    pub value: WhatsAppValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppValue {
    pub messaging_product: String,
    pub metadata: WhatsAppMetadata,
    pub contacts: Option<Vec<WhatsAppContact>>,
    pub messages: Option<Vec<WhatsAppReceivedMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppMetadata {
    pub display_phone_number: String,
    pub phone_number_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppContact {
    pub profile: WhatsAppProfile,
    pub wa_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppProfile {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppReceivedMessage {
    pub from: String,
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: Option<WhatsAppText>,
    pub image: Option<WhatsAppMedia>,
    pub document: Option<WhatsAppMedia>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppText {
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppMedia {
    pub id: String,
    pub mime_type: Option<String>,
    pub sha256: Option<String>,
    pub caption: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = WhatsAppConfig::default();
        assert!(!config.enabled);
        assert!(config.phone_number_id.is_empty());
    }

    #[test]
    fn test_channel_type() {
        let config = WhatsAppConfig::default();
        let channel = WhatsAppChannel::new(config);
        assert_eq!(channel.channel_type(), ChannelType::WhatsApp);
    }
}
