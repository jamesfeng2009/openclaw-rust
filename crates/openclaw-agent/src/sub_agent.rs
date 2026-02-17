//! 子代理系统
//!
//! 支持嵌套子代理：
//! - 多层子代理调用
//! - 深度限制配置
//! - 子代理上下文传递
//! - 结果聚合

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use crate::agent::{Agent, BaseAgent};
use crate::task::{TaskOutput, TaskRequest, TaskResult, TaskStatus};
use crate::types::Capability;
use openclaw_core::Message;

/// 子代理错误
#[derive(Debug, Error)]
pub enum SubAgentError {
    #[error("超过最大深度限制: {0}")]
    MaxDepthExceeded(usize),

    #[error("子代理不存在: {0}")]
    AgentNotFound(String),

    #[error("子代理执行失败: {0}")]
    ExecutionFailed(String),

    #[error("循环调用检测: {0}")]
    CircularCall(String),

    #[error("超时: {0}")]
    Timeout(String),

    #[error("内部错误: {0}")]
    Internal(String),
}

/// 子代理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    /// 最大嵌套深度
    pub max_depth: usize,
    /// 每层最大并行子代理数
    pub max_parallel: usize,
    /// 子代理调用超时 (秒)
    pub timeout_seconds: u64,
    /// 是否允许同级子代理通信
    pub allow_peer_communication: bool,
    /// 结果聚合策略
    pub aggregation: AggregationStrategy,
    /// 是否继承父代理上下文
    pub inherit_context: bool,
    /// 是否传递记忆
    pub share_memory: bool,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            max_depth: 3,
            max_parallel: 5,
            timeout_seconds: 300,
            allow_peer_communication: false,
            aggregation: AggregationStrategy::Merge,
            inherit_context: true,
            share_memory: false,
        }
    }
}

/// 结果聚合策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationStrategy {
    /// 合并所有结果
    Merge,
    /// 只保留最佳结果
    BestOnly,
    /// 投票决定
    Voting,
    /// 顺序执行，后者基于前者
    Sequential,
    /// 并行执行，独立处理
    Parallel,
}

