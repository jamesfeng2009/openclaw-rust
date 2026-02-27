//! Discord 通道实现
//!
//! 支持 Discord Bot API

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::{OpenClawError, Result};
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[cfg(feature = "discord")]
use crate::discord_gateway::{DiscordGatewayClient, DiscordGatewayEvent};
use crate::base::{Channel, ChannelEvent};
use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// Discord @提及
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMention {
    /// 提及类型: user, role, channel
    pub mention_type: String,
    /// 提及的 ID
    pub id: String,
    /// 提及的名称
    pub name: String,
}

/// Discord 消息解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessageParseResult {
    /// 原始消息内容
    pub raw_content: String,
    /// 清理后的文本内容
    pub clean_content: String,
    /// 解析出的 @提及列表
    pub mentions: Vec<DiscordMention>,
    /// 频道 ID
    pub channel_id: Option<String>,
    /// 消息 ID
    pub message_id: Option<String>,
    /// 服务器 ID
    pub guild_id: Option<String>,
    /// 发送者 ID
    pub author_id: Option<String>,
    /// 发送者名称
    pub author_name: Option<String>,
    /// 发送者是否为机器人
    pub author_is_bot: bool,
}

/// Discord 服务器信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordGuildInfo {
    /// 服务器 ID
    pub id: String,
    /// 服务器名称
    pub name: String,
    /// 服务器图标
    pub icon: Option<String>,
    /// 描述
    pub description: Option<String>,
    /// 成员数
    pub member_count: Option<i32>,
    /// 是否可用
    pub unavailable: bool,
}

/// Discord 频道信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordChannelInfo {
    /// 频道 ID
    pub id: String,
    /// 频道类型 (0: 文字, 2: 语音, 4: 分类)
    pub channel_type: i32,
    /// 频道名称
    pub name: String,
    /// 所属服务器 ID
    pub guild_id: Option<String>,
    /// 父频道 ID (分类)
    pub parent_id: Option<String>,
    /// 主题 (文字频道)
    pub topic: Option<String>,
}

/// Discord 成员信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMemberInfo {
    /// 用户 ID
    pub user_id: String,
    /// 用户名
    pub nick: Option<String>,
    /// 头像
    pub avatar: Option<String>,
    /// 角色列表
    pub roles: Vec<String>,
    /// 加入时间
    pub joined_at: Option<String>,
}

/// Discord 消息响应
#[derive(Debug, Deserialize)]
struct DiscordMessageResponse {
    id: String,
    channel_id: String,
    guild_id: Option<String>,
    author: DiscordAuthor,
    content: String,
    mentions: Vec<DiscordMentionData>,
    #[serde(rename = "type")]
    message_type: i32,
}

#[derive(Debug, Deserialize)]
struct DiscordAuthor {
    id: String,
    username: String,
    bot: bool,
}

#[derive(Debug, Deserialize)]
struct DiscordMentionData {
    id: String,
    username: String,
    bot: bool,
    #[serde(rename = "type")]
    mention_type: i32,
}

/// Discord 服务器列表响应
#[derive(Debug, Deserialize)]
struct DiscordGuildsResponse {
    id: String,
    name: String,
    icon: Option<String>,
    description: Option<String>,
    approximate_member_count: Option<i32>,
    unavailable: Option<bool>,
}

/// Discord 频道列表响应
#[derive(Debug, Deserialize)]
struct DiscordChannelsResponse {
    id: String,
    #[serde(rename = "type")]
    channel_type: i32,
    name: String,
    guild_id: Option<String>,
    parent_id: Option<String>,
    topic: Option<String>,
}

/// Discord 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot Token
    pub bot_token: String,
    /// Webhook URL (可选)
    pub webhook_url: Option<String>,
    /// 是否启用
    pub enabled: bool,
    /// 是否使用 Gateway (WebSocket)
    #[cfg(feature = "discord")]
    #[serde(default)]
    pub use_gateway: bool,
}

