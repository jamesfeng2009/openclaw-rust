//! 基础提供商实现 (占位符)

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::{OpenClawError, Result};
use std::pin::Pin;

use crate::providers::{AIProvider, ProviderConfig};
use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, StreamChunk};

/// 基础提供商 (用于测试和回退)
pub struct BaseProvider {
    config: ProviderConfig,
}

impl BaseProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl AIProvider for BaseProvider {
    fn name(&self) -> &str {
        &self.config.name
    }

    async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
        // 占位符实现
        Err(OpenClawError::AIProvider(
            "Base provider does not support chat. Please configure a real provider.".to_string(),
        ))
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(OpenClawError::AIProvider(
            "Base provider does not support streaming. Please configure a real provider."
                .to_string(),
        ))
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        Err(OpenClawError::AIProvider(
            "Base provider does not support embeddings. Please configure a real provider."
                .to_string(),
        ))
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(false)
    }
}
