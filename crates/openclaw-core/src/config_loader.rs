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
    #[serde(default)]
    pub agentic_rag: bool,
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
            .join(".openclaw-rust")
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

    pub fn to_config(&self) -> crate::config::Config {
        let providers: Vec<crate::config::ProviderConfig> = self
            .providers
            .entries
            .iter()
            .map(|(name, entry)| {
                let (api_key, api_base) = match entry {
                    ProviderEntry::WithKey { api_key, api_base } => {
                        (Some(api_key.clone()), api_base.clone())
                    }
                    ProviderEntry::NoKey { api_base } => {
                        (None, api_base.clone())
                    }
                };
                let provider_type = match name.as_str() {
                    "openai" => crate::config::ProviderType::OpenAI,
                    "anthropic" => crate::config::ProviderType::Anthropic,
                    "google" | "gemini" => crate::config::ProviderType::Google,
                    "azure" => crate::config::ProviderType::Azure,
                    "deepseek" => crate::config::ProviderType::DeepSeek,
                    "openrouter" => crate::config::ProviderType::OpenRouter,
                    "ollama" => crate::config::ProviderType::Ollama,
                    "qwen" => crate::config::ProviderType::Qwen,
                    "doubao" => crate::config::ProviderType::Doubao,
                    "glm" => crate::config::ProviderType::Glm,
                    _ => crate::config::ProviderType::OpenAI,
                };
                crate::config::ProviderConfig {
                    name: name.clone(),
                    provider_type,
                    api_key,
                    base_url: api_base,
                    default_model: self.agents.defaults.model.clone(),
                    models: vec![],
                    auth: crate::config::AuthConfig::default(),
                }
            })
            .collect();

        let ai_config = crate::config::AiConfig {
            default_provider: self.agents.defaults.provider.clone(),
            providers,
            token_budget: crate::config::TokenBudget::default(),
            auth_profiles: vec![],
            use_accurate_token_count: false,
        };

        let server_config = crate::config::ServerConfig {
            host: "0.0.0.0".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            enable_agents: true,
            enable_channels: self.channels.telegram.enabled 
                || self.channels.discord.enabled 
                || self.channels.whatsapp.enabled
                || self.channels.feishu.enabled
                || self.channels.dingtalk.enabled,
            enable_voice: self.voice.enabled,
            enable_canvas: false,
            enable_agentic_rag: self.features.agentic_rag,
        };

        let security_config = crate::config::SecurityConfig {
            enable_input_filter: self.security.enable_input_filter,
            enable_classifier: true,
            enable_output_validation: true,
            enable_audit: self.security.enable_audit,
            enable_self_healer: self.security.enable_self_healer,
            classifier_strict_mode: false,
            stuck_timeout: std::time::Duration::from_secs(300),
        };

        crate::config::Config {
            server: server_config,
            ai: ai_config,
            memory: crate::config::MemoryConfig::default(),
            vector: crate::config::VectorConfig::default(),
            channels: crate::config::ChannelsConfig::default(),
            security: security_config,
            agents: crate::config::AgentsConfig::default(),
            devices: crate::config::DevicesConfig::default(),
            workspaces: crate::config::WorkspacesConfig::default(),
            voice: None,
            browser: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_config_to_config() {
        let unified = UnifiedConfig {
            user: UserSection {
                user_id: Some("test_user".to_string()),
                user_name: Some("Test User".to_string()),
                language: Some("zh".to_string()),
                timezone: Some("Asia/Shanghai".to_string()),
            },
            providers: ProvidersSection {
                entries: [
                    ("openai".to_string(), ProviderEntry::WithKey {
                        api_key: "test-key".to_string(),
                        api_base: Some("https://api.openai.com".to_string()),
                    }),
                    ("anthropic".to_string(), ProviderEntry::NoKey {
                        api_base: Some("https://api.anthropic.com".to_string()),
                    }),
                ].into_iter().collect(),
            },
            agents: AgentsSection {
                defaults: DefaultAgentConfig {
                    model: "gpt-4".to_string(),
                    provider: "openai".to_string(),
                    temperature: Some(0.7),
                    max_tokens: Some(4096),
                },
                agents: HashMap::new(),
            },
            channels: ChannelsSection {
                telegram: ChannelConfig { enabled: true, token: Some("bot_token".to_string()), ..Default::default() },
                discord: ChannelConfig::default(),
                whatsapp: ChannelConfig::default(),
                feishu: ChannelConfig::default(),
                dingtalk: ChannelConfig::default(),
                wecom: ChannelConfig::default(),
                slack: ChannelConfig::default(),
                email: ChannelConfig::default(),
                custom: HashMap::new(),
            },
            features: FeaturesSection::default(),
            security: SecuritySection {
                enable_input_filter: true,
                enable_audit: true,
                enable_self_healer: false,
                risk_threshold: "medium".to_string(),
            },
            sandbox: SandboxSection::default(),
            voice: VoiceSection {
                enabled: true,
                provider: Some("azure".to_string()),
                stt_model: Some("whisper".to_string()),
                tts_model: Some("tts-1".to_string()),
            },
        };

        let config = unified.to_config();

        assert_eq!(config.server.enable_agents, true);
        assert_eq!(config.server.enable_channels, true);
        assert_eq!(config.server.enable_voice, true);
        
        assert_eq!(config.ai.default_provider, "openai");
        assert_eq!(config.ai.providers.len(), 2);
        
        let openai_provider = config.ai.providers.iter().find(|p| p.name == "openai").unwrap();
        assert_eq!(openai_provider.provider_type, crate::config::ProviderType::OpenAI);
        assert_eq!(openai_provider.api_key, Some("test-key".to_string()));
        assert_eq!(openai_provider.base_url, Some("https://api.openai.com".to_string()));
        assert_eq!(openai_provider.default_model, "gpt-4");
    }

    #[test]
    fn test_unified_config_to_config_empty() {
        let unified = UnifiedConfig::default();
        let config = unified.to_config();

        assert_eq!(config.server.enable_agents, true);
        assert_eq!(config.server.enable_channels, false);
        assert_eq!(config.server.enable_voice, false);
        assert_eq!(config.ai.providers.len(), 0);
    }

    #[test]
    fn test_unified_config_provider_entry() {
        let with_key = ProviderEntry::WithKey {
            api_key: "key123".to_string(),
            api_base: Some("https://api.example.com".to_string()),
        };
        let no_key = ProviderEntry::NoKey {
            api_base: Some("https://api.example.com".to_string()),
        };

        let (key, base) = match &with_key {
            ProviderEntry::WithKey { api_key, api_base } => (api_key.clone(), api_base.clone()),
            _ => panic!("expected WithKey"),
        };
        assert_eq!(key, "key123");
        assert_eq!(base, Some("https://api.example.com".to_string()));

        let (key, base) = match &no_key {
            ProviderEntry::NoKey { api_base } => (String::new(), api_base.clone()),
            _ => panic!("expected NoKey"),
        };
        assert_eq!(key, "");
        assert_eq!(base, Some("https://api.example.com".to_string()));
    }
}
