//! Agent Orchestrator - 任务编排和协调

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};

use openclaw_core::{Message, OpenClawError, Result};
use openclaw_ai::AIProvider;
use openclaw_memory::MemoryManager;

use crate::agent::{Agent, BaseAgent};
use crate::task::{TaskInput, TaskOutput, TaskPriority, TaskRequest, TaskResult, TaskStatus, TaskType};
use crate::team::{AgentTeam, TeamConfig};
use crate::types::{AgentType, Capability};

/// Orchestrator 配置
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// 默认超时时间 (秒)
    pub default_timeout: u64,
    /// 最大并行任务数
    pub max_parallel_tasks: usize,
    /// 是否启用任务分解
    pub enable_task_decomposition: bool,
    /// 是否启用结果聚合
    pub enable_result_aggregation: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            default_timeout: 300,
            max_parallel_tasks: 10,
            enable_task_decomposition: true,
            enable_result_aggregation: true,
        }
    }
}

/// Agent Orchestrator - 协调多个 Agent 处理任务
pub struct Orchestrator {
    /// Team 管理
    team: AgentTeam,
    /// AI 提供商
    ai_provider: Option<Arc<dyn AIProvider>>,
    /// 记忆管理器
    memory: Option<Arc<MemoryManager>>,
    /// 配置
    config: OrchestratorConfig,
    /// 活跃任务
    active_tasks: RwLock<HashMap<uuid::Uuid, TaskRequest>>,
}

