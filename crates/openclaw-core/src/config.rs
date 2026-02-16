//! 配置管理

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 主配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 服务配置
    pub server: ServerConfig,
    /// AI 提供商配置
    pub ai: AiConfig,
    /// 记忆配置
    pub memory: MemoryConfig,
    /// 向量存储配置
    pub vector: VectorConfig,
    /// 通道配置
    pub channels: ChannelsConfig,
    /// 智能体配置
    pub agents: AgentsConfig,
    /// 工作区配置
    #[serde(default)]
    pub workspaces: WorkspacesConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            ai: AiConfig::default(),
            memory: MemoryConfig::default(),
            vector: VectorConfig::default(),
            channels: ChannelsConfig::default(),
            agents: AgentsConfig::default(),
            workspaces: WorkspacesConfig::default(),
        }
    }
}

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    #[serde(default)]
    pub enable_agents: bool,
    #[serde(default)]
    pub enable_channels: bool,
    #[serde(default)]
    pub enable_voice: bool,
    #[serde(default)]
    pub enable_canvas: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 18789,
            log_level: "info".to_string(),
            enable_agents: false,
            enable_channels: false,
            enable_voice: false,
            enable_canvas: false,
        }
    }
}

/// AI 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// 默认提供商
    pub default_provider: String,
    /// 提供商列表
    pub providers: Vec<ProviderConfig>,
    /// Token 预算
    pub token_budget: TokenBudget,
    #[serde(default)]
    pub auth_profiles: Vec<AuthProfile>,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            providers: vec![],
            token_budget: TokenBudget::default(),
            auth_profiles: vec![],
        }
    }
}

/// 提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub name: String,
    pub provider_type: ProviderType,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: String,
    pub models: Vec<String>,
    #[serde(default)]
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Azure,
    DeepSeek,
    OpenRouter,
    Ollama,
    Qwen,
    Doubao,
    Glm,
    Minimax,
    Kimi,
    Custom,
}

/// 认证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthConfig {
    /// API 密钥认证
    ApiKey {
        key: String,
    },
    /// OAuth 认证
    OAuth {
        client_id: String,
        client_secret: String,
        refresh_token: Option<String>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        scopes: Vec<String>,
    },
    /// Azure AD 认证
    AzureAd {
        tenant_id: String,
        client_id: String,
        client_secret: String,
    },
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::ApiKey { key: String::new() }
    }
}

/// Auth Profile - 认证配置轮换
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub auth: AuthConfig,
    pub priority: u8,
    pub enabled: bool,
}

impl AuthProfile {
    pub fn is_expired(&self) -> bool {
        if let AuthConfig::OAuth { expires_at, .. } = &self.auth {
            if let Some(exp) = expires_at {
                return chrono::Utc::now() >= *exp;
            }
        }
        false
    }
}

/// Token 预算
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// 最大上下文 token
    pub max_context: usize,
    /// 最大响应 token
    pub max_response: usize,
    /// 警告阈值 (0.0 - 1.0)
    pub warning_threshold: f32,
    /// 是否自动压缩
    pub auto_compress: bool,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_context: 16000,
            max_response: 4096,
            warning_threshold: 0.8,
            auto_compress: true,
        }
    }
}

/// 记忆配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 工作记忆配置
    pub working: WorkingMemoryConfig,
    /// 短期记忆配置
    pub short_term: ShortTermMemoryConfig,
    /// 长期记忆配置
    pub long_term: LongTermMemoryConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            working: WorkingMemoryConfig::default(),
            short_term: ShortTermMemoryConfig::default(),
            long_term: LongTermMemoryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryConfig {
    /// 最大消息数
    pub max_messages: usize,
    /// 最大 token 数
    pub max_tokens: usize,
}

impl Default for WorkingMemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 20,
            max_tokens: 8000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryConfig {
    /// 压缩阈值 (消息数)
    pub compress_after: usize,
    /// 最大摘要数
    pub max_summaries: usize,
}

impl Default for ShortTermMemoryConfig {
    fn default() -> Self {
        Self {
            compress_after: 10,
            max_summaries: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryConfig {
    /// 是否启用
    pub enabled: bool,
    /// 向量存储后端
    pub backend: String,
    /// 嵌入模型
    pub embedding_model: String,
}

impl Default for LongTermMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "lancedb".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
        }
    }
}

