//! 会话修剪功能
//!
//! 提供自动清理旧会话和记忆的功能：
//! - 基于时间的清理
//! - 基于数量的限制
//! - 重要消息保护
//! - 清理策略配置

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// 修剪策略配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruningConfig {
    /// 最大会话年龄 (天)
    pub max_session_age_days: u64,
    /// 最大会话数量
    pub max_sessions: usize,
    /// 每个会话最大消息数
    pub max_messages_per_session: usize,
    /// 最大工作记忆大小 (条目)
    pub max_working_memory: usize,
    /// 最大短期记忆摘要数
    pub max_short_term_summaries: usize,
    /// 是否保护重要消息
    pub protect_important: bool,
    /// 重要性阈值 (高于此值不删除)
    pub importance_threshold: f32,
    /// 自动修剪间隔 (小时)
    pub auto_prune_interval_hours: u64,
    /// 是否启用自动修剪
    pub auto_prune_enabled: bool,
}

impl Default for PruningConfig {
    fn default() -> Self {
        Self {
            max_session_age_days: 30,
            max_sessions: 100,
            max_messages_per_session: 1000,
            max_working_memory: 50,
            max_short_term_summaries: 10,
            protect_important: true,
            importance_threshold: 0.7,
            auto_prune_interval_hours: 24,
            auto_prune_enabled: true,
        }
    }
}

/// 修剪统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PruningStats {
    /// 已删除的会话数
    pub sessions_pruned: usize,
    /// 已删除的消息数
    pub messages_pruned: usize,
    /// 已删除的记忆项数
    pub memory_items_pruned: usize,
    /// 受保护的消息数
    pub messages_protected: usize,
    /// 上次修剪时间
    pub last_pruned: Option<DateTime<Utc>>,
    /// 释放的空间 (字节估算)
    pub space_freed: usize,
}

/// 可修剪项 Trait
pub trait Prunable {
    /// 获取创建时间
    fn created_at(&self) -> DateTime<Utc>;
    /// 获取最后访问时间
    fn last_accessed(&self) -> DateTime<Utc>;
    /// 获取重要性分数 (0.0 - 1.0)
    fn importance(&self) -> f32;
    /// 获取大小估算 (字节)
    fn size_estimate(&self) -> usize;
    /// 是否应保护
    fn should_protect(&self) -> bool;
}

/// 会话修剪器
pub struct SessionPruner {
    config: PruningConfig,
    stats: Arc<RwLock<PruningStats>>,
}

impl SessionPruner {
    pub fn new(config: PruningConfig) -> Self {
        Self {
            config,
            stats: Arc::new(RwLock::new(PruningStats::default())),
        }
    }

    /// 修剪过期会话
    pub async fn prune_expired_sessions<T: Prunable + Clone>(
        &self,
        sessions: &mut HashMap<String, T>,
    ) -> usize {
        let now = Utc::now();
        let max_age = Duration::days(self.config.max_session_age_days as i64);
        let cutoff = now - max_age;

        let mut to_remove = Vec::new();
        let mut pruned = 0;
        let mut protected = 0;
        let mut space = 0;

        for (id, session) in sessions.iter() {
            let last_accessed = session.last_accessed();

            if last_accessed < cutoff {
                // 检查是否应保护
                if self.config.protect_important && session.should_protect() {
                    protected += 1;
                    continue;
                }

                if session.importance() >= self.config.importance_threshold {
                    protected += 1;
                    continue;
                }

                to_remove.push(id.clone());
                space += session.size_estimate();
            }
        }

        for id in to_remove {
            sessions.remove(&id);
            pruned += 1;
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.sessions_pruned += pruned;
            stats.messages_protected += protected;
            stats.space_freed += space;
            stats.last_pruned = Some(now);
        }

        if pruned > 0 {
            info!("修剪了 {} 个过期会话，保护了 {} 个", pruned, protected);
        }

        pruned
    }

    /// 限制会话数量
    pub async fn limit_session_count<T: Prunable + Clone>(
        &self,
        sessions: &mut HashMap<String, T>,
    ) -> usize {
        if sessions.len() <= self.config.max_sessions {
            return 0;
        }

        let excess = sessions.len() - self.config.max_sessions;
        if excess == 0 {
            return 0;
        }

        // 收集所有会话并按最后访问时间排序
        let mut session_list: Vec<_> = sessions.iter().collect();
        session_list.sort_by_key(|(_, s)| s.last_accessed());

        let mut to_remove = Vec::new();
        let mut pruned = 0;
        let mut space = 0;

        for (id, session) in session_list.into_iter().take(excess) {
            // 检查保护
            if self.config.protect_important && session.should_protect() {
                continue;
            }

            if session.importance() >= self.config.importance_threshold {
                continue;
            }

            to_remove.push(id.clone());
            space += session.size_estimate();
            pruned += 1;
        }

        for id in to_remove {
            sessions.remove(&id);
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.sessions_pruned += pruned;
            stats.space_freed += space;
        }

        if pruned > 0 {
            info!("限制会话数量，删除了 {} 个最旧会话", pruned);
        }

        pruned
    }

