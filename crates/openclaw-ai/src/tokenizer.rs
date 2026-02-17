//! Token 计数器

use openclaw_core::{Message, Result};
use tiktoken_rs::{cl100k_base, o200k_base, CoreBPE};

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

/// Tiktoken 计数器 (实际使用 tiktoken-rs)
pub struct TiktokenCounter {
    bpe: CoreBPE,
}

impl TiktokenCounter {
    pub fn new(encoder: &str) -> Result<Self> {
        let bpe = match encoder {
            "o200k_base" => o200k_base(),
            "cl100k_base" | _ => cl100k_base(),
        }.map_err(|e| openclaw_core::OpenClawError::TokenCount(format!("加载分词器失败: {}", e)))?;
        
        Ok(Self { bpe })
    }

    pub fn for_model(model: &str) -> Result<Self> {
        let encoder = if model.starts_with("gpt-4o") || model.starts_with("o1") {
            "o200k_base"
        } else {
            "cl100k_base"
        };
        Self::new(encoder)
    }
}

impl TokenCounter for TiktokenCounter {
    fn count(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }
}

/// 创建 Token 计数器
/// 
/// # Arguments
/// * `use_accurate` - 是否使用精确的 tiktoken 计数
/// * `model` - 模型名称，用于选择正确的编码器
/// 
/// # Returns
/// 返回 Box<dyn TokenCounter>，调用方可以通过dyn Trait使用
pub fn create_token_counter(use_accurate: bool, model: &str) -> Result<Box<dyn TokenCounter>> {
    if use_accurate {
        Ok(Box::new(TiktokenCounter::for_model(model)?))
    } else {
        Ok(Box::new(SimpleTokenCounter::new()))
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

    #[test]
    fn test_tiktoken_counter() {
        let counter = TiktokenCounter::for_model("gpt-4").unwrap();
        
        let en_text = "Hello, this is a test message for token counting.";
        let en_count = counter.count(en_text);
        assert!(en_count > 0);

        let zh_text = "这是一个中文测试消息";
        let zh_count = counter.count(zh_text);
        assert!(zh_count > 0);

        let mixed_text = "Hello 你好 World 世界";
        let mixed_count = counter.count(mixed_text);
        assert!(mixed_count > 0);
    }

    #[test]
    fn test_tiktoken_counter_gpt4o() {
        let counter = TiktokenCounter::for_model("gpt-4o").unwrap();
        
        let text = "Hello, world!";
        let count = counter.count(text);
        assert!(count > 0);
    }

    #[test]
    fn test_create_token_counter_simple() {
        let counter = create_token_counter(false, "gpt-4").unwrap();
        let count = counter.count("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_create_token_counter_tiktoken() {
        let counter = create_token_counter(true, "gpt-4").unwrap();
        let count = counter.count("Hello, world!");
        assert!(count > 0);
    }
}
