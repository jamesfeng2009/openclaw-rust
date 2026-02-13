//! Telegram 通道实现

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use std::sync::Arc;

use crate::base::{Channel, ChannelHandler};
use crate::types::{ChannelMessage, ChannelType, Chat, ChatType, MessageContent, ParseMode, Sender, SendMessage};

/// Telegram 配置
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    pub bot_token: String,
}

/// Telegram 通道
pub struct TelegramChannel {
    config: TelegramConfig,
    handler: Option<Arc<dyn ChannelHandler>>,
    // teloxide Bot 实例 (实际使用时添加)
}

impl TelegramChannel {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            handler: None,
        }
    }

    pub fn with_handler(mut self, handler: Arc<dyn ChannelHandler>) -> Self {
        self.handler = Some(handler);
        self
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    fn channel_type(&self) -> ChannelType {
        ChannelType::Telegram
    }

    fn name(&self) -> &str {
        "telegram"
    }

    async fn start(&mut self) -> Result<()> {
        // TODO: 使用 teloxide 启动 bot
        // let bot = teloxide::Bot::new(&self.config.bot_token);
        tracing::info!("Telegram channel started with token: {}...", &self.config.bot_token[..10]);
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Telegram channel stopped");
        Ok(())
    }

    async fn send(&self, message: SendMessage) -> Result<ChannelMessage> {
        // TODO: 实际发送消息
        // 目前返回模拟响应
        Ok(ChannelMessage {
            id: uuid::Uuid::new_v4().to_string(),
            channel: ChannelType::Telegram,
            sender: Sender {
                id: "bot".to_string(),
                name: Some("OpenClaw Bot".to_string()),
                username: Some("openclaw_bot".to_string()),
                is_bot: true,
            },
            chat: Chat {
                id: message.chat_id,
                chat_type: ChatType::Private,
                title: None,
            },
            content: message.content.clone(),
            timestamp: chrono::Utc::now(),
            reply_to: message.reply_to,
            raw: None,
        })
    }

    async fn health_check(&self) -> Result<bool> {
        // TODO: 调用 getMe API 检查
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_telegram_channel() {
        let config = TelegramConfig {
            bot_token: "test_token".to_string(),
        };
        let mut channel = TelegramChannel::new(config);
        
        assert_eq!(channel.channel_type(), ChannelType::Telegram);
        
        channel.start().await.unwrap();
    }
}
