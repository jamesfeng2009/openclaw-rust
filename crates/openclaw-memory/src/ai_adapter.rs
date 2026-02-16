use async_trait::async_trait;
use std::sync::Arc;
use openclaw_ai::{AIProvider, EmbeddingRequest};
use openclaw_core::{OpenClawError, Result};

use crate::embedding::{Embedding, Embeddings, EmbeddingProvider};

pub struct AIProviderEmbeddingAdapter {
    provider: Arc<dyn AIProvider>,
    model: String,
    dimensions: usize,
}

impl AIProviderEmbeddingAdapter {
    pub fn new(provider: Arc<dyn AIProvider>, model: String, dimensions: usize) -> Self {
        Self {
            provider,
            model,
            dimensions,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for AIProviderEmbeddingAdapter {
    fn name(&self) -> &str {
        "ai-provider-adapter"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    async fn embed(&self, text: &str) -> Result<Embedding> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Embeddings> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let response = self.provider.embed(request).await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        Ok(response.embeddings)
    }
}
