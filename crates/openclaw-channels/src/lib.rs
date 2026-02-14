//! OpenClaw Channels - 消息通道抽象
//!
//! 支持多种消息通道：
//! 
//! ## 国内平台
//! - 钉钉 (DingTalk)
//! - 企业微信 (WeCom)
//! - 飞书 (Feishu)
//!
//! ## 国际平台
//! - Telegram
//! - Discord
//! - WhatsApp
//! - Slack

pub mod base;
pub mod types;
// pub mod telegram; // TODO: 更新以匹配新的 ChannelMessage 结构
pub mod dingtalk;
pub mod wecom;

pub use base::*;
pub use types::*;
// pub use telegram::*;
pub use dingtalk::*;
pub use wecom::*;
