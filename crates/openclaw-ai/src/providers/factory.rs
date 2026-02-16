//! AI 提供商工厂 - 支持零厂商锁定

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::{AIProvider, ProviderConfig, openai_compatible::ProviderInfo};

/// 提供商类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    DeepSeek,
    Qwen,
    Doubao,
    Glm,
    Minimax,
    Kimi,
    OpenRouter,
    Ollama,
    Custom,
}

impl ProviderType {
    /// 从字符串解析提供商类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" | "claude" => Some(Self::Anthropic),
            "gemini" | "google" => Some(Self::Gemini),
            "deepseek" => Some(Self::DeepSeek),
            "qwen" | "alibaba" => Some(Self::Qwen),
            "doubao" | "bytedance" => Some(Self::Doubao),
            "glm" | "zhipu" => Some(Self::Glm),
            "minimax" => Some(Self::Minimax),
            "kimi" | "moonshot" => Some(Self::Kimi),
            "openrouter" => Some(Self::OpenRouter),
            "ollama" | "local" => Some(Self::Ollama),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// 获取默认模型
    pub fn default_model(&self) -> &'static str {
        match self {
            Self::OpenAI => "gpt-4o",
            Self::Anthropic => "claude-4-sonnet-20241022",
            Self::Gemini => "gemini-2.0-flash-exp",
            Self::DeepSeek => "deepseek-chat",
            Self::Qwen => "qwen-plus",
            Self::Doubao => "doubao-pro-32k",
            Self::Glm => "glm-4-plus",
            Self::Minimax => "abab6.5s-chat",
            Self::Kimi => "moonshot-v1-8k",
            Self::OpenRouter => "openai/gpt-4o",
            Self::Ollama => "llama3.1",
            Self::Custom => "gpt-4o",
        }
    }
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Qwen => "qwen",
            Self::Doubao => "doubao",
            Self::Glm => "glm",
            Self::Minimax => "minimax",
            Self::Kimi => "kimi",
            Self::OpenRouter => "openrouter",
            Self::Ollama => "ollama",
            Self::Custom => "custom",
        };
        write!(f, "{}", name)
    }
}

/// 提供商工厂
pub struct ProviderFactory;

impl ProviderFactory {
    /// 根据配置创建提供商实例 (返回 Arc)
    #[allow(clippy::result_large_err)]
    pub fn create(
        provider_type: ProviderType,
        config: ProviderConfig,
    ) -> Result<Arc<dyn AIProvider>, String> {
        use super::*;

        match provider_type {
            ProviderType::OpenAI => {
                Ok(Arc::new(OpenAIProvider::new(config)))
            }
            ProviderType::Anthropic => {
                Ok(Arc::new(AnthropicProvider::new(config)))
            }
            ProviderType::Gemini => {
                Ok(Arc::new(GeminiProvider::new(config)))
            }
            ProviderType::DeepSeek => {
                Ok(Arc::new(DeepSeekProvider::new(config)))
            }
            ProviderType::Qwen => {
                Ok(Arc::new(QwenProvider::new(config)))
            }
            ProviderType::Doubao => {
                Ok(Arc::new(DoubaoProvider::new(config)))
            }
            ProviderType::Glm => {
                Ok(Arc::new(GlmProvider::new(config)))
            }
            ProviderType::Minimax => {
                Ok(Arc::new(MinimaxProvider::new(config)))
            }
            ProviderType::Kimi => {
                Ok(Arc::new(KimiProvider::new(config)))
            }
            ProviderType::OpenRouter => {
                let info = openai_compatible::ProviderInfo {
                    name: "openrouter",
                    default_base_url: "https://openrouter.ai/api/v1",
                    default_models: &[
                        "openai/gpt-4o",
                        "openai/gpt-4o-mini",
                        "anthropic/claude-3.5-sonnet",
                        "meta-llama/llama-3.1-70b-instruct",
                        "mistralai/mistral-7b-instruct",
                    ],
                };
                Ok(Arc::new(OpenAICompatibleProvider::new(config, info)))
            }
            ProviderType::Ollama => {
                Ok(Arc::new(OllamaProvider::new(config)))
            }
            ProviderType::Custom => {
                let base_url = config.base_url.clone()
                    .unwrap_or_else(|| "https://api.example.com/v1".to_string());
                let api_key = config.api_key.clone()
                    .unwrap_or_else(|| "dummy".to_string());
                Ok(Arc::new(CustomProvider::new(
                    "custom",
                    base_url,
                    api_key,
                )))
            }
        }
    }

    /// 从提供商名称字符串创建提供商 (返回 Arc)
    #[allow(clippy::result_large_err)]
    pub fn create_from_name(
        name: &str,
        api_key: Option<String>,
        base_url: Option<String>,
    ) -> Result<Arc<dyn AIProvider>, String> {
        let provider_type = ProviderType::from_str(name)
            .ok_or_else(|| format!("Unknown provider: {}", name))?;

        let mut config = ProviderConfig::new(
            name,
            api_key.unwrap_or_else(|| "dummy".to_string()),
        );

        if let Some(url) = base_url {
            config = config.with_base_url(url);
        }

        config = config.with_default_model(provider_type.default_model());

        Self::create(provider_type, config)
    }

    /// 获取所有支持的提供商列表
    pub fn supported_providers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("openai", "OpenAI (GPT-4, GPT-4o)"),
            ("anthropic", "Anthropic (Claude)"),
            ("gemini", "Google Gemini"),
            ("deepseek", "DeepSeek"),
            ("qwen", "Alibaba Qwen (通义千问)"),
            ("doubao", "ByteDance Doubao (豆包)"),
            ("glm", "Zhipu GLM (智谱)"),
            ("minimax", "MiniMax"),
            ("kimi", "Moonshot Kimi (月之暗面)"),
            ("openrouter", "OpenRouter (100+ models)"),
            ("ollama", "Ollama (Local models)"),
            ("custom", "Custom (user-defined)"),
        ]
    }
}

/// 默认提供商映射表
pub fn default_provider_info() -> HashMap<&'static str, ProviderInfo> {
    use super::openai_compatible::ProviderInfo;

    let mut map = HashMap::new();

    map.insert("openai", ProviderInfo {
        name: "openai",
        default_base_url: "https://api.openai.com/v1",
        default_models: &["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "o1", "o1-mini"],
    });

    map.insert("anthropic", ProviderInfo {
        name: "anthropic",
        default_base_url: "https://api.anthropic.com/v1",
        default_models: &["claude-4-opus", "claude-4-sonnet", "claude-3.5-sonnet"],
    });

    map.insert("deepseek", ProviderInfo {
        name: "deepseek",
        default_base_url: "https://api.deepseek.com/v1",
        default_models: &["deepseek-chat", "deepseek-coder"],
    });

    map.insert("qwen", ProviderInfo {
        name: "qwen",
        default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        default_models: &["qwen-plus", "qwen-turbo", "qwen-max"],
    });

    map.insert("openrouter", ProviderInfo {
        name: "openrouter",
        default_base_url: "https://openrouter.ai/api/v1",
        default_models: &[
            "openai/gpt-4o",
            "openai/gpt-4o-mini",
            "anthropic/claude-3.5-sonnet",
        ],
    });

    map.insert("ollama", ProviderInfo {
        name: "ollama",
        default_base_url: "http://localhost:11434",
        default_models: &["llama3.1", "mistral", "codellama"],
    });

    map
}
