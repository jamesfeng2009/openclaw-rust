//! Agent Safety Wrapper - Agent 安全包装器
//!
//! 为 Agent 提供安全防护：
//! - Turn 计数限制
//! - 超时控制
//! - 任务中止
//! - 会话树支持

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::safety::turn_limiter::{TurnLimiter, TurnLimitConfig};
use crate::safety::timeout::{TimeoutController, TimeoutConfig};
use crate::safety::abort_control::AbortableTask;
use crate::session_tree::SessionTree;
use crate::task::{TaskInput, TaskRequest, TaskResult};

#[derive(Debug, Clone)]
pub struct AgentSafetyConfig {
    pub turn_limit: TurnLimitConfig,
    pub timeout_config: TimeoutConfig,
    pub enable_session_tree: bool,
}

impl Default for AgentSafetyConfig {
    fn default() -> Self {
        Self {
            turn_limit: TurnLimitConfig::default(),
            timeout_config: TimeoutConfig::default(),
            enable_session_tree: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyAction {
    Continue,
    Stop,
    Branch,
    SwitchTo(String),
}

pub struct AgentSafetyWrapper {
    config: AgentSafetyConfig,
    turn_limiter: TurnLimiter,
    timeout_controller: TimeoutController,
    abort_task: AbortableTask,
    session_tree: Arc<RwLock<Option<SessionTree>>>,
    current_session_id: Arc<RwLock<Option<String>>>,
}

impl AgentSafetyWrapper {
    pub fn new(config: AgentSafetyConfig) -> Self {
        Self {
            config: config.clone(),
            turn_limiter: TurnLimiter::new(config.turn_limit),
            timeout_controller: TimeoutController::new(config.timeout_config),
            abort_task: AbortableTask::new(),
            session_tree: Arc::new(RwLock::new(None)),
            current_session_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_default() -> Self {
        Self::new(AgentSafetyConfig::default())
    }

    pub fn config(&self) -> &AgentSafetyConfig {
        &self.config
    }

    pub fn turn_limiter(&self) -> &TurnLimiter {
        &self.turn_limiter
    }

    pub fn timeout_controller(&self) -> &TimeoutController {
        &self.timeout_controller
    }

    pub fn abort_task(&self) -> &AbortableTask {
        &self.abort_task
    }

    pub async fn init_session_tree(&self) {
        if self.config.enable_session_tree {
            let tree = SessionTree::new();
            tree.init_root("root").await;
            *self.session_tree.write().await = Some(tree);
        }
    }

    pub async fn start_task(&self, task_id: &str) -> Result<(), String> {
        self.timeout_controller.start().await;
        *self.current_session_id.write().await = Some(task_id.to_string());
        Ok(())
    }

    pub async fn end_task(&self) {
        self.timeout_controller.stop().await;
        *self.current_session_id.write().await = None;
    }

    pub async fn record_turn(&self, tokens: u64) -> Result<(), String> {
        self.turn_limiter
            .increment_turn(tokens)
            .map_err(|e| e.to_string())?;
        
        self.timeout_controller.record_activity().await;
        
        if self.timeout_controller.check_timeout().await {
            self.abort_task.abort();
            return Err("Task timeout".to_string());
        }
        
        if self.turn_limiter.is_limited() {
            return Err("Turn limit reached".to_string());
        }
        
        Ok(())
    }

    pub fn is_aborted(&self) -> bool {
        self.abort_task.is_cancelled()
    }

    pub async fn abort(&self, reason: &str) {
        tracing::warn!("Agent safety abort: {}", reason);
        self.abort_task.abort();
    }

    pub async fn check_safety(&self) -> Result<SafetyAction, String> {
        if self.is_aborted() {
            return Ok(SafetyAction::Stop);
        }

        if self.turn_limiter.is_limited() {
            return Ok(SafetyAction::Stop);
        }

        if self.timeout_controller.check_timeout().await {
            self.abort_task.abort();
            return Ok(SafetyAction::Stop);
        }

        Ok(SafetyAction::Continue)
    }

    pub async fn branch(&self, branch_name: &str) -> Result<String, String> {
        let tree = self.session_tree.read().await;
        if let Some(tree) = tree.as_ref() {
            tree.branch(branch_name)
                .await
                .map(|node| node.id.to_string())
                .ok_or_else(|| "Failed to create branch".to_string())
        } else {
            Err("Session tree not initialized".to_string())
        }
    }

    pub async fn switch_to(&self, node_id: &str) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(node_id)
            .map_err(|e| format!("Invalid UUID: {}", e))?;
        let tree = self.session_tree.read().await;
        if let Some(tree) = tree.as_ref() {
            tree.switch_to(uuid).await;
            Ok(())
        } else {
            Err("Session tree not initialized".to_string())
        }
    }

    pub async fn switch_to_parent(&self) -> Result<(), String> {
        let tree = self.session_tree.read().await;
        if let Some(tree) = tree.as_ref() {
            tree.switch_to_parent().await;
            Ok(())
        } else {
            Err("Session tree not initialized".to_string())
        }
    }

    pub async fn switch_to_root(&self) -> Result<(), String> {
        let tree = self.session_tree.read().await;
        if let Some(tree) = tree.as_ref() {
            tree.switch_to_root().await;
            Ok(())
        } else {
            Err("Session tree not initialized".to_string())
        }
    }

    pub async fn add_message(&self, role: &str, content: &str) -> Result<(), String> {
        let tree = self.session_tree.read().await;
        if let Some(tree) = tree.as_ref() {
            tree.add_message(role, content, 0).await;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub async fn get_session_tree(&self) -> Option<SessionTree> {
        let tree = self.session_tree.read().await;
        tree.clone()
    }

    pub async fn reset(&mut self) {
        self.turn_limiter.reset();
        self.timeout_controller.reset().await;
        self.abort_task = AbortableTask::new();
        
        if let Some(_tree) = self.session_tree.write().await.take() {
            *self.session_tree.write().await = Some(SessionTree::new());
        }
    }

    pub fn stats(&self) -> SafetyStats {
        SafetyStats {
            current_turn: self.turn_limiter.current_turn(),
            remaining_turns: self.turn_limiter.remaining_turns(),
            current_tokens: self.turn_limiter.current_tokens(),
            remaining_tokens: self.turn_limiter.remaining_tokens(),
            is_aborted: self.is_aborted(),
        }
    }
}

impl Default for AgentSafetyWrapper {
    fn default() -> Self {
        Self::with_default()
    }
}

#[derive(Debug, Clone)]
pub struct SafetyStats {
    pub current_turn: u64,
    pub remaining_turns: u64,
    pub current_tokens: u64,
    pub remaining_tokens: u64,
    pub is_aborted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safety_wrapper_init() {
        let wrapper = AgentSafetyWrapper::with_default();
        
        wrapper.init_session_tree().await;
        
        let stats = wrapper.stats();
        assert_eq!(stats.current_turn, 0);
        assert!(!stats.is_aborted);
    }

    #[tokio::test]
    async fn test_record_turn() {
        let wrapper = AgentSafetyWrapper::new(AgentSafetyConfig {
            turn_limit: TurnLimitConfig {
                max_turns: 5,
                max_tokens_per_turn: 1000,
                max_total_tokens: 10000,
                window_size: 3,
            },
            timeout_config: TimeoutConfig {
                operation_timeout_ms: 60000,
                total_timeout_ms: 60000,
                idle_timeout_ms: 30000,
                warn_threshold_ms: 50000,
            },
            enable_session_tree: true,
        });
        
        wrapper.init_session_tree().await;
        wrapper.start_task("test-task").await.unwrap();
        
        wrapper.record_turn(100).await.unwrap();
        
        let stats = wrapper.stats();
        assert_eq!(stats.current_turn, 1);
    }

    #[tokio::test]
    async fn test_abort() {
        let wrapper = AgentSafetyWrapper::with_default();
        
        wrapper.abort("test abort").await;
        
        assert!(wrapper.is_aborted());
    }

    #[tokio::test]
    async fn test_session_tree_branch() {
        let wrapper = AgentSafetyWrapper::new(AgentSafetyConfig {
            turn_limit: TurnLimitConfig::default(),
            timeout_config: TimeoutConfig {
                operation_timeout_ms: 60000,
                total_timeout_ms: 60000,
                idle_timeout_ms: 30000,
                warn_threshold_ms: 50000,
            },
            enable_session_tree: true,
        });
        
        wrapper.init_session_tree().await;
        
        let branch_id = wrapper.branch("exploration").await.unwrap();
        assert!(!branch_id.is_empty());
    }
}
