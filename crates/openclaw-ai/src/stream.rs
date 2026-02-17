//! 流式响应支持

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use std::pin::Pin;

use crate::types::StreamChunk;
use openclaw_core::Result;

/// 流式响应处理器
pub struct StreamHandler;

impl StreamHandler {
    /// 处理流式响应，收集为完整消息
    pub async fn collect_stream(
        mut stream: Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>,
    ) -> Result<StreamResult> {
        let mut content = String::new();
        let mut tool_calls: Vec<CollectedToolCall> = Vec::new();
        let mut model = String::new();
        let mut id = String::new();
        let mut finish_reason = None;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;

            if id.is_empty() {
                id = chunk.id.clone();
            }
            if model.is_empty() {
                model = chunk.model.clone();
            }

            // 收集内容
            if let Some(text) = &chunk.delta.content {
                content.push_str(text);
            }

            // 收集工具调用
            for tc in &chunk.delta.tool_calls {
                // 找到或创建工具调用
                while tool_calls.len() <= tc.index {
                    tool_calls.push(CollectedToolCall {
                        id: None,
                        call_type: tc.call_type.clone(),
                        name: None,
                        arguments: String::new(),
                    });
                }

                let call = &mut tool_calls[tc.index];
                if let Some(id) = &tc.id {
                    call.id = Some(id.clone());
                }
                if let Some(func) = &tc.function {
                    if let Some(name) = &func.name {
                        call.name = Some(name.clone());
                    }
                    if let Some(args) = &func.arguments {
                        call.arguments.push_str(args);
                    }
                }
            }

            if chunk.finished {
                finish_reason = chunk.finish_reason;
            }
        }

        Ok(StreamResult {
            id,
            model,
            content,
            tool_calls,
            finish_reason,
        })
    }
}

/// 收集后的流式结果
#[derive(Debug, Clone)]
pub struct StreamResult {
    pub id: String,
    pub model: String,
    pub content: String,
    pub tool_calls: Vec<CollectedToolCall>,
    pub finish_reason: Option<crate::types::FinishReason>,
}

/// 收集后的工具调用
#[derive(Debug, Clone)]
pub struct CollectedToolCall {
    pub id: Option<String>,
    pub call_type: String,
    pub name: Option<String>,
    pub arguments: String,
}

impl StreamResult {
    /// 转换为消息
    pub fn to_message(&self) -> openclaw_core::Message {
        if !self.tool_calls.is_empty() {
            let content = if self.content.is_empty() {
                format!("Called {} tool(s)", self.tool_calls.len())
            } else {
                self.content.clone()
            };
            openclaw_core::Message::assistant(&content)
        } else {
            openclaw_core::Message::assistant(&self.content)
        }
    }
}

/// 流式响应 Trait
#[async_trait]
pub trait StreamResponder: Send + Sync {
    /// 发送流式块
    async fn send_chunk(&self, chunk: StreamChunk) -> Result<()>;

    /// 发送完成
    async fn send_done(&self) -> Result<()>;

    /// 发送错误
    async fn send_error(&self, error: &str) -> Result<()>;
}
