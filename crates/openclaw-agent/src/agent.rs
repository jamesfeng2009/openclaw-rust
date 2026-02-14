//! Agent Trait 和实现

use async_trait::async_trait;
use std::sync::Arc;
use chrono::Utc;

use openclaw_core::{Message, OpenClawError, Result};
use openclaw_memory::MemoryManager;
use openclaw_ai::{AIProvider, ChatRequest};

use crate::types::{AgentConfig, AgentInfo, AgentStatus, AgentType, Capability};
use crate::task::{TaskInput, TaskOutput, TaskRequest, TaskResult, TaskStatus};

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

    /// 获取系统提示词
    fn system_prompt(&self) -> Option<&str>;
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

    /// 构建发送给 AI 的消息列表
    fn build_messages(&self, task: &TaskRequest) -> Vec<Message> {
        let mut messages = Vec::new();

        // 添加系统提示词
        if let Some(prompt) = &self.config.system_prompt {
            messages.push(Message::system(prompt));
        }

        // 添加上下文消息
        messages.extend(task.context.clone());

        // 根据任务输入添加用户消息
        match &task.input {
            TaskInput::Message { message } => {
                messages.push(message.clone());
            }
            TaskInput::Text { content } => {
                messages.push(Message::user(content));
            }
            TaskInput::Code { language, code } => {
                messages.push(Message::user(format!("```{}\n{}\n```", language, code)));
            }
            TaskInput::Data { data } => {
                messages.push(Message::user(format!("Data: {}", serde_json::to_string_pretty(data).unwrap_or_default())));
            }
            TaskInput::File { path, content } => {
                messages.push(Message::user(format!("File: {}\n\n{}", path, content)));
            }
            TaskInput::SearchQuery { query } => {
                messages.push(Message::user(format!("Search for: {}", query)));
            }
            TaskInput::ToolCall { name, arguments } => {
                messages.push(Message::user(format!("Execute tool '{}' with arguments: {}", name, arguments)));
            }
        }

        messages
    }

    /// 获取要使用的模型
    fn get_model(&self) -> String {
        self.config.model.clone().unwrap_or_else(|| "gpt-4o".to_string())
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
        let started_at = Utc::now();

        // 检查是否有 AI 提供商
        let ai_provider = match &self.ai_provider {
            Some(provider) => provider,
            None => {
                return Ok(TaskResult::failure(
                    task.id,
                    self.id().to_string(),
                    "No AI provider configured for this agent".to_string(),
                ));
            }
        };

        // 构建消息
        let messages = self.build_messages(&task);

        // 创建 ChatRequest
        let model = self.get_model();
        let chat_request = ChatRequest::new(&model, messages);

        // 调用 AI
        match ai_provider.chat(chat_request).await {
            Ok(response) => {
                // 提取响应消息
                let reply_message = response.message;
                let tokens_used = response.usage.total_tokens;

                // 构建任务结果
                Ok(TaskResult {
                    task_id: task.id,
                    agent_id: self.id().to_string(),
                    status: TaskStatus::Completed,
                    output: Some(TaskOutput::Message { message: reply_message }),
                    error: None,
                    started_at,
                    completed_at: Some(Utc::now()),
                    tokens_used: Some(crate::task::TokenUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: tokens_used,
                    }),
                    sub_tasks: Vec::new(),
                })
            }
            Err(e) => {
                Ok(TaskResult::failure(
                    task.id,
                    self.id().to_string(),
                    format!("AI provider error: {}", e),
                ))
            }
        }
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

    fn system_prompt(&self) -> Option<&str> {
        self.config.system_prompt.as_deref()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = BaseAgent::coder();
        assert_eq!(agent.id(), "coder");
        assert_eq!(agent.agent_type(), AgentType::Coder);
        assert!(agent.is_available());
    }

    #[test]
    fn test_agent_capabilities() {
        let agent = BaseAgent::coder();
        assert!(agent.has_capability(&Capability::CodeGeneration));
        assert!(agent.has_capability(&Capability::CodeReview));
    }
}
