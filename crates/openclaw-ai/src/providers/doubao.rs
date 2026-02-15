//! Doubao 豆包提供商实现

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::Result;
use std::pin::Pin;

use super::openai_compatible::{OpenAICompatibleProvider, ProviderInfo};
use crate::providers::{AIProvider, ProviderConfig};
use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, StreamChunk};

/// Doubao 豆包提供商
pub struct DoubaoProvider(OpenAICompatibleProvider);

impl DoubaoProvider {
    /// 创建新的 Doubao 提供商
    pub fn new(config: ProviderConfig) -> Self {
        Self(OpenAICompatibleProvider::new(config, ProviderInfo {
            name: "doubao",
            default_base_url: "https://ark.cn-beijing.volces.com/api/v3",
            default_models: &[
                "doubao-pro-32k",
                "doubao-pro-4k",
                "doubao-lite-32k",
                "doubao-lite-4k",
            ],
        }))
    }
}

#[async_trait]
impl AIProvider for DoubaoProvider {
    fn name(&self) -> &str {
        self.0.name()
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        self.0.chat(request).await
    }

    async fn chat_stream(&self, request: ChatRequest) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        self.0.chat_stream(request).await
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        self.0.embed(request).await
    }

    async fn models(&self) -> Result<Vec<String>> {
        self.0.models().await
    }

    async fn health_check(&self) -> Result<bool> {
        self.0.health_check().await
    }
}
