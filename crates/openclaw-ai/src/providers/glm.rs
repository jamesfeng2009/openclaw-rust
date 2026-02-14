//! GLM 智谱提供商实现

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::{Message, OpenClawError, Result, Role};
use reqwest::header;
use std::pin::Pin;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk, TokenUsage};
use crate::providers::{AIProvider, ProviderConfig};

/// GLM 智谱提供商
pub struct GlmProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GlmProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://open.bigmodel.cn/api/paas/v4")
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages.into_iter().map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            let content = m.text_content().unwrap_or("").to_string();

            serde_json::json!({
                "role": role,
                "content": content
            })
        }).collect()
    }
}

#[async_trait]
impl AIProvider for GlmProvider {
    fn name(&self) -> &str {
        "glm"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.get_base_url());

        let body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(request.messages),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")))
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("GLM API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("GLM API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let choice = &json["choices"][0];
        let message_content = choice["message"]["content"].as_str().unwrap_or("").to_string();
        
        let usage = TokenUsage::new(
            json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            json["usage"]["completion_tokens"].as_u64().unwrap_or(0) as usize,
        );

        let message = Message::assistant(&message_content);

        Ok(ChatResponse {
            id: json["id"].as_str().unwrap_or("").to_string(),
            model: json["model"].as_str().unwrap_or("").to_string(),
            message,
            usage,
            finish_reason: FinishReason::Stop,
        })
    }

    async fn chat_stream(
        &self,
        _request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        Err(OpenClawError::AIProvider(
            "Streaming not yet implemented for GLM".to_string()
        ))
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let url = format!("{}/embeddings", self.get_base_url());

        let body = serde_json::json!({
            "model": request.model,
            "input": request.input
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("GLM Embedding API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("GLM Embedding API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let embeddings: Vec<Vec<f32>> = json["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item["embedding"].as_array())
                    .map(|emb| emb.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect())
                    .collect()
            })
            .unwrap_or_default();

        let usage = TokenUsage::new(
            json["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as usize,
            0,
        );

        Ok(EmbeddingResponse {
            embeddings,
            model: json["model"].as_str().unwrap_or("").to_string(),
            usage,
        })
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "glm-4-plus".to_string(),
            "glm-4-air".to_string(),
            "glm-4-flash".to_string(),
            "glm-z1-air".to_string(),
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.api_key.is_some())
    }
}