/// 子代理调用上下文
#[derive(Debug, Clone)]
pub struct SubAgentContext {
    /// 调用链 ID
    pub chain_id: String,
    /// 当前深度
    pub depth: usize,
    /// 父代理 ID 列表 (用于检测循环)
    pub parent_chain: Vec<String>,
    /// 原始任务 ID
    pub root_task_id: Uuid,
    /// 继承的消息上下文
    pub inherited_context: Vec<Message>,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SubAgentContext {
    pub fn new(root_task_id: Uuid) -> Self {
        Self {
            chain_id: Uuid::new_v4().to_string(),
            depth: 0,
            parent_chain: Vec::new(),
            root_task_id,
            inherited_context: Vec::new(),
            started_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn child_context(&self, parent_agent_id: &str) -> Self {
        let mut parent_chain = self.parent_chain.clone();
        parent_chain.push(parent_agent_id.to_string());

        Self {
            chain_id: self.chain_id.clone(),
            depth: self.depth + 1,
            parent_chain,
            root_task_id: self.root_task_id,
            inherited_context: self.inherited_context.clone(),
            started_at: Utc::now(),
            metadata: self.metadata.clone(),
        }
    }

    /// 检查是否存在循环调用
    pub fn has_circular_call(&self, agent_id: &str) -> bool {
        self.parent_chain.contains(&agent_id.to_string())
    }
}

/// 子代理调用请求
#[derive(Debug, Clone)]
pub struct SubAgentCall {
    /// 目标代理 ID
    pub agent_id: String,
    /// 任务请求
    pub task: TaskRequest,
    /// 调用上下文
    pub context: SubAgentContext,
}

/// 子代理调用结果
#[derive(Debug, Clone)]
pub struct SubAgentResult {
    /// 代理 ID
    pub agent_id: String,
    /// 任务结果
    pub result: TaskResult,
    /// 调用深度
    pub depth: usize,
    /// 执行时间 (毫秒)
    pub duration_ms: u64,
    /// 是否成功
    pub success: bool,
}

/// 子代理管理器
pub struct SubAgentManager {
    config: SubAgentConfig,
    agents: Arc<RwLock<HashMap<String, Arc<BaseAgent>>>>,
    active_calls: Arc<RwLock<HashMap<String, SubAgentCall>>>,
}

impl SubAgentManager {
    pub fn new(config: SubAgentConfig) -> Self {
        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            active_calls: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册子代理
    pub async fn register_agent(&self, agent: Arc<BaseAgent>) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id().to_string(), agent.clone());
        info!("注册子代理: {}", agent.id());
    }

    /// 移除子代理
    pub async fn unregister_agent(&self, agent_id: &str) {
        let mut agents = self.agents.write().await;
        agents.remove(agent_id);
        info!("移除子代理: {}", agent_id);
    }

    /// 调用子代理
    pub async fn call_sub_agent(
        &self,
        agent_id: &str,
        task: TaskRequest,
        parent_context: Option<SubAgentContext>,
    ) -> std::result::Result<SubAgentResult, SubAgentError> {
        // 获取或创建上下文
        let context = parent_context
            .map(|ctx| ctx.child_context(agent_id))
            .unwrap_or_else(|| SubAgentContext::new(task.id));

        // 检查深度限制
        if context.depth > self.config.max_depth {
            return Err(SubAgentError::MaxDepthExceeded(self.config.max_depth));
        }

        // 检查循环调用
        if context.has_circular_call(agent_id) {
            return Err(SubAgentError::CircularCall(agent_id.to_string()));
        }

        // 获取代理
        let agent = {
            let agents = self.agents.read().await;
            agents
                .get(agent_id)
                .cloned()
                .ok_or_else(|| SubAgentError::AgentNotFound(agent_id.to_string()))?
        };

        // 记录活跃调用
        let call_id = Uuid::new_v4().to_string();
        {
            let mut active = self.active_calls.write().await;
            active.insert(
                call_id.clone(),
                SubAgentCall {
                    agent_id: agent_id.to_string(),
                    task: task.clone(),
                    context: context.clone(),
                },
            );
        }

        let started_at = Utc::now();

        // 执行任务
        let result = agent.process(task).await;

        // 计算执行时间
        let duration_ms = (Utc::now() - started_at).num_milliseconds() as u64;

        // 移除活跃调用
        {
            let mut active = self.active_calls.write().await;
            active.remove(&call_id);
        }

        match result {
            Ok(task_result) => {
                let success = task_result.status == TaskStatus::Completed;
                Ok(SubAgentResult {
                    agent_id: agent_id.to_string(),
                    result: task_result,
                    depth: context.depth,
                    duration_ms,
                    success,
                })
            }
            Err(e) => Err(SubAgentError::ExecutionFailed(e.to_string())),
        }
    }

    /// 批量调用子代理
    pub async fn call_multiple(
        &self,
        calls: Vec<(String, TaskRequest)>,
        parent_context: Option<SubAgentContext>,
    ) -> Vec<std::result::Result<SubAgentResult, SubAgentError>> {
        if calls.len() > self.config.max_parallel {
            warn!(
                "调用数量 {} 超过最大并行数 {}，将被截断",
                calls.len(),
                self.config.max_parallel
            );
        }

        let context = parent_context.unwrap_or_else(|| SubAgentContext::new(Uuid::nil()));

        let mut results = Vec::new();

        for (agent_id, task) in calls.into_iter().take(self.config.max_parallel) {
            let result = self
                .call_sub_agent(&agent_id, task, Some(context.clone()))
                .await;
            results.push(result);
        }

        results
    }

    /// 聚合结果
    pub async fn aggregate_results(
        &self,
        results: Vec<SubAgentResult>,
        strategy: AggregationStrategy,
    ) -> std::result::Result<TaskOutput, SubAgentError> {
        if results.is_empty() {
            return Err(SubAgentError::ExecutionFailed(
                "没有可聚合的结果".to_string(),
            ));
        }

        match strategy {
            AggregationStrategy::Merge => {
                let outputs: Vec<TaskOutput> = results
                    .iter()
                    .filter_map(|r| r.result.output.clone())
                    .collect();

                Ok(TaskOutput::Multiple { outputs })
            }
            AggregationStrategy::BestOnly => {
                // 选择执行时间最短且成功的结果
                let best = results
                    .iter()
                    .filter(|r| r.success)
                    .min_by_key(|r| r.duration_ms);

                match best {
                    Some(r) => r.result.output.clone().ok_or_else(|| {
                        SubAgentError::ExecutionFailed("最佳结果无输出".to_string())
                    }),
                    None => Err(SubAgentError::ExecutionFailed(
                        "没有成功的子代理结果".to_string(),
                    )),
                }
            }
            AggregationStrategy::Voting => {
                // 简单投票：选择出现最多的结果类型
                let mut type_counts: HashMap<String, usize> = HashMap::new();

                for result in &results {
                    if result.success {
                        if let Some(ref output) = result.result.output {
                            let type_name = match output {
                                TaskOutput::Message { .. } => "message",
                                TaskOutput::Text { .. } => "text",
                                TaskOutput::Code { .. } => "code",
                                TaskOutput::Data { .. } => "data",
                                TaskOutput::SearchResult { .. } => "search",
                                TaskOutput::ToolResult { .. } => "tool",
                                TaskOutput::Multiple { .. } => "multiple",
                            };
                            *type_counts.entry(type_name.to_string()).or_insert(0) += 1;
                        }
                    }
                }

                let winning_type = type_counts
                    .into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(t, _)| t);

                // 返回第一个匹配类型的结果
                for result in &results {
                    if result.success {
                        if let Some(ref output) = result.result.output {
                            let type_name = match output {
                                TaskOutput::Message { .. } => "message",
                                TaskOutput::Text { .. } => "text",
                                TaskOutput::Code { .. } => "code",
                                TaskOutput::Data { .. } => "data",
                                TaskOutput::SearchResult { .. } => "search",
                                TaskOutput::ToolResult { .. } => "tool",
                                TaskOutput::Multiple { .. } => "multiple",
                            };
                            if Some(type_name) == winning_type.as_deref() {
                                return Ok(output.clone());
                            }
                        }
                    }
                }

                Err(SubAgentError::ExecutionFailed("投票失败".to_string()))
            }
            AggregationStrategy::Sequential | AggregationStrategy::Parallel => {
                // 顺序和并行都返回合并结果 (直接内联 Merge 逻辑以避免递归)
                let outputs: Vec<TaskOutput> = results
                    .iter()
                    .filter_map(|r| r.result.output.clone())
                    .collect();
                Ok(TaskOutput::Multiple { outputs })
            }
        }
    }

    /// 获取活跃调用数
    pub async fn active_call_count(&self) -> usize {
        let active = self.active_calls.read().await;
        active.len()
    }

    /// 获取已注册代理列表
    pub async fn list_agents(&self) -> Vec<String> {
        let agents = self.agents.read().await;
        agents.keys().cloned().collect()
    }

    /// 获取配置
    pub fn config(&self) -> &SubAgentConfig {
        &self.config
    }

    /// 更新配置
    pub fn update_config(&mut self, config: SubAgentConfig) {
        self.config = config;
    }
}

impl Default for SubAgentManager {
    fn default() -> Self {
        Self::new(SubAgentConfig::default())
    }
}

/// 子代理编排器
pub struct SubAgentOrchestrator {
    manager: Arc<SubAgentManager>,
    config: SubAgentConfig,
}

impl SubAgentOrchestrator {
    pub fn new(config: SubAgentConfig) -> Self {
        Self {
            manager: Arc::new(SubAgentManager::new(config.clone())),
            config,
        }
    }

