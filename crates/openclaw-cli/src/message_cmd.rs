//! 消息发送 CLI 工具

use anyhow::Result;
use clap::Subcommand;
use openclaw_channels::{ChannelType, SendMessage};

#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Send a message to a channel
    Send {
        /// Channel type (telegram, discord, slack, whatsapp, signal, teams, dingtalk, wecom, feishu, matrix)
        #[arg(short, long)]
        channel: String,
        /// Recipient (chat_id, phone number, etc.)
        #[arg(short, long)]
        to: String,
        /// Message content
        #[arg(short, long)]
        message: String,
        /// Message type (text, markdown, image, etc.)
        #[arg(long, default_value = "text")]
        message_type: String,
    },
}

impl MessageCommand {
    pub async fn execute(&self) -> Result<()> {
        match self {
            MessageCommand::Send {
                channel,
                to,
                message,
                message_type,
            } => {
                self.send_message(channel, to, message, message_type)
                    .await?;
            }
        }
        Ok(())
    }

    async fn send_message(
        &self,
        channel_type: &str,
        to: &str,
        message: &str,
        message_type: &str,
    ) -> Result<()> {
        let channel = match channel_type.to_lowercase().as_str() {
            "telegram" => ChannelType::Telegram,
            "discord" => ChannelType::Discord,
            "slack" => ChannelType::Slack,
            "whatsapp" => ChannelType::WhatsApp,
            "signal" => ChannelType::Signal,
            "teams" => ChannelType::Teams,
            "dingtalk" | "ding" => ChannelType::DingTalk,
            "wecom" => ChannelType::WeCom,
            "feishu" => ChannelType::Feishu,
            "matrix" => ChannelType::Matrix,
            "webchat" => ChannelType::WebChat,
            "email" => ChannelType::Email,
            "sms" => ChannelType::SMS,
            _ => {
                anyhow::bail!("Unsupported channel type: {}", channel_type);
            }
        };

        let send_msg = SendMessage {
            chat_id: to.to_string(),
            message_type: message_type.to_string(),
            content: message.to_string(),
            title: None,
            url: None,
            at_mobiles: None,
            mentioned_list: None,
            base64: None,
            md5: None,
            articles: None,
            media_id: None,
        };

        println!(
            "Sending message to {} via {}: {}",
            to, channel_type, message
        );
        println!(
            "Channel: {:?}, ChatId: {}, MessageType: {}",
            channel, send_msg.chat_id, send_msg.message_type
        );

        println!("✅ Message queued for sending (Gateway mode not yet implemented)");
        println!("To send messages, start the gateway and use the WebSocket API");

        Ok(())
    }
}
