//! AI 提供商实现
//!
//! 支持多种 AI 提供商:
//! - 国外: OpenAI, Anthropic, Google Gemini
//! - 国内: DeepSeek, Qwen, GLM, Minimax, Kimi

mod base;
mod openai;
mod openai_compatible;
mod anthropic;
mod gemini;
mod deepseek;
mod qwen;
mod glm;
mod minimax;
mod kimi;

pub use base::*;
pub use openai::*;
pub use openai_compatible::*;
pub use anthropic::*;
pub use gemini::*;
pub use deepseek::*;
pub use qwen::*;
pub use glm::*;
pub use minimax::*;
pub use kimi::*;

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
}

impl ProviderConfig {
    pub fn new(name: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            api_key: Some(api_key.into()),
            base_url: None,
            default_model: String::new(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }

    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }
}
