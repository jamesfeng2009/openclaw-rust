//! Discord Gateway 事件类型
//!
//! 定义从 Discord Gateway 接收的事件类型

#[cfg(feature = "discord")]
use serde::{Deserialize, Serialize};
use crate::types::ChannelMessage;

#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGatewayMessage {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub author: DiscordGatewayUser,
    pub content: String,
    pub timestamp: String,
    pub mentions: Vec<DiscordGatewayUser>,
    pub mention_roles: Vec<String>,
    pub is_bot: bool,
}

#[cfg(feature = "discord")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGatewayUser {
    pub id: String,
    pub username: String,
    pub discriminator: Option<String>,
    pub avatar: Option<String>,
    pub bot: bool,
}

#[cfg(feature = "discord")]
impl From<DiscordGatewayMessage> for ChannelMessage {
    fn from(msg: DiscordGatewayMessage) -> Self {
        ChannelMessage {
            id: msg.id,
            channel_type: crate::types::ChannelType::Discord,
            chat_id: msg.channel_id,
            user_id: msg.author.id,
            content: msg.content,
            timestamp: chrono::Utc::now(),
            metadata: None,
        }
    }
}

#[cfg(feature = "discord")]
#[derive(Debug, Clone)]
pub enum DiscordGatewayEvent {
    MessageCreate(DiscordGatewayMessage),
    MessageUpdate { id: String, channel_id: String, content: String },
    MessageDelete { id: String, channel_id: String },
    PresenceUpdate { user_id: String, status: String },
    VoiceStateUpdate { user_id: String, channel_id: Option<String> },
}

#[cfg(feature = "discord")]
impl DiscordGatewayEvent {
    pub fn into_channel_message(self) -> Option<ChannelMessage> {
        match self {
            DiscordGatewayEvent::MessageCreate(msg) => Some(msg.into()),
            _ => None,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "discord")]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_message_conversion() {
        use crate::types::ChannelType;
        
        let msg = DiscordGatewayMessage {
            id: "12345".to_string(),
            channel_id: "channel_123".to_string(),
            guild_id: Some("guild_123".to_string()),
            author: DiscordGatewayUser {
                id: "user_123".to_string(),
                username: "testuser".to_string(),
                discriminator: Some("0001".to_string()),
                avatar: None,
                bot: false,
            },
            content: "Hello world".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            mentions: vec![],
            mention_roles: vec![],
            is_bot: false,
        };

        let channel_msg: ChannelMessage = msg.into();
        assert_eq!(channel_msg.id, "12345");
        assert_eq!(channel_msg.chat_id, "channel_123");
        assert_eq!(channel_msg.content, "Hello world");
        assert_eq!(channel_msg.channel_type, ChannelType::Discord);
    }

    #[test]
    fn test_gateway_event_to_channel_message() {
        use crate::types::ChannelType;
        
        let event = DiscordGatewayEvent::MessageCreate(DiscordGatewayMessage {
            id: "msg_001".to_string(),
            channel_id: "ch_001".to_string(),
            guild_id: None,
            author: DiscordGatewayUser {
                id: "author_001".to_string(),
                username: "alice".to_string(),
                discriminator: Some("1234".to_string()),
                avatar: None,
                bot: false,
            },
            content: "@goclaw help".to_string(),
            timestamp: "2024-01-01T12:00:00Z".to_string(),
            mentions: vec![],
            mention_roles: vec![],
            is_bot: false,
        });

        let channel_msg = event.into_channel_message().unwrap();
        assert_eq!(channel_msg.channel_type, ChannelType::Discord);
        assert_eq!(channel_msg.content, "@goclaw help");
    }

    #[test]
    fn test_gateway_event_ignore_non_message() {
        let event = DiscordGatewayEvent::MessageUpdate {
            id: "msg_001".to_string(),
            channel_id: "ch_001".to_string(),
            content: "updated content".to_string(),
        };

        let result = event.into_channel_message();
        assert!(result.is_none());
    }
}
