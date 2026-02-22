//! OpenClaw Channels - 消息通道抽象
//!
//! 支持多种消息通道：
//!
//! ## 国内平台
//! - 钉钉 (DingTalk) - 企业办公平台
//! - 企业微信 (WeCom) - 企业通讯工具
//! - 飞书 (Feishu) - 字节跳动企业协作平台
//! - Zalo - 越南聊天应用 (Official Account)
//! - Zalo Personal - 越南个人聊天应用
//!
//! ## 国际平台
//! - Telegram - 即时通讯应用
//! - Discord - 游戏社区平台
//! - Slack - 企业协作工具
//! - Microsoft Teams - 微软企业协作平台
//! - Google Chat - Google Workspace 消息应用
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
pub mod bluebubbles;
pub mod dingtalk;
pub mod discord;
pub mod dm_policy;
pub mod email;
pub mod factory;
pub mod feishu;
pub mod googlechat;
pub mod imessage;
pub mod manager;
pub mod matrix;
pub mod registry;
pub mod signal;
pub mod slack;
pub mod sms;
pub mod teams;
pub mod telegram;
pub mod types;
pub mod webchat;
pub mod wecom;
pub mod whatsapp;
pub mod zalo;
pub mod zalo_personal;

pub use base::*;
pub use bluebubbles::*;
pub use dingtalk::*;
pub use discord::*;
pub use dm_policy::*;
pub use email::*;
pub use factory::*;
pub use feishu::*;
pub use googlechat::*;
pub use imessage::*;
pub use manager::*;
pub use matrix::*;
pub use registry::*;
pub use signal::*;
pub use slack::*;
pub use sms::*;
pub use teams::*;
pub use telegram::*;
pub use types::*;
pub use webchat::*;
pub use wecom::*;
pub use whatsapp::*;
pub use zalo::*;
pub use zalo_personal::*;