    /// 修剪会话中的消息
    pub async fn prune_session_messages<T: Prunable>(
        &self,
        messages: &mut Vec<T>,
    ) -> usize {
        if messages.len() <= self.config.max_messages_per_session {
            return 0;
        }

        let excess = messages.len() - self.config.max_messages_per_session;
        let mut to_remove = Vec::new();
        let mut pruned = 0;
        let protected = 0;
        let mut space = 0;

        // 按重要性排序，保留重要消息
        for (idx, msg) in messages.iter().enumerate() {
            if self.config.protect_important && msg.should_protect() {
                continue;
            }

            if msg.importance() >= self.config.importance_threshold {
                continue;
            }

            to_remove.push(idx);
            space += msg.size_estimate();

            if to_remove.len() >= excess {
                break;
            }
        }

        // 从后往前删除以保持索引正确
        to_remove.sort();
        to_remove.reverse();

        for idx in to_remove {
            messages.remove(idx);
            pruned += 1;
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.messages_pruned += pruned;
            stats.messages_protected += protected;
            stats.space_freed += space;
        }

        pruned
    }

    /// 修剪工作记忆
    pub async fn prune_working_memory<T: Prunable>(
        &self,
        items: &mut Vec<T>,
    ) -> usize {
        if items.len() <= self.config.max_working_memory {
            return 0;
        }

        let _excess = items.len() - self.config.max_working_memory;
        let mut pruned = 0;
        let mut space = 0;

        // 按重要性排序
        items.sort_by(|a, b| {
            b.importance()
                .partial_cmp(&a.importance())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // 移除最不重要的项
        while items.len() > self.config.max_working_memory {
            if let Some(item) = items.pop() {
                space += item.size_estimate();
                pruned += 1;
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.memory_items_pruned += pruned;
            stats.space_freed += space;
        }

        pruned
    }

    /// 修剪短期记忆摘要
    pub async fn prune_short_term<T: Prunable + Clone>(
        &self,
        summaries: &mut Vec<T>,
    ) -> usize {
        if summaries.len() <= self.config.max_short_term_summaries {
            return 0;
        }

        let _excess = summaries.len() - self.config.max_short_term_summaries;
        let mut pruned = 0;
        let mut space = 0;

        // 按创建时间排序，保留最新的
        summaries.sort_by_key(|s| s.created_at());

        // 移除最旧的
        while summaries.len() > self.config.max_short_term_summaries {
            if let Some(item) = summaries.first().cloned() {
                space += item.size_estimate();
                summaries.remove(0);
                pruned += 1;
            }
        }

        // 更新统计
        {
            let mut stats = self.stats.write().await;
            stats.memory_items_pruned += pruned;
            stats.space_freed += space;
        }

        pruned
    }

    /// 执行完整修剪
    pub async fn full_prune<
        T: Prunable + Clone,
        M: Prunable,
        W: Prunable,
        S: Prunable + Clone,
    >(
        &self,
        sessions: &mut HashMap<String, T>,
        session_messages: &mut HashMap<String, Vec<M>>,
        working_memory: &mut Vec<W>,
        short_term: &mut Vec<S>,
    ) -> PruningStats {
        // 修剪过期会话
        self.prune_expired_sessions(sessions).await;

        // 限制会话数量
        self.limit_session_count(sessions).await;

        // 修剪每个会话的消息
        for messages in session_messages.values_mut() {
            self.prune_session_messages(messages).await;
        }

        // 修剪记忆
        self.prune_working_memory(working_memory).await;
        self.prune_short_term(short_term).await;

        let stats = self.stats.read().await.clone();
        info!(
            "完整修剪完成: 会话={}, 消息={}, 记忆={}, 释放空间={}字节",
            stats.sessions_pruned,
            stats.messages_pruned,
            stats.memory_items_pruned,
            stats.space_freed
        );

        stats
    }

    /// 获取统计信息
    pub async fn get_stats(&self) -> PruningStats {
        self.stats.read().await.clone()
    }

    /// 重置统计
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = PruningStats::default();
    }

    /// 估算总大小
    pub fn estimate_total_size<T: Prunable>(items: &Vec<T>) -> usize {
        items.iter().map(|i| i.size_estimate()).sum()
    }

    /// 检查是否需要修剪
    pub fn needs_pruning<T: Prunable>(
        &self,
        sessions: &HashMap<String, T>,
    ) -> bool {
        let now = Utc::now();
        let max_age = Duration::days(self.config.max_session_age_days as i64);
        let cutoff = now - max_age;

        sessions.len() > self.config.max_sessions
            || sessions.values().any(|s| s.last_accessed() < cutoff)
    }
}

impl Default for SessionPruner {
    fn default() -> Self {
        Self::new(PruningConfig::default())
    }
}

/// 自动修剪任务
pub struct AutoPruner {
    config: PruningConfig,
    running: Arc<RwLock<bool>>,
}

impl AutoPruner {
    pub fn new(config: PruningConfig) -> Self {
        Self {
            config,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// 启动自动修剪任务
    pub async fn start(&self, _pruner: Arc<SessionPruner>) {
        if !self.config.auto_prune_enabled {
            info!("自动修剪已禁用");
            return;
        }

        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        let interval = self.config.auto_prune_interval_hours;
        let running_clone = self.running.clone();

        tokio::spawn(async move {
            let mut interval_timer =
                tokio::time::interval(tokio::time::Duration::from_secs(interval * 3600));

            loop {
                interval_timer.tick().await;

                let is_running = *running_clone.read().await;
                if !is_running {
                    break;
                }

                info!("执行自动修剪...");
                // 实际修剪由外部调用 full_prune
            }
        });

        info!("自动修剪任务已启动，间隔: {} 小时", interval);
    }

    /// 停止自动修剪
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("自动修剪任务已停止");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestItem {
        created: DateTime<Utc>,
        accessed: DateTime<Utc>,
        importance: f32,
        size: usize,
    }

    impl Prunable for TestItem {
        fn created_at(&self) -> DateTime<Utc> {
            self.created
        }
        fn last_accessed(&self) -> DateTime<Utc> {
            self.accessed
        }
        fn importance(&self) -> f32 {
            self.importance
        }
        fn size_estimate(&self) -> usize {
            self.size
        }
        fn should_protect(&self) -> bool {
            self.importance >= 0.9
        }
    }

    #[tokio::test]
    async fn test_prune_expired() {
        let config = PruningConfig {
            max_session_age_days: 1,
            ..Default::default()
        };
        let pruner = SessionPruner::new(config);

        let mut sessions = HashMap::new();

        // 添加过期会话
        sessions.insert(
            "old".to_string(),
            TestItem {
                created: Utc::now() - Duration::days(2),
                accessed: Utc::now() - Duration::days(2),
                importance: 0.1,
                size: 100,
            },
        );

        // 添加新会话
        sessions.insert(
            "new".to_string(),
            TestItem {
                created: Utc::now(),
                accessed: Utc::now(),
                importance: 0.1,
                size: 100,
            },
        );

        let pruned = pruner.prune_expired_sessions(&mut sessions).await;
        assert_eq!(pruned, 1);
        assert_eq!(sessions.len(), 1);
    }

    #[tokio::test]
    async fn test_limit_count() {
        let config = PruningConfig {
            max_sessions: 2,
            ..Default::default()
        };
        let pruner = SessionPruner::new(config);

        let mut sessions = HashMap::new();

        for i in 0..5 {
            sessions.insert(
                format!("session-{}", i),
                TestItem {
                    created: Utc::now(),
                    accessed: Utc::now() - Duration::hours(i as i64),
                    importance: 0.1,
                    size: 100,
                },
            );
        }

        let pruned = pruner.limit_session_count(&mut sessions).await;
        assert_eq!(pruned, 3);
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_protect_important() {
        let config = PruningConfig {
            max_sessions: 1,
            protect_important: true,
            importance_threshold: 0.8,
            ..Default::default()
        };
        let pruner = SessionPruner::new(config);

        let mut sessions = HashMap::new();

        // 添加重要会话
        sessions.insert(
            "important".to_string(),
            TestItem {
                created: Utc::now() - Duration::days(10),
                accessed: Utc::now() - Duration::days(10),
                importance: 0.9,
                size: 100,
            },
        );

        // 添加普通会话
        sessions.insert(
            "normal".to_string(),
            TestItem {
                created: Utc::now(),
                accessed: Utc::now(),
                importance: 0.1,
                size: 100,
            },
        );

        let pruned = pruner.prune_expired_sessions(&mut sessions).await;
        // 重要会话应该被保护
        assert!(sessions.contains_key("important"));
    }
}
