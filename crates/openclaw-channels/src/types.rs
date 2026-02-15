//! 通道类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 通道类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    // 国际平台
    Telegram,
    Discord,
    WhatsApp,
    Slack,
    Signal,
    Matrix,
    Teams,  // Microsoft Teams
    IMessage, // Apple iMessage
    GoogleChat, // Google Chat
    // 国内平台
    DingTalk,
    WeCom,  // 企业微信
    Feishu, // 飞书
    Zalo,   // Zalo
    ZaloPersonal, // Zalo Personal (越南个人)
    // 其他
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
    pub channel_type: ChannelType,
    /// 聊天 ID
    pub chat_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
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
    /// 聊天 ID
    pub chat_id: String,
    /// 消息类型 (text, markdown, link, image, news, file)
    pub message_type: String,
    /// 消息内容
    pub content: String,
    /// 标题（可选）
    pub title: Option<String>,
    /// URL（可选）
    pub url: Option<String>,
    /// @ 手机号列表（钉钉）
    pub at_mobiles: Option<Vec<String>>,
    /// @ 用户列表（企业微信）
    pub mentioned_list: Option<Vec<String>>,
    /// Base64 数据（图片）
    pub base64: Option<String>,
    /// MD5 值（图片）
    pub md5: Option<String>,
    /// 图文消息文章列表
    pub articles: Option<Vec<crate::wecom::NewsArticle>>,
    /// 媒体文件 ID
    pub media_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ParseMode {
    Markdown,
    MarkdownV2,
    Html,
    Plain,
}