/// Discord 通道
pub struct DiscordChannel {
    config: DiscordConfig,
    client: Client,
    #[cfg(feature = "discord")]
    gateway_client: Option<DiscordGatewayClient>,
    #[cfg(feature = "discord")]
    event_receiver: Option<mpsc::UnboundedReceiver<DiscordGatewayEvent>>,
    #[cfg(feature = "discord")]
    event_tx: Option<mpsc::UnboundedSender<ChannelEvent>>,
    #[cfg(feature = "discord")]
    message_tx: Option<mpsc::Sender<ChannelMessage>>,
    #[cfg(feature = "discord")]
    msg_receiver: Option<Arc<Mutex<mpsc::Receiver<ChannelMessage>>>>,
}

impl DiscordChannel {
    /// 创建新的 Discord 通道
    pub fn new(config: DiscordConfig) -> Self {
        let client = Client::new();
        Self {
            config,
            client,
            #[cfg(feature = "discord")]
            gateway_client: None,
            #[cfg(feature = "discord")]
            event_receiver: None,
            #[cfg(feature = "discord")]
            event_tx: None,
            #[cfg(feature = "discord")]
            message_tx: None,
            #[cfg(feature = "discord")]
            msg_receiver: None,
        }
    }

    #[cfg(feature = "discord")]
    pub async fn start_gateway(&mut self) -> Result<()> {
        if !self.config.use_gateway {
            return Ok(());
        }

        let intents = serenity::all::GatewayIntents::GUILD_MESSAGES
            | serenity::all::GatewayIntents::DIRECT_MESSAGES
            | serenity::all::GatewayIntents::MESSAGE_CONTENT;

        let (client, receiver): (DiscordGatewayClient, mpsc::UnboundedReceiver<DiscordGatewayEvent>) = 
            DiscordGatewayClient::new(&self.config.bot_token, intents)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create Discord Gateway: {}", e)))?;
        
        self.gateway_client = Some(client);
        self.event_receiver = Some(receiver);

        let mut client_ref = self.gateway_client.take().unwrap();
        tokio::spawn(async move {
            client_ref.run_with_reconnect().await;
        });

        Ok(())
    }

