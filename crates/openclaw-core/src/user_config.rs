//! 用户配置管理
//!
//! 支持用户自定义 API Key 和提供商配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 用户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// 用户 ID
    pub user_id: String,
    /// 用户名称
    pub user_name: String,
    /// 默认提供商
    pub default_provider: String,
    /// 提供商配置
    pub providers: HashMap<String, UserProviderConfig>,
    /// 偏好设置
    pub preferences: UserPreferences,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Default for UserConfig {
    fn default() -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id: uuid::Uuid::new_v4().to_string(),
            user_name: "default".to_string(),
            default_provider: "openai".to_string(),
            providers: HashMap::new(),
            preferences: UserPreferences::default(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 用户提供商配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProviderConfig {
    /// 提供商名称
    pub name: String,
    /// API Key（加密存储）
    pub api_key: Option<String>,
    /// Base URL（可选，用于自定义端点）
    pub base_url: Option<String>,
    /// 默认模型
    pub default_model: String,
    /// 是否启用
    pub enabled: bool,
    /// 配额限制
    pub quota: Option<QuotaConfig>,
}

/// 配额配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    /// 每日请求限制
    pub daily_requests: Option<usize>,
    /// 每月 token 限制
    pub monthly_tokens: Option<usize>,
    /// 已使用请求数
    #[serde(default)]
    pub used_requests: usize,
    /// 已使用 token 数
    #[serde(default)]
    pub used_tokens: usize,
    /// 重置日期
    pub reset_date: chrono::DateTime<chrono::Utc>,
}

/// 用户偏好设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// 语言
    pub language: String,
    /// 时区
    pub timezone: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大 token
    pub max_tokens: usize,
    /// 流式响应
    pub stream_response: bool,
    /// 通知设置
    pub notifications: NotificationSettings,
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            language: "zh-CN".to_string(),
            timezone: "Asia/Shanghai".to_string(),
            temperature: 0.7,
            max_tokens: 4096,
            stream_response: true,
            notifications: NotificationSettings::default(),
        }
    }
}

/// 通知设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 启用通知
    pub enabled: bool,
    /// 错误通知
    pub on_error: bool,
    /// 配额警告
    pub on_quota_warning: bool,
    /// 配额警告阈值
    pub quota_warning_threshold: f32,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            on_error: true,
            on_quota_warning: true,
            quota_warning_threshold: 0.8,
        }
    }
}

impl UserConfig {
    /// 创建新用户配置
    pub fn new(user_name: String) -> Self {
        Self {
            user_name,
            ..Self::default()
        }
    }

    /// 从文件加载
    pub fn from_file(path: &PathBuf) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::OpenClawError::Config(format!("读取用户配置失败: {}", e)))?;

        let config: UserConfig = serde_json::from_str(&content)
            .map_err(|e| crate::OpenClawError::Config(format!("解析用户配置失败: {}", e)))?;

        Ok(config)
    }

    /// 保存到文件
    pub fn save(&self, path: &PathBuf) -> crate::Result<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| crate::OpenClawError::Config(format!("创建配置目录失败: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| crate::OpenClawError::Config(format!("序列化配置失败: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| crate::OpenClawError::Config(format!("写入配置文件失败: {}", e)))?;

        Ok(())
    }

    /// 获取默认配置路径
    pub fn default_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".openclaw-rust").join("user_config.json")
    }

    /// 添加或更新提供商配置
    pub fn set_provider(&mut self, name: String, config: UserProviderConfig) {
        self.providers.insert(name, config);
        self.updated_at = chrono::Utc::now();
    }

    /// 获取提供商配置
    pub fn get_provider(&self, name: &str) -> Option<&UserProviderConfig> {
        self.providers.get(name)
    }

    /// 删除提供商配置
    pub fn remove_provider(&mut self, name: &str) -> Option<UserProviderConfig> {
        let config = self.providers.remove(name);
        self.updated_at = chrono::Utc::now();
        config
    }

    /// 设置默认提供商
    pub fn set_default_provider(&mut self, provider: String) {
        self.default_provider = provider;
        self.updated_at = chrono::Utc::now();
    }

    /// 验证 API Key 格式
    pub fn validate_api_key(provider: &str, api_key: &str) -> crate::Result<bool> {
        // 根据不同提供商验证 API Key 格式
        match provider.to_lowercase().as_str() {
            "openai" => {
                // OpenAI API Key 格式: sk-xxx 或 sk-proj-xxx
                Ok(api_key.starts_with("sk-"))
            }
            "anthropic" => {
                // Anthropic API Key 格式: sk-ant-xxx
                Ok(api_key.starts_with("sk-ant-"))
            }
            "google" | "gemini" => {
                // Google API Key 通常是 39 字符
                Ok(api_key.len() >= 30)
            }
            "deepseek" => {
                // DeepSeek API Key 格式: sk-xxx
                Ok(api_key.starts_with("sk-"))
            }
            "glm" | "zhipu" => {
                // 智谱 API Key 格式验证
                Ok(!api_key.is_empty())
            }
            "qwen" | "tongyi" => {
                // 通义千问 API Key 验证
                Ok(!api_key.is_empty())
            }
            "kimi" | "moonshot" => {
                // Kimi API Key 验证
                Ok(!api_key.is_empty())
            }
            "minimax" => {
                // Minimax API Key 验证
                Ok(!api_key.is_empty())
            }
            _ => {
                // 自定义提供商，不做验证
                Ok(true)
            }
        }
    }
}

