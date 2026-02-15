//! Agent 类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent ID 类型
pub type AgentId = String;

/// Agent 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// 主控 Agent - 负责任务分发和协调
    Orchestrator,
    /// 研究 Agent - 搜索、分析信息
    Researcher,
    /// 代码 Agent - 编写、审查代码
    Coder,
    /// 写作 Agent - 生成文案、文档
    Writer,
    /// 数据 Agent - 数据分析、处理
    DataAnalyst,
    /// 对话 Agent - 日常对话、问答
    Conversationalist,
    /// 工具 Agent - 执行特定工具
    ToolUser,
    /// 自定义 Agent
    Custom(String),
}

impl AgentType {
    /// 获取 Agent 的默认能力
    pub fn default_capabilities(&self) -> Vec<Capability> {
        match self {
            AgentType::Orchestrator => vec![
                Capability::TaskRouting,
                Capability::AgentCoordination,
                Capability::DecisionMaking,
            ],
            AgentType::Researcher => vec![
                Capability::WebSearch,
                Capability::InformationAnalysis,
                Capability::Summarization,
            ],
            AgentType::Coder => vec![
                Capability::CodeGeneration,
                Capability::CodeReview,
                Capability::Debugging,
            ],
            AgentType::Writer => vec![
                Capability::ContentGeneration,
                Capability::Editing,
                Capability::Translation,
            ],
            AgentType::DataAnalyst => vec![
                Capability::DataAnalysis,
                Capability::Visualization,
                Capability::Reporting,
            ],
            AgentType::Conversationalist => vec![
                Capability::Conversation,
                Capability::QAndA,
                Capability::ContextAwareness,
            ],
            AgentType::ToolUser => vec![
                Capability::ToolExecution,
                Capability::FileOperations,
                Capability::SystemCommands,
            ],
            AgentType::Custom(_) => vec![Capability::General],
        }
    }
}

/// Agent 能力
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    // 通用能力
    General,
    Conversation,
    QAndA,
    ContextAwareness,
    DecisionMaking,
    
    // 任务处理
    TaskRouting,
    AgentCoordination,
    
    // 信息处理
    WebSearch,
    InformationAnalysis,
    Summarization,
    Translation,
    
    // 代码相关
    CodeGeneration,
    CodeReview,
    Debugging,
    
    // 内容创作
    ContentGeneration,
    Editing,
    
    // 数据处理
    DataAnalysis,
    Visualization,
    Reporting,
    
    // 工具使用
    ToolExecution,
    FileOperations,
    SystemCommands,
}

/// Agent 状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// 空闲
    Idle,
    /// 处理中
    Processing,
    /// 等待中
    Waiting,
    /// 离线
    Offline,
}

/// Agent 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent ID
    pub id: String,
    /// Agent 名称
    pub name: String,
    /// Agent 类型
    pub agent_type: AgentType,
    /// 描述
    pub description: Option<String>,
    /// 系统提示词
    pub system_prompt: Option<String>,
    /// 使用的模型
    pub model: Option<String>,
    /// 能力列表
    pub capabilities: Vec<Capability>,
    /// 优先级 (0-100, 越高越优先)
    pub priority: u8,
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 是否启用
    pub enabled: bool,
}

impl AgentConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, agent_type: AgentType) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            agent_type: agent_type.clone(),
            description: None,
            system_prompt: None,
            model: None,
            capabilities: agent_type.default_capabilities(),
            priority: 50,
            max_concurrent_tasks: 1,
            enabled: true,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        self.capabilities = capabilities;
        self
    }
}

/// Agent 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub current_tasks: usize,
    pub total_tasks_completed: usize,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl AgentInfo {
    pub fn new(config: AgentConfig) -> Self {
        let now = Utc::now();
        Self {
            config,
            status: AgentStatus::Idle,
            current_tasks: 0,
            total_tasks_completed: 0,
            created_at: now,
            last_active_at: now,
        }
    }

    pub fn is_available(&self) -> bool {
        self.config.enabled 
            && self.status == AgentStatus::Idle
            && self.current_tasks < self.config.max_concurrent_tasks
    }
}
