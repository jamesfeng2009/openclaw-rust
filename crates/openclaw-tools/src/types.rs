//! 工具类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 工具 ID
pub type ToolId = String;

/// 任务 ID
pub type TaskId = String;

/// Webhook ID
pub type WebhookId = String;

/// 技能 ID
pub type SkillId = String;

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub id: ToolId,
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
    pub category: ToolCategory,
    pub enabled: bool,
}

/// 工具参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    pub properties: HashMap<String, ParameterProperty>,
    pub required: Vec<String>,
}

/// 参数属性
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterProperty {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    #[serde(default)]
    pub enum_values: Vec<String>,
    pub default: Option<serde_json::Value>,
}

/// 工具分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ToolCategory {
    Browser,
    File,
    Network,
    System,
    Data,
    Custom,
}

/// 工具执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub user_id: String,
    pub session_id: String,
    pub variables: HashMap<String, serde_json::Value>,
}

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl ToolResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
            metadata: HashMap::new(),
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(msg),
            metadata: HashMap::new(),
        }
    }
}

/// 定时任务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleTask {
    pub id: TaskId,
    pub name: String,
    pub description: String,
    pub schedule: Schedule,
    pub action: TaskAction,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u32,
}

/// 调度配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schedule {
    #[serde(rename = "type")]
    pub schedule_type: ScheduleType,
    pub cron: Option<String>,
    pub interval_secs: Option<u64>,
    pub timezone: Option<String>,
}

/// 调度类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ScheduleType {
    Once,
    Cron,
    Interval,
}

/// 任务动作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TaskAction {
    /// 执行工具
    ExecuteTool {
        tool_id: ToolId,
        parameters: HashMap<String, serde_json::Value>,
    },
    /// 调用 HTTP 端点
    HttpCall {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: Option<String>,
    },
    /// 执行脚本
    Script {
        language: String,
        code: String,
    },
    /// 触发 Webhook
    TriggerWebhook {
        webhook_id: WebhookId,
        payload: serde_json::Value,
    },
}

/// Webhook 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: WebhookId,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: Vec<WebhookEvent>,
    pub headers: HashMap<String, String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
    pub trigger_count: u32,
}

/// Webhook 事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum WebhookEvent {
    /// 沙箱事件
    SandboxCreated,
    SandboxStarted,
    SandboxStopped,
    /// 画布事件
    CanvasCreated,
    CanvasUpdated,
    CanvasDeleted,
    /// 工具事件
    ToolExecuted,
    /// 任务事件
    TaskCompleted,
    TaskFailed,
    /// 系统事件
    SystemStarted,
    SystemStopped,
    /// 自定义事件
    Custom { name: String },
}

/// Webhook 触发记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTrigger {
    pub id: String,
    pub webhook_id: WebhookId,
    pub event: WebhookEvent,
    pub payload: serde_json::Value,
    pub status: TriggerStatus,
    pub response_code: Option<u16>,
    pub response_body: Option<String>,
    pub error: Option<String>,
    pub triggered_at: DateTime<Utc>,
    pub duration_ms: u64,
}

/// 触发状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TriggerStatus {
    Pending,
    Success,
    Failed,
    Retry,
}

/// 技能定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: Option<String>,
    pub category: SkillCategory,
    pub tools: Vec<ToolBinding>,
    pub triggers: Vec<SkillTrigger>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 技能分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SkillCategory {
    Productivity,
    Automation,
    Analysis,
    Communication,
    Development,
    Custom,
}

/// 工具绑定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBinding {
    pub tool_id: ToolId,
    pub alias: Option<String>,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// 技能触发器
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SkillTrigger {
    /// 命令触发
    Command { pattern: String },
    /// 关键词触发
    Keyword { keywords: Vec<String> },
    /// 定时触发
    Schedule { schedule: Schedule },
    /// Webhook 触发
    Webhook { webhook_id: WebhookId },
}

/// 技能执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecution {
    pub id: String,
    pub skill_id: SkillId,
    pub trigger: SkillTrigger,
    pub context: ToolContext,
    pub results: Vec<ToolResult>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: SkillExecutionStatus,
}

/// 技能执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SkillExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}
