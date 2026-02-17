//! 会话管理

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Result;

/// 会话作用域
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SessionScope {
    /// 所有私聊共享一个会话
    Main,
    /// 每个联系人独立会话
    PerPeer,
    /// 每个频道+联系人独立会话
    PerChannelPeer,
    /// 每个账号+频道+联系人独立会话
    PerAccountChannelPeer,
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    Active,
    Idle,
    Closed,
}

/// 会话重置策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetPolicy {
    pub mode: ResetMode,
    pub at_hour: Option<u8>,
    pub idle_minutes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResetMode {
    Daily,
    Idle,
    Manual,
    Never,
}

/// 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub scope: SessionScope,
    pub state: SessionState,
    pub agent_id: String,
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub message_count: usize,
    pub total_tokens: usize,
}

impl Session {
    pub fn new(agent_id: impl Into<String>, scope: SessionScope) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            scope,
            state: SessionState::Active,
            agent_id: agent_id.into(),
            channel: None,
            account_id: None,
            peer_id: None,
            created_at: now,
            updated_at: now,
            last_active_at: now,
            message_count: 0,
            total_tokens: 0,
        }
    }

    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    pub fn with_peer(mut self, peer_id: impl Into<String>) -> Self {
        self.peer_id = Some(peer_id.into());
        self
    }

    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
        self.updated_at = Utc::now();
    }

    pub fn is_expired(&self, policy: &ResetPolicy) -> bool {
        match policy.mode {
            ResetMode::Never => false,
            ResetMode::Manual => false,
            ResetMode::Daily => {
                // 检查是否超过 24 小时
                let elapsed = Utc::now() - self.last_active_at;
                elapsed.num_hours() >= 24
            }
            ResetMode::Idle => {
                if let Some(minutes) = policy.idle_minutes {
                    let elapsed = Utc::now() - self.last_active_at;
                    elapsed.num_minutes() >= minutes as i64
                } else {
                    false
                }
            }
        }
    }
}

/// 会话存储 Trait
#[async_trait::async_trait]
pub trait SessionStore: Send + Sync {
    /// 获取或创建会话
    async fn get_or_create(&self, key: &SessionKey) -> Result<Session>;

    /// 获取会话
    async fn get(&self, id: Uuid) -> Result<Option<Session>>;

    /// 保存会话
    async fn save(&self, session: &Session) -> Result<()>;

    /// 删除会话
    async fn delete(&self, id: Uuid) -> Result<()>;

    /// 列出会话
    async fn list(&self, agent_id: &str) -> Result<Vec<Session>>;

    /// 清理过期会话
    async fn prune(&self, max_age_days: u64, max_count: usize) -> Result<usize>;
}

/// 会话键
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct SessionKey {
    pub agent_id: String,
    pub channel: Option<String>,
    pub account_id: Option<String>,
    pub peer_id: Option<String>,
}
