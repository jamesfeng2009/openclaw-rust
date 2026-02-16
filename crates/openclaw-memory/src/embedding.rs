//! Embedding Provider Trait
//!
//! 定义独立的嵌入向量生成接口

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};

pub type Embedding = Vec<f32>;
pub type Embeddings = Vec<Embedding>;

#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, text: &str) -> Result<Embedding>;
    async fn embed_batch(&self, texts: &[String]) -> Result<Embeddings>;
    fn similarity(&self, a: &Embedding, b: &Embedding) -> f32 {
        dot_product(a, b) / (magnitude(a) * magnitude(b))
    }
}

pub fn dot_product(a: &Embedding, b: &Embedding) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

pub fn magnitude(v: &Embedding) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub fn euclidean_distance(a: &Embedding, b: &Embedding) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt()
}

pub struct OpenAIEmbedding {
    model: String,
    dimensions: usize,
    api_key: String,
    base_url: String,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String) -> Self {
        Self {
            model: "text-embedding-3-small".to_string(),
            dimensions: 1536,
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self.dimensions = match model {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536,
        };
        self
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAIEmbedding {
    fn name(&self) -> &str { "openai" }
    fn model(&self) -> &str { &self.model }
    fn dimensions(&self) -> usize { self.dimensions }

    async fn embed(&self, text: &str) -> Result<Embedding> {
        let embeddings = self.embed_batch(&[text.to_string()]).await?;
        Ok(embeddings.into_iter().next().unwrap_or_default())
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Embeddings> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(&format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "input": texts,
                "model": self.model,
            }))
            .send()
            .await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(error));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        let embeddings: Embeddings = json["data"]
            .as_array()
            .ok_or_else(|| OpenClawError::AIProvider("Invalid response format".to_string()))?
            .iter()
            .map(|item| {
                item["embedding"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|v| v.as_f64().unwrap_or(0.0) as f32)
                    .collect()
            })
            .collect();

        Ok(embeddings)
    }
}

pub struct OllamaEmbedding {
    model: String,
    base_url: String,
}

impl OllamaEmbedding {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
            base_url: "http://localhost:11434".to_string(),
        }
    }

    pub fn with_base_url(mut self, url: &str) -> Self {
        self.base_url = url.to_string();
        self
    }

    fn calc_dimensions(&self) -> usize {
        match self.model.as_str() {
            "nomic-embed-text" => 768,
            "mxbai-embed-large" => 1024,
            _ => 768,
        }
    }
}

#[async_trait]
impl EmbeddingProvider for OllamaEmbedding {
    fn name(&self) -> &str { "ollama" }
    fn model(&self) -> &str { &self.model }
    fn dimensions(&self) -> usize { self.calc_dimensions() }

    async fn embed(&self, text: &str) -> Result<Embedding> {
        let client = reqwest::Client::new();
        
        let response = client
            .post(&format!("{}/api/embeddings", self.base_url))
            .json(&serde_json::json!({
                "model": self.model,
                "prompt": text,
            }))
            .send()
            .await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(error));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(e.to_string()))?;

        let embedding: Embedding = json["embedding"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Embeddings> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }
}
