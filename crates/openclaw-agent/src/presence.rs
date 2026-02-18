//! Presence 在线状态管理
//!
//! 管理 Agent、Channel、User 的在线状态

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// Presence 状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum PresenceStatus {
    Online,
    Away,
    Busy,
    #[default]
    Offline,
    Unknown,
}


impl std::fmt::Display for PresenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresenceStatus::Online => write!(f, "online"),
            PresenceStatus::Away => write!(f, "away"),
            PresenceStatus::Busy => write!(f, "busy"),
            PresenceStatus::Offline => write!(f, "offline"),
            PresenceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Presence 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Presence {
    /// 实体 ID (Agent ID / Channel ID / User ID)
    pub id: String,
    /// 实体类型
    pub entity_type: PresenceEntityType,
    /// 当前状态
    pub status: PresenceStatus,
    /// 最后活跃时间
    pub last_active: chrono::DateTime<chrono::Utc>,
    /// 自定义状态消息
    pub status_message: Option<String>,
    /// 设备信息
    pub device: Option<String>,
    /// 是否可接收消息
    pub can_receive: bool,
}

/// Presence 实体类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PresenceEntityType {
    Agent,
    Channel,
    User,
}

/// Presence 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceConfig {
    /// 在线超时时间 (秒)
    pub online_timeout: u64,
    /// 离开超时时间 (秒)
    pub away_timeout: u64,
    /// 是否启用自动状态更新
    pub auto_update: bool,
    /// 是否启用状态广播
    pub broadcast: bool,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            online_timeout: 300, // 5 分钟
            away_timeout: 1800,  // 30 分钟
            auto_update: true,
            broadcast: true,
        }
    }
}

/// Presence 管理器
pub struct PresenceManager {
    presences: Arc<RwLock<HashMap<String, Presence>>>,
    config: Arc<RwLock<PresenceConfig>>,
    last_updates: Arc<RwLock<HashMap<String, Instant>>>,
}

