//! Custom Provider - 用户自定义 AI 提供商
//!
//! 允许用户通过配置创建自己的 AI 提供商，无需编写代码

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::Result;
use std::pin::Pin;

use super::openai_compatible::{OpenAICompatibleProvider, ProviderInfo};
use crate::providers::{AIProvider, ProviderConfig};
use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, StreamChunk};

/// Custom Provider - 用户自定义 AI 提供商
pub struct CustomProvider {
    inner: OpenAICompatibleProvider,
}

impl CustomProvider {
    /// 创建新的 Custom Provider
    ///
    /// # Arguments
    /// * `name` - 提供商名称
    /// * `base_url` - API 基础 URL
    /// * `api_key` - API 密钥
    ///
    /// # Example
    /// ```rust
    /// use openclaw_ai::providers::CustomProvider;
    ///
    /// let provider = CustomProvider::new(
    ///     "my-provider",
    ///     "https://api.example.com/v1",
    ///     "sk-xxxx",
    /// );
    /// ```
    pub fn new(
        name: impl Into<String>,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let config = ProviderConfig::new(name, api_key).with_base_url(base_url);
        let inner = OpenAICompatibleProvider::new(
            config,
            ProviderInfo {
                name: "custom",
                default_base_url: "",
                default_models: &[],
            },
        );
        Self { inner }
    }

    /// 从配置创建 Custom Provider
    pub fn from_config(config: ProviderConfig) -> Self {
        let inner = OpenAICompatibleProvider::new(
            config,
            ProviderInfo {
                name: "custom",
                default_base_url: "",
                default_models: &[],
            },
        );
        Self { inner }
    }

    /// 设置默认模型
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        let config = self.inner.config().with_default_model(model);
        self.inner = OpenAICompatibleProvider::new(
            config,
            ProviderInfo {
                name: "custom",
                default_base_url: "",
                default_models: &[],
            },
        );
        self
    }
}

#[async_trait]
impl AIProvider for CustomProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.inner.chat(request).await
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        self.inner.chat_stream(request).await
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        self.inner.embed(request).await
    }

    async fn models(&self) -> Result<Vec<String>> {
        self.inner.models().await
    }

    async fn health_check(&self) -> Result<bool> {
        self.inner.health_check().await
    }
}

/// Builder 模式创建 Custom Provider
pub struct CustomProviderBuilder {
    name: String,
    base_url: Option<String>,
    api_key: Option<String>,
    default_model: Option<String>,
    models: Vec<String>,
}

impl CustomProviderBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            base_url: None,
            api_key: None,
            default_model: None,
            models: Vec::new(),
        }
    }

    /// 设置 API 基础 URL
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// 设置 API 密钥
    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// 设置默认模型
    pub fn default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = Some(model.into());
        self
    }

    /// 添加可用模型
    pub fn add_model(mut self, model: impl Into<String>) -> Self {
        self.models.push(model.into());
        self
    }

    /// 设置多个可用模型
    pub fn models(mut self, models: Vec<String>) -> Self {
        self.models = models;
        self
    }

    /// 构建 Custom Provider
    pub fn build(self) -> Result<CustomProvider> {
        let base_url = self
            .base_url
            .ok_or_else(|| openclaw_core::OpenClawError::Config("base_url is required".into()))?;

        let api_key = self
            .api_key
            .ok_or_else(|| openclaw_core::OpenClawError::Config("api_key is required".into()))?;

        let mut provider = CustomProvider::new(&self.name, base_url, api_key);

        if let Some(model) = self.default_model {
            provider = provider.with_default_model(model);
        }

        Ok(provider)
    }
}