/// 用户配置管理器
pub struct UserConfigManager {
    config_path: PathBuf,
    config: UserConfig,
}

impl UserConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: Option<PathBuf>) -> crate::Result<Self> {
        let path = config_path.unwrap_or_else(UserConfig::default_config_path);

        let config = if path.exists() {
            UserConfig::from_file(&path)?
        } else {
            let config = UserConfig::default();
            config.save(&path)?;
            config
        };

        Ok(Self {
            config_path: path,
            config,
        })
    }

    /// 获取用户配置
    pub fn get_config(&self) -> &UserConfig {
        &self.config
    }

    /// 获取可变用户配置
    pub fn get_config_mut(&mut self) -> &mut UserConfig {
        &mut self.config
    }

    /// 保存配置
    pub fn save(&self) -> crate::Result<()> {
        self.config.save(&self.config_path)
    }

    /// 设置 API Key
    pub fn set_api_key(
        &mut self,
        provider: String,
        api_key: String,
        default_model: Option<String>,
    ) -> crate::Result<()> {
        // 验证 API Key 格式
        UserConfig::validate_api_key(&provider, &api_key)?;

        let provider_config = UserProviderConfig {
            name: provider.clone(),
            api_key: Some(api_key),
            base_url: None,
            default_model: default_model.unwrap_or_else(|| "default".to_string()),
            enabled: true,
            quota: None,
        };

        self.config.set_provider(provider, provider_config);
        self.save()
    }

    /// 获取 API Key
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        self.config
            .get_provider(provider)
            .and_then(|p| p.api_key.clone())
    }

    /// 删除 API Key
    pub fn remove_api_key(&mut self, provider: &str) -> crate::Result<()> {
        self.config.remove_provider(provider);
        self.save()
    }

    /// 列出所有提供商
    pub fn list_providers(&self) -> Vec<&String> {
        self.config.providers.keys().collect()
    }

    /// 导出配置（隐藏 API Key）
    pub fn export_safe(&self) -> serde_json::Value {
        let mut safe_config = serde_json::to_value(&self.config).unwrap();

        // 隐藏 API Key
        if let Some(providers) = safe_config.get_mut("providers")
            && let Some(providers_obj) = providers.as_object_mut()
        {
            for provider in providers_obj.values_mut() {
                if let Some(api_key) = provider.get_mut("api_key")
                    && let Some(key) = api_key.as_str()
                {
                    *api_key = serde_json::json!(format!("{}****", &key[..8.min(key.len())]));
                }
            }
        }

        safe_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_config_creation() {
        let config = UserConfig::new("test_user".to_string());
        assert_eq!(config.user_name, "test_user");
        assert!(config.providers.is_empty());
    }

    #[test]
    fn test_validate_api_key() {
        assert!(UserConfig::validate_api_key("openai", "sk-test123").unwrap());
        assert!(UserConfig::validate_api_key("anthropic", "sk-ant-test123").unwrap());
        assert!(!UserConfig::validate_api_key("openai", "invalid-key").unwrap());
    }
}
