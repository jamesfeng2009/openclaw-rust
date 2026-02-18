//! 任务定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::Capability;
use openclaw_core::Message;

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// 对话
    Conversation,
    /// 问答
    QuestionAnswer,
    /// 代码编写
    CodeGeneration,
    /// 代码审查
    CodeReview,
    /// 网络搜索
    WebSearch,
    /// 数据分析
    DataAnalysis,
    /// 文档生成
    Documentation,
    /// 翻译
    Translation,
    /// 总结
    Summarization,
    /// 工具调用
    ToolExecution,
    /// 自定义
    Custom(String),
}

impl TaskType {
    /// 获取任务所需的能力
    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            TaskType::Conversation => vec![Capability::Conversation, Capability::ContextAwareness],
            TaskType::QuestionAnswer => vec![Capability::QAndA, Capability::InformationAnalysis],
            TaskType::CodeGeneration => vec![Capability::CodeGeneration],
            TaskType::CodeReview => vec![Capability::CodeReview, Capability::Debugging],
            TaskType::WebSearch => vec![Capability::WebSearch, Capability::InformationAnalysis],
            TaskType::DataAnalysis => vec![Capability::DataAnalysis, Capability::Reporting],
            TaskType::Documentation => vec![Capability::ContentGeneration, Capability::Editing],
            TaskType::Translation => vec![Capability::Translation],
            TaskType::Summarization => vec![Capability::Summarization],
            TaskType::ToolExecution => vec![Capability::ToolExecution],
            TaskType::Custom(_) => vec![Capability::General],
        }
    }
}

/// 任务优先级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Urgent = 3,
}


/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// 待处理
    Pending,
    /// 已分配
    Assigned,
    /// 处理中
    Processing,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 任务请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub id: Uuid,
    pub task_type: TaskType,
    pub priority: TaskPriority,
    pub input: TaskInput,
    pub context: Vec<Message>,
    pub required_capabilities: Vec<Capability>,
    pub preferred_agent: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub created_at: DateTime<Utc>,
}

impl TaskRequest {
    pub fn new(task_type: TaskType, input: TaskInput) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type: task_type.clone(),
            priority: TaskPriority::default(),
            input,
            context: Vec::new(),
            required_capabilities: task_type.required_capabilities(),
            preferred_agent: None,
            timeout_seconds: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_context(mut self, context: Vec<Message>) -> Self {
        self.context = context;
        self
    }

    pub fn with_preferred_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.preferred_agent = Some(agent_id.into());
        self
    }

    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = Some(seconds);
        self
    }

    /// 从用户消息创建对话任务
    pub fn from_message(message: Message) -> Self {
        let input = TaskInput::Message {
            message: message.clone(),
        };
        Self::new(TaskType::Conversation, input).with_context(vec![message])
    }
}

/// 任务输入
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskInput {
    /// 消息输入
    Message { message: Message },
    /// 文本输入
    Text { content: String },
    /// 代码输入
    Code { language: String, code: String },
    /// 数据输入
    Data { data: serde_json::Value },
    /// 文件输入
    File { path: String, content: String },
    /// 搜索查询
    SearchQuery { query: String },
    /// 工具调用
    ToolCall {
        name: String,
        arguments: serde_json::Value,
    },
}

/// 任务结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub agent_id: String,
    pub status: TaskStatus,
    pub output: Option<TaskOutput>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub tokens_used: Option<TokenUsage>,
    pub sub_tasks: Vec<TaskResult>,
}

impl TaskResult {
    pub fn success(task_id: Uuid, agent_id: String, output: TaskOutput) -> Self {
        Self {
            task_id,
            agent_id,
            status: TaskStatus::Completed,
            output: Some(output),
            error: None,
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            tokens_used: None,
            sub_tasks: Vec::new(),
        }
    }

    pub fn failure(task_id: Uuid, agent_id: String, error: String) -> Self {
        Self {
            task_id,
            agent_id,
            status: TaskStatus::Failed,
            output: None,
            error: Some(error),
            started_at: Utc::now(),
            completed_at: Some(Utc::now()),
            tokens_used: None,
            sub_tasks: Vec::new(),
        }
    }
}

/// 任务输出
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskOutput {
    /// 消息输出
    Message { message: Message },
    /// 文本输出
    Text { content: String },
    /// 代码输出
    Code { language: String, code: String },
    /// 数据输出
    Data { data: serde_json::Value },
    /// 搜索结果
    SearchResult { results: Vec<SearchResultItem> },
    /// 工具结果
    ToolResult { result: serde_json::Value },
    /// 多个输出
    Multiple { outputs: Vec<TaskOutput> },
}

/// 搜索结果项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResultItem {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub relevance_score: f32,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub total_tokens: usize,
}
