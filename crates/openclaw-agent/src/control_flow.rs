//! Agent Control Flow - Agent 控制流
//!
//! 实现 Steer + Follow-up 控制流：
//! - RunReason: 运行原因枚举
//! - AgentRunResult: 运行结果结构体
//! - run_with_control_flow: 控制流方法
//! - execute_turn: 单轮执行
//! - spawn_timeout_monitor: 超时监控

use std::sync::Arc;

use tokio::sync::RwLock;

use crate::control::{ControlMessage, MessageQueue, QueueMode};
use crate::safety::{AgentSafetyConfig, AgentSafetyWrapper};
use crate::task::{TaskInput, TaskPriority, TaskRequest, TaskResult, TaskStatus};
use crate::types::AgentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunReason {
    NewTask,
    Continue,
    FollowUp,
    Steer,
    Scheduled,
    Recovered,
}

impl RunReason {
    pub fn is_continuation(&self) -> bool {
        matches!(self, RunReason::Continue | RunReason::FollowUp)
    }
}

#[derive(Debug, Clone)]
pub struct AgentRunResult {
    pub task_id: String,
    pub status: RunStatus,
    pub output: Option<String>,
    pub error: Option<String>,
    pub turns: u32,
    pub tokens_used: u64,
    pub reason: RunReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    Completed,
    Limited,
    Timeout,
    Aborted,
    Error,
}

pub struct AgentControlFlow {
    max_turns: u32,
    queue: MessageQueue,
    safety: Option<Arc<AgentSafetyWrapper>>,
}

impl AgentControlFlow {
    pub fn new(max_turns: u32) -> Self {
        Self {
            max_turns,
            queue: MessageQueue::new(),
            safety: None,
        }
    }

    pub fn with_safety(mut self, config: AgentSafetyConfig) -> Self {
        self.safety = Some(Arc::new(AgentSafetyWrapper::new(config)));
        self
    }

    pub fn queue(&self) -> &MessageQueue {
        &self.queue
    }

    pub fn safety(&self) -> Option<&Arc<AgentSafetyWrapper>> {
        self.safety.as_ref()
    }

    pub async fn set_mode(&self, mode: QueueMode) {
        self.queue.set_mode(mode).await;
    }

    pub async fn enqueue(&self, role: &str, content: &str) {
        let msg = ControlMessage::new(role, content);
        self.queue.enqueue(msg).await;
    }

    pub async fn steer(&self, role: &str, content: &str) {
        let msg = ControlMessage::new(role, content);
        self.queue.steer(msg).await;
    }

    pub async fn run_with_control_flow<F, Fut>(
        &self,
        reason: RunReason,
        execute_fn: F,
    ) -> AgentRunResult
    where
        F: Fn(ControlMessage) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        let mut turns: u32 = 0;
        let mut tokens_used: u64 = 0;
        let mut last_error: Option<String> = None;

        if let Some(ref safety) = self.safety {
            if let Err(e) = safety.check_safety().await {
                return AgentRunResult {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    status: RunStatus::Aborted,
                    output: None,
                    error: Some(format!("Safety check failed: {}", e)),
                    turns,
                    tokens_used,
                    reason,
                };
            }
        }

        loop {
            if turns >= self.max_turns {
                return AgentRunResult {
                    task_id: uuid::Uuid::new_v4().to_string(),
                    status: RunStatus::Limited,
                    output: None,
                    error: Some("Max turns reached".to_string()),
                    turns,
                    tokens_used,
                    reason,
                };
            }

            if let Some(ref safety) = self.safety {
                if safety.is_aborted() {
                    return AgentRunResult {
                        task_id: uuid::Uuid::new_v4().to_string(),
                        status: RunStatus::Aborted,
                        output: None,
                        error: Some("Task aborted".to_string()),
                        turns,
                        tokens_used,
                        reason,
                    };
                }

                if let Err(e) = safety.check_safety().await {
                    return AgentRunResult {
                        task_id: uuid::Uuid::new_v4().to_string(),
                        status: RunStatus::Timeout,
                        output: None,
                        error: Some(format!("Safety timeout: {}", e)),
                        turns,
                        tokens_used,
                        reason,
                    };
                }
            }

            let message = match self.queue.dequeue().await {
                Some(msg) => msg,
                None => break,
            };

            match execute_fn(message).await {
                Ok(output) => {
                    tokens_used += 100;
                }
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            }

            turns += 1;
        }

