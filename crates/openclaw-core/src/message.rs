//! 消息模型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text {
        text: String,
    },
    Image {
        url: String,
    },
    Audio {
        url: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
    },
    ToolResult {
        id: String,
        content: String,
    },
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: Role,
    pub content: Vec<Content>,
    pub created_at: DateTime<Utc>,
    pub metadata: MessageMetadata,
}

/// 消息元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    /// 来源通道
    pub channel: Option<String>,
    /// 发送者 ID
    pub sender_id: Option<String>,
    /// 会话 ID
    pub session_id: Option<Uuid>,
    /// Token 数量
    pub token_count: Option<usize>,
    /// 重要性评分 (0.0 - 1.0)
    pub importance_score: Option<f32>,
    /// 是否已压缩
    pub is_compressed: bool,
    /// 自定义标签
    pub tags: Vec<String>,
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            channel: None,
            sender_id: None,
            session_id: None,
            token_count: None,
            importance_score: None,
            is_compressed: false,
            tags: Vec::new(),
        }
    }
}

impl Message {
    pub fn new(role: Role, content: Vec<Content>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content,
            created_at: Utc::now(),
            metadata: MessageMetadata::default(),
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, vec![Content::Text { text: text.into() }])
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, vec![Content::Text { text: text.into() }])
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, vec![Content::Text { text: text.into() }])
    }

    /// 获取文本内容
    pub fn text_content(&self) -> Option<&str> {
        self.content.iter().find_map(|c| {
            if let Content::Text { text } = c {
                Some(text.as_str())
            } else {
                None
            }
        })
    }

    /// 估算 token 数量 (简单实现，实际应使用 tiktoken)
    pub fn estimate_tokens(&self) -> usize {
        self.content
            .iter()
            .map(|c| {
                match c {
                    Content::Text { text } => text.len() / 4, // 粗略估算
                    Content::Image { .. } => 85,              // GPT-4V 图像基础 token
                    Content::Audio { .. } => 0,
                    Content::ToolCall { arguments, .. } => arguments.to_string().len() / 4,
                    Content::ToolResult { content, .. } => content.len() / 4,
                }
            })
            .sum()
    }
}
