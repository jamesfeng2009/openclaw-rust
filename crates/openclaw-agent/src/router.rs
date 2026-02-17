//! Agent 路由 - 根据消息来源分配 Agent
//!
//! 支持多种路由策略：
//! - 按通道类型 (channel_type)
//! - 按用户 ID (user_id)
//! - 按聊天 ID (chat_id)
//! - 按关键词匹配 (keyword)
//! - 按正则表达式 (regex)
//! - 轮询 (round_robin)
//! - 随机 (random)

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::types::AgentId;

/// 路由来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RouteSource {
    ChannelType(String),
    UserId(String),
    ChatId(String),
    Keyword(String),
    Regex(String),
}

impl RouteSource {
    pub fn channel_type(channel: impl Into<String>) -> Self {
        RouteSource::ChannelType(channel.into())
    }

    pub fn user_id(user_id: impl Into<String>) -> Self {
        RouteSource::UserId(user_id.into())
    }

    pub fn chat_id(chat_id: impl Into<String>) -> Self {
        RouteSource::ChatId(chat_id.into())
    }

    pub fn keyword(keyword: impl Into<String>) -> Self {
        RouteSource::Keyword(keyword.into())
    }

    pub fn regex(pattern: impl Into<String>) -> Self {
        RouteSource::Regex(pattern.into())
    }
}

/// 路由策略类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteStrategy {
    /// 直接指定 Agent ID
    Direct(AgentId),
    /// 按通道类型路由
    ByChannel,
    /// 按用户 ID 路由
    ByUser,
    /// 按聊天 ID 路由
    ByChat,
    /// 关键词匹配
    ByKeyword,
    /// 正则表达式匹配
    ByRegex,
    /// 轮询分配
    RoundRobin,
    /// 随机分配
    Random,
    /// 首选 + 备用
    Fallback {
        primary: AgentId,
        fallback: AgentId,
    },
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 匹配条件
    pub source: RouteSource,
    /// 目标 Agent ID
    pub target: AgentId,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 优先级 (越高越先匹配)
    pub priority: i32,
}

fn default_true() -> bool {
    true
}

impl RouteRule {
    pub fn new(id: impl Into<String>, name: impl Into<String>, source: RouteSource, target: AgentId) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            source,
            target,
            enabled: true,
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// 路由上下文 - 用于匹配规则
#[derive(Debug, Clone)]
pub struct RouteContext {
    /// 通道类型
    pub channel_type: String,
    /// 用户 ID
    pub user_id: String,
    /// 聊天 ID
    pub chat_id: String,
    /// 消息内容
    pub content: String,
    /// 元数据
    pub metadata: Option<serde_json::Value>,
}

impl RouteContext {
    pub fn new(
        channel_type: impl Into<String>,
        user_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel_type: channel_type.into(),
            user_id: user_id.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            metadata: None,
        }
    }
}

/// Agent 路由器
pub struct AgentRouter {
    rules: Arc<RwLock<Vec<RouteRule>>>,
    default_agent: Arc<RwLock<Option<AgentId>>>,
    strategy: Arc<RwLock<RouteStrategy>>,
    round_robin_index: Arc<RwLock<usize>>,
    available_agents: Arc<RwLock<Vec<AgentId>>>,
}

