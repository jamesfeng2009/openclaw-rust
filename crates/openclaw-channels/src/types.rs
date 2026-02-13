//! 通道类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通道类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Telegram,
    Discord,
    WhatsApp,
    Slack,
    Signal,
    Matrix,
    WebChat,
    Email,
    SMS,
}

/// 通道消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// 消息 ID
    pub id: String,
    /// 通道类型
    pub channel: ChannelType,
    /// 发送者信息
    pub sender: Sender,
    /// 聊天信息
    pub chat: Chat,
    /// 消息内容
    pub content: MessageContent,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 回复的消息 ID
    pub reply_to: Option<String>,
    /// 原始消息数据
    pub raw: Option<serde_json::Value>,
}

/// 发送者信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sender {
    pub id: String,
    pub name: Option<String>,
    pub username: Option<String>,
    pub is_bot: bool,
}

/// 聊天信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chat {
    pub id: String,
    pub chat_type: ChatType,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChatType {
    Private,
    Group,
    Channel,
    SuperGroup,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContent {
    Text { text: String },
    Photo { url: String, caption: Option<String> },
    Video { url: String, caption: Option<String> },
    Audio { url: String },
    Document { url: String, filename: String },
    Location { latitude: f64, longitude: f64 },
    Contact { name: String, phone: String },
    Sticker { url: String },
    Unknown,
}

/// 发送消息请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessage {
    pub chat_id: String,
    pub content: MessageContent,
    pub reply_to: Option<String>,
    pub parse_mode: Option<ParseMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParseMode {
    Markdown,
    MarkdownV2,
    Html,
    Plain,
}
