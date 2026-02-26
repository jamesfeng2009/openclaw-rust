//! Control Flow - 队列模式控制
//!
//! 实现 Steer + Follow-up 控制流：
//! - 四种队列模式：collect/steer/follow_up/steer_backlog
//! - 外部消息实时注入
//! - 长任务续接

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

/// 队列模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMode {
    Collect,
    Steer,
    FollowUp,
    SteerBacklog,
}

impl Default for QueueMode {
    fn default() -> Self {
        QueueMode::Collect
    }
}

/// 控制消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub priority: MessagePriority,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ControlMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            priority: MessagePriority::Normal,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }
}

/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for MessagePriority {
    fn default() -> Self {
        MessagePriority::Normal
    }
}

/// 消息队列
#[derive(Clone)]
pub struct MessageQueue {
    inner: Arc<RwLock<MessageQueueInner>>,
}

#[derive(Debug)]
struct MessageQueueInner {
    mode: QueueMode,
    collect_queue: VecDeque<ControlMessage>,
    steer_queue: VecDeque<ControlMessage>,
    follow_up_queue: VecDeque<ControlMessage>,
    backlog: VecDeque<ControlMessage>,
    history: Vec<ControlMessage>,
}

impl Default for MessageQueueInner {
    fn default() -> Self {
        Self {
            mode: QueueMode::Collect,
            collect_queue: VecDeque::new(),
            steer_queue: VecDeque::new(),
            follow_up_queue: VecDeque::new(),
            backlog: VecDeque::new(),
            history: Vec::new(),
        }
    }
}

