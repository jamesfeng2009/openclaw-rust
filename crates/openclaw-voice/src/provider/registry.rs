//! 提供商注册表
//! 支持用户通过配置文件添加自定义提供商

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::{CustomSttConfig, CustomSttProvider, CustomTtsConfig, CustomTtsProvider};

/// 自定义提供商配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[derive(Default)]
pub struct CustomProviderConfig {
    /// 自定义 TTS 提供商配置
    pub custom_tts: Vec<CustomTtsConfig>,
    /// 自定义 STT 提供商配置
    pub custom_stt: Vec<CustomSttConfig>,
}


/// TTS 自定义提供商存储
pub struct TtsRegistry {
    configs: Arc<RwLock<HashMap<String, CustomTtsConfig>>>,
}

impl TtsRegistry {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, config: CustomTtsConfig) {
        let mut configs = self.configs.write().await;
        configs.insert(config.name.clone(), config);
    }

    pub async fn get(&self, name: &str) -> Option<CustomTtsConfig> {
        let configs = self.configs.read().await;
        configs.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<String> {
        let configs = self.configs.read().await;
        configs.keys().cloned().collect()
    }
}

impl Default for TtsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// STT 自定义提供商存储
pub struct SttRegistry {
    configs: Arc<RwLock<HashMap<String, CustomSttConfig>>>,
}

