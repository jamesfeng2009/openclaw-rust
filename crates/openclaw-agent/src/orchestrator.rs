//! Agent Orchestrator - 任务编排和协调

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use openclaw_ai::AIProvider;
use openclaw_core::{Message, OpenClawError, Result};
use openclaw_memory::MemoryManager;

use crate::agent::Agent;
use crate::device_tool_registry::DeviceToolRegistry;
use crate::evo::registry::SharedSkillRegistry;
use crate::evo::EvolutionEngine;
use crate::evo::autonomous::{HandExecutor, HandOutputManager};
use crate::evo::autonomous::executor::ExecutionContext;
use crate::evo::autonomous::output::ExecutionResult as HandOutputResult;
use crate::task::{TaskClassification, TaskClassifier, TaskOutput, TaskRequest, TaskResult, TaskStatus, TaskType};
use crate::team::{AgentTeam, TeamConfig};
use crate::types::Capability;

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
    /// 设备管理器
    device_manager: Option<Arc<openclaw_device::UnifiedDeviceManager>>,
    /// 设备工具注册中心
    device_tool_registry: Option<Arc<DeviceToolRegistry>>,
    /// 配置
    config: OrchestratorConfig,
    /// 活跃任务
    active_tasks: RwLock<HashMap<uuid::Uuid, TaskRequest>>,
    /// 共享技能注册表 (Evo 进化)
    shared_skill_registry: Option<Arc<SharedSkillRegistry>>,
    /// 进化引擎 (Evo)
    evolution_engine: Option<Arc<EvolutionEngine>>,
    /// 任务分类器
    task_classifier: TaskClassifier,
    /// Hand 执行器
    hand_executor: Option<Arc<HandExecutor>>,
    /// Hand 输出管理器
    hand_output_manager: Option<Arc<HandOutputManager>>,
}