impl AgentRouter {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(RwLock::new(Vec::new())),
            default_agent: Arc::new(RwLock::new(None)),
            strategy: Arc::new(RwLock::new(RouteStrategy::ByChannel)),
            round_robin_index: Arc::new(RwLock::new(0)),
            available_agents: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 添加路由规则
    pub async fn add_rule(&self, rule: RouteRule) {
        let mut rules = self.rules.write().await;
        rules.push(rule);
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 批量添加规则
    pub async fn add_rules(&self, rules: Vec<RouteRule>) {
        let mut rules_lock = self.rules.write().await;
        rules_lock.extend(rules);
        rules_lock.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// 删除路由规则
    pub async fn remove_rule(&self, rule_id: &str) {
        let mut rules = self.rules.write().await;
        rules.retain(|r| r.id != rule_id);
    }

    /// 设置默认 Agent
    pub async fn set_default_agent(&self, agent_id: AgentId) {
        let mut default = self.default_agent.write().await;
        *default = Some(agent_id);
    }

    /// 设置路由策略
    pub async fn set_strategy(&self, strategy: RouteStrategy) {
        let mut strat = self.strategy.write().await;
        *strat = strategy;
    }

    /// 设置可用 Agent 列表 (用于 RoundRobin/Random)
    pub async fn set_available_agents(&self, agents: Vec<AgentId>) {
        let mut available = self.available_agents.write().await;
        *available = agents;
    }

    /// 根据上下文路由到 Agent
    pub async fn route(&self, context: &RouteContext) -> Option<AgentId> {
        let strategy = self.strategy.read().await.clone();
        
        match strategy {
            RouteStrategy::Direct(id) => Some(id),
            RouteStrategy::ByChannel => self.route_by_channel(context).await,
            RouteStrategy::ByUser => self.route_by_user(context).await,
            RouteStrategy::ByChat => self.route_by_chat(context).await,
            RouteStrategy::ByKeyword => self.route_by_keyword(context).await,
            RouteStrategy::ByRegex => self.route_by_regex(context).await,
            RouteStrategy::RoundRobin => self.route_round_robin().await,
            RouteStrategy::Random => self.route_random().await,
            RouteStrategy::Fallback { primary: _, fallback } => {
                if let Some(id) = self.try_route(context).await {
                    Some(id)
                } else {
                    Some(fallback)
                }
            }
        }
    }

    /// 尝试匹配规则
    async fn try_route(&self, context: &RouteContext) -> Option<AgentId> {
        let rules = self.rules.read().await;
        
        for rule in rules.iter() {
            if !rule.enabled {
                continue;
            }
            
            if self.match_rule(rule, context) {
                return Some(rule.target.clone());
            }
        }
        
        None
    }

    /// 匹配规则
    fn match_rule(&self, rule: &RouteRule, context: &RouteContext) -> bool {
        match &rule.source {
            RouteSource::ChannelType(channel) => {
                context.channel_type.to_lowercase() == channel.to_lowercase()
            }
            RouteSource::UserId(user_id) => {
                context.user_id == *user_id
            }
            RouteSource::ChatId(chat_id) => {
                context.chat_id == *chat_id
            }
            RouteSource::Keyword(keyword) => {
                context.content.to_lowercase().contains(&keyword.to_lowercase())
            }
            RouteSource::Regex(pattern) => {
                if let Ok(re) = regex::Regex::new(pattern) {
                    re.is_match(&context.content)
                } else {
                    false
                }
            }
        }
    }

    /// 按通道类型路由
    async fn route_by_channel(&self, context: &RouteContext) -> Option<AgentId> {
        if let Some(id) = self.try_route(context).await {
            return Some(id);
        }
        self.get_default().await
    }

    /// 按用户 ID 路由
    async fn route_by_user(&self, context: &RouteContext) -> Option<AgentId> {
        if let Some(id) = self.try_route(context).await {
            return Some(id);
        }
        self.get_default().await
    }

    /// 按聊天 ID 路由
    async fn route_by_chat(&self, context: &RouteContext) -> Option<AgentId> {
        if let Some(id) = self.try_route(context).await {
            return Some(id);
        }
        self.get_default().await
    }

    /// 按关键词路由
    async fn route_by_keyword(&self, context: &RouteContext) -> Option<AgentId> {
        if let Some(id) = self.try_route(context).await {
            return Some(id);
        }
        self.get_default().await
    }

    /// 按正则表达式路由
    async fn route_by_regex(&self, context: &RouteContext) -> Option<AgentId> {
        if let Some(id) = self.try_route(context).await {
            return Some(id);
        }
        self.get_default().await
    }

    /// 轮询分配
    async fn route_round_robin(&self) -> Option<AgentId> {
        let agents = self.available_agents.read().await;
        if agents.is_empty() {
            return self.get_default().await;
        }
        
        let mut index = self.round_robin_index.write().await;
        let agent = agents[*index % agents.len()].clone();
        *index += 1;
        Some(agent)
    }

    /// 随机分配
    async fn route_random(&self) -> Option<AgentId> {
        let agents = self.available_agents.read().await;
        if agents.is_empty() {
            return self.get_default().await;
        }
        
        use std::time::{SystemTime, UNIX_EPOCH};
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as usize;
        
        let index = seed % agents.len();
        Some(agents[index].clone())
    }

    /// 获取默认 Agent
    async fn get_default(&self) -> Option<AgentId> {
        let default = self.default_agent.read().await;
        default.clone()
    }

    /// 列出所有路由规则
    pub async fn list_rules(&self) -> Vec<RouteRule> {
        let rules = self.rules.read().await;
        rules.clone()
    }

    /// 更新规则状态
    pub async fn set_rule_enabled(&self, rule_id: &str, enabled: bool) {
        let mut rules = self.rules.write().await;
        if let Some(rule) = rules.iter_mut().find(|r| r.id == rule_id) {
            rule.enabled = enabled;
        }
    }
}

impl Default for AgentRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// 路由管理器 - 管理多个路由器
pub struct RouterManager {
    routers: Arc<RwLock<HashMap<String, Arc<AgentRouter>>>>,
}

impl RouterManager {
    pub fn new() -> Self {
        Self {
            routers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建或获取路由器
    pub async fn get_or_create(&self, name: &str) -> Arc<AgentRouter> {
        let mut routers = self.routers.write().await;
        
        if let Some(router) = routers.get(name) {
            return router.clone();
        }
        
        let router = Arc::new(AgentRouter::new());
        routers.insert(name.to_string(), router.clone());
        router
    }

    /// 删除路由器
    pub async fn remove(&self, name: &str) {
        let mut routers = self.routers.write().await;
        routers.remove(name);
    }

    /// 列出所有路由器
    pub async fn list_routers(&self) -> Vec<String> {
        let routers = self.routers.read().await;
        routers.keys().cloned().collect()
    }
}

impl Default for RouterManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_route_by_channel() {
        let router = AgentRouter::new();
        
        router.add_rule(RouteRule::new(
            "telegram-rule",
            "Telegram 路由",
            RouteSource::channel_type("telegram"),
            "agent-telegram".to_string(),
        )).await;
        
        router.set_default_agent("default-agent".to_string()).await;
        
        let context = RouteContext {
            channel_type: "telegram".to_string(),
            user_id: "user1".to_string(),
            chat_id: "chat1".to_string(),
            content: "Hello".to_string(),
            metadata: None,
        };
        
        let agent = router.route(&context).await;
        assert_eq!(agent, Some("agent-telegram".to_string()));
    }

    #[tokio::test]
    async fn test_route_by_keyword() {
        let router = AgentRouter::new();
        
        router.add_rule(RouteRule::new(
            "help-rule",
            "帮助命令",
            RouteSource::keyword("help"),
            "agent-help".to_string(),
        )).await;
        
        router.set_default_agent("default-agent".to_string()).await;
        
        let context = RouteContext {
            channel_type: "telegram".to_string(),
            user_id: "user1".to_string(),
            chat_id: "chat1".to_string(),
            content: "Please help me".to_string(),
            metadata: None,
        };
        
        let agent = router.route(&context).await;
        assert_eq!(agent, Some("agent-help".to_string()));
    }

    #[tokio::test]
    async fn test_round_robin() {
        let router = AgentRouter::new();
        
        router.set_strategy(RouteStrategy::RoundRobin).await;
        router.set_available_agents(vec![
            "agent-1".to_string(),
            "agent-2".to_string(),
            "agent-3".to_string(),
        ]).await;
        
        let context = RouteContext {
            channel_type: "telegram".to_string(),
            user_id: "user1".to_string(),
            chat_id: "chat1".to_string(),
            content: "Hello".to_string(),
            metadata: None,
        };
        
        let agent1 = router.route(&context).await;
        let agent2 = router.route(&context).await;
        let agent3 = router.route(&context).await;
        let agent4 = router.route(&context).await;
        
        assert_eq!(agent1, Some("agent-1".to_string()));
        assert_eq!(agent2, Some("agent-2".to_string()));
        assert_eq!(agent3, Some("agent-3".to_string()));
        assert_eq!(agent4, Some("agent-1".to_string()));
    }
}
