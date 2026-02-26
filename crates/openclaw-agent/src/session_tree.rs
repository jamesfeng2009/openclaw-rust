//! Session Tree - 树形会话管理
//!
//! 提供非线性会话支持：
//! - 分支创建与切换
//! - 会话历史回溯
//! - 分支合并
//! - 上下文路径构建

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use openclaw_core::session::SessionScope;

use crate::types::AgentId;

/// 会话节点（支持树形分支）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNode {
    /// 节点唯一标识
    pub id: Uuid,
    /// 父节点 ID（根节点为 None）
    pub parent_id: Option<Uuid>,
    /// 分支标识（同一父节点的分支用 branch_id 区分）
    pub branch_id: Uuid,
    /// 会话名称
    pub name: String,
    /// 关联的 Agent ID
    pub agent_id: AgentId,
    /// 会话作用域
    pub scope: SessionScope,
    /// 通道类型
    pub channel_type: Option<String>,
    /// 账户 ID
    pub account_id: Option<String>,
    /// 对端 ID
    pub peer_id: Option<String>,
    /// 会话状态
    pub state: SessionTreeState,
    /// 消息历史
    pub message_history: Vec<SessionMessage>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 消息数量
    pub message_count: usize,
    /// Token 数量
    pub token_count: u64,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SessionNode {
    /// 创建根节点
    pub fn root(
        name: impl Into<String>,
        agent_id: AgentId,
        scope: SessionScope,
    ) -> Self {
        let now = Utc::now();
        let branch_id = Uuid::new_v4();
        Self {
            id: Uuid::new_v4(),
            parent_id: None,
            branch_id,
            name: name.into(),
            agent_id,
            scope,
            channel_type: None,
            account_id: None,
            peer_id: None,
            state: SessionTreeState::Active,
            message_history: Vec::new(),
            created_at: now,
            updated_at: now,
            last_active_at: now,
            message_count: 0,
            token_count: 0,
            metadata: HashMap::new(),
        }
    }

    /// 从现有会话创建根节点
    pub fn from_session(session: &crate::sessions::Session) -> Self {
        let now = Utc::now();
        Self {
            id: session.id,
            parent_id: None,
            branch_id: Uuid::new_v4(),
            name: session.name.clone(),
            agent_id: session.agent_id.clone(),
            scope: session.scope.clone(),
            channel_type: session.channel_type.clone(),
            account_id: session.account_id.clone(),
            peer_id: session.peer_id.clone(),
            state: SessionTreeState::Active,
            message_history: Vec::new(),
            created_at: session.created_at,
            updated_at: now,
            last_active_at: session.last_active_at,
            message_count: session.message_count,
            token_count: session.token_count,
            metadata: session.metadata.clone(),
        }
    }

    /// 创建分支
    pub fn branch(&self, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            parent_id: Some(self.id),
            branch_id: Uuid::new_v4(),
            name: name.into(),
            agent_id: self.agent_id.clone(),
            scope: self.scope.clone(),
            channel_type: self.channel_type.clone(),
            account_id: self.account_id.clone(),
            peer_id: self.peer_id.clone(),
            state: SessionTreeState::Active,
            message_history: Vec::new(),
            created_at: now,
            updated_at: now,
            last_active_at: now,
            message_count: 0,
            token_count: 0,
            metadata: self.metadata.clone(),
        }
    }

    /// 检查是否为根节点
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// 检查是否为分支节点
    #[inline]
    pub fn is_branch(&self) -> bool {
        self.parent_id.is_some()
    }

    /// 添加消息
    pub fn add_message(&mut self, role: &str, content: impl Into<String>, tokens: u64) {
        self.message_history.push(SessionMessage {
            id: Uuid::new_v4(),
            role: role.to_string(),
            content: content.into(),
            timestamp: Utc::now(),
        });
        self.message_count += 1;
        self.token_count += tokens;
        self.updated_at = Utc::now();
        self.last_active_at = Utc::now();
    }

    /// 关闭节点
    pub fn close(&mut self) {
        self.state = SessionTreeState::Closed;
        self.updated_at = Utc::now();
    }

    /// 检查是否活跃
    #[inline]
    pub fn is_active(&self) -> bool {
        self.state == SessionTreeState::Active
    }

    /// 获取会话键
    pub fn session_key(&self) -> String {
        match self.scope {
            SessionScope::Main => "main".to_string(),
            SessionScope::PerPeer => format!(
                "{}:{}",
                self.channel_type.as_deref().unwrap_or("unknown"),
                self.peer_id.as_deref().unwrap_or("unknown")
            ),
            SessionScope::PerChannelPeer => format!(
                "{}:{}:{}",
                self.channel_type.as_deref().unwrap_or("unknown"),
                self.account_id.as_deref().unwrap_or("unknown"),
                self.peer_id.as_deref().unwrap_or("unknown")
            ),
            SessionScope::PerAccountChannelPeer => format!(
                "{}:{}:{}",
                self.channel_type.as_deref().unwrap_or("unknown"),
                self.account_id.as_deref().unwrap_or("unknown"),
                self.peer_id.as_deref().unwrap_or("unknown")
            ),
        }
    }
}