impl Orchestrator {
    pub fn new(team_config: TeamConfig) -> Self {
        Self {
            team: AgentTeam::new(team_config),
            ai_provider: None,
            memory: None,
            device_manager: None,
            device_tool_registry: None,
            config: OrchestratorConfig::default(),
            active_tasks: RwLock::new(HashMap::new()),
            shared_skill_registry: None,
            evolution_engine: None,
            task_classifier: TaskClassifier::new(),
            hand_executor: None,
            hand_output_manager: None,
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

    /// 设置设备管理器
    pub fn with_device_manager(
        mut self,
        manager: Arc<openclaw_device::UnifiedDeviceManager>,
    ) -> Self {
        self.device_manager = Some(manager);
        self
    }

    /// 设置设备工具注册中心
    pub fn with_device_tool_registry(mut self, registry: Arc<DeviceToolRegistry>) -> Self {
        self.device_tool_registry = Some(registry);
        self
    }

    /// 设置配置
    pub fn with_config(mut self, config: OrchestratorConfig) -> Self {
        self.config = config;
        self
    }

    /// 启用 Evo 进化功能
    pub fn with_evolution(mut self) -> Self {
        let registry = SharedSkillRegistry::new();
        let engine = EvolutionEngine::new();
        self.shared_skill_registry = Some(Arc::new(registry));
        self.evolution_engine = Some(Arc::new(engine));
        self
    }

    /// 获取设备管理器
    pub fn get_device_manager(&self) -> Option<Arc<openclaw_device::UnifiedDeviceManager>> {
        self.device_manager.clone()
    }

    /// 获取设备工具注册中心
    pub fn get_device_tool_registry(&self) -> Option<Arc<DeviceToolRegistry>> {
        self.device_tool_registry.clone()
    }

    /// 处理任务
    pub async fn process(&self, request: TaskRequest) -> Result<TaskResult> {
        info!(
            "Processing task {} of type {:?}",
            request.id, request.task_type
        );

        // 任务分类决策
        let classification = self.task_classifier.classify(&request).await;
        debug!("Task classification: {:?}", classification);

        // 根据分类结果路由任务
        match classification {
            TaskClassification::Hand { hand_id, input } => {
                info!("Routing task to Hand: {}", hand_id);
                return self.execute_hand(hand_id, input, request).await;
            }
            TaskClassification::WithSkill { task_type, skill_id } => {
                info!("Routing task with Skill: {} for type {:?}", skill_id, task_type);
                return self.execute_with_skill(skill_id, request).await;
            }
            TaskClassification::Direct { task_type: _ } => {
                debug!("Routing task for direct execution");
            }
        }

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
            let agent_id = self
                .team
                .select_agent(&task.required_capabilities, task.preferred_agent.as_deref());

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
                let content = results
                    .iter()
                    .map(|r| format!("**{}**\n{}\n{}", r.title, r.snippet, r.url))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Ok(Message::assistant(content))
            }
            Some(TaskOutput::ToolResult { result }) => Ok(Message::assistant(result.to_string())),
            Some(TaskOutput::Multiple { outputs }) => {
                // 合并多个输出
                let content: String = outputs
                    .iter()
                    .filter_map(|o| match o {
                        TaskOutput::Text { content } => Some(content.clone()),
                        TaskOutput::Message { message } => {
                            message.text_content().map(|s| s.to_string())
                        }
                        TaskOutput::Code { code, .. } => Some(code.clone()),
                        TaskOutput::Data { data } => Some(data.to_string()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Ok(Message::assistant(content))
            }
            None => Err(OpenClawError::Unknown("No output from agent".to_string())),
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
            if let Some(agent) = self.team.get_agent(&agent_id)
                && request
                    .required_capabilities
                    .iter()
                    .all(|c| agent.has_capability(c))
            {
                analysis.suggested_agents.push(agent.id().to_string());
            }
        }

        Ok(analysis)
    }

    /// 分解任务
    async fn decompose_task(
        &self,
        request: &TaskRequest,
        _analysis: &TaskAnalysis,
    ) -> Result<Vec<TaskRequest>> {
        // 简单的任务分解逻辑
        match &request.task_type {
            TaskType::Documentation => {
                // 文档任务分解为：研究 + 写作
                Ok(vec![
                    TaskRequest::new(TaskType::WebSearch, request.input.clone())
                        .with_priority(request.priority),
                    TaskRequest::new(TaskType::Documentation, request.input.clone())
                        .with_priority(request.priority),
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
        // 获取 Agent
        let agent = match self.team.get_agent(agent_id) {
            Some(agent) => agent,
            None => {
                return Ok(TaskResult::failure(
                    task.id,
                    agent_id.to_string(),
                    format!("Agent '{}' not found", agent_id),
                ));
            }
        };

        // 调用 Agent 处理任务
        debug!("Agent {} processing task {}", agent_id, task.id);
        let result = agent.process(task.clone()).await;

        // 如果失败且启用了进化，尝试触发进化
        if let Ok(ref task_result) = result
            && task_result.status == TaskStatus::Failed && self.should_try_evolution(task_result)
                && let Some(evolution_result) = self.try_evolution(agent_id, &task).await
                    && evolution_result {
                        // 进化成功，重试任务
                        info!("Evolution successful, retrying task {}", task.id);
                        return agent.process(task).await;
                    }

        result
    }

    /// 使用 Hand 执行自主任务
    async fn execute_hand(
        &self,
        hand_id: String,
        input: Option<String>,
        request: TaskRequest,
    ) -> Result<TaskResult> {
        info!("Executing Hand: {} with input: {:?}", hand_id, input);

        let executor = match &self.hand_executor {
            Some(e) => e,
            None => {
                return Ok(TaskResult::failure(
                    request.id,
                    hand_id.clone(),
                    "Hand executor not configured".to_string(),
                ));
            }
        };

        let mut ctx = ExecutionContext::new(hand_id.clone());
        if let Some(input_str) = input {
            ctx = ctx.with_input(serde_json::json!({ "input": input_str }));
        }

        let exec_result = executor.execute(&hand_id, ctx).await;

        if let Some(output_manager) = &self.hand_output_manager
            && let Some(hand) = executor.get_hand(&hand_id).await {
                let output_result = HandOutputResult {
                    execution_id: exec_result.task_id.clone(),
                    hand_id: hand_id.clone(),
                    success: exec_result.success,
                    output: serde_json::from_value(serde_json::json!({
                        "result": exec_result.output
                    })).unwrap_or_default(),
                    tool_calls: None,
                    duration_ms: exec_result.duration_ms,
                    timestamp: exec_result.timestamp,
                };
                let messages = output_manager.format_result(&hand, &output_result).await;
                for (channel_type, message) in messages {
                    info!("Sending Hand result to channel: {} - {}", channel_type, message.len());
                }
            }

        if exec_result.success {
            Ok(TaskResult::success(
                request.id,
                hand_id.clone(),
                TaskOutput::Text {
                    content: serde_json::to_string(&exec_result.output).unwrap_or_default(),
                },
            ))
        } else {
            Ok(TaskResult::failure(
                request.id,
                hand_id,
                exec_result.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }

    /// 使用 Skill 执行任务
    async fn execute_with_skill(
        &self,
        skill_id: String,
        request: TaskRequest,
    ) -> Result<TaskResult> {
        info!("Executing with Skill: {}", skill_id);

        let registry = self.shared_skill_registry.as_ref()
            .ok_or_else(|| openclaw_core::OpenClawError::Config("Skill registry not initialized".into()))?;

        let skill = registry.get_skill(&skill_id).await
            .ok_or_else(|| openclaw_core::OpenClawError::Config(format!("Skill '{}' not found", skill_id)))?;

        let result = match skill.skill_type {
            crate::evo::registry::SkillType::Code => {
                if let Some(code) = &skill.code {
                    info!("Executing Code skill: {} in {}", skill.name, skill.language);
                    self.execute_code_skill(&skill, code, request).await
                } else {
                    Err(openclaw_core::OpenClawError::Config("Code skill has no code".into()))
                }
            }
            crate::evo::registry::SkillType::Prompt => {
                info!("Executing Prompt skill: {}", skill.name);
                self.execute_prompt_skill(&skill, request).await
            }
            crate::evo::registry::SkillType::Workflow => {
                info!("Executing Workflow skill: {}", skill.name);
                self.execute_workflow_skill(&skill, request).await
            }
            _ => {
                info!("Skill type {:?} execution delegated to default handler", skill.skill_type);
                let sid = skill_id.clone();
                Ok(TaskResult::success(
                    request.id,
                    skill_id,
                    TaskOutput::Data { data: serde_json::json!({
                        "status": "skill_loaded",
                        "skill_id": sid,
                        "skill_name": skill.name,
                        "skill_type": format!("{:?}", skill.skill_type),
                        "message": "Skill loaded, execution delegated to default handler"
                    }) },
                ))
            }
        };

        result
    }

    async fn execute_code_skill(
        &self,
        skill: &crate::evo::registry::DynamicSkill,
        code: &str,
        request: TaskRequest,
    ) -> Result<TaskResult> {
        info!("Code skill '{}' execution - runtime not available, logging only", skill.name);

        Ok(TaskResult::success(
            request.id,
            skill.id.clone(),
            TaskOutput::Data { data: serde_json::json!({
                "status": "code_skill_loaded",
                "skill_id": skill.id,
                "skill_name": skill.name,
                "language": skill.language,
                "code_length": code.len(),
                "message": "Code skill loaded, actual execution requires WASM/VM runtime"
            }) },
        ))
    }

    async fn execute_prompt_skill(
        &self,
        skill: &crate::evo::registry::DynamicSkill,
        request: TaskRequest,
    ) -> Result<TaskResult> {
        let prompt = skill.instructions.as_deref().unwrap_or("");
        
        info!("Prompt skill '{}' loaded with {} chars", skill.name, prompt.len());

        Ok(TaskResult::success(
            request.id,
            skill.id.clone(),
            TaskOutput::Data { data: serde_json::json!({
                "status": "prompt_skill_loaded",
                "skill_id": skill.id,
                "skill_name": skill.name,
                "prompt_length": prompt.len(),
                "message": "Prompt skill loaded successfully"
            }) },
        ))
    }

    async fn execute_workflow_skill(
        &self,
        skill: &crate::evo::registry::DynamicSkill,
        request: TaskRequest,
    ) -> Result<TaskResult> {
        info!("Workflow skill '{}' execution", skill.name);

        Ok(TaskResult::success(
            request.id,
            skill.id.clone(),
            TaskOutput::Data { data: serde_json::json!({
                "status": "workflow_skill_loaded",
                "skill_id": skill.id,
                "skill_name": skill.name,
                "message": "Workflow skill loaded, execution requires workflow engine"
            }) },
        ))
    }

    /// 检查是否应该触发进化
    fn should_try_evolution(&self, result: &TaskResult) -> bool {
        if self.shared_skill_registry.is_none() || self.evolution_engine.is_none() {
            return false;
        }
        
        if let Some(error) = &result.error {
            error.contains("tool") 
                || error.contains("not found") 
                || error.contains("cannot")
                || error.contains("unknown")
        } else {
            false
        }
    }

    /// 尝试触发进化
    async fn try_evolution(&self, agent_id: &str, task: &TaskRequest) -> Option<bool> {
        use crate::evo::registry::DynamicSkill;
        
        let registry = self.shared_skill_registry.as_ref()?;
        let engine = self.evolution_engine.as_ref()?;

        let context = format!("Task: {:?}, Error: {:?}", task.task_type, task.input);
        
        let result = engine.evolve(&context).await;
        
        if result.status == crate::evo::EvolutionStatus::Completed {
            let skill_code = result.skill.as_ref()?.code.clone();
            let skill_lang = format!("{:?}", result.skill.as_ref()?.language);
            
            let dynamic_skill = DynamicSkill::new(
                uuid::Uuid::new_v4().to_string(),
                "generated_skill".to_string(),
                skill_code,
                skill_lang,
                agent_id.to_string(),
            );
            
            registry.register_skill(dynamic_skill).await;
            info!("Evolution: registered new skill");
            return Some(true);
        }
        
        Some(false)
    }

    /// 聚合多个结果
    async fn aggregate_results(
        &self,
        task_id: uuid::Uuid,
        results: Vec<TaskResult>,
    ) -> Result<TaskResult> {
        let successful: Vec<_> = results
            .iter()
            .filter(|r| r.status == TaskStatus::Completed)
            .collect();

        if successful.is_empty() {
            return Ok(TaskResult::failure(
                task_id,
                "orchestrator".to_string(),
                "All sub-tasks failed".to_string(),
            ));
        }

        // 收集所有输出
        let outputs: Vec<TaskOutput> = successful.iter().filter_map(|r| r.output.clone()).collect();

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

    /// 使用 Graph 执行任务
    pub async fn run_with_graph(
        &self,
        graph: crate::graph::GraphDef,
        input: serde_json::Value,
    ) -> Result<crate::graph::GraphResponse> {
        use crate::graph::ParallelGraphExecutor;

        let executor = ParallelGraphExecutor::new(graph);
        let result = executor
            .execute(uuid::Uuid::new_v4().to_string(), input)
            .await
            .map_err(OpenClawError::Execution)?;

        Ok(result)
    }

    /// 使用 Graph 执行任务（带上下文）
    pub async fn run_with_graph_with_context(
        &self,
        graph: crate::graph::GraphDef,
        input: serde_json::Value,
        context: serde_json::Value,
    ) -> Result<crate::graph::GraphResponse> {
        use crate::graph::ParallelGraphExecutor;

        let executor = ParallelGraphExecutor::new(graph);
        let result = executor
            .execute_with_context(uuid::Uuid::new_v4().to_string(), input, context)
            .await
            .map_err(OpenClawError::Execution)?;

        Ok(result)
    }
}

use chrono::Utc;

/// 任务分析结果
#[derive(Debug)]
#[allow(dead_code)]
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
    use crate::task::TaskInput;

    #[tokio::test]
    async fn test_orchestrator_without_ai_provider() {
        // 没有配置 AI 提供商时，任务应该失败
        let orchestrator = Orchestrator::with_default_team();

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "Hello".to_string(),
            },
        );

        let result = orchestrator.process(task).await.unwrap();
        // 没有 AI 提供商，应该返回失败
        assert_eq!(result.status, TaskStatus::Failed);
    }

    #[tokio::test]
    async fn test_orchestrator_task_routing() {
        let orchestrator = Orchestrator::with_default_team();

        // 测试任务路由选择
        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "Hello".to_string(),
            },
        );

        // 测试 agent 选择
        let agent_id = orchestrator
            .team()
            .select_agent(&task.required_capabilities, None);

        // 应该选择一个可用的 agent
        assert!(agent_id.is_some());
    }

    #[tokio::test]
    async fn test_orchestrator_run_with_graph() {
        use crate::graph::{ExecutionStatus, GraphPatterns};

        let orchestrator = Orchestrator::with_default_team();

        // 使用预定义的并行模式创建图
        let graph = GraphPatterns::parallel(&["agent_1", "agent_2"]);

        // 使用 Graph 执行任务
        let result = orchestrator
            .run_with_graph(graph, serde_json::json!({"query": "test"}))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Completed);
    }

    #[tokio::test]
    async fn test_orchestrator_run_with_sequential_graph() {
        use crate::graph::{ExecutionStatus, GraphPatterns};

        let orchestrator = Orchestrator::with_default_team();

        // 使用顺序模式创建图
        let graph = GraphPatterns::sequential(&["agent_1", "agent_2", "agent_3"]);

        let result = orchestrator
            .run_with_graph(graph, serde_json::json!({"query": "test"}))
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Completed);
    }

    #[tokio::test]
    async fn test_orchestrator_run_with_custom_graph() {
        use crate::graph::{EdgeDef, ExecutionStatus, GraphDef, NodeDef, NodeType};

        let orchestrator = Orchestrator::with_default_team();

        let graph = GraphDef::new("custom", "Custom Graph")
            .with_node(NodeDef::new("start", NodeType::Router))
            .with_node(NodeDef::new("process", NodeType::Executor).with_agent("agent_1"))
            .with_node(NodeDef::new("end", NodeType::Terminal))
            .with_edge(EdgeDef::new("start", "process"))
            .with_edge(EdgeDef::new("process", "end"))
            .with_end("end");

        let result = orchestrator
            .run_with_graph(graph, serde_json::json!({"data": "test"}))
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_orchestrator_run_with_graph_with_context() {
        use crate::graph::{EdgeDef, ExecutionStatus, GraphDef, NodeDef, NodeType};

        let orchestrator = Orchestrator::with_default_team();

        let graph = GraphDef::new("context_test", "Context Test Graph")
            .with_node(NodeDef::new("start", NodeType::Router))
            .with_node(NodeDef::new("process", NodeType::Executor).with_agent("agent_1"))
            .with_node(NodeDef::new("end", NodeType::Terminal))
            .with_edge(EdgeDef::new("start", "process"))
            .with_edge(EdgeDef::new("process", "end"))
            .with_end("end");

        let context = serde_json::json!({
            "working": {
                "current_agent": "test_agent",
                "current_task": "context_test_task"
            },
            "knowledge": []
        });

        let result = orchestrator
            .run_with_graph_with_context(graph, serde_json::json!({"data": "test"}), context)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.events.is_empty());
    }

    #[tokio::test]
    async fn test_orchestrator_execute_hand_not_configured() {
        let orchestrator = Orchestrator::with_default_team();

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "test hand execution".to_string(),
            },
        );

        let result = orchestrator.execute_hand("test_hand".to_string(), None, task).await.unwrap();
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("not configured"));
    }

    #[tokio::test]
    async fn test_orchestrator_execute_hand_with_executor() {
        use crate::evo::autonomous::presets::researcher_hand;
        use crate::evo::autonomous::schedule::ScheduleManager;
        use crate::evo::autonomous::metrics::MetricsCollector;
        use crate::evo::autonomous::HandRegistry;
        use crate::evo::autonomous::HandExecutor;

        let registry = Arc::new(HandRegistry::new());
        let mut hand = researcher_hand();
        hand.enable();
        registry.register(hand).await;

        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());
        let executor = Arc::new(HandExecutor::new(
            registry,
            schedule_manager,
            metrics,
        ));

        let mut orchestrator = Orchestrator::with_default_team();
        orchestrator.hand_executor = Some(executor);

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "test hand execution".to_string(),
            },
        );

        let result = orchestrator.execute_hand("researcher".to_string(), None, task).await.unwrap();
        assert_eq!(result.status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn test_orchestrator_execute_hand_not_found() {
        use crate::evo::autonomous::schedule::ScheduleManager;
        use crate::evo::autonomous::metrics::MetricsCollector;
        use crate::evo::autonomous::HandRegistry;
        use crate::evo::autonomous::HandExecutor;

        let registry = Arc::new(HandRegistry::new());
        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());
        let executor = Arc::new(HandExecutor::new(
            registry,
            schedule_manager,
            metrics,
        ));

        let mut orchestrator = Orchestrator::with_default_team();
        orchestrator.hand_executor = Some(executor);

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "test".to_string(),
            },
        );

        let result = orchestrator.execute_hand("nonexistent".to_string(), None, task).await.unwrap();
        assert_eq!(result.status, TaskStatus::Failed);
        assert!(result.error.unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn test_orchestrator_execute_hand_with_input() {
        use crate::evo::autonomous::presets::collector_hand;
        use crate::evo::autonomous::schedule::ScheduleManager;
        use crate::evo::autonomous::metrics::MetricsCollector;
        use crate::evo::autonomous::HandRegistry;
        use crate::evo::autonomous::HandExecutor;

        let registry = Arc::new(HandRegistry::new());
        let mut hand = collector_hand();
        hand.enable();
        registry.register(hand).await;

        let schedule_manager = Arc::new(ScheduleManager::new());
        let metrics = Arc::new(MetricsCollector::new());
        let executor = Arc::new(HandExecutor::new(
            registry,
            schedule_manager,
            metrics,
        ));

        let mut orchestrator = Orchestrator::with_default_team();
        orchestrator.hand_executor = Some(executor);

        let task = TaskRequest::new(
            TaskType::Conversation,
            TaskInput::Text {
                content: "test".to_string(),
            },
        );

        let result = orchestrator
            .execute_hand("collector".to_string(), Some("test input".to_string()), task)
            .await
            .unwrap();
        assert_eq!(result.status, TaskStatus::Completed);
    }
}
