//! OpenClaw Channels - 消息通道抽象
//!
//! 支持多种消息通道：
//! 
//! ## 国内平台
//! - 钉钉 (DingTalk) - 企业办公平台
//! - 企业微信 (WeCom) - 企业通讯工具
//! - 飞书 (Feishu) - 字节跳动企业协作平台
//! - Zalo - 越南聊天应用
//!
//! ## 国际平台
//! - Telegram - 即时通讯应用
//! - Discord - 游戏社区平台
//! - Slack - 企业协作工具
//! - Microsoft Teams - 微软企业协作平台
//! - WhatsApp - Meta 即时通讯应用
//! - Signal - 隐私优先的即时通讯
//! - iMessage - Apple 消息服务 (仅 macOS)
//! - BlueBubbles - macOS iMessage REST API
//! - Matrix - 开源去中心化通信协议
//!
//! ## 其他
//! - WebChat - 自定义 Webhook
//! - Email - 邮件发送
//! - SMS - 短信发送

pub mod base;
pub mod types;
pub mod telegram;
pub mod dingtalk;
pub mod wecom;
pub mod feishu;
pub mod zalo;
pub mod discord;
pub mod teams;
pub mod slack;
pub mod whatsapp;
pub mod signal;
pub mod imessage;
pub mod bluebubbles;
pub mod matrix;
pub mod webchat;
pub mod email;
pub mod sms;
pub mod dm_policy;

pub use base::*;
pub use types::*;
pub use telegram::*;
pub use dingtalk::*;
pub use wecom::*;
pub use feishu::*;
pub use zalo::*;
pub use discord::*;
pub use teams::*;
pub use slack::*;
pub use whatsapp::*;
pub use signal::*;
pub use imessage::*;
pub use bluebubbles::*;
pub use matrix::*;
pub use webchat::*;
pub use email::*;
pub use sms::*;
pub use dm_policy::*;
