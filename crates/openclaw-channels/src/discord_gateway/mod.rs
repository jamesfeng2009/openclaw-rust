//! Discord Gateway 模块
//!
//! 使用 Discord Gateway (WebSocket) 接收实时消息，比 HTTP API 轮询更快

pub mod gateway;
pub mod handler;
pub mod metrics;
pub mod types;

pub use gateway::{ConnectionHealth, DiscordGatewayClient};
pub use metrics::GatewayMetrics;
pub use types::DiscordGatewayEvent;
