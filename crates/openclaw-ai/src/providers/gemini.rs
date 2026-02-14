//! Google Gemini 提供商实现

use async_trait::async_trait;
use futures::Stream;
use openclaw_core::{Message, OpenClawError, Result, Role};
use reqwest::header;
use std::pin::Pin;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk, TokenUsage};
use crate::providers::{AIProvider, ProviderConfig};

/// Google Gemini 提供商
pub struct GeminiProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl GeminiProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://generativelanguage.googleapis.com/v1beta")
    }

    fn convert_messages(&self, messages: Vec<Message>) -> serde_json::Value {
        let contents: Vec<serde_json::Value> = messages.into_iter()
            .filter(|m| m.role != Role::System) // Gemini 单独处理 system
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "model",
                    _ => "user",
                };

                let content = m.text_content().unwrap_or("").to_string();

                serde_json::json!({
                    "role": role,
                    "parts": [{"text": content}]
                })
            })
            .collect();

        serde_json::json!(contents)
    }

    fn get_system_instruction(&self, messages: &[Message]) -> Option<String> {
        messages.iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text_content().map(|s| s.to_string()))
    }
}

#[async_trait]
impl AIProvider for GeminiProvider {
    fn name(&self) -> &str {
        "google"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.get_base_url(),
            request.model,
            self.config.api_key.as_deref().unwrap_or("")
        );

        let system_instruction = self.get_system_instruction(&request.messages);
        let contents = self.convert_messages(request.messages.clone());

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": request.temperature.unwrap_or(1.0),
                "maxOutputTokens": request.max_tokens.unwrap_or(8192),
            }
        });

        if let Some(system) = system_instruction {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": system}]
            });
        }

        let response = self.client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Gemini API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Gemini API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        let usage = TokenUsage::new(
            json["usageMetadata"]["promptTokenCount"].as_u64().unwrap_or(0) as usize,
            json["usageMetadata"]["candidatesTokenCount"].as_u64().unwrap_or(0) as usize,
        );

        let message = Message::assistant(&text);

        Ok(ChatResponse {
            id: uuid::Uuid::new_v4().to_string(),
            model: request.model,
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
            "Streaming not yet implemented for Gemini".to_string()
        ))
    }

    async fn embed(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let url = format!(
            "{}/models/{}:embedContent?key={}",
            self.get_base_url(),
            request.model,
            self.config.api_key.as_deref().unwrap_or("")
        );

        // Gemini 只支持单个文本嵌入
        let text = request.input.first().cloned().unwrap_or_default();

        let body = serde_json::json!({
            "model": request.model,
            "content": {
                "parts": [{"text": text}]
            }
        });

        let response = self.client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Gemini Embedding API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Gemini Embedding API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let embedding: Vec<f32> = json["embedding"]["values"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_f64()).map(|v| v as f32).collect())
            .unwrap_or_default();

        Ok(EmbeddingResponse {
            embeddings: vec![embedding],
            model: request.model,
            usage: TokenUsage::new(0, 0),
        })
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "gemini-2.0-flash".to_string(),
            "gemini-2.0-pro".to_string(),
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.config.api_key.is_some())
    }
}
