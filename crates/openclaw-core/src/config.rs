//! 配置管理

use serde::{Deserialize, Serialize};
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
        }
    }
}

/// 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 18789,
            log_level: "info".to_string(),
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
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            providers: vec![],
            token_budget: TokenBudget::default(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Azure,
    DeepSeek,
    Ollama,
    Custom,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppConfig {
    /// 桥接服务地址
    pub bridge_url: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub bot_token: String,
    pub app_token: String,
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