/// 会话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: Uuid,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

/// 会话树状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionTreeState {
    Active,
    Paused,
    Closed,
}

/// 会话树管理器
#[derive(Clone)]
pub struct SessionTree {
    nodes: Arc<RwLock<HashMap<Uuid, SessionNode>>>,
    active_path: Arc<RwLock<Vec<Uuid>>>,
    root_id: Arc<RwLock<Option<Uuid>>>,
}

impl SessionTree {
    /// 创建新的空会话树
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            active_path: Arc::new(RwLock::new(Vec::new())),
            root_id: Arc::new(RwLock::new(None)),
        }
    }

    /// 初始化根节点（用于测试）
    pub async fn init_root(&self, name: impl Into<String>) {
        let node = SessionNode::root(name, "system".to_string(), SessionScope::Main);
        let id = node.id;
        
        let mut nodes = self.nodes.write().await;
        nodes.insert(id, node);
        
        let mut root_id = self.root_id.write().await;
        *root_id = Some(id);
        
        let mut active_path = self.active_path.write().await;
        active_path.push(id);
    }

    /// 从现有会话创建根节点
    pub async fn create_from_session(&self, session: &crate::sessions::Session) -> SessionNode {
        let node = SessionNode::from_session(session);
        let id = node.id;
        
        let mut nodes = self.nodes.write().await;
        nodes.insert(id, node.clone());
        
        let mut root_id = self.root_id.write().await;
        *root_id = Some(id);
        
        let mut active_path = self.active_path.write().await;
        active_path.push(id);
        
        node
    }

    /// 创建新的根会话
    pub async fn create_session(
        &self,
        name: impl Into<String>,
        agent_id: AgentId,
        scope: SessionScope,
    ) -> SessionNode {
        let node = SessionNode::root(name, agent_id, scope);
        let id = node.id;
        
        let mut nodes = self.nodes.write().await;
        nodes.insert(id, node.clone());
        
        let mut root_id = self.root_id.write().await;
        *root_id = Some(id);
        
        let mut active_path = self.active_path.write().await;
        active_path.push(id);
        
        node
    }

    /// 在当前节点创建分支
    pub async fn branch(&self, name: impl Into<String>) -> Option<SessionNode> {
        let current_id = {
            let active_path = self.active_path.read().await;
            *active_path.last()?
        };
        
        let mut nodes = self.nodes.write().await;
        let parent = nodes.get(&current_id)?.clone();
        let child = parent.branch(name);
        
        nodes.insert(child.id, child.clone());
        
        let mut active_path = self.active_path.write().await;
        active_path.push(child.id);
        
        Some(child)
    }

    /// 切换到指定分支
    pub async fn switch_to(&self, node_id: Uuid) -> Option<SessionNode> {
        let _node = {
            let nodes = self.nodes.read().await;
            if !nodes.contains_key(&node_id) {
                return None;
            }
            nodes.get(&node_id)?.clone()
        };
        
        let mut active_path = self.active_path.write().await;
        
        let mut path = Vec::new();
        let mut current_id: Option<Uuid> = Some(node_id);
        
        while let Some(id) = current_id {
            path.push(id);
            let nodes = self.nodes.read().await;
            current_id = nodes.get(&id)?.parent_id;
        }
        
        path.reverse();
        *active_path = path;
        
        let nodes = self.nodes.read().await;
        nodes.get(&node_id).cloned()
    }

    /// 切换到父节点
    pub async fn switch_to_parent(&self) -> Option<SessionNode> {
        let mut active_path = self.active_path.write().await;
        
        if active_path.len() <= 1 {
            return None;
        }
        
        active_path.pop();
        let parent_id = *active_path.last()?;
        
        drop(active_path);
        
        let nodes = self.nodes.read().await;
        nodes.get(&parent_id).cloned()
    }

    /// 切换到根节点
    pub async fn switch_to_root(&self) -> Option<SessionNode> {
        let id = {
            let root_id = self.root_id.read().await;
            *root_id.as_ref()?
        };
        
        self.switch_to(id).await
    }

    /// 获取当前节点
    pub async fn current(&self) -> Option<SessionNode> {
        let active_path = self.active_path.read().await;
        let id = active_path.last()?;
        
        let nodes = self.nodes.read().await;
        nodes.get(id).cloned()
    }

    /// 获取当前节点的父节点
    pub async fn parent(&self) -> Option<SessionNode> {
        let active_path = self.active_path.read().await;
        
        if active_path.len() <= 1 {
            return None;
        }
        
        let parent_id = active_path[active_path.len() - 2];
        
        let nodes = self.nodes.read().await;
        nodes.get(&parent_id).cloned()
    }

    /// 获取根节点
    pub async fn root(&self) -> Option<SessionNode> {
        let root_id = self.root_id.read().await;
        let id = root_id.as_ref()?;
        
        let nodes = self.nodes.read().await;
        nodes.get(id).cloned()
    }

    /// 获取当前节点的所有祖先（用于上下文构建）
    pub async fn ancestors(&self) -> Vec<SessionNode> {
        let active_path = self.active_path.read().await;
        let nodes = self.nodes.read().await;
        
        active_path
            .iter()
            .filter_map(|id| nodes.get(id).cloned())
            .collect()
    }

    /// 获取从根到当前节点的完整路径
    pub async fn path(&self) -> Vec<SessionNode> {
        self.ancestors().await
    }

    /// 获取当前节点的所有直接子节点
    pub async fn children(&self) -> Vec<SessionNode> {
        let current_id = {
            let active_path = self.active_path.read().await;
            active_path.last().copied()
        };
        
        let current_id = match current_id {
            Some(id) => id,
            None => return Vec::new(),
        };
        
        let nodes = self.nodes.read().await;
        
        nodes
            .values()
            .filter(|node| node.parent_id.as_ref() == Some(&current_id))
            .cloned()
            .collect()
    }

    /// 获取当前节点的所有分支
    pub async fn branches(&self) -> Vec<SessionNode> {
        self.children().await
    }

    /// 获取所有节点
    pub async fn all_nodes(&self) -> Vec<SessionNode> {
        let nodes = self.nodes.read().await;
        nodes.values().cloned().collect()
    }

    /// 获取根节点的所有后代节点数量
    pub async fn descendant_count(&self) -> usize {
        let root_id = self.root_id.read().await;
        let root = root_id.as_ref();
        
        if root.is_none() {
            return 0;
        }
        
        let nodes = self.nodes.read().await;
        let mut count: usize = 0;
        let mut stack = vec![*root.unwrap()];
        
        while let Some(id) = stack.pop() {
            if nodes.get(&id).is_some() {
                count += 1;
                for child_id in nodes.values()
                    .filter(|n| n.parent_id.as_ref() == Some(&id))
                    .map(|n| n.id)
                {
                    stack.push(child_id);
                }
            }
        }
        
        count.saturating_sub(1)
    }

    /// 检查节点是否存在
    pub async fn contains(&self, node_id: Uuid) -> bool {
        let nodes = self.nodes.read().await;
        nodes.contains_key(&node_id)
    }

    /// 获取节点数量
    pub async fn len(&self) -> usize {
        let nodes = self.nodes.read().await;
        nodes.len()
    }

    /// 检查是否为空
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 添加消息到当前节点
    pub async fn add_message(&self, role: &str, content: impl Into<String>, tokens: u64) -> Option<()> {
        let active_path = self.active_path.read().await;
        let current_id = active_path.last()?;
        
        let mut nodes = self.nodes.write().await;
        let node = nodes.get_mut(current_id)?;
        node.add_message(role, content, tokens);
        
        Some(())
    }

    /// 关闭当前节点
    pub async fn close_current(&self) -> Option<()> {
        let active_path = self.active_path.read().await;
        let current_id = active_path.last()?;
        
        let mut nodes = self.nodes.write().await;
        let node = nodes.get_mut(current_id)?;
        node.close();
        
        Some(())
    }

    /// 删除节点及其所有后代
    pub async fn delete_node(&self, node_id: Uuid) -> bool {
        let mut nodes = self.nodes.write().await;
        
        if !nodes.contains_key(&node_id) {
            return false;
        }
        
        let mut to_delete = vec![node_id];
        let mut deleted = 0;
        
        while let Some(id) = to_delete.pop() {
            if nodes.remove(&id).is_some() {
                deleted += 1;
                for child in nodes.values()
                    .filter(|n| n.parent_id == Some(id))
                {
                    to_delete.push(child.id);
                }
            }
        }
        
        if deleted > 0 {
            let mut active_path = self.active_path.write().await;
            active_path.retain(|id| nodes.contains_key(id));
            
            let mut root_id = self.root_id.write().await;
            if let Some(current_root) = *root_id {
                if !nodes.contains_key(&current_root) {
                    *root_id = None;
                }
            }
        }
        
        deleted > 0
    }

    /// 获取当前路径深度
    pub async fn depth(&self) -> usize {
        let active_path = self.active_path.read().await;
        active_path.len()
    }
}

