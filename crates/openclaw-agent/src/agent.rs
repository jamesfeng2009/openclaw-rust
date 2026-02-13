//! Agent Trait 和实现

use async_trait::async_trait;
use std::sync::Arc;

use openclaw_core::{Message, Result};
use openclaw_memory::MemoryManager;
use openclaw_ai::AIProvider;

use crate::types::{AgentConfig, AgentInfo, AgentStatus, AgentType, Capability};
use crate::task::{TaskRequest, TaskResult};

/// Agent Trait - 所有 Agent 必须实现
#[async_trait]
pub trait Agent: Send + Sync {
    /// 获取 Agent ID
    fn id(&self) -> &str;

    /// 获取 Agent 名称
    fn name(&self) -> &str;

    /// 获取 Agent 类型
    fn agent_type(&self) -> AgentType;

    /// 获取 Agent 能力
    fn capabilities(&self) -> Vec<Capability>;

    /// 检查是否有指定能力
    fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities().contains(capability)
    }

    /// 获取 Agent 信息
    fn info(&self) -> AgentInfo;

    /// 处理任务
    async fn process(&self, task: TaskRequest) -> Result<TaskResult>;

    /// 是否可用
    fn is_available(&self) -> bool;

    /// 获取当前负载 (0.0 - 1.0)
    fn load(&self) -> f32;

    /// 设置 AI 提供商
    fn set_ai_provider(&mut self, provider: Arc<dyn AIProvider>);

    /// 设置记忆管理器
    fn set_memory(&mut self, memory: Arc<MemoryManager>);
}

/// 基础 Agent 实现
pub struct BaseAgent {
    config: AgentConfig,
    status: AgentStatus,
    current_tasks: usize,
    ai_provider: Option<Arc<dyn AIProvider>>,
    memory: Option<Arc<MemoryManager>>,
}

impl BaseAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            status: AgentStatus::Idle,
            current_tasks: 0,
            config,
            ai_provider: None,
            memory: None,
        }
    }

    pub fn from_type(id: impl Into<String>, name: impl Into<String>, agent_type: AgentType) -> Self {
        let config = AgentConfig::new(id, name, agent_type);
        Self::new(config)
    }

    /// 创建默认的 Orchestrator Agent
    pub fn orchestrator() -> Self {
        Self::from_type("orchestrator", "Orchestrator", AgentType::Orchestrator)
            .with_system_prompt(ORCHESTRATOR_PROMPT)
    }

    /// 创建默认的 Researcher Agent
    pub fn researcher() -> Self {
        Self::from_type("researcher", "Researcher", AgentType::Researcher)
            .with_system_prompt(RESEARCHER_PROMPT)
    }

    /// 创建默认的 Coder Agent
    pub fn coder() -> Self {
        Self::from_type("coder", "Coder", AgentType::Coder)
            .with_system_prompt(CODER_PROMPT)
    }

    /// 创建默认的 Writer Agent
    pub fn writer() -> Self {
        Self::from_type("writer", "Writer", AgentType::Writer)
            .with_system_prompt(WRITER_PROMPT)
    }

    /// 创建默认的 Conversationalist Agent
    pub fn conversationalist() -> Self {
        Self::from_type("chat", "Conversationalist", AgentType::Conversationalist)
            .with_system_prompt(CONVERSATIONALIST_PROMPT)
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.config.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.config.model = Some(model.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.config.priority = priority;
        self
    }
}

#[async_trait]
impl Agent for BaseAgent {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn agent_type(&self) -> AgentType {
        self.config.agent_type.clone()
    }

    fn capabilities(&self) -> Vec<Capability> {
        self.config.capabilities.clone()
    }

    fn info(&self) -> AgentInfo {
        AgentInfo::new(self.config.clone())
    }

    async fn process(&self, task: TaskRequest) -> Result<TaskResult> {
        // 基础实现：调用 AI 提供商
        // 具体实现由子类或 Orchestrator 协调
        todo!("Implement agent processing")
    }

    fn is_available(&self) -> bool {
        self.config.enabled 
            && self.status == AgentStatus::Idle
            && self.current_tasks < self.config.max_concurrent_tasks
    }

    fn load(&self) -> f32 {
        if self.config.max_concurrent_tasks == 0 {
            return 1.0;
        }
        self.current_tasks as f32 / self.config.max_concurrent_tasks as f32
    }

    fn set_ai_provider(&mut self, provider: Arc<dyn AIProvider>) {
        self.ai_provider = Some(provider);
    }

    fn set_memory(&mut self, memory: Arc<MemoryManager>) {
        self.memory = Some(memory);
    }
}

// 默认系统提示词
const ORCHESTRATOR_PROMPT: &str = r#"You are an orchestrator agent responsible for:
- Analyzing incoming tasks and routing them to appropriate agents
- Coordinating multiple agents for complex tasks
- Making decisions about task prioritization
- Synthesizing results from multiple agents

You should think step by step and clearly communicate your reasoning."#;

const RESEARCHER_PROMPT: &str = r#"You are a research agent specialized in:
- Searching and gathering information
- Analyzing and synthesizing data
- Providing accurate and comprehensive answers
- Citing sources when available

Always strive for accuracy and completeness in your research."#;

const CODER_PROMPT: &str = r#"You are a coding agent specialized in:
- Writing clean, efficient, and well-documented code
- Code review and debugging
- Explaining code concepts
- Following best practices and design patterns

Always consider edge cases and write testable code."#;

const WRITER_PROMPT: &str = r#"You are a writing agent specialized in:
- Creating clear and engaging content
- Editing and improving text
- Adapting tone and style for different audiences
- Grammar and language optimization

Focus on clarity, coherence, and impact in your writing."#;

const CONVERSATIONALIST_PROMPT: &str = r#"You are a conversational agent specialized in:
- Natural and engaging dialogue
- Context-aware responses
- Answering questions helpfully
- Maintaining conversation flow

Be friendly, helpful, and responsive to user needs."#;
