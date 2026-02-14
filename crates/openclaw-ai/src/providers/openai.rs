//! OpenAI 提供商实现 (包含流式响应)

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use openclaw_core::{Message, OpenClawError, Result, Role};
use reqwest::{header, Response};
use std::pin::Pin;

use crate::types::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk, TokenUsage, StreamDelta, ToolCallDelta, FunctionDelta};
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
        let url = format!("{}/chat/completions", self.get_base_url());

        let body = serde_json::json!({
            "model": request.model,
            "messages": self.convert_messages(request.messages),
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": true
        });

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key.as_deref().unwrap_or("")))
            .header(header::ACCEPT, "text/event-stream")
            .json(&body)
            .send()
            .await
            .map_err(|e| OpenClawError::Http(format!("OpenAI Stream API 请求失败: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OpenClawError::AIProvider(format!("OpenAI Stream API 错误: {}", error_text)));
        }

        // 创建 SSE 流
        let stream = Self::parse_sse_stream(response);

        Ok(Box::pin(stream))
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
            "gpt-4o-audio-preview".to_string(),
            "gpt-4-turbo".to_string(),
            "o1".to_string(),
            "o1-mini".to_string(),
            "o3-mini".to_string(),
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

impl OpenAIProvider {
    /// 解析 SSE 流
    fn parse_sse_stream(response: Response) -> impl Stream<Item = Result<StreamChunk>> + Send {
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
                            if let Some(result) = Self::parse_sse_event(&event) {
                                yield result;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(OpenClawError::Http(format!("流读取错误: {}", e)));
                        return;
                    }
                }
            }
        }
    }

    /// 解析单个 SSE 事件
    fn parse_sse_event(event: &str) -> Option<Result<StreamChunk>> {
        for line in event.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                // 检查是否是结束标记
                if data == "[DONE]" {
                    return None;
                }
                
                // 解析 JSON
                match serde_json::from_str::<serde_json::Value>(data) {
                    Ok(json) => {
                        if let Some(chunk) = Self::parse_stream_chunk(&json) {
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

    /// 解析单个流式块
    fn parse_stream_chunk(json: &serde_json::Value) -> Option<StreamChunk> {
        let id = json["id"].as_str().unwrap_or("").to_string();
        let model = json["model"].as_str().unwrap_or("").to_string();
        
        let choice = &json["choices"].get(0)?;
        let delta = &choice["delta"];

        let content = delta["content"].as_str().map(|s| s.to_string());
        let role = delta["role"].as_str().map(|s| s.to_string());

        // 解析工具调用
        let tool_calls: Vec<ToolCallDelta> = delta["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter().enumerate().filter_map(|(i, tc)| {
                    Some(ToolCallDelta {
                        index: i,
                        id: tc["id"].as_str().map(|s| s.to_string()),
                        call_type: tc["type"].as_str().unwrap_or("function").to_string(),
                        function: tc["function"].as_object().map(|f| FunctionDelta {
                            name: f.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
                            arguments: f.get("arguments").and_then(|v| v.as_str()).map(|s| s.to_string()),
                        }),
                    })
                }).collect()
            })
            .unwrap_or_default();

        let finish_reason = choice["finish_reason"].as_str();
        let finished = finish_reason.is_some();
        
        let finish_reason_enum = finish_reason.map(|r| match r {
            "stop" => FinishReason::Stop,
            "length" => FinishReason::Length,
            "tool_calls" => FinishReason::ToolCalls,
            "content_filter" => FinishReason::ContentFilter,
            _ => FinishReason::Error,
        });

        Some(StreamChunk {
            id,
            model,
            delta: StreamDelta {
                role,
                content,
                tool_calls,
            },
            finished,
            finish_reason: finish_reason_enum,
        })
    }
}
