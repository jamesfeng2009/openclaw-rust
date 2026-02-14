//! Anthropic Claude 提供商实现

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use openclaw_core::{Message, OpenClawError, Result, Role};
use reqwest::header;
use std::pin::Pin;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk, TokenUsage};
use crate::providers::{AIProvider, ProviderConfig};

/// Anthropic 提供商
pub struct AnthropicProvider {
    config: ProviderConfig,
    client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(config: ProviderConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    fn get_base_url(&self) -> &str {
        self.config.base_url.as_deref().unwrap_or("https://api.anthropic.com/v1")
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages.into_iter().map(|m| {
            let role = match m.role {
                Role::System => "system",  // Anthropic 使用 system
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "user", // Anthropic 没有 tool role
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
impl AIProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/messages", self.get_base_url());

        // 分离系统消息和用户消息
        let system_message: Option<String> = request.messages.iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text_content().map(|s| s.to_string()));

        let other_messages: Vec<Message> = request.messages.into_iter()
            .filter(|m| m.role != Role::System)
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(other_messages),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(system) = system_message {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let api_key = self.config.api_key.as_deref().unwrap_or("");

        let response = self.client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Anthropic API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("Anthropic API 错误: {}", error_text)));
        }

        let json: serde_json::Value = response.json().await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        // 解析 Anthropic 响应格式
        let content = json["content"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|c| c["text"].as_str())
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage::new(
            json["usage"]["input_tokens"].as_u64().unwrap_or(0) as usize,
            json["usage"]["output_tokens"].as_u64().unwrap_or(0) as usize,
        );

        let message = Message::assistant(&content);

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
        // TODO: 实现 SSE 流式响应
        Err(OpenClawError::AIProvider(
            "Streaming not yet implemented for Anthropic".to_string()
        ))
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Anthropic 目前不提供 embedding API
        Err(OpenClawError::AIProvider(
            "Anthropic does not provide embedding API".to_string()
        ))
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        // Anthropic 没有专门的 health check 端点
        // 检查 API key 是否配置
        Ok(self.config.api_key.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_creation() {
        let config = ProviderConfig {
            name: "anthropic".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: None,
            default_model: "claude-3-sonnet".to_string(),
        };
        let provider = AnthropicProvider::new(config);
        assert_eq!(provider.name(), "anthropic");
    }
}
