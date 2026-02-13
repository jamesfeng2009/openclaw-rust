//! 记忆压缩器

use openclaw_core::{Message, OpenClawError, Result};
use crate::types::{MemoryItem, ShortTermMemoryConfig};

/// 记忆压缩器
pub struct MemoryCompressor {
    config: ShortTermMemoryConfig,
}

impl MemoryCompressor {
    pub fn new(config: ShortTermMemoryConfig) -> Self {
        Self { config }
    }

    /// 压缩消息列表为摘要
    /// 
    /// 注意：实际的摘要生成需要调用 AI API
    /// 这里提供的是一个简单的占位符实现
    pub async fn compress(&self, items: Vec<MemoryItem>) -> Result<MemoryItem> {
        if items.is_empty() {
            return Err(OpenClawError::Memory("无法压缩空消息列表".to_string()));
        }

        // 提取消息内容
        let messages: Vec<&Message> = items
            .iter()
            .filter_map(|item| {
                if let crate::types::MemoryContent::Message { message } = &item.content {
                    Some(message)
                } else {
                    None
                }
            })
            .collect();

        // 计算原始 token 数
        let original_tokens: usize = items.iter().map(|i| i.token_count).sum();

        // TODO: 实际的 AI 摘要生成
        // 目前使用简单的拼接摘要
        let summary = self.generate_simple_summary(&messages);

        // 估算摘要 token 数 (通常为原始的 20-30%)
        let summary_tokens = (original_tokens as f32 * 0.25) as usize;

        Ok(MemoryItem::summary(summary, items.len(), summary_tokens))
    }

    /// 生成简单摘要 (占位符实现)
    fn generate_simple_summary(&self, messages: &[&Message]) -> String {
        let mut parts = Vec::new();
        
        for msg in messages {
            if let Some(text) = msg.text_content() {
                let preview = if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.to_string()
                };
                parts.push(format!("{:?}: {}", msg.role, preview));
            }
        }

        format!("[摘要] 共 {} 条消息: {}", messages.len(), parts.join(" | "))
    }

    /// 检查是否需要压缩
    pub fn should_compress(&self, message_count: usize) -> bool {
        message_count >= self.config.compress_after
    }
}

impl Default for MemoryCompressor {
    fn default() -> Self {
        Self::new(ShortTermMemoryConfig::default())
    }
}

/// AI 驱动的压缩器 (需要 AI 提供商)
pub struct AICompressor {
    config: ShortTermMemoryConfig,
    // AI 提供商引用 (实际使用时需要配置)
}

impl AICompressor {
    pub fn new(config: ShortTermMemoryConfig) -> Self {
        Self { config }
    }

    /// 使用 AI 生成智能摘要
    pub async fn compress_with_ai(
        &self,
        items: Vec<MemoryItem>,
        _ai_provider: &dyn crate::AICompressProvider,
    ) -> Result<MemoryItem> {
        // TODO: 实现基于 AI 的智能摘要
        // 1. 提取关键信息
        // 2. 保留重要实体
        // 3. 生成连贯的摘要

        let compressor = MemoryCompressor::new(self.config.clone());
        compressor.compress(items).await
    }
}

/// AI 压缩提供商 Trait (用于依赖注入)
#[async_trait::async_trait]
pub trait AICompressProvider: Send + Sync {
    async fn generate_summary(&self, messages: &[Message]) -> Result<String>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_core::Role;

    #[tokio::test]
    async fn test_compressor() {
        let compressor = MemoryCompressor::default();

        let messages = vec![
            MemoryItem::from_message(Message::user("你好"), 0.5),
            MemoryItem::from_message(Message::assistant("你好！有什么我可以帮助你的吗？"), 0.5),
            MemoryItem::from_message(Message::user("我想了解 Rust"), 0.5),
        ];

        let summary = compressor.compress(messages).await.unwrap();
        
        assert!(matches!(summary.content, crate::types::MemoryContent::Summary { .. }));
        assert!(summary.token_count > 0);
    }
}
