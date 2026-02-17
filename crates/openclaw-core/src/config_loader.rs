//! 统一配置加载器
//!
//! 整合所有配置源，提供统一的配置访问接口

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnifiedConfig {
    pub user: UserSection,
    pub providers: ProvidersSection,
    pub agents: AgentsSection,
    pub channels: ChannelsSection,
    pub features: FeaturesSection,
    pub security: SecuritySection,
    pub sandbox: SandboxSection,
    pub voice: VoiceSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserSection {
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub language: Option<String>,
    pub timezone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersSection {
    #[serde(flatten)]
    pub entries: HashMap<String, ProviderEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderEntry {
    WithKey {
        api_key: String,
        api_base: Option<String>,
    },
    NoKey {
        api_base: Option<String>,
    },
}

impl Default for ProviderEntry {
    fn default() -> Self {
        ProviderEntry::NoKey { api_base: None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentsSection {
    #[serde(default)]
    pub defaults: DefaultAgentConfig,
    #[serde(default)]
    pub agents: HashMap<String, AgentInstanceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultAgentConfig {
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<usize>,
}

fn default_model() -> String {
    "gpt-4o".to_string()
}
fn default_provider() -> String {
    "openai".to_string()
}

impl Default for DefaultAgentConfig {
    fn default() -> Self {
        Self {
            model: default_model(),
            provider: default_provider(),
            temperature: None,
            max_tokens: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentInstanceConfig {
    pub name: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsSection {
    #[serde(default)]
    pub telegram: ChannelConfig,
    #[serde(default)]
    pub discord: ChannelConfig,
    #[serde(default)]
    pub whatsapp: ChannelConfig,
    #[serde(default)]
    pub feishu: ChannelConfig,
    #[serde(default)]
    pub dingtalk: ChannelConfig,
    #[serde(default)]
    pub wecom: ChannelConfig,
    #[serde(default)]
    pub slack: ChannelConfig,
    #[serde(default)]
    pub email: ChannelConfig,
    #[serde(flatten)]
    pub custom: HashMap<String, ChannelConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub api_secret: Option<String>,
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub app_secret: Option<String>,
    #[serde(default)]
    pub allow_from: Option<Vec<String>>,
    #[serde(default)]
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeaturesSection {
    #[serde(default = "default_true")]
    pub voice: bool,
    #[serde(default = "default_true")]
    pub browser: bool,
    #[serde(default = "default_true")]
    pub canvas: bool,
    #[serde(default = "default_true")]
    pub cron: bool,
    #[serde(default = "default_true")]
    pub webhook: bool,
    #[serde(default = "default_true")]
    pub sandbox: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecuritySection {
    #[serde(default)]
    pub enable_input_filter: bool,
    #[serde(default)]
    pub enable_audit: bool,
    #[serde(default)]
    pub enable_self_healer: bool,
    #[serde(default)]
    pub risk_threshold: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SandboxSection {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub backend: String,
    #[serde(default)]
    pub memory_limit_mb: u64,
    #[serde(default)]
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceSection {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub stt_model: Option<String>,
    #[serde(default)]
    pub tts_model: Option<String>,
}

impl UnifiedConfig {
    pub fn load(path: &PathBuf) -> crate::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(path)
            .map_err(|e| crate::OpenClawError::Config(format!("读取配置失败: {}", e)))?;

        let config: Self = serde_json::from_str(&content)
            .map_err(|e| crate::OpenClawError::Config(format!("解析配置失败: {}", e)))?;

        Ok(config)
    }

    pub fn save(&self, path: &PathBuf) -> crate::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| crate::OpenClawError::Config(format!("创建目录失败: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::OpenClawError::Config(format!("序列化配置失败: {}", e)))?;

        fs::write(path, content)
            .map_err(|e| crate::OpenClawError::Config(format!("写入配置失败: {}", e)))?;

        Ok(())
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".openclaw")
            .join("config.json")
    }

    pub fn get_provider_config(&self, name: &str) -> Option<(String, Option<String>)> {
        self.providers.entries.get(name).map(|entry| match entry {
            ProviderEntry::WithKey { api_key, api_base } => (api_key.clone(), api_base.clone()),
            ProviderEntry::NoKey { api_base } => (String::new(), api_base.clone()),
        })
    }

    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        self.get_provider_config(provider).map(|(key, _)| key)
    }
}