impl Default for SessionTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionTree {
    /// 获取上下文构建所需的消息（从根到当前）
    pub async fn get_context_messages(&self) -> Vec<SessionMessage> {
        let path = self.path().await;
        path.into_iter()
            .flat_map(|node| node.message_history.clone())
            .collect()
    }
}

/// 分支信息（用于显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchInfo {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub message_count: usize,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
}

impl From<&SessionNode> for BranchInfo {
    fn from(node: &SessionNode) -> Self {
        Self {
            id: node.id,
            name: node.name.clone(),
            parent_id: node.parent_id,
            message_count: node.message_count,
            created_at: node.created_at,
            is_active: node.is_active(),
        }
    }
}

/// 会话树状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTreeStatus {
    pub total_nodes: usize,
    pub active_path_length: usize,
    pub root_id: Option<Uuid>,
    pub current_id: Option<Uuid>,
    pub current_branch_name: Option<String>,
    pub depth: usize,
}

impl SessionTree {
    /// 获取会话树状态
    pub async fn status(&self) -> SessionTreeStatus {
        let nodes = self.nodes.read().await;
        let active_path = self.active_path.read().await;
        let root_id = self.root_id.read().await;
        
        let current = active_path.last().copied();
        let current_name = current.and_then(|id| {
            nodes.get(&id).map(|n| n.name.clone())
        });
        
        SessionTreeStatus {
            total_nodes: nodes.len(),
            active_path_length: active_path.len(),
            root_id: *root_id,
            current_id: current,
            current_branch_name: current_name,
            depth: active_path.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> SessionTree {
        SessionTree::new()
    }

    #[tokio::test]
    async fn test_create_session() {
        let tree = create_test_tree();
        
        let node = tree
            .create_session("test-session", "agent-1".to_string(), SessionScope::PerPeer)
            .await;
        
        assert_eq!(node.name, "test-session");
        assert_eq!(node.agent_id, "agent-1");
        assert!(node.is_root());
        assert!(!node.is_branch());
        
        let current = tree.current().await.unwrap();
        assert_eq!(current.id, node.id);
    }

    #[tokio::test]
    async fn test_branch() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        let branch = tree.branch("experiment-1").await.unwrap();
        
        assert!(branch.is_branch());
        assert_eq!(branch.parent_id, Some(tree.root().await.unwrap().id));
        
        let current = tree.current().await.unwrap();
        assert_eq!(current.id, branch.id);
    }

    #[tokio::test]
    async fn test_switch_to_parent() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        let root_id = tree.root().await.unwrap().id;
        
        tree.branch("experiment-1").await.unwrap();
        
        let parent = tree.switch_to_parent().await.unwrap();
        assert_eq!(parent.id, root_id);
    }

