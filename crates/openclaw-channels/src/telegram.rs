//! Telegram Bot 完整实现

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use openclaw_core::{OpenClawError, Result};

use crate::base::Channel;
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Telegram 配置
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub enabled: bool,
}

/// Telegram Bot
pub struct TelegramBot {
    config: TelegramConfig,
    client: reqwest::Client,
    offset: std::sync::RwLock<i64>,
    running: std::sync::RwLock<bool>,
}

impl TelegramBot {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            offset: std::sync::RwLock::new(0),
            running: std::sync::RwLock::new(false),
            config,
        }
    }

    fn get_api_url(&self, method: &str) -> String {
        format!(
            "https://api.telegram.org/bot{}/{}",
            self.config.bot_token, method
        )
    }

    /// 获取更新
    async fn get_updates(&self) -> Result<Vec<TelegramUpdate>> {
        let offset = *self.offset.read().unwrap();

        let url = format!(
            "{}?offset={}&timeout=30",
            self.get_api_url("getUpdates"),
            offset
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Telegram API 错误: {}", e)))?;

        let result: TelegramResponse<Vec<TelegramUpdate>> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if !result.ok {
            return Err(OpenClawError::Channel("Telegram API 返回错误".into()));
        }

        Ok(result.result.unwrap_or_default())
    }

    /// 发送消息
    pub async fn send_text(
        &self,
        chat_id: i64,
        text: &str,
        parse_mode: Option<&str>,
    ) -> Result<TelegramMessage> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });

        if let Some(mode) = parse_mode {
            body["parse_mode"] = serde_json::json!(mode);
        }

        let response = self
            .client
            .post(self.get_api_url("sendMessage"))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Telegram API 错误: {}", e)))?;

        let result: TelegramResponse<TelegramMessage> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        if !result.ok {
            return Err(OpenClawError::Channel("发送消息失败".into()));
        }

        result
            .result
            .ok_or_else(|| OpenClawError::Channel("无返回消息".into()))
    }

    /// 发送图片
    pub async fn send_photo(
        &self,
        chat_id: i64,
        photo_url: &str,
        caption: Option<&str>,
    ) -> Result<TelegramMessage> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "photo": photo_url,
        });

        if let Some(cap) = caption {
            body["caption"] = serde_json::json!(cap);
        }

        let response = self
            .client
            .post(self.get_api_url("sendPhoto"))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Telegram API 错误: {}", e)))?;

        let result: TelegramResponse<TelegramMessage> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        result
            .result
            .ok_or_else(|| OpenClawError::Channel("发送图片失败".into()))
    }

    /// 发送文档
    pub async fn send_document(
        &self,
        chat_id: i64,
        document_url: &str,
        caption: Option<&str>,
    ) -> Result<TelegramMessage> {
        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "document": document_url,
        });

        if let Some(cap) = caption {
            body["caption"] = serde_json::json!(cap);
        }

        let response = self
            .client
            .post(self.get_api_url("sendDocument"))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Telegram API 错误: {}", e)))?;

        let result: TelegramResponse<TelegramMessage> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        result
            .result
            .ok_or_else(|| OpenClawError::Channel("发送文档失败".into()))
    }

    /// 获取 bot 信息
    async fn get_me(&self) -> Result<TelegramUser> {
        let response = self
            .client
            .get(self.get_api_url("getMe"))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Telegram API 错误: {}", e)))?;

        let result: TelegramResponse<TelegramUser> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        result
            .result
            .ok_or_else(|| OpenClawError::Channel("获取 Bot 信息失败".into()))
    }

    /// 处理更新
    async fn handle_update(&self, update: TelegramUpdate) -> Result<Option<ChannelMessage>> {
        let message = match update.message {
            Some(m) => m,
            None => return Ok(None),
        };

        // 更新 offset
        {
            let mut offset = self.offset.write().unwrap();
            *offset = update.update_id + 1;
        }

        // 提取内容
        let content = if let Some(text) = &message.text {
            text.clone()
        } else if let Some(caption) = &message.caption {
            caption.clone()
        } else {
            "[媒体消息]".to_string()
        };

        // 构建元数据
        let metadata = serde_json::to_value(&message).ok();

        Ok(Some(ChannelMessage {
            id: message.message_id.to_string(),
            channel_type: ChannelType::Telegram,
            chat_id: message.chat.id.to_string(),
            user_id: message.from.id.to_string(),
            content,
            timestamp: message.date,
            metadata,
        }))
    }
}

