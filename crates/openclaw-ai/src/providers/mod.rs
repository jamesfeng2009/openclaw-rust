//! AI 提供商实现

mod base;
mod openai;

pub use base::*;
pub use openai::*;

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::{Message, Result};
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
