//! OpenAI 提供商实现

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use openclaw_core::{Content, Message, OpenClawError, Result, Role};
use std::pin::Pin;
use std::sync::Arc;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk, StreamDelta, TokenUsage};
use crate::providers::{AIProvider, ProviderConfig};

/// OpenAI 提供商
pub struct OpenAIProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl OpenAIProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://api.openai.com/v1")
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
impl AIProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.get_base_url());

        let body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(request.messages),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": false
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")))
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("OpenAI API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("OpenAI API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        // 解析响应
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
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        // TODO: 实现 SSE 流式响应
        // 目前返回错误
        Err(OpenClawError::AIProvider(
            "Streaming not yet implemented".to_string()
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
            .map_err(|e| OpenClawError::Http(format!("OpenAI Embedding API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("OpenAI Embedding API 错误: {}", error_text)));
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
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-3.5-turbo".to_string(),
            "text-embedding-3-small".to_string(),
            "text-embedding-3-large".to_string(),
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/models", self.get_base_url());
        
        let response = self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")))
            .send()
            .await;

        Ok(response.map(|r| r.status().is_success()).unwrap_or(false))
    }
}
