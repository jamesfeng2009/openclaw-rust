//! OpenClaw Channels - 消息通道抽象
//!
//! 支持多种消息通道：
//! - Telegram (teloxide)
//! - Discord (serenity)
//! - WhatsApp (桥接)
//! - Slack (桥接)

pub mod base;
pub mod telegram;
pub mod types;

pub use base::*;
pub use telegram::*;
pub use types::*;