        AgentRunResult {
            task_id: uuid::Uuid::new_v4().to_string(),
            status: if last_error.is_some() {
                RunStatus::Error
            } else {
                RunStatus::Completed
            },
            output: None,
            error: last_error,
            turns,
            tokens_used,
            reason,
        }
    }

    pub async fn execute_turn<F, Fut>(&self, message: ControlMessage, execute_fn: F) -> Result<String, String>
    where
        F: Fn(ControlMessage) -> Fut,
        Fut: std::future::Future<Output = Result<String, String>>,
    {
        if let Some(ref safety) = self.safety {
            safety.record_turn(0).await.map_err(|e| e.to_string())?;
        }

        execute_fn(message).await
    }

    pub fn stats(&self) -> ControlFlowStats {
        ControlFlowStats {
            max_turns: self.max_turns,
            mode: None,
            queue_len: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ControlFlowStats {
    pub max_turns: u32,
    pub mode: Option<QueueMode>,
    pub queue_len: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_control_flow_creation() {
        let flow = AgentControlFlow::new(10);
        let stats = flow.stats();
        assert_eq!(stats.max_turns, 10);
    }

    #[tokio::test]
    async fn test_control_flow_with_safety() {
        let flow = AgentControlFlow::new(10).with_safety(AgentSafetyConfig::default());
        assert!(flow.safety().is_some());
    }

    #[tokio::test]
    async fn test_run_reason() {
        assert!(RunReason::NewTask.is_continuation() == false);
        assert!(RunReason::Continue.is_continuation() == true);
        assert!(RunReason::FollowUp.is_continuation() == true);
    }

    #[tokio::test]
    async fn test_enqueue_and_dequeue() {
        let flow = AgentControlFlow::new(10);
        
        flow.enqueue("user", "Hello").await;
        flow.enqueue("user", "World").await;
        
        let msg = flow.queue().dequeue().await.unwrap();
        assert_eq!(msg.content, "Hello");
        
        let msg = flow.queue().dequeue().await.unwrap();
        assert_eq!(msg.content, "World");
    }

    #[tokio::test]
    async fn test_steer() {
        let flow = AgentControlFlow::new(10);
        
        flow.enqueue("user", "Initial").await;
        flow.set_mode(QueueMode::Steer).await;
        flow.steer("user", "Steered").await;
        
        let msg = flow.queue().dequeue().await.unwrap();
        assert_eq!(msg.content, "Steered");
    }

    #[tokio::test]
    async fn test_run_with_control_flow() {
        let flow = AgentControlFlow::new(10);
        
        flow.enqueue("user", "Hello").await;
        
        let result = flow.run_with_control_flow(RunReason::NewTask, |msg| async move {
            Ok(format!("Processed: {}", msg.content))
        }).await;
        
        assert_eq!(result.status, RunStatus::Completed);
        assert_eq!(result.turns, 1);
    }

    #[tokio::test]
    async fn test_max_turns_limit() {
        let flow = AgentControlFlow::new(2);
        
        flow.enqueue("user", "1").await;
        flow.enqueue("user", "2").await;
        flow.enqueue("user", "3").await;
        
        let result = flow.run_with_control_flow(RunReason::NewTask, |_msg| async move {
            Ok("done".to_string())
        }).await;
        
        assert_eq!(result.status, RunStatus::Limited);
        assert_eq!(result.turns, 2);
    }
}