/// 向量存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    /// 后端类型
    pub backend: VectorBackend,
    /// Qdrant 配置
    pub qdrant: Option<QdrantConfig>,
    /// LanceDB 配置
    pub lancedb: Option<LanceDbConfig>,
}

impl Default for VectorConfig {
    fn default() -> Self {
        Self {
            backend: VectorBackend::LanceDB,
            qdrant: None,
            lancedb: Some(LanceDbConfig::default()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VectorBackend {
    Qdrant,
    LanceDB,
    PgVector,
    SQLite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantConfig {
    pub url: String,
    pub collection: String,
    pub api_key: Option<String>,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:6333".to_string(),
            collection: "openclaw_memories".to_string(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceDbConfig {
    pub path: PathBuf,
}

impl Default for LanceDbConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data/lancedb"),
        }
    }
}

/// 通道配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    // 国内平台
    pub dingtalk: Option<DingTalkConfig>,
    pub wecom: Option<WeComConfig>,
    pub feishu: Option<FeishuConfig>,
    // 国际平台
    pub telegram: Option<TelegramConfig>,
    pub discord: Option<DiscordConfig>,
    pub whatsapp: Option<WhatsAppConfig>,
    pub slack: Option<SlackConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub bot_token: String,
    pub enabled: bool,
}

/// WhatsApp Cloud API 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// WhatsApp Business Account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_account_id: Option<String>,
    /// Phone Number ID
    pub phone_number_id: String,
    /// Access Token
    pub access_token: String,
    /// Webhook Verify Token (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_token: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 钉钉配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DingTalkConfig {
    /// Webhook 地址
    pub webhook: String,
    /// 加签密钥（可选）
    pub secret: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 企业微信配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeComConfig {
    /// Webhook 地址
    pub webhook: String,
    /// 是否启用
    pub enabled: bool,
}

/// 飞书配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    /// App ID
    pub app_id: String,
    /// App Secret
    pub app_secret: String,
    /// Webhook 地址（可选）
    pub webhook: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Discord 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    /// Bot Token
    pub bot_token: String,
    /// Webhook URL (可选)
    pub webhook_url: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Microsoft Teams 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamsConfig {
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// Bot ID (可选)
    pub bot_id: Option<String>,
    /// Bot Password (可选)
    pub bot_password: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// Slack 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Bot Token
    pub bot_token: Option<String>,
    /// Webhook URL
    pub webhook_url: Option<String>,
    /// App Token (可选)
    pub app_token: Option<String>,
    /// 是否启用
    pub enabled: bool,
}

/// 智能体配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsConfig {
    pub list: Vec<AgentConfig>,
    pub defaults: AgentDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub id: String,
    pub workspace: PathBuf,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub aieos_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefaults {
    pub workspace: PathBuf,
}

impl Default for AgentDefaults {
    fn default() -> Self {
        Self {
            workspace: PathBuf::from("~/.openclaw/workspace"),
        }
    }
}

/// 工作区配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspacesConfig {
    /// 工作区列表
    #[serde(default)]
    pub workspaces: Vec<WorkspaceConfig>,
    /// 默认工作区 ID
    #[serde(default)]
    pub default: Option<String>,
}

/// 单个工作区配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// 工作区 ID
    pub id: String,
    /// 工作区名称
    pub name: String,
    /// 工作区路径
    pub path: PathBuf,
    /// 关联的通道 (channel_id -> 配置)
    #[serde(default)]
    pub channels: HashMap<String, serde_json::Value>,
    /// 关联的智能体 IDs
    #[serde(default)]
    pub agent_ids: Vec<String>,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl WorkspaceConfig {
    pub fn new(id: impl Into<String>, name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            path: path.into(),
            channels: HashMap::new(),
            agent_ids: Vec::new(),
            enabled: true,
        }
    }
}

impl Config {
    /// 从文件加载配置
    pub fn from_file(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::OpenClawError::Config(format!("读取配置文件失败: {}", e)))?;
        
        let config: Config = serde_json::from_str(&content)
            .map_err(|e| crate::OpenClawError::Config(format!("解析配置文件失败: {}", e)))?;
        
        Ok(config)
    }

    /// 保存配置到文件
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::OpenClawError::Config(format!("序列化配置失败: {}", e)))?;
        
        std::fs::write(path, content)
            .map_err(|e| crate::OpenClawError::Config(format!("写入配置文件失败: {}", e)))?;
        
        Ok(())
    }
}