impl PresenceManager {
    pub fn new() -> Self {
        Self {
            presences: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(PresenceConfig::default())),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn with_config(config: PresenceConfig) -> Self {
        Self {
            presences: Arc::new(RwLock::new(HashMap::new())),
            config: Arc::new(RwLock::new(config)),
            last_updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 更新配置
    pub async fn set_config(&self, config: PresenceConfig) {
        let mut cfg = self.config.write().await;
        *cfg = config;
    }

    /// 获取配置
    pub async fn get_config(&self) -> PresenceConfig {
        let cfg = self.config.read().await;
        cfg.clone()
    }

    /// 设置状态
    pub async fn set_status(
        &self,
        id: impl Into<String>,
        entity_type: PresenceEntityType,
        status: PresenceStatus,
        status_message: Option<String>,
    ) {
        let id = id.into();
        let now = chrono::Utc::now();

        let presence = Presence {
            id: id.clone(),
            entity_type,
            status,
            last_active: now,
            status_message,
            device: None,
            can_receive: true,
        };

        let mut presences = self.presences.write().await;
        presences.insert(id.clone(), presence);

        let mut updates = self.last_updates.write().await;
        updates.insert(id, Instant::now());
    }

    /// 更新在线状态 (自动计算)
    pub async fn update_online(&self, id: impl Into<String>, entity_type: PresenceEntityType) {
        let id = id.into();
        let _now = chrono::Utc::now();

        let config = self.get_config().await;
        let updates = self.last_updates.read().await;

        if let Some(last_update) = updates.get(&id) {
            let elapsed = last_update.elapsed().as_secs();

            let status = if elapsed < config.online_timeout {
                PresenceStatus::Online
            } else if elapsed < config.away_timeout {
                PresenceStatus::Away
            } else {
                PresenceStatus::Offline
            };

            drop(updates);

            self.set_status(id, entity_type, status, None).await;
        } else {
            drop(updates);
            self.set_status(id, entity_type, PresenceStatus::Online, None)
                .await;
        }
    }

    /// 获取状态
    pub async fn get_status(&self, id: &str) -> Option<Presence> {
        let presences = self.presences.read().await;
        presences.get(id).cloned()
    }

    /// 获取所有状态
    pub async fn get_all_presences(&self) -> Vec<Presence> {
        let presences = self.presences.read().await;
        presences.values().cloned().collect()
    }

    /// 按类型获取状态
    pub async fn get_by_type(&self, entity_type: PresenceEntityType) -> Vec<Presence> {
        let presences = self.presences.read().await;
        presences
            .values()
            .filter(|p| p.entity_type == entity_type)
            .cloned()
            .collect()
    }

    /// 获取在线的 Agents
    pub async fn get_online_agents(&self) -> Vec<Presence> {
        self.get_by_type(PresenceEntityType::Agent)
            .await
            .into_iter()
            .filter(|p| p.status == PresenceStatus::Online)
            .collect()
    }

    /// 获取在线的 Channels
    pub async fn get_online_channels(&self) -> Vec<Presence> {
        self.get_by_type(PresenceEntityType::Channel)
            .await
            .into_iter()
            .filter(|p| p.status == PresenceStatus::Online)
            .collect()
    }

    /// 移除状态
    pub async fn remove(&self, id: &str) {
        let mut presences = self.presences.write().await;
        presences.remove(id);

        let mut updates = self.last_updates.write().await;
        updates.remove(id);
    }

    /// 清理离线状态
    pub async fn cleanup_offline(&self) {
        let config = self.get_config().await;
        let updates = self.last_updates.write().await;
        let mut presences = self.presences.write().await;

        let now = Instant::now();
        let offline_timeout = Duration::from_secs(config.away_timeout);

        let offline_ids: Vec<String> = updates
            .iter()
            .filter(|(_, last_update)| now.duration_since(**last_update) > offline_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in offline_ids {
            if let Some(presence) = presences.get_mut(&id) {
                presence.status = PresenceStatus::Offline;
            }
        }
    }

    /// 心跳 - 保持在线状态
    pub async fn heartbeat(&self, id: &str) {
        let mut updates = self.last_updates.write().await;
        updates.insert(id.to_string(), Instant::now());

        let mut presences = self.presences.write().await;
        if let Some(presence) = presences.get_mut(id) {
            presence.status = PresenceStatus::Online;
            presence.last_active = chrono::Utc::now();
        }
    }

    /// 批量心跳
    pub async fn batch_heartbeat(&self, ids: Vec<String>) {
        for id in ids {
            self.heartbeat(&id).await;
        }
    }
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Presence 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum PresenceEvent {
    /// 状态变化
    StatusChanged {
        id: String,
        old_status: PresenceStatus,
        new_status: PresenceStatus,
    },
    /// 实体加入
    Joined {
        id: String,
        entity_type: PresenceEntityType,
    },
    /// 实体离开
    Left {
        id: String,
        entity_type: PresenceEntityType,
    },
    /// 心跳
    Heartbeat { id: String },
}

/// Presence 事件发布者
pub struct PresenceEventPublisher {
    subscribers: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<PresenceEvent>>>>,
}

impl PresenceEventPublisher {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 订阅事件
    pub async fn subscribe(&self) -> tokio::sync::mpsc::Receiver<PresenceEvent> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let mut subscribers = self.subscribers.write().await;
        subscribers.push(tx);
        rx
    }

    /// 发布事件
    pub async fn publish(&self, event: PresenceEvent) {
        let subscribers = self.subscribers.read().await;
        for tx in subscribers.iter() {
            let _ = tx.send(event.clone()).await;
        }
    }
}

impl Default for PresenceEventPublisher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get_status() {
        let manager = PresenceManager::new();

        manager
            .set_status(
                "agent-1",
                PresenceEntityType::Agent,
                PresenceStatus::Online,
                Some("Ready".to_string()),
            )
            .await;

        let presence = manager.get_status("agent-1").await;
        assert!(presence.is_some());
        assert_eq!(presence.unwrap().status, PresenceStatus::Online);
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let manager = PresenceManager::new();

        manager
            .set_status(
                "agent-1",
                PresenceEntityType::Agent,
                PresenceStatus::Online,
                None,
            )
            .await;

        manager.heartbeat("agent-1").await;

        let presence = manager.get_status("agent-1").await;
        assert_eq!(presence.unwrap().status, PresenceStatus::Online);
    }

    #[tokio::test]
    async fn test_get_online_agents() {
        let manager = PresenceManager::new();

        manager
            .set_status(
                "agent-1",
                PresenceEntityType::Agent,
                PresenceStatus::Online,
                None,
            )
            .await;
        manager
            .set_status(
                "agent-2",
                PresenceEntityType::Agent,
                PresenceStatus::Away,
                None,
            )
            .await;
        manager
            .set_status(
                "channel-1",
                PresenceEntityType::Channel,
                PresenceStatus::Online,
                None,
            )
            .await;

        let online_agents = manager.get_online_agents().await;
        assert_eq!(online_agents.len(), 1);
    }
}