    #[tokio::test]
    async fn test_switch_to_root() {
        let tree = create_test_tree();
        
        let root = tree
            .create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.branch("branch-1").await.unwrap();
        tree.branch("branch-2").await.unwrap();
        
        tree.switch_to_root().await.unwrap();
        
        let current = tree.current().await.unwrap();
        assert_eq!(current.id, root.id);
    }

    #[tokio::test]
    async fn test_multiple_branches() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.branch("branch-1").await.unwrap();
        tree.switch_to_root().await.unwrap();
        tree.branch("branch-2").await.unwrap();
        
        let branches = tree.branches().await;
        assert_eq!(branches.len(), 0);
        
        tree.switch_to_root().await.unwrap();
        let root_branches = tree.branches().await;
        assert_eq!(root_branches.len(), 2);
    }

    #[tokio::test]
    async fn test_add_message() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.add_message("user", "Hello", 10).await;
        tree.add_message("assistant", "Hi there", 20).await;
        
        let current = tree.current().await.unwrap();
        assert_eq!(current.message_count, 2);
        assert_eq!(current.token_count, 30);
    }

    #[tokio::test]
    async fn test_path() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        let root_id = tree.root().await.unwrap().id;
        
        tree.branch("branch-1").await.unwrap();
        
        let path = tree.path().await;
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].id, root_id);
    }

    #[tokio::test]
    async fn test_depth() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        assert_eq!(tree.depth().await, 1);
        
        tree.branch("branch-1").await.unwrap();
        assert_eq!(tree.depth().await, 2);
        
        tree.branch("branch-2").await.unwrap();
        assert_eq!(tree.depth().await, 3);
        
        tree.switch_to_root().await.unwrap();
        assert_eq!(tree.depth().await, 1);
    }

    #[tokio::test]
    async fn test_close_current() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.close_current().await;
        
        let current = tree.current().await.unwrap();
        assert!(!current.is_active());
    }

    #[tokio::test]
    async fn test_delete_node() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        let branch = tree.branch("branch-1").await.unwrap();
        let branch_id = branch.id;
        
        tree.switch_to_root().await.unwrap();
        
        let deleted = tree.delete_node(branch_id).await;
        assert!(deleted);
        
        let contains = tree.contains(branch_id).await;
        assert!(!contains);
    }

    #[tokio::test]
    async fn test_status() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.branch("branch-1").await.unwrap();
        
        let status = tree.status().await;
        assert_eq!(status.total_nodes, 2);
        assert_eq!(status.depth, 2);
        assert!(status.root_id.is_some());
        assert!(status.current_id.is_some());
    }

    #[tokio::test]
    async fn test_len_and_is_empty() {
        let tree = create_test_tree();
        
        assert!(tree.is_empty().await);
        assert_eq!(tree.len().await, 0);
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        assert!(!tree.is_empty().await);
        assert_eq!(tree.len().await, 1);
    }

    #[tokio::test]
    async fn test_ancestors() {
        let tree = create_test_tree();
        
        let root = tree
            .create_session("root", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        tree.branch("branch-1").await.unwrap();
        
        let ancestors = tree.ancestors().await;
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0].name, "root");
    }

    #[tokio::test]
    async fn test_switch_to_existing() {
        let tree = create_test_tree();
        
        tree.create_session("main", "agent-1".to_string(), SessionScope::Main)
            .await;
        
        let branch1 = tree.branch("branch-1").await.unwrap();
        tree.switch_to_root().await.unwrap();
        
        tree.branch("branch-2").await.unwrap();
        
        let switched = tree.switch_to(branch1.id).await.unwrap();
        assert_eq!(switched.id, branch1.id);
        
        let current = tree.current().await.unwrap();
        assert_eq!(current.name, "branch-1");
    }
}