impl Orchestrator {
    pub fn new(team_config: TeamConfig) -> Self {
        Self {
            team: AgentTeam::new(team_config),
            ai_provider: None,
            memory: None,
            config: OrchestratorConfig::default(),
            active_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// 使用默认 Team 创建
    pub fn with_default_team() -> Self {
        Self::new(TeamConfig::default_team())
    }

    /// 设置 AI 提供商
    pub fn with_ai_provider(mut self, provider: Arc<dyn AIProvider>) -> Self {
        self.ai_provider = Some(provider);
        self
    }

    /// 设置记忆管理器
    pub fn with_memory(mut self, memory: Arc<MemoryManager>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// 设置配置
    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// 处理任务
    pub async fn process(&self, request: TaskRequest) -> Result<TaskResult> {
        info!("Processing task {} of type {:?}", request.id, request.task_type);

        // 添加到活跃任务
        {
            let mut active = self.active_tasks.write().await;
            active.insert(request.id, request.clone());
        }

        // 1. 分析任务
        let analysis = self.analyze_task(&request).await?;
        debug!("Task analysis: {:?}", analysis);

        // 2. 选择或创建子任务
        let sub_tasks = if self.config.enable_task_decomposition && analysis.needs_decomposition {
            self.decompose_task(&request, &analysis).await?
        } else {
            vec![request.clone()]
        };

        // 3. 分配任务给 Agent
        let mut results = Vec::new();
        for task in sub_tasks {
            let agent_id = self.team.select_agent(
                &task.required_capabilities,
                task.preferred_agent.as_deref(),
            );

            match agent_id {
                Some(agent_id) => {
                    info!("Assigning task {} to agent {}", task.id, agent_id);
                    let result = self.execute_with_agent(&agent_id, task).await?;
                    results.push(result);
                }
                None => {
                    warn!("No available agent for task {}", task.id);
                    results.push(TaskResult::failure(
                        task.id,
                        "orchestrator".to_string(),
                        "No available agent with required capabilities".to_string(),
                    ));
                }
            }
        }

        // 4. 聚合结果
        let final_result = if self.config.enable_result_aggregation && results.len() > 1 {
            self.aggregate_results(request.id, results).await?
        } else {
            results.into_iter().next().unwrap_or_else(|| {
                TaskResult::failure(
                    request.id,
                    "orchestrator".to_string(),
                    "No results produced".to_string(),
                )
            })
        };

        // 从活跃任务中移除
        {
            let mut active = self.active_tasks.write().await;
            active.remove(&request.id);
        }

        Ok(final_result)
    }

    /// 处理用户消息
    pub async fn handle_message(&self, message: Message) -> Result<Message> {
        // 创建对话任务
        let task = TaskRequest::from_message(message);

        // 处理任务
        let result = self.process(task).await?;

        // 提取响应消息
        match result.output {
            Some(TaskOutput::Message { message }) => Ok(message),
            Some(TaskOutput::Text { content }) => Ok(Message::assistant(content)),
            Some(TaskOutput::Code { code, .. }) => Ok(Message::assistant(code)),
            Some(TaskOutput::Data { data }) => Ok(Message::assistant(data.to_string())),
            Some(TaskOutput::SearchResult { results }) => {
                let content = results.iter()
                    .map(|r| format!("**{}**\n{}\n{}", r.title, r.snippet, r.url))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Ok(Message::assistant(content))
            }
            Some(TaskOutput::ToolResult { result }) => Ok(Message::assistant(result.to_string())),
            Some(TaskOutput::Multiple { outputs }) => {
                // 合并多个输出
                let content: String = outputs.iter()
                    .filter_map(|o| match o {
                        TaskOutput::Text { content } => Some(content.clone()),
                        TaskOutput::Message { message } => message.text_content().map(|s| s.to_string()),
                        TaskOutput::Code { code, .. } => Some(code.clone()),
                        TaskOutput::Data { data } => Some(data.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Ok(Message::assistant(content))
            }
            None => {
                Err(OpenClawError::Unknown("No output from agent".to_string()))
            }
        }
    }

    /// 分析任务
    async fn analyze_task(&self, request: &TaskRequest) -> Result<TaskAnalysis> {
        let mut analysis = TaskAnalysis {
            task_type: request.task_type.clone(),
            complexity: TaskComplexity::Simple,
            needs_decomposition: false,
            required_capabilities: request.required_capabilities.clone(),
            suggested_agents: Vec::new(),
        };

        // 根据任务类型判断复杂度
        match &request.task_type {
            TaskType::Conversation | TaskType::QuestionAnswer => {
                analysis.complexity = TaskComplexity::Simple;
                analysis.needs_decomposition = false;
            }
            TaskType::CodeGeneration | TaskType::CodeReview => {
                analysis.complexity = TaskComplexity::Medium;
                analysis.needs_decomposition = false;
            }
            TaskType::WebSearch | TaskType::DataAnalysis => {
                analysis.complexity = TaskComplexity::Medium;
                analysis.needs_decomposition = false;
            }
            TaskType::Documentation => {
                analysis.complexity = TaskComplexity::Medium;
                analysis.needs_decomposition = true;
            }
            TaskType::Custom(_) => {
                analysis.complexity = TaskComplexity::Complex;
                analysis.needs_decomposition = true;
            }
            _ => {}
        }

        // 推荐适合的 Agent
        for agent_id in self.team.agent_ids() {
            if let Some(agent) = self.team.get_agent(&agent_id) {
                if request.required_capabilities.iter().all(|c| agent.has_capability(c)) {
                    analysis.suggested_agents.push(agent.id().to_string());
                }
            }
        }

        Ok(analysis)
    }

    /// 分解任务
    async fn decompose_task(&self, request: &TaskRequest, _analysis: &TaskAnalysis) -> Result<Vec<TaskRequest>> {
        // 简单的任务分解逻辑
        match &request.task_type {
            TaskType::Documentation => {
                // 文档任务分解为：研究 + 写作
                Ok(vec![
                    TaskRequest::new(TaskType::WebSearch, request.input.clone())
                        .with_priority(request.priority.clone()),
                    TaskRequest::new(TaskType::Documentation, request.input.clone())
                        .with_priority(request.priority.clone()),
                ])
            }
            _ => {
                // 默认不分解
                Ok(vec![request.clone()])
            }
        }
    }

    /// 使用指定 Agent 执行任务
    async fn execute_with_agent(&self, agent_id: &str, task: TaskRequest) -> Result<TaskResult> {
        // TODO: 实际调用 Agent 处理
        // 目前返回模拟结果
        let output = match &task.input {
            TaskInput::Message { message } => {
                TaskOutput::Message {
                    message: Message::assistant(format!("[{}] Received: {}", agent_id, message.text_content().unwrap_or("")))
                }
            }
            TaskInput::Text { content } => {
                TaskOutput::Text {
                    content: format!("[{}] Processed: {}", agent_id, content)
                }
            }
            _ => {
                TaskOutput::Text {
                    content: format!("[{}] Task completed", agent_id)
                }
            }
        };

        Ok(TaskResult::success(task.id, agent_id.to_string(), output))
    }

    /// 聚合多个结果
    async fn aggregate_results(&self, task_id: uuid::Uuid, results: Vec<TaskResult>) -> Result<TaskResult> {
        let successful: Vec<_> = results.iter().filter(|r| r.status == TaskStatus::Completed).collect();

        if successful.is_empty() {
            return Ok(TaskResult::failure(
                task_id,
                "orchestrator".to_string(),
                "All sub-tasks failed".to_string(),
            ));
        }

        // 收集所有输出
        let outputs: Vec<TaskOutput> = successful.iter()
            .filter_map(|r| r.output.clone())
            .collect();

        Ok(TaskResult {
            task_id,
            agent_id: "orchestrator".to_string(),
            status: TaskStatus::Completed,
            output: Some(TaskOutput::Multiple { outputs }),
            error: None,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            tokens_used: None,
            sub_tasks: results,
        })
    }

    /// 获取活跃任务数
    pub async fn active_task_count(&self) -> usize {
        self.active_tasks.read().await.len()
    }

    /// 获取 Team 信息
    pub fn team(&self) -> &AgentTeam {
        &self.team
    }
}

use chrono::Utc;

/// 任务分析结果
#[derive(Debug)]
struct TaskAnalysis {
    task_type: TaskType,
    complexity: TaskComplexity,
    needs_decomposition: bool,
    required_capabilities: Vec<Capability>,
    suggested_agents: Vec<String>,
}

/// 任务复杂度
#[derive(Debug, Clone, Copy, PartialEq)]
enum TaskComplexity {
    Simple,
    Medium,
    Complex,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator() {
        let orchestrator = Orchestrator::with_default_team();

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text { content: "Hello".to_string() },
        );

        let result = orchestrator.process(task).await.unwrap();
        assert_eq!(result.status, TaskStatus::Completed);
    }
}
