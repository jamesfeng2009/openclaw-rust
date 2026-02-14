//! Token 计数器

use openclaw_core::{Message, Result};

/// Token 计数器 Trait
pub trait TokenCounter: Send + Sync {
    /// 计算文本的 token 数量
    fn count(&self, text: &str) -> usize;

    /// 计算消息的 token 数量
    fn count_message(&self, message: &Message) -> usize {
        let base_tokens = 4; // 消息格式开销
        let role_tokens = match message.role {
            openclaw_core::Role::System => 4,
            openclaw_core::Role::User => 4,
            openclaw_core::Role::Assistant => 4,
            openclaw_core::Role::Tool => 4,
        };

        let content_tokens: usize = message.content.iter().map(|c| {
            match c {
                openclaw_core::Content::Text { text } => self.count(text),
                openclaw_core::Content::Image { .. } => 85,
                openclaw_core::Content::Audio { .. } => 0,
                openclaw_core::Content::ToolCall { arguments, .. } => {
                    self.count(&arguments.to_string()) + 10
                }
                openclaw_core::Content::ToolResult { content, .. } => self.count(content),
            }
        }).sum();

        base_tokens + role_tokens + content_tokens
    }

    /// 计算消息列表的 token 数量
    fn count_messages(&self, messages: &[Message]) -> usize {
        let base_tokens = 3; // 对话格式开销
        messages.iter().map(|m| self.count_message(m)).sum::<usize>() + base_tokens
    }
}

/// 简单 Token 计数器 (基于字符估算)
pub struct SimpleTokenCounter {
    chars_per_token: f32,
}

impl SimpleTokenCounter {
    pub fn new() -> Self {
        // 英文约 4 字符 = 1 token
        // 中文约 1.5 字符 = 1 token
        Self {
            chars_per_token: 3.0,
        }
    }

    fn is_chinese(c: char) -> bool {
        matches!(c, '\u{4E00}'..='\u{9FFF}')
    }
}

impl Default for SimpleTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for SimpleTokenCounter {
    fn count(&self, text: &str) -> usize {
        let chinese_chars = text.chars().filter(|c| Self::is_chinese(*c)).count();
        let other_chars = text.chars().filter(|c| !Self::is_chinese(*c)).count();

        // 中文约 1.5 字符 = 1 token
        let chinese_tokens = (chinese_chars as f32 / 1.5).ceil() as usize;
        // 其他约 4 字符 = 1 token
        let other_tokens = (other_chars as f32 / 4.0).ceil() as usize;

        chinese_tokens + other_tokens
    }
}

/// Tiktoken 计数器 (实际实现需要 tiktoken-rs)
pub struct TiktokenCounter {
    encoder: Option<String>,
}

impl TiktokenCounter {
    pub fn new(encoder: Option<String>) -> Result<Self> {
        Ok(Self { encoder })
    }

    pub fn for_model(model: &str) -> Self {
        let encoder = if model.starts_with("gpt-4") {
            Some("cl100k_base".to_string())
        } else if model.starts_with("gpt-3.5") {
            Some("cl100k_base".to_string())
        } else if model.starts_with("claude") {
            Some("cl100k_base".to_string())
        } else {
            None
        };
        Self { encoder }
    }
}

impl TokenCounter for TiktokenCounter {
    fn count(&self, text: &str) -> usize {
        // TODO: 使用 tiktoken-rs 实际计算
        // 目前回退到简单估算
        SimpleTokenCounter::new().count(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_counter() {
        let counter = SimpleTokenCounter::new();
        
        // 英文测试
        let en_text = "Hello, this is a test message for token counting.";
        let en_count = counter.count(en_text);
        assert!(en_count > 0);

        // 中文测试
        let zh_text = "这是一个中文测试消息";
        let zh_count = counter.count(zh_text);
        assert!(zh_count > 0);

        // 混合测试
        let mixed_text = "Hello 你好 World 世界";
        let mixed_count = counter.count(mixed_text);
        assert!(mixed_count > 0);
    }
}