#[async_trait]
impl Channel for TelegramBot {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }

    fn name(&self) -> &str {
        "Telegram"
    }

    async fn start(&mut self) -> Result<()> {
        tracing::info!("Telegram Bot 启动中...");

        // 设置运行状态
        {
            let mut running = self.running.write().unwrap();
            *running = true;
        }

        // 获取 bot 信息
        let me = self.get_me().await?;
        tracing::info!("Telegram Bot 已连接: @{}", me.username.unwrap_or_default());

        // 开始轮询
        loop {
            // 检查是否应该停止
            {
                let running = self.running.read().unwrap();
                if !*running {
                    break;
                }
            }

            // 获取更新
            match self.get_updates().await {
                Ok(updates) => {
                    for update in updates {
                        if let Some(_msg) = self.handle_update(update).await? {
                            // 这里可以添加消息处理逻辑
                            tracing::debug!("收到 Telegram 消息");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("获取更新失败: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Telegram Bot 停止中...");
        {
            let mut running = self.running.write().unwrap();
            *running = false;
        }
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        if !self.config.enabled {
            return Err(OpenClawError::Config("Telegram 通道未启用".to_string()));
        }

        let chat_id: i64 = message
            .chat_id
            .parse()
            .map_err(|_| OpenClawError::Config("无效的 chat_id，需要数字格式".into()))?;

        let parse_mode = message.message_type.as_str();
        let mode = match parse_mode {
            "markdown" => Some("Markdown"),
            "markdownv2" | "markdown_v2" => Some("MarkdownV2"),
            "html" => Some("HTML"),
            _ => None,
        };

        // 根据消息类型发送
        match message.message_type.as_str() {
            "image" | "photo" => {
                let sent = self
                    .send_photo(chat_id, &message.content, message.title.as_deref())
                    .await?;
                self.create_channel_message(sent, &message.content)
            }
            "document" | "file" => {
                let sent = self
                    .send_document(chat_id, &message.content, message.title.as_deref())
                    .await?;
                self.create_channel_message(sent, &message.content)
            }
            _ => {
                let sent = self.send_text(chat_id, &message.content, mode).await?;
                self.create_channel_message(sent, &message.content)
            }
        }
    }

    async fn health_check(&self) -> Result<bool> {
        match self.get_me().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

impl TelegramBot {
    fn create_channel_message(
        &self,
        sent: TelegramMessage,
        content: &str,
    ) -> Result<ChannelMessage> {
        Ok(ChannelMessage {
            id: sent.message_id.to_string(),
            channel_type: ChannelType::Telegram,
            chat_id: sent.chat.id.to_string(),
            user_id: sent.from.id.to_string(),
            content: content.to_string(),
            timestamp: sent.date,
            metadata: None,
        })
    }
}

// ============== Telegram API 类型 ==============

#[derive(Debug, Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramMessage {
    message_id: i64,
    from: TelegramUser,
    chat: TelegramChat,
    date: chrono::DateTime<chrono::Utc>,
    text: Option<String>,
    photo: Option<Vec<TelegramPhotoSize>>,
    caption: Option<String>,
    document: Option<TelegramDocument>,
    #[serde(rename = "reply_to_message")]
    reply_to: Option<Box<TelegramMessage>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramUser {
    id: i64,
    is_bot: bool,
    first_name: String,
    last_name: Option<String>,
    username: Option<String>,
}

impl TelegramUser {
    fn full_name(&self) -> String {
        match &self.last_name {
            Some(last) => format!("{} {}", self.first_name, last),
            None => self.first_name.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
    title: Option<String>,
    username: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramPhotoSize {
    file_id: String,
    width: i32,
    height: i32,
}

#[derive(Debug, Deserialize, Serialize)]
struct TelegramDocument {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telegram_config() {
        let config = TelegramConfig {
            bot_token: "test_token".to_string(),
            enabled: true,
        };
        assert!(config.enabled);
    }

    #[test]
    fn test_user_full_name() {
        let user = TelegramUser {
            id: 123,
            is_bot: false,
            first_name: "Test".to_string(),
            last_name: Some("User".to_string()),
            username: Some("testuser".to_string()),
        };
        assert_eq!(user.full_name(), "Test User");
    }
}
