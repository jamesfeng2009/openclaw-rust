//! AI 提供商实现
//!
//! 支持多种 AI 提供商:
//! - 国外: OpenAI, Anthropic, Google Gemini, DeepSeek, OpenRouter, Ollama
//! - 国内: Qwen, Doubao, GLM, Minimax, Kimi
//! - 自定义: CustomProvider (用户自定义)

mod anthropic;
mod base;
mod custom;
mod deepseek;
mod doubao;
mod factory;
mod gemini;
mod glm;
mod kimi;
mod minimax;
mod ollama;
mod openai;
mod openai_compatible;
mod qwen;

pub use anthropic::*;
pub use base::*;
pub use custom::*;
pub use deepseek::*;
pub use doubao::*;
pub use factory::*;
pub use gemini::*;
pub use glm::*;
pub use kimi::*;
pub use minimax::*;
pub use ollama::*;
pub use openai::*;
pub use openai_compatible::*;
pub use qwen::*;

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::Result;
use std::pin::Pin;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, StreamChunk};

/// AI 提供商 Trait
#[async_trait]
pub trait AIProvider: Send + Sync {
    /// 提供商名称
    fn name(&self) -> &str;

    /// 发送聊天请求
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// 流式聊天请求
    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>>;

    /// 生成嵌入向量
    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse>;

    /// 获取可用模型列表
    async fn models(&self) -> Result<Vec<String>>;

    /// 检查健康状态
    async fn health_check(&self) -> Result<bool>;
}

/// 提供商配置
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub name: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub default_model: String,
    pub timeout: Option<std::time::Duration>,
    pub headers: std::collections::HashMap<String, String>,
    pub organization: Option<String>,
}

impl ProviderConfig {
    pub fn new(name: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            api_key: Some(api_key.into()),
            base_url: None,
            default_model: String::new(),
            timeout: None,
            headers: std::collections::HashMap::new(),
            organization: None,
        }
    }

    /// 设置自定义基础 URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// 设置默认模型
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// 设置请求超时
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// 添加自定义 Header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 设置 Organization (用于 OpenAI)
    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }
}
