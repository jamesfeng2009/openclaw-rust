//! Anthropic Claude 提供商实现

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use openclaw_core::{Message, OpenClawError, Result, Role};
use reqwest::{Response, header};
use std::pin::Pin;

use crate::providers::{AIProvider, ProviderConfig};
use crate::types::{
    ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk,
    StreamDelta, TokenUsage,
};

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
        self.config
            .base_url
            .as_deref()
            .unwrap_or("https://api.anthropic.com/v1")
    }

    fn convert_messages(&self, messages: Vec<Message>) -> Vec<serde_json::Value> {
        messages
            .into_iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system", // Anthropic 使用 system
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "user", // Anthropic 没有 tool role
                };

                let content = m.text_content().unwrap_or("").to_string();

                serde_json::json!({
                    "role": role,
                    "content": content
                })
            })
            .collect()
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
        let system_message: Option<String> = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text_content().map(|s| s.to_string()));

        let other_messages: Vec<Message> = request
            .messages
            .into_iter()
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

        let response = self
            .client
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
            return Err(OpenClawError::AIProvider(format!(
                "Anthropic API 错误: {}",
                error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        // 解析 Anthropic 响应格式
        let content = json["content"]
            .as_array()
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
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!("{}/messages", self.get_base_url());

        // 分离系统消息和用户消息
        let system_message: Option<String> = request
            .messages
            .iter()
            .find(|m| m.role == Role::System)
            .and_then(|m| m.text_content().map(|s| s.to_string()));

        let other_messages: Vec<Message> = request
            .messages
            .into_iter()
            .filter(|m| m.role != Role::System)
            .collect();

        let mut body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(other_messages),
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "stream": true,
        });

        if let Some(system) = system_message {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        let api_key = self.config.api_key.as_deref().unwrap_or("");

        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Anthropic Stream API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Anthropic Stream API 错误: {}",
                error_text
            )));
        }

        // 创建 Anthropic SSE 流
        let stream = Self::parse_anthropic_sse_stream(response);

        Ok(Box::pin(stream))
    }

    async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        // Anthropic 目前不提供 embedding API
        Err(OpenClawError::AIProvider(
            "Anthropic does not provide embedding API".to_string(),
        ))
    }

    async fn models(&self) -> Result<Vec<String>> {
        Ok(vec![
            // Claude 4 系列 (最新)
            "claude-4-opus".to_string(),
            "claude-4-sonnet".to_string(),
            // Claude 3.7 系列
            "claude-3-7-sonnet".to_string(),
            // Claude 3.5 系列
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
        ])
    }

    async fn health_check(&self) -> Result<bool> {
        // Anthropic 没有专门的 health check 端点
        // 检查 API key 是否配置
        Ok(self.config.api_key.is_some())
    }
}

impl AnthropicProvider {
    /// 解析 Anthropic SSE 流
    fn parse_anthropic_sse_stream(
        response: Response,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        async_stream::stream! {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut message_id = String::new();
            let mut model = String::new();

            while let Some(bytes_result) = byte_stream.next().await {
                match bytes_result {
                    Ok(bytes) => {
                        // 将字节转换为字符串
                        if let Ok(text) = std::str::from_utf8(&bytes) {
                            buffer.push_str(text);
                        }

                        // 处理缓冲区中的完整事件
                        while let Some(event_end) = buffer.find("\n\n") {
                            let event = buffer[..event_end].to_string();
                            buffer = buffer[event_end + 2..].to_string();

                            // 解析 SSE 事件并 yield 结果
                            if let Some(result) = Self::parse_anthropic_sse_event(&event, &mut message_id, &mut model) {
                                yield result;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(OpenClawError::Http(format!("Anthropic 流读取错误: {}", e)));
                        return;
                    }
                }
            }
        }
    }

    /// 解析单个 Anthropic SSE 事件
    fn parse_anthropic_sse_event(
        event: &str,
        message_id: &mut String,
        model: &mut String,
    ) -> Option<Result<StreamChunk>> {
        let mut event_type = None;
        let mut data = None;

        for line in event.lines() {
            if let Some(et) = line.strip_prefix("event: ") {
                event_type = Some(et.to_string());
            } else if let Some(d) = line.strip_prefix("data: ") {
                data = Some(d.to_string());
            }
        }

        let event_type = event_type?;
        let data = data?;

        // 解析 JSON
        let json: serde_json::Value = match serde_json::from_str(&data) {
            Ok(j) => j,
            Err(e) => return Some(Err(OpenClawError::Serialization(e))),
        };

        // 处理不同的事件类型
        match event_type.as_str() {
            "message_start" => {
                // 提取消息 ID 和模型
                if let Some(msg) = json["message"].as_object() {
                    if let Some(id) = msg.get("id").and_then(|v| v.as_str()) {
                        *message_id = id.to_string();
                    }
                    if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                        *model = m.to_string();
                    }
                }
                None
            }
            "content_block_delta" => {
                // 内容增量事件
                if let Some(delta) = json.get("delta")
                    && let Some(text) = delta.get("text").and_then(|v| v.as_str()) {
                        return Some(Ok(StreamChunk {
                            id: message_id.clone(),
                            model: model.clone(),
                            delta: StreamDelta {
                                role: Some("assistant".to_string()),
                                content: Some(text.to_string()),
                                tool_calls: Vec::new(),
                            },
                            finished: false,
                            finish_reason: None,
                        }));
                    }
                None
            }
            "message_delta" => {
                // 消息结束事件
                let stop_reason = json["delta"]["stop_reason"].as_str();
                let finished = stop_reason.is_some();

                let finish_reason = stop_reason.map(|r| match r {
                    "end_turn" => FinishReason::Stop,
                    "max_tokens" => FinishReason::Length,
                    "stop_sequence" => FinishReason::Stop,
                    _ => FinishReason::Error,
                });

                Some(Ok(StreamChunk {
                    id: message_id.clone(),
                    model: model.clone(),
                    delta: StreamDelta {
                        role: None,
                        content: None,
                        tool_calls: Vec::new(),
                    },
                    finished,
                    finish_reason,
                }))
            }
            "message_stop" => {
                // 消息完全结束，不需要返回额外内容
                None
            }
            "content_block_start" | "content_block_stop" | "ping" => {
                // 忽略这些事件
                None
            }
            "error" => {
                // 错误事件
                let error_msg = json["error"]["message"].as_str().unwrap_or("Unknown error");
                Some(Err(OpenClawError::AIProvider(format!(
                    "Anthropic API 错误: {}",
                    error_msg
                ))))
            }
            _ => None,
        }
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
            timeout: None,
            headers: std::collections::HashMap::new(),
            organization: None,
        };
        let provider = AnthropicProvider::new(config);
        assert_eq!(provider.name(), "anthropic");
    }
}
