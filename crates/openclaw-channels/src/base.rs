//! 通道基础 Trait

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::Result;
use std::pin::Pin;

use crate::types::{ChannelMessage, ChannelType, SendMessage};

/// 消息通道 Trait
#[async_trait]
pub trait Channel: Send + Sync {
    /// 通道类型
    fn channel_type(&self) -> ChannelType;

    /// 通道名称
    fn name(&self) -> &str;

    /// 启动通道
    async fn start(&mut self) -> Result<()>;

    /// 停止通道
    async fn stop(&mut self) -> Result<()>;

    /// 发送消息
    async fn send(&self, message: SendMessage) -> Result<ChannelMessage>;

    /// 消息流
    fn messages(&self) -> Option<Pin<Box<dyn Stream<Item = ChannelMessage> + Send>>> {
        None
    }

    /// 健康检查
    async fn health_check(&self) -> Result<bool>;
}

/// 通道事件
#[derive(Debug, Clone)]
pub enum ChannelEvent {
    /// 收到消息
    Message(ChannelMessage),
    /// 消息已发送
    MessageSent { message_id: String },
    /// 用户加入
    UserJoined { chat_id: String, user_id: String },
    /// 用户离开
    UserLeft { chat_id: String, user_id: String },
    /// 错误
    Error(String),
}

/// 通道处理器 Trait
#[async_trait]
pub trait ChannelHandler: Send + Sync {
    /// 处理消息
    async fn handle(&self, message: ChannelMessage) -> Result<Option<SendMessage>>;

    /// 处理通道事件
    async fn handle_event(&self, event: ChannelEvent) -> Result<()> {
        if let ChannelEvent::Message(msg) = event {
            self.handle(msg).await?;
        }
        Ok(())
    }
}