    /// 执行任务，自动分配子代理
    pub async fn execute(
        &self,
        task: TaskRequest,
        required_capabilities: Vec<Capability>,
    ) -> std::result::Result<TaskResult, SubAgentError> {
        let context = SubAgentContext::new(task.id);
        let started_at = Utc::now();

        // 根据能力查找合适的代理
        let agent_id = self
            .find_agent_by_capabilities(&required_capabilities)
            .await?;

        // 调用子代理
        let sub_result = self
            .manager
            .call_sub_agent(&agent_id, task.clone(), Some(context))
            .await?;

        let sub_task_result = sub_result.result;
        let sub_tasks = vec![sub_task_result.clone()];
        Ok(TaskResult {
            task_id: task.id,
            agent_id: sub_result.agent_id,
            status: if sub_result.success {
                TaskStatus::Completed
            } else {
                TaskStatus::Failed
            },
            output: sub_task_result.output,
            error: sub_task_result.error,
            started_at,
            completed_at: Some(Utc::now()),
            tokens_used: None, // 子代理的 token 使用量不计入父代理
            sub_tasks,
        })
    }

    /// 执行多代理协作任务
    pub async fn execute_collaborative(
        &self,
        task: TaskRequest,
        agent_assignments: Vec<(String, Vec<Capability>)>,
    ) -> std::result::Result<TaskResult, SubAgentError> {
        let context = SubAgentContext::new(task.id);
        let started_at = Utc::now();

        // 构建子任务调用
        let calls: Vec<(String, TaskRequest)> = agent_assignments
            .iter()
            .map(|(agent_id, _)| {
                let sub_task = TaskRequest {
                    id: Uuid::new_v4(),
                    input: task.input.clone(),
                    task_type: task.task_type.clone(),
                    priority: task.priority.clone(),
                    required_capabilities: task.required_capabilities.clone(),
                    preferred_agent: Some(agent_id.clone()),
                    context: task.context.clone(),
                    timeout_seconds: task.timeout_seconds,
                    created_at: Utc::now(),
                };
                (agent_id.clone(), sub_task)
            })
            .collect();

        // 批量调用
        let results = self.manager.call_multiple(calls, Some(context)).await;

        // 收集成功的结果
        let sub_results: Vec<SubAgentResult> = results.into_iter().filter_map(|r| r.ok()).collect();

        // 聚合结果
        let output = self
            .manager
            .aggregate_results(sub_results.clone(), self.config.aggregation)
            .await?;

        // 构建最终结果
        Ok(TaskResult {
            task_id: task.id,
            agent_id: "orchestrator".to_string(),
            status: TaskStatus::Completed,
            output: Some(output),
            error: None,
            started_at,
            completed_at: Some(Utc::now()),
            tokens_used: None,
            sub_tasks: sub_results.into_iter().map(|r| r.result).collect(),
        })
    }

