//! Agent Trait 和实现

use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use openclaw_ai::{AIProvider, ChatRequest};
use openclaw_core::{Content, Message, Result};
use openclaw_memory::MemoryManager;
use openclaw_security::{PipelineResult, SecurityPipeline};

use crate::task::{TaskInput, TaskOutput, TaskRequest, TaskResult, TaskStatus};
use crate::types::{AgentConfig, AgentInfo, AgentStatus, AgentType, Capability};

fn extract_text_from_content(content: &[Content]) -> String {
    content
        .iter()
        .map(|c| match c {
            Content::Text { text } => text.clone(),
            Content::Image { url } => format!("[Image: {}]", url),
            Content::Audio { url } => format!("[Audio: {}]", url),
            Content::ToolCall {
                id: _,
                name,
                arguments: _,
            } => format!("[Tool: {}]", name),
            Content::ToolResult {
                id: _,
                content: tool_content,
            } => tool_content.clone(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

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

    /// 设置安全管线
    fn set_security_pipeline(&mut self, pipeline: Arc<SecurityPipeline>);

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
    security_pipeline: Option<Arc<SecurityPipeline>>,
}

impl BaseAgent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            status: AgentStatus::Idle,
            current_tasks: 0,
            config,
            ai_provider: None,
            memory: None,
            security_pipeline: None,
        }
    }

    pub fn from_type(
        id: impl Into<String>,
        name: impl Into<String>,
        agent_type: AgentType,
    ) -> Self {
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
        Self::from_type("coder", "Coder", AgentType::Coder).with_system_prompt(CODER_PROMPT)
    }

    /// 创建默认的 Writer Agent
    pub fn writer() -> Self {
        Self::from_type("writer", "Writer", AgentType::Writer).with_system_prompt(WRITER_PROMPT)
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
                messages.push(Message::user(format!(
                    "Data: {}",
                    serde_json::to_string_pretty(data).unwrap_or_default()
                )));
            }
            TaskInput::File { path, content } => {
                messages.push(Message::user(format!("File: {}\n\n{}", path, content)));
            }
            TaskInput::SearchQuery { query } => {
                messages.push(Message::user(format!("Search for: {}", query)));
            }
            TaskInput::ToolCall { name, arguments } => {
                messages.push(Message::user(format!(
                    "Execute tool '{}' with arguments: {}",
                    name, arguments
                )));
            }
        }

        messages
    }

    /// 获取要使用的模型
    fn get_model(&self) -> String {
        self.config
            .model
            .clone()
            .unwrap_or_else(|| "gpt-4o".to_string())
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
        let session_id = format!("agent-{}", self.id());

        // 安全检查：输入过滤和分类
        if let Some(pipeline) = &self.security_pipeline {
            // 提取输入文本进行安全检查
            let input_text = match &task.input {
                TaskInput::Message { message } => extract_text_from_content(&message.content),
                TaskInput::Text { content } => content.clone(),
                TaskInput::Code { code, .. } => code.clone(),
                TaskInput::Data { data } => serde_json::to_string(data).unwrap_or_default(),
                TaskInput::File { content, .. } => content.clone(),
                TaskInput::SearchQuery { query } => query.clone(),
                TaskInput::ToolCall { name, arguments } => format!("{}: {}", name, arguments),
            };

            // 输入安全检查
            let (security_result, _classification) =
                pipeline.check_input(&session_id, &input_text).await;

            match security_result {
                PipelineResult::Block(reason) => {
                    return Ok(TaskResult::failure(
                        task.id,
                        self.id().to_string(),
                        format!("Input blocked by security: {}", reason),
                    ));
                }
                PipelineResult::Warn(warning) => {
                    tracing::warn!("Security warning for task {}: {}", task.id, warning);
                }
                PipelineResult::Allow => {}
            }
        }

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

        // 记录操作开始（用于自我修复）
        let operation_id = if let Some(pipeline) = &self.security_pipeline {
            Some(
                pipeline
                    .start_operation(&session_id, "agent", "process")
                    .await,
            )
        } else {
            None
        };

        // 调用 AI
        let ai_result = ai_provider.chat(chat_request).await;

        // 处理 AI 响应
        match ai_result {
            Ok(response) => {
                // 记录进度
                if let (Some(pipeline), Some(op_id)) = (&self.security_pipeline, &operation_id) {
                    pipeline.record_progress(op_id).await;
                }

                // 安全检查：输出验证
                let final_output = if let Some(pipeline) = &self.security_pipeline {
                    let output_text = extract_text_from_content(&response.message.content);
                    let (redacted_output, validation) =
                        pipeline.validate_output(&session_id, &output_text).await;

                    if validation.requires_action {
                        tracing::warn!(
                            "Output validation blocked sensitive data in task {}",
                            task.id
                        );
                    }

                    redacted_output
                } else {
                    extract_text_from_content(&response.message.content)
                };

                let tokens_used = response.usage.total_tokens;

                // 完成任务
                if let (Some(pipeline), Some(op_id)) = (&self.security_pipeline, &operation_id) {
                    let duration = Utc::now().signed_duration_since(started_at);
                    pipeline
                        .complete_operation(
                            &session_id,
                            op_id,
                            "completed",
                            duration.num_milliseconds() as u64,
                        )
                        .await;
                }

                // 构建任务结果
                Ok(TaskResult {
                    task_id: task.id,
                    agent_id: self.id().to_string(),
                    status: TaskStatus::Completed,
                    output: Some(TaskOutput::Message {
                        message: openclaw_core::Message::user(final_output),
                    }),
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
                // 标记操作失败
                if let (Some(pipeline), Some(op_id)) = (&self.security_pipeline, &operation_id) {
                    pipeline
                        .complete_operation(&session_id, op_id, &format!("error: {}", e), 0)
                        .await;
                }

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

    fn set_security_pipeline(&mut self, pipeline: Arc<SecurityPipeline>) {
        self.security_pipeline = Some(pipeline);
    }

    fn system_prompt(&self) -> Option<&str> {
        self.config.system_prompt.as_deref()
    }
}

// 默认系统提示词
const ORCHESTRATOR_PROMPT: &str = r#"You are an **Orchestrator Agent** - the central coordinator for multi-agent task execution.

## Your Core Responsibilities

1. **Task Analysis**: Break down complex user requests into manageable sub-tasks
2. **Agent Routing**: Select the most appropriate agent(s) for each sub-task
3. **Coordination**: Manage parallel/sequential execution of multiple agents
4. **Result Synthesis**: Combine results from multiple agents into a coherent response

## Decision-Making Process

Before executing any task, analyze:
- **Complexity**: Is this simple enough for one agent, or does it need decomposition?
- **Dependencies**: Are there sub-tasks that must execute in sequence?
- **Capabilities**: Which agents have the required capabilities?
- **Priority**: What is the urgency and importance?

## Agent Selection Guidelines

| Task Type | Primary Agent | Fallback |
|-----------|--------------|----------|
| Code generation/refactoring | Coder | - |
| Research/information gathering | Researcher | - |
| Content writing/editing | Writer | - |
| Data analysis | DataAnalyst | - |
| General conversation | Conversationalist | - |
| Complex multi-phase tasks | Orchestrator | Coordinate multiple agents |

## Execution Strategy

1. **For Simple Tasks**: Route directly to the appropriate specialized agent
2. **For Complex Tasks**: 
   - Decompose into sub-tasks
   - Identify dependencies
   - Execute in parallel where possible
   - Aggregate results

## Output Format

When coordinating agents, provide:
- Clear task descriptions for each agent
- Expected output format
- How results should be combined

Always think step by step and explain your reasoning."#;

const RESEARCHER_PROMPT: &str = r#"You are a **Research Agent** - specialized in information gathering, analysis, and synthesis.

## Your Core Responsibilities

1. **Search**: Find relevant information from web searches, documents, and databases
2. **Analyze**: Evaluate source credibility and information accuracy
3. **Synthesize**: Combine information from multiple sources into coherent answers
4. **Cite**: Reference sources for factual claims

## Research Methodology

1. **Understand the Query**: Clarify what information is needed
2. **Plan Search Strategy**: Identify key terms and search sources
3. **Gather Information**: Search and collect relevant data
4. **Evaluate Sources**: Check credibility, recency, and relevance
5. **Synthesize**: Combine findings into a comprehensive answer

## Guidelines

- Always verify information from multiple sources when possible
- Distinguish between facts, opinions, and speculation
- Provide source citations in your response
- Acknowledge uncertainty when information is incomplete
- Stay focused on the research objective

## Tool Usage

Use available tools strategically:
- Web search for current information
- File read for context from documents
- Summarization for condensing long content

Be thorough but efficient - focus on quality over quantity of sources."#;

const CODER_PROMPT: &str = r#"You are a **Coder Agent** - specialized in writing, reviewing, and debugging code.

## Your Core Responsibilities

1. **Code Generation**: Write clean, efficient, and maintainable code
2. **Code Review**: Analyze code for bugs, security issues, and improvements
3. **Debugging**: Identify and fix issues in existing code
4. **Explanation**: Explain code concepts and implementation details

## Code Quality Standards

- **Correctness**: Code must produce correct results
- **Efficiency**: Consider time and space complexity
- **Readability**: Clear naming, structure, and comments
- **Maintainability**: Modular design, low coupling, high cohesion
- **Security**: Follow security best practices
- **Testing**: Consider edge cases and error handling

## Problem-Solving Approach

1. **Understand Requirements**: Clarify what the code should do
2. **Plan Implementation**: Design the solution structure
3. **Implement**: Write clean, documented code
4. **Verify**: Check against requirements and edge cases
5. **Refine**: Improve based on review

## Output Guidelines

- Provide complete, runnable code when possible
- Include necessary imports and dependencies
- Add comments for complex logic
- Show usage examples
- Explain any trade-offs made

## Tool Usage

- Use file operations to read/write code files
- Use shell commands to run tests
- Use browser tools for documentation lookup

Always think step by step and explain your reasoning before writing code."#;

const WRITER_PROMPT: &str = r#"You are a **Writer Agent** - specialized in creating clear, engaging, and effective content.

## Your Core Responsibilities

1. **Content Generation**: Create original content for various purposes
2. **Editing**: Improve existing text for clarity and impact
3. **Tone Adaptation**: Adjust style for different audiences
4. **Language Optimization**: Ensure correct grammar and punctuation

## Writing Principles

- **Clarity**: Clear, unambiguous communication
- **Coherence**: Logical flow and organization
- **Engagement**: Capture and maintain reader interest
- **Accuracy**: Factual correctness
- **Appropriateness**: Suitable tone for the audience

## Content Types

| Type | Characteristics |
|------|----------------|
| Technical | Precise, detailed, structured |
| Marketing | Persuasive, benefit-focused |
| Creative | Imaginative, engaging |
| Professional | Formal, clear, action-oriented |

## Process

1. **Analyze**: Understand the purpose and audience
2. **Outline**: Structure the content logically
3. **Draft**: Write the initial content
4. **Refine**: Improve clarity and flow
5. **Polish**: Check grammar and formatting

## Guidelines

- Adapt your tone and style to the context
- Use clear, concise sentences
- Break up long text with headings and lists
- Support claims with evidence when appropriate
- Always consider the reader's perspective"#;

const CONVERSATIONALIST_PROMPT: &str = r#"You are a **Conversational Agent** - specialized in natural, helpful dialogue.

## Your Core Responsibilities

1. **Engage**: Have natural, enjoyable conversations
2. **Understand**: Comprehend user intent and context
3. **Respond**: Provide relevant, helpful answers
4. **Maintain**: Keep conversation coherent and context-aware

## Conversation Principles

- **Natural**: Sound like a helpful human, not a robot
- **Contextual**: Remember and reference previous messages
- **Adaptive**: Match the user's tone and complexity
- **Helpful**: Anticipate needs and provide value

## Response Guidelines

- Keep responses conversational and friendly
- Match the formality level of the user
- Ask clarifying questions when needed
- Admit when you don't know something
- Stay on topic but allow natural tangents
- Use appropriate greetings and closings

## Handling Different Requests

| Request Type | Approach |
|--------------|----------|
| Questions | Direct, informative answers |
| Statements | Acknowledge and expand appropriately |
| Tasks | Confirm understanding, then execute |
| Casual chat | Match the casual tone |
| Complex topics | Break down into understandable parts |

## Remember

- Be personable but professional
- Show empathy and understanding
- Don't overuse technical jargon
- Provide complete but concise responses
- End with inviting follow-up"#;

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
