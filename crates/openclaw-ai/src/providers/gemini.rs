//! Google Gemini 提供商实现

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
        self.config
            .base_url
            .as_deref()
            .unwrap_or("https://generativelanguage.googleapis.com/v1beta")
    }

    fn convert_messages(&self, messages: Vec<Message>) -> serde_json::Value {
        let contents: Vec<serde_json::Value> = messages
            .into_iter()
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
        messages
            .iter()
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

        let response = self
            .client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Gemini API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Gemini API 错误: {}",
                error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage::new(
            json["usageMetadata"]["promptTokenCount"]
                .as_u64()
                .unwrap_or(0) as usize,
            json["usageMetadata"]["candidatesTokenCount"]
                .as_u64()
                .unwrap_or(0) as usize,
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
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
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

        let response = self
            .client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Gemini Stream API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Gemini Stream API 错误: {}",
                error_text
            )));
        }

        // 创建 Gemini SSE 流
        let stream = Self::parse_gemini_sse_stream(response, request.model);

        Ok(Box::pin(stream))
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

        let response = self
            .client
            .post(&url)
            .header(header::CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("Gemini Embedding API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!(
                "Gemini Embedding API 错误: {}",
                error_text
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| OpenClawError::AIProvider(format!("解析响应失败: {}", e)))?;

        let embedding: Vec<f32> = json["embedding"]["values"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_f64())
                    .map(|v| v as f32)
                    .collect()
            })
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

impl GeminiProvider {
    /// 解析 Gemini SSE 流
    fn parse_gemini_sse_stream(
        response: Response,
        model: String,
    ) -> impl Stream<Item = Result<StreamChunk>> + Send {
        async_stream::stream! {
            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();

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
                            if let Some(result) = Self::parse_gemini_sse_event(&event, &model) {
                                yield result;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(OpenClawError::Http(format!("Gemini 流读取错误: {}", e)));
                        return;
                    }
                }
            }
        }
    }

    /// 解析单个 Gemini SSE 事件
    fn parse_gemini_sse_event(event: &str, model: &str) -> Option<Result<StreamChunk>> {
        for line in event.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                // 解析 JSON
                match serde_json::from_str::<serde_json::Value>(data) {
                    Ok(json) => {
                        if let Some(chunk) = Self::parse_gemini_stream_chunk(&json, model) {
                            return Some(Ok(chunk));
                        }
                    }
                    Err(e) => {
                        return Some(Err(OpenClawError::Serialization(e)));
                    }
                }
            }
        }
        None
    }

    /// 解析单个 Gemini 流式块
    fn parse_gemini_stream_chunk(json: &serde_json::Value, model: &str) -> Option<StreamChunk> {
        let candidate = &json["candidates"].get(0)?;
        let content = &candidate["content"];

        // 提取文本内容
        let text = content["parts"]
            .as_array()
            .and_then(|parts| parts.get(0))
            .and_then(|part| part["text"].as_str())
            .map(|s| s.to_string());

        // 提取角色
        let role = content["role"].as_str().map(|s| s.to_string());

        // 检查是否完成
        let finish_reason_str = candidate["finishReason"].as_str();
        let finished = finish_reason_str.is_some();

        let finish_reason = finish_reason_str.map(|r| match r {
            "STOP" => FinishReason::Stop,
            "MAX_TOKENS" => FinishReason::Length,
            "SAFETY" => FinishReason::ContentFilter,
            "RECITATION" => FinishReason::ContentFilter,
            _ => FinishReason::Error,
        });

        Some(StreamChunk {
            id: uuid::Uuid::new_v4().to_string(),
            model: model.to_string(),
            delta: StreamDelta {
                role,
                content: text,
                tool_calls: Vec::new(), // Gemini 暂不支持工具调用流式
            },
            finished,
            finish_reason,
        })
    }
}