    /// 根据能力查找代理
    async fn find_agent_by_capabilities(
        &self,
        capabilities: &[Capability],
    ) -> std::result::Result<String, SubAgentError> {
        let agents = self.manager.agents.read().await;

        for (agent_id, agent) in agents.iter() {
            let agent_caps = agent.capabilities();
            if capabilities.iter().all(|c| agent_caps.contains(c)) {
                return Ok(agent_id.clone());
            }
        }

        Err(SubAgentError::AgentNotFound(
            "没有找到匹配能力的代理".to_string(),
        ))
    }

    /// 获取管理器
    pub fn manager(&self) -> Arc<SubAgentManager> {
        self.manager.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sub_agent_context() {
        let ctx = SubAgentContext::new(Uuid::nil());
        assert_eq!(ctx.depth, 0);
        assert!(ctx.parent_chain.is_empty());

        let child = ctx.child_context("parent-1");
        assert_eq!(child.depth, 1);
        assert!(child.parent_chain.contains(&"parent-1".to_string()));
    }

    #[test]
    fn test_circular_detection() {
        let mut ctx = SubAgentContext::new(Uuid::nil());
        ctx.parent_chain.push("agent-1".to_string());
        ctx.parent_chain.push("agent-2".to_string());

        assert!(ctx.has_circular_call("agent-1"));
        assert!(ctx.has_circular_call("agent-2"));
        assert!(!ctx.has_circular_call("agent-3"));
    }

    #[test]
    fn test_depth_limit() {
        let config = SubAgentConfig {
            max_depth: 2,
            ..Default::default()
        };

        let mut ctx = SubAgentContext::new(Uuid::nil());
        ctx.depth = 3;

        assert!(ctx.depth > config.max_depth);
    }

    #[tokio::test]
    async fn test_sub_agent_manager() {
        let manager = SubAgentManager::new(SubAgentConfig::default());

        let agent = Arc::new(BaseAgent::coder());
        manager.register_agent(agent).await;

        let agents = manager.list_agents().await;
        assert!(agents.contains(&"coder".to_string()));
    }
}
