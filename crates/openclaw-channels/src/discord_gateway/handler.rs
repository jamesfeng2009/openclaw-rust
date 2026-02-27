//! Discord Gateway 事件处理器
//!
//! 使用 serenity 库处理 Discord Gateway 事件

#[cfg(feature = "discord")]
use serenity::{
    all::Message,
    async_trait,
    client::Context,
};
#[cfg(feature = "discord")]
use serenity::prelude::EventHandler;
#[cfg(feature = "discord")]
use tokio::sync::mpsc;

#[cfg(feature = "discord")]
use super::types::{DiscordGatewayEvent, DiscordGatewayMessage, DiscordGatewayUser};

#[cfg(feature = "discord")]
pub struct GatewayEventHandler {
    event_sender: mpsc::UnboundedSender<DiscordGatewayEvent>,
}

#[cfg(feature = "discord")]
impl GatewayEventHandler {
    pub fn new(sender: mpsc::UnboundedSender<DiscordGatewayEvent>) -> Self {
        Self {
            event_sender: sender,
        }
    }
}

#[cfg(feature = "discord")]
#[async_trait]
impl EventHandler for GatewayEventHandler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        let gateway_msg = DiscordGatewayMessage {
            id: msg.id.to_string(),
            channel_id: msg.channel_id.to_string(),
            guild_id: msg.guild_id.map(|g| g.to_string()),
            author: DiscordGatewayUser {
                id: msg.author.id.to_string(),
                username: msg.author.name.clone(),
                discriminator: msg.author.discriminator.map(|d| d.get().to_string()),
                avatar: msg.author.avatar.map(|a| a.to_string()),
                bot: msg.author.bot,
            },
            content: msg.content.clone(),
            timestamp: msg.timestamp.to_string(),
            mentions: msg.mentions.iter().map(|u| DiscordGatewayUser {
                id: u.id.to_string(),
                username: u.name.clone(),
                discriminator: u.discriminator.map(|d| d.get().to_string()),
                avatar: u.avatar.map(|a| a.to_string()),
                bot: u.bot,
            }).collect(),
            mention_roles: msg.mention_roles.iter().map(|r| r.to_string()).collect(),
            is_bot: msg.author.bot,
        };

        let _ = self.event_sender.send(DiscordGatewayEvent::MessageCreate(gateway_msg));
    }
}
