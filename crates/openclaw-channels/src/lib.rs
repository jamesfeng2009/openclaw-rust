//! OpenClaw Channels - 消息通道抽象
//!
//! 支持多种消息通道：
//! 
//! ## 国内平台
//! - 钉钉 (DingTalk) - 企业办公平台
//! - 企业微信 (WeCom) - 企业通讯工具
//! - 飞书 (Feishu) - 字节跳动企业协作平台
//!
//! ## 国际平台
//! - Telegram - 即时通讯应用
//! - Discord - 游戏社区平台
//! - Slack - 企业协作工具
//! - Microsoft Teams - 微软企业协作平台

pub mod base;
pub mod types;
// pub mod telegram; // TODO: 更新以匹配新的 ChannelMessage 结构
pub mod dingtalk;
pub mod wecom;
pub mod feishu;
pub mod discord;
pub mod teams;
pub mod slack;

pub use base::*;
pub use types::*;
// pub use telegram::*;
pub use dingtalk::*;
pub use wecom::*;
pub use feishu::*;
pub use discord::*;
pub use teams::*;
pub use slack::*;