impl SttRegistry {
    pub fn new() -> Self {
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register(&self, config: CustomSttConfig) {
        let mut configs = self.configs.write().await;
        configs.insert(config.name.clone(), config);
    }

    pub async fn get(&self, name: &str) -> Option<CustomSttConfig> {
        let configs = self.configs.read().await;
        configs.get(name).cloned()
    }

    pub async fn list(&self) -> Vec<String> {
        let configs = self.configs.read().await;
        configs.keys().cloned().collect()
    }
}

impl Default for SttRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 提供商注册表 - 用于管理自定义提供商
/// 内置提供商通过 create_tts/create_stt 函数直接使用
pub struct ProviderRegistry {
    pub tts: TtsRegistry,
    pub stt: SttRegistry,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            tts: TtsRegistry::new(),
            stt: SttRegistry::new(),
        }
    }

    pub async fn register_custom_tts(&self, config: CustomTtsConfig) {
        self.tts.register(config).await;
    }

    pub async fn register_custom_stt(&self, config: CustomSttConfig) {
        self.stt.register(config).await;
    }

    pub async fn load_from_config(&self, config: &CustomProviderConfig) {
        for tts_config in &config.custom_tts {
            self.register_custom_tts(tts_config.clone()).await;
        }
        for stt_config in &config.custom_stt {
            self.register_custom_stt(stt_config.clone()).await;
        }
    }

    pub async fn create_custom_tts(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::tts::TextToSpeech + Send + Sync>> {
        if let Some(config) = self.tts.get(name).await {
            Some(Box::new(CustomTtsProvider::new(config)))
        } else {
            None
        }
    }

    pub async fn create_custom_stt(
        &self,
        name: &str,
    ) -> Option<Box<dyn crate::stt::SpeechToText + Send + Sync>> {
        if let Some(config) = self.stt.get(name).await {
            Some(Box::new(CustomSttProvider::new(config)))
        } else {
            None
        }
    }

    pub async fn list_tts_providers(&self) -> Vec<String> {
        let mut names = vec![
            "openai".to_string(),
            "edge".to_string(),
            "azure".to_string(),
            "google".to_string(),
            "elevenlabs".to_string(),
        ];

        let custom = self.tts.list().await;
        names.extend(custom);

        names
    }

    pub async fn list_stt_providers(&self) -> Vec<String> {
        let mut names = vec![
            "openai".to_string(),
            "local".to_string(),
            "azure".to_string(),
            "google".to_string(),
        ];

        let custom = self.stt.list().await;
        names.extend(custom);

        names
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::CustomResponseType;
    use crate::provider::CustomSttResponseType;

    #[tokio::test]
    async fn test_provider_registry_new() {
        let registry = ProviderRegistry::new();

        let tts_providers = registry.list_tts_providers().await;
        assert!(tts_providers.contains(&"openai".to_string()));
        assert!(tts_providers.contains(&"azure".to_string()));

        let stt_providers = registry.list_stt_providers().await;
        assert!(stt_providers.contains(&"openai".to_string()));
        assert!(stt_providers.contains(&"azure".to_string()));
    }

    #[tokio::test]
    async fn test_register_custom_tts() {
        let registry = ProviderRegistry::new();

        let config = CustomTtsConfig {
            name: "my_custom_tts".to_string(),
            endpoint: "https://my-api.com/tts".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_template: r#"{"text": "{{text}}"}"#.to_string(),
            response_type: CustomResponseType::Binary,
        };

        registry.register_custom_tts(config).await;

        let providers = registry.list_tts_providers().await;
        assert!(providers.contains(&"my_custom_tts".to_string()));
    }

    #[tokio::test]
    async fn test_register_custom_stt() {
        let registry = ProviderRegistry::new();

        let config = CustomSttConfig {
            name: "my_custom_stt".to_string(),
            endpoint: "https://my-api.com/stt".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            audio_field: "audio".to_string(),
            response_type: CustomSttResponseType::Text,
        };

        registry.register_custom_stt(config).await;

        let providers = registry.list_stt_providers().await;
        assert!(providers.contains(&"my_custom_stt".to_string()));
    }

    #[tokio::test]
    async fn test_load_from_config() {
        let config = CustomProviderConfig {
            custom_tts: vec![
                CustomTtsConfig {
                    name: "custom1".to_string(),
                    endpoint: "https://api1.com/tts".to_string(),
                    method: "POST".to_string(),
                    headers: HashMap::new(),
                    request_template: "{}".to_string(),
                    response_type: CustomResponseType::Binary,
                },
                CustomTtsConfig {
                    name: "custom2".to_string(),
                    endpoint: "https://api2.com/tts".to_string(),
                    method: "POST".to_string(),
                    headers: HashMap::new(),
                    request_template: "{}".to_string(),
                    response_type: CustomResponseType::Binary,
                },
            ],
            custom_stt: vec![CustomSttConfig {
                name: "custom_stt1".to_string(),
                endpoint: "https://api1.com/stt".to_string(),
                method: "POST".to_string(),
                headers: HashMap::new(),
                audio_field: "audio".to_string(),
                response_type: CustomSttResponseType::Text,
            }],
        };

        let registry = ProviderRegistry::new();
        registry.load_from_config(&config).await;

        let tts_providers = registry.list_tts_providers().await;
        assert!(tts_providers.contains(&"custom1".to_string()));
        assert!(tts_providers.contains(&"custom2".to_string()));

        let stt_providers = registry.list_stt_providers().await;
        assert!(stt_providers.contains(&"custom_stt1".to_string()));
    }

    #[tokio::test]
    async fn test_create_custom_tts() {
        let registry = ProviderRegistry::new();

        let config = CustomTtsConfig {
            name: "create_test".to_string(),
            endpoint: "https://test.com/tts".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            request_template: r#"{"text": "{{text}}"}"#.to_string(),
            response_type: CustomResponseType::Binary,
        };

        registry.register_custom_tts(config).await;

        let provider = registry.create_custom_tts("create_test").await;
        assert!(provider.is_some());

        let not_found = registry.create_custom_tts("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_create_custom_stt() {
        let registry = ProviderRegistry::new();

        let config = CustomSttConfig {
            name: "create_test_stt".to_string(),
            endpoint: "https://test.com/stt".to_string(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            audio_field: "audio".to_string(),
            response_type: CustomSttResponseType::Text,
        };

        registry.register_custom_stt(config).await;

        let provider = registry.create_custom_stt("create_test_stt").await;
        assert!(provider.is_some());

        let not_found = registry.create_custom_stt("nonexistent").await;
        assert!(not_found.is_none());
    }

    #[test]
    fn test_custom_provider_config_default() {
        let config = CustomProviderConfig::default();

        assert!(config.custom_tts.is_empty());
        assert!(config.custom_stt.is_empty());
    }
}