    #[cfg(feature = "discord")]
    pub async fn start_gateway_with_sender(
        &mut self, 
        event_tx: mpsc::UnboundedSender<ChannelEvent>
    ) -> Result<()> {
        if !self.config.use_gateway {
            return Ok(());
        }

        let intents = serenity::all::GatewayIntents::GUILD_MESSAGES
            | serenity::all::GatewayIntents::DIRECT_MESSAGES
            | serenity::all::GatewayIntents::MESSAGE_CONTENT;

        let (client, receiver): (DiscordGatewayClient, mpsc::UnboundedReceiver<DiscordGatewayEvent>) = 
            DiscordGatewayClient::new(&self.config.bot_token, intents)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create Discord Gateway: {}", e)))?;
        
        self.gateway_client = Some(client);

        let mut client_ref = self.gateway_client.take().unwrap();
        tokio::spawn(async move {
            client_ref.run_with_reconnect().await;
        });

        let mut rx = receiver;
        let event_tx = event_tx;
        
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(discord_event) => {
                        if let Some(channel_msg) = discord_event.into_channel_message() {
                            let event = ChannelEvent::Message(channel_msg);
                            if event_tx.send(event).is_err() {
                                tracing::error!("Failed to send Gateway event to handler");
                                break;
                            }
                        }
                    }
                    None => {
                        tracing::info!("Gateway event receiver closed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    #[cfg(feature = "discord")]
    pub async fn start_gateway_with_message_sender_with_receiver(
        &mut self,
        msg_tx: mpsc::Sender<ChannelMessage>,
    ) -> Result<()> {
        if !self.config.use_gateway {
            return Err(OpenClawError::Config("Gateway not enabled".to_string()));
        }

        self.message_tx = Some(msg_tx.clone());

        let intents = serenity::all::GatewayIntents::GUILD_MESSAGES
            | serenity::all::GatewayIntents::DIRECT_MESSAGES
            | serenity::all::GatewayIntents::MESSAGE_CONTENT;

        let (client, receiver): (DiscordGatewayClient, mpsc::UnboundedReceiver<DiscordGatewayEvent>) = 
            DiscordGatewayClient::new(&self.config.bot_token, intents)
            .await
            .map_err(|e| OpenClawError::Config(format!("Failed to create Discord Gateway: {}", e)))?;
        
        self.gateway_client = Some(client);

        let mut client_ref = self.gateway_client.take().unwrap();
        tokio::spawn(async move {
            client_ref.run_with_reconnect().await;
        });

        let mut rx = receiver;
        let mut msg_tx_clone = msg_tx.clone();
        
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Some(discord_event) => {
                        if let Some(channel_msg) = discord_event.into_channel_message() {
                            if msg_tx_clone.try_send(channel_msg).is_err() {
                                tracing::error!("Failed to send Gateway message");
                                break;
                            }
                        }
                    }
                    None => {
                        tracing::info!("Gateway event receiver closed");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    #[cfg(feature = "discord")]
    pub async fn try_recv_gateway_event(&mut self) -> Option<DiscordGatewayEvent> {
        if let Some(rx) = self.event_receiver.as_mut() {
            rx.try_recv().ok()
        } else {
            None
        }
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

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
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

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 使用 Webhook 发送消息
    pub async fn send_webhook(&self, content: &str, username: Option<&str>) -> Result<()> {
        let webhook_url = self
            .config
            .webhook_url
            .as_ref()
            .ok_or_else(|| OpenClawError::Config("未配置 Webhook URL".to_string()))?;

        let mut body = json!({
            "content": content
        });

        if let Some(name) = username {
            body["username"] = json!(name);
        }

        let response = self
            .client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord Webhook 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord Webhook 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// 打字提示
    pub async fn trigger_typing(&self, channel_id: &str) -> Result<()> {
        let url = self.get_api_url(&format!("channels/{}/typing", channel_id));

        let response = self
            .client
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

    /// 解析 Discord @消息
    ///
    /// Discord @消息格式:
    /// - `@用户名` - 用户提及
    /// - `@角色名` - 角色提及
    /// - `#频道名` - 频道提及
    pub fn parse_mentions(&self, content: &str) -> DiscordMessageParseResult {
        let mut mentions = Vec::new();
        let mut clean_content = content.to_string();

        let user_pattern = Regex::new(r"<@!?(\d+)>").unwrap();
        for cap in user_pattern.captures_iter(content) {
            if let Some(id) = cap.get(1) {
                let mention_str = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                mentions.push(DiscordMention {
                    mention_type: "user".to_string(),
                    id: id.as_str().to_string(),
                    name: format!("<@{}>", id.as_str()),
                });
                clean_content = clean_content.replace(mention_str, "").trim().to_string();
            }
        }

        let role_pattern = Regex::new(r"<@&(\d+)>").unwrap();
        for cap in role_pattern.captures_iter(content) {
            if let Some(id) = cap.get(1) {
                let mention_str = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                mentions.push(DiscordMention {
                    mention_type: "role".to_string(),
                    id: id.as_str().to_string(),
                    name: format!("<@&{}>", id.as_str()),
                });
                clean_content = clean_content.replace(mention_str, "").trim().to_string();
            }
        }

        let channel_pattern = Regex::new(r"<#(\d+)>").unwrap();
        for cap in channel_pattern.captures_iter(content) {
            if let Some(id) = cap.get(1) {
                let mention_str = cap.get(0).map(|m| m.as_str()).unwrap_or("");
                mentions.push(DiscordMention {
                    mention_type: "channel".to_string(),
                    id: id.as_str().to_string(),
                    name: format!("<#{}>", id.as_str()),
                });
                clean_content = clean_content.replace(mention_str, "").trim().to_string();
            }
        }

        DiscordMessageParseResult {
            raw_content: content.to_string(),
            clean_content,
            mentions,
            channel_id: None,
            message_id: None,
            guild_id: None,
            author_id: None,
            author_name: None,
            author_is_bot: false,
        }
    }

    /// 解析 Discord API 返回的消息
    pub fn parse_message(&self, msg: DiscordMessageResponse) -> DiscordMessageParseResult {
        let mentions: Vec<DiscordMention> = msg
            .mentions
            .iter()
            .map(|m| DiscordMention {
                mention_type: "user".to_string(),
                id: m.id.clone(),
                name: m.username.clone(),
            })
            .collect();

        let raw_content = msg.content.clone();
        let mut clean_content = raw_content.clone();

        let user_pattern = Regex::new(r"<@!?(\d+)>").unwrap();
        for cap in user_pattern.captures_iter(&raw_content) {
            if let Some(m) = cap.get(0) {
                clean_content = clean_content.replace(m.as_str(), "").trim().to_string();
            }
        }

        DiscordMessageParseResult {
            raw_content,
            clean_content,
            mentions,
            channel_id: Some(msg.channel_id),
            message_id: Some(msg.id),
            guild_id: msg.guild_id,
            author_id: Some(msg.author.id),
            author_name: Some(msg.author.username),
            author_is_bot: msg.author.bot,
        }
    }

    /// 获取服务器列表
    pub async fn list_guilds(&self) -> Result<Vec<DiscordGuildInfo>> {
        let url = self.get_api_url("users/@me/guilds");

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        let guilds: Vec<DiscordGuildsResponse> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(guilds
            .into_iter()
            .map(|g| DiscordGuildInfo {
                id: g.id,
                name: g.name,
                icon: g.icon,
                description: g.description,
                member_count: g.approximate_member_count,
                unavailable: g.unavailable.unwrap_or(false),
            })
            .collect())
    }

    /// 获取服务器频道列表
    pub async fn list_channels(&self, guild_id: &str) -> Result<Vec<DiscordChannelInfo>> {
        let url = self.get_api_url(&format!("guilds/{}/channels", guild_id));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        let channels: Vec<DiscordChannelsResponse> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(channels
            .into_iter()
            .map(|c| DiscordChannelInfo {
                id: c.id,
                channel_type: c.channel_type,
                name: c.name,
                guild_id: c.guild_id,
                parent_id: c.parent_id,
                topic: c.topic,
            })
            .collect())
    }

    /// 获取频道消息历史
    pub async fn get_messages(
        &self,
        channel_id: &str,
        limit: Option<u8>,
    ) -> Result<Vec<DiscordMessageParseResult>> {
        let limit = limit.unwrap_or(50).min(100);
        let url = self.get_api_url(&format!(
            "channels/{}/messages?limit={}",
            channel_id, limit
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        let messages: Vec<DiscordMessageResponse> = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(messages
            .into_iter()
            .map(|m| self.parse_message(m))
            .collect())
    }

    /// 获取服务器成员
    pub async fn get_member(
        &self,
        guild_id: &str,
        user_id: &str,
    ) -> Result<DiscordMemberInfo> {
        let url = self.get_api_url(&format!(
            "guilds/{}/members/{}",
            guild_id, user_id
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        #[derive(Deserialize)]
        struct DiscordMemberResponse {
            user: DiscordUser,
            nick: Option<String>,
            avatar: Option<String>,
            roles: Vec<String>,
            joined_at: String,
        }

        #[derive(Deserialize)]
        struct DiscordUser {
            id: String,
        }

        let member: DiscordMemberResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(DiscordMemberInfo {
            user_id: member.user.id,
            nick: member.nick,
            avatar: member.avatar,
            roles: member.roles,
            joined_at: Some(member.joined_at),
        })
    }

    /// 创建消息回复 (Reply)
    pub async fn send_reply(
        &self,
        channel_id: &str,
        message_id: &str,
        content: &str,
    ) -> Result<String> {
        let url = self.get_api_url(&format!(
            "channels/{}/messages",
            channel_id
        ));

        let body = json!({
            "content": content,
            "message_reference": {
                "message_id": message_id
            }
        });

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        #[derive(Deserialize)]
        struct DiscordCreateMessageResponse {
            id: String,
        }

        let msg: DiscordCreateMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(msg.id)
    }

    pub async fn edit_message(
        &self,
        channel_id: &str,
        message_id: &str,
        new_content: &str,
    ) -> Result<String> {
        let url = self.get_api_url(&format!(
            "channels/{}/messages/{}",
            channel_id, message_id
        ));

        let body = json!({
            "content": new_content
        });

        let response = self
            .client
            .patch(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        #[derive(Deserialize)]
        struct DiscordEditMessageResponse {
            id: String,
        }

        let msg: DiscordEditMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(msg.id)
    }

    pub async fn delete_message(&self, channel_id: &str, message_id: &str) -> Result<()> {
        let url = self.get_api_url(&format!(
            "channels/{}/messages/{}",
            channel_id, message_id
        ));

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    pub async fn get_message(&self, channel_id: &str, message_id: &str) -> Result<DiscordMessageParseResult> {
        let url = self.get_api_url(&format!(
            "channels/{}/messages/{}",
            channel_id, message_id
        ));

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        let msg: DiscordMessageResponse = response
            .json()
            .await
            .map_err(|e| OpenClawError::Http(format!("解析响应失败: {}", e)))?;

        Ok(self.parse_message(msg))
    }

    pub async fn add_reaction(&self, channel_id: &str, message_id: &str, emoji: &str) -> Result<()> {
        let encoded_emoji = urlencoding::encode(emoji);
        let url = self.get_api_url(&format!(
            "channels/{}/messages/{}/reactions/{}/@me",
            channel_id, message_id, encoded_emoji
        ));

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
        }

        Ok(())
    }

    pub async fn remove_reaction(&self, channel_id: &str, message_id: &str, emoji: &str, user_id: Option<&str>) -> Result<()> {
        let encoded_emoji = urlencoding::encode(emoji);
        let user = user_id.unwrap_or("@me");
        let url = self.get_api_url(&format!(
            "channels/{}/messages/{}/reactions/{}/{}",
            channel_id, message_id, encoded_emoji, user
        ));

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bot {}", self.config.bot_token))
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Discord API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Discord API 错误: {}",
                error_text
            )));
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

        #[cfg(feature = "discord")]
        {
            if self.config.use_gateway {
                let (msg_tx, msg_rx) = mpsc::channel(100);
                self.start_gateway_with_message_sender_with_receiver(msg_tx.clone()).await?;
                self.message_tx = Some(msg_tx);
                self.msg_receiver = Some(Arc::new(Mutex::new(msg_rx)));
            }
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
                    self.send_embed(&message.chat_id, title, &message.content, None)
                        .await?;
                }
                _ => {
                    self.send_to_channel(&message.chat_id, &message.content)
                        .await?;
                }
            }
        } else {
            return Err(OpenClawError::Config(
                "Discord 通道需要配置 webhook_url 或提供 chat_id".to_string(),
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

    #[cfg(feature = "discord")]
    fn messages(&self) -> Option<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>> {
        self.msg_receiver.as_ref().map(|rx| {
            let rx = Arc::clone(rx);
            let stream = async_stream::stream! {
                loop {
                    let msg = {
                        let mut rx = rx.lock().await;
                        rx.recv().await
                    };
                    match msg {
                        Some(msg) => yield msg,
                        None => break,
                    }
                }
            };
            Box::pin(stream) as Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>
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
            #[cfg(feature = "discord")]
            use_gateway: false,
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
    }

    #[cfg(feature = "discord")]
    #[test]
    fn test_discord_channel_with_gateway() {
        let config = DiscordConfig {
            bot_token: "BOT_TOKEN".to_string(),
            webhook_url: None,
            enabled: true,
            use_gateway: true,
        };
        let channel = DiscordChannel::new(config);
        assert_eq!(channel.name(), "discord");
    }

    #[test]
    fn test_discord_config_serialization() {
        #[cfg(feature = "discord")]
        {
            use serde_json;
            let config = DiscordConfig {
                bot_token: "test_token".to_string(),
                webhook_url: Some("https://webhook.url".to_string()),
                enabled: true,
                use_gateway: true,
            };
            let json = serde_json::to_string(&config).unwrap();
            assert!(json.contains("test_token"));
            assert!(json.contains("use_gateway"));
        }
    }

    #[test]
    fn test_message_edit_url_format() {
        let channel_id = "123456789";
        let message_id = "987654321";
        let url = format!("https://discord.com/api/v10/channels/{}/messages/{}", channel_id, message_id);
        assert_eq!(url, "https://discord.com/api/v10/channels/123456789/messages/987654321");
    }

    #[test]
    fn test_emoji_encoding() {
        let emoji = "👍";
        let encoded = urlencoding::encode(emoji);
        assert_eq!(encoded, "%F0%9F%91%8D");
    }

    #[test]
    fn test_reaction_url_format() {
        let channel_id = "123456789";
        let message_id = "987654321";
        let emoji = "👍";
        let encoded = urlencoding::encode(emoji);
        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages/{}/reactions/{}/@me",
            channel_id, message_id, encoded
        );
        assert!(url.contains("%F0%9F%91%8D"));
    }
}