impl MessageQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(MessageQueueInner::default())),
        }
    }

    /// 获取当前模式
    pub async fn mode(&self) -> QueueMode {
        let inner = self.inner.read().await;
        inner.mode
    }

    /// 设置模式
    pub async fn set_mode(&self, mode: QueueMode) {
        let (new_mode, new_collect, new_steer, new_follow_up, new_backlog) = {
            let mut inner = self.inner.write().await;
            
            let collect = std::mem::take(&mut inner.collect_queue);
            let steer = std::mem::take(&mut inner.steer_queue);
            let follow_up = std::mem::take(&mut inner.follow_up_queue);
            let backlog = std::mem::take(&mut inner.backlog);
            
            let (new_collect, new_steer, new_follow_up, new_backlog) = match mode {
                 QueueMode::Collect => {
                     let mut q = collect;
                     q.extend(backlog);
                     (q, VecDeque::new(), VecDeque::new(), VecDeque::new())
                 }
                 QueueMode::Steer => {
                     let mut q = collect;
                     q.extend(steer);
                     q.extend(backlog);
                     (VecDeque::new(), q, VecDeque::new(), VecDeque::new())
                 }
                 QueueMode::FollowUp => {
                     let mut q = follow_up;
                     q.extend(collect);
                     (VecDeque::new(), VecDeque::new(), q, VecDeque::new())
                 }
                 QueueMode::SteerBacklog => {
                     let mut q = collect;
                     q.extend(steer);
                     (VecDeque::new(), q, VecDeque::new(), backlog)
                 }
             };
             
             (mode, new_collect, new_steer, new_follow_up, new_backlog)
        };
        
        let mut inner = self.inner.write().await;
        inner.mode = new_mode;
        inner.collect_queue = new_collect;
        inner.steer_queue = new_steer;
        inner.follow_up_queue = new_follow_up;
        inner.backlog = new_backlog;
    }

    /// 添加消息到收集队列
    pub async fn enqueue(&self, message: ControlMessage) {
        let mut inner = self.inner.write().await;
        inner.collect_queue.push_back(message);
    }

    /// 添加用户消息
    pub async fn enqueue_user(&self, content: impl Into<String>) {
        self.enqueue(ControlMessage::user(content)).await;
    }

    /// 添加系统消息
    pub async fn enqueue_system(&self, content: impl Into<String>) {
        self.enqueue(ControlMessage::system(content)).await;
    }

    /// 添加助手消息
    pub async fn enqueue_assistant(&self, content: impl Into<String>) {
        self.enqueue(ControlMessage::assistant(content)).await;
    }

    /// 实时注入消息（Steer 模式）
    pub async fn steer(&self, message: ControlMessage) {
        let mut inner = self.inner.write().await;
        
        match inner.mode {
            QueueMode::Collect => {
                inner.backlog.push_back(message);
            }
            QueueMode::Steer => {
                inner.steer_queue.push_front(message);
            }
            QueueMode::FollowUp => {
                inner.follow_up_queue.push_back(message);
            }
            QueueMode::SteerBacklog => {
                inner.backlog.push_back(message);
            }
        }
    }

    /// 注入用户消息
    pub async fn steer_user(&self, content: impl Into<String>) {
        self.steer(ControlMessage::user(content)).await;
    }

    /// 注入系统消息
    pub async fn steer_system(&self, content: impl Into<String>) {
        self.steer(ControlMessage::system(content)).await;
    }

    /// 标记为长任务，等待后续续接（FollowUp 模式）
    pub async fn follow_up(&self, message: ControlMessage) {
        let mut inner = self.inner.write().await;
        
        inner.follow_up_queue.push_back(message);
    }

    /// 获取下一条消息（按优先级和模式）
    pub async fn dequeue(&self) -> Option<ControlMessage> {
        let message = {
            let mut inner = self.inner.write().await;
            
            let message = match inner.mode {
                QueueMode::Collect => {
                    inner.collect_queue.pop_front()
                }
                QueueMode::Steer => {
                    inner.steer_queue.pop_front()
                        .or_else(|| {
                            inner.collect_queue.pop_front()
                        })
                        .or_else(|| {
                            inner.backlog.pop_front()
                        })
                }
                QueueMode::FollowUp => {
                    inner.follow_up_queue.pop_front()
                        .or_else(|| {
                            inner.collect_queue.pop_front()
                        })
                }
                QueueMode::SteerBacklog => {
                    inner.backlog.pop_front()
                        .or_else(|| {
                            inner.steer_queue.pop_front()
                        })
                        .or_else(|| {
                            inner.collect_queue.pop_front()
                        })
                }
            };
            
            if let Some(ref msg) = message {
                inner.history.push(msg.clone());
            }
            
            message
        };
        
        message
    }

    /// 批量获取消息
    pub async fn dequeue_batch(&self, max_count: usize) -> Vec<ControlMessage> {
        let mut messages = Vec::with_capacity(max_count);
        
        for _ in 0..max_count {
            match self.dequeue().await {
                Some(msg) => messages.push(msg),
                None => break,
            }
        }
        
        messages
    }

    /// 查看下一条消息（不取出）
    pub async fn peek(&self) -> Option<ControlMessage> {
        let inner = self.inner.read().await;
        
        match inner.mode {
            QueueMode::Collect => inner.collect_queue.front().cloned(),
            QueueMode::Steer => {
                inner.steer_queue.front().cloned().or_else(|| inner.backlog.front().cloned())
            }
            QueueMode::FollowUp => {
                inner.follow_up_queue
                    .front()
                    .cloned()
                    .or_else(|| inner.collect_queue.front().cloned())
            }
            QueueMode::SteerBacklog => {
                inner.steer_queue
                    .front()
                    .cloned()
                    .or_else(|| inner.collect_queue.front().cloned())
                    .or_else(|| inner.backlog.front().cloned())
            }
        }
    }

    /// 刷新积压消息到队列
    pub async fn flush_backlog(&self) {
        let (mode, backlog) = {
            let mut inner = self.inner.write().await;
            let mode = inner.mode;
            let backlog = std::mem::take(&mut inner.backlog);
            (mode, backlog)
        };
        
        let mut inner = self.inner.write().await;
        
        match mode {
            QueueMode::Collect | QueueMode::SteerBacklog => {
                inner.collect_queue.extend(backlog);
            }
            QueueMode::Steer => {
                inner.steer_queue.extend(backlog);
            }
            QueueMode::FollowUp => {
                inner.follow_up_queue.extend(backlog);
            }
        }
    }

    /// 清空所有队列
    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        
        inner.collect_queue.clear();
        inner.steer_queue.clear();
        inner.follow_up_queue.clear();
        inner.backlog.clear();
    }

    /// 清空历史
    pub async fn clear_history(&self) {
        let mut inner = self.inner.write().await;
        inner.history.clear();
    }

    /// 获取队列长度
    pub async fn len(&self) -> usize {
        let inner = self.inner.read().await;
        
        match inner.mode {
            QueueMode::Collect => inner.collect_queue.len(),
            QueueMode::Steer => inner.steer_queue.len() + inner.backlog.len(),
            QueueMode::FollowUp => inner.follow_up_queue.len() + inner.collect_queue.len(),
            QueueMode::SteerBacklog => {
                inner.steer_queue.len() + inner.collect_queue.len() + inner.backlog.len()
            }
        }
    }

    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 获取各队列长度
    pub async fn queue_lengths(&self) -> QueueLengths {
        let inner = self.inner.read().await;
        
        QueueLengths {
            collect: inner.collect_queue.len(),
            steer: inner.steer_queue.len(),
            follow_up: inner.follow_up_queue.len(),
            backlog: inner.backlog.len(),
            history: inner.history.len(),
        }
    }

    /// 获取历史消息
    pub async fn history(&self) -> Vec<ControlMessage> {
        let inner = self.inner.read().await;
        inner.history.clone()
    }

    /// 获取历史消息数量
    pub async fn history_len(&self) -> usize {
        let inner = self.inner.read().await;
        inner.history.len()
    }
}

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 队列长度统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueLengths {
    pub collect: usize,
    pub steer: usize,
    pub follow_up: usize,
    pub backlog: usize,
    pub history: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("Hello").await;
        queue.enqueue_user("World").await;
        
        assert_eq!(queue.len().await, 2);
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Hello");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "World");
        
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_steer_mode() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("Initial").await;
        queue.set_mode(QueueMode::Steer).await;
        
        queue.steer_user("Steered message").await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Steered message");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Initial");
    }

    #[tokio::test]
    async fn test_follow_up_mode() {
        let queue = MessageQueue::new();
        
        queue.set_mode(QueueMode::FollowUp).await;
        
        queue.enqueue_user("First").await;
        queue.follow_up(ControlMessage::user("Continue")).await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Continue");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "First");
    }

    #[tokio::test]
    async fn test_steer_backlog() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("Queued 1").await;
        queue.enqueue_user("Queued 2").await;
        
        queue.set_mode(QueueMode::SteerBacklog).await;
        
        queue.steer_user("Steered").await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Steered");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Queued 1");
    }

    #[tokio::test]
    async fn test_flush_backlog() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("Collected").await;
        queue.steer_user("In backlog").await;
        
        queue.flush_backlog().await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "Collected");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "In backlog");
    }

    #[tokio::test]
    async fn test_history() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("Message 1").await;
        queue.dequeue().await;
        
        queue.enqueue_user("Message 2").await;
        queue.dequeue().await;
        
        let history = queue.history().await;
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn test_queue_lengths() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("1").await;
        queue.enqueue_user("2").await;
        queue.steer_user("3").await;
        
        let lengths = queue.queue_lengths().await;
        assert_eq!(lengths.collect, 2);
        assert_eq!(lengths.backlog, 1);
    }

    #[tokio::test]
    async fn test_priority() {
        let queue = MessageQueue::new();
        
        queue.enqueue(ControlMessage::user("Normal").with_priority(MessagePriority::Normal)).await;
        queue.enqueue(ControlMessage::user("High").with_priority(MessagePriority::High)).await;
        queue.enqueue(ControlMessage::user("Low").with_priority(MessagePriority::Low)).await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.priority, MessagePriority::Normal);
    }

    #[tokio::test]
    async fn test_mode_switch_collect() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("1").await;
        queue.enqueue_user("2").await;
        
        queue.set_mode(QueueMode::Collect).await;
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "1");
    }

    #[tokio::test]
    async fn test_clear() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("1").await;
        queue.steer_user("2").await;
        
        queue.clear().await;
        
        assert!(queue.is_empty().await);
    }

    #[tokio::test]
    async fn test_peek() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("First").await;
        
        let peeked = queue.peek().await.unwrap();
        assert_eq!(peeked.content, "First");
        
        let msg = queue.dequeue().await.unwrap();
        assert_eq!(msg.content, "First");
        
        let peeked = queue.peek().await;
        assert!(peeked.is_none());
    }

    #[tokio::test]
    async fn test_dequeue_batch() {
        let queue = MessageQueue::new();
        
        queue.enqueue_user("1").await;
        queue.enqueue_user("2").await;
        queue.enqueue_user("3").await;
        queue.enqueue_user("4").await;
        
        let batch = queue.dequeue_batch(2).await;
        assert_eq!(batch.len(), 2);
        
        let remaining = queue.len().await;
        assert_eq!(remaining, 2);
    }
}
