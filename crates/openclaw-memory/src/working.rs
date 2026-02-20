//! 工作记忆实现

use std::collections::VecDeque;
use std::sync::RwLock;

use crate::types::{MemoryItem, WorkingMemoryConfig};
use openclaw_core::Message;

/// 工作记忆 - 存储最近的消息
pub struct WorkingMemory {
    items: RwLock<VecDeque<MemoryItem>>,
    config: WorkingMemoryConfig,
}

impl WorkingMemory {
    pub fn new(config: WorkingMemoryConfig) -> Self {
        Self {
            items: RwLock::new(VecDeque::with_capacity(config.max_messages)),
            config,
        }
    }

    /// 添加消息到工作记忆
    pub fn add(&self, item: MemoryItem) -> Option<Vec<MemoryItem>> {
        let mut items = self.items.write().unwrap();
        items.push_back(item);

        // 检查是否需要压缩
        let should_compress = self.should_compress_internal(&items);

        if should_compress {
            // 计算需要移除的条数：同时考虑条数和 token 数
            let count_overflow = items.len().saturating_sub(self.config.max_messages);
            let current_tokens = self.total_tokens_internal(&items);
            let token_overflow = current_tokens.saturating_sub(self.config.max_tokens);
            
            // 估算每条消息的平均 token 数
            let avg_tokens_per_item = if !items.is_empty() {
                current_tokens / items.len()
            } else {
                1
            };
            
            // 计算按 token 需要移除的条数
            let token_based_overflow = if avg_tokens_per_item > 0 {
                (token_overflow + avg_tokens_per_item - 1) / avg_tokens_per_item
            } else {
                0
            };
            
            // 取两者的最大值
            let overflow = count_overflow.max(token_based_overflow);
            
            // 确保不会移除过多，至少保留一半的 max_messages
            let min_keep = self.config.max_messages / 2;
            let actual_overflow = overflow.min(items.len().saturating_sub(min_keep));
            
            if actual_overflow > 0 {
                let drained: Vec<MemoryItem> = items.drain(..actual_overflow).collect();
                return Some(drained);
            }
        }
        None
    }

    /// 获取所有消息
    pub fn get_all(&self) -> Vec<MemoryItem> {
        let items = self.items.read().unwrap();
        items.iter().cloned().collect()
    }

    /// 获取最近 N 条消息
    pub fn get_recent(&self, n: usize) -> Vec<MemoryItem> {
        let items = self.items.read().unwrap();
        items.iter().rev().take(n).rev().cloned().collect()
    }

    /// 获取总 token 数
    pub fn total_tokens(&self) -> usize {
        let items = self.items.read().unwrap();
        items.iter().map(|i| i.token_count).sum()
    }

    /// 获取消息数量
    pub fn len(&self) -> usize {
        let items = self.items.read().unwrap();
        items.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 检查是否需要压缩
    pub fn should_compress(&self) -> bool {
        let items = self.items.read().unwrap();
        self.should_compress_internal(&items)
    }

    fn should_compress_internal(&self, items: &VecDeque<MemoryItem>) -> bool {
        items.len() > self.config.max_messages
            || self.total_tokens_internal(items) > self.config.max_tokens
    }

    fn total_tokens_internal(&self, items: &VecDeque<MemoryItem>) -> usize {
        items.iter().map(|i| i.token_count).sum()
    }

    /// 清空工作记忆
    pub fn clear(&self) {
        let mut items = self.items.write().unwrap();
        items.clear();
    }

    /// 转换为消息列表 (用于发送给 AI)
    pub fn to_messages(&self) -> Vec<Message> {
        let items = self.items.read().unwrap();
        items
            .iter()
            .filter_map(|item| {
                if let crate::types::MemoryContent::Message { message } = &item.content {
                    Some(message.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new(WorkingMemoryConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openclaw_core::Role;

    #[test]
    fn test_working_memory() {
        let memory = WorkingMemory::new(WorkingMemoryConfig {
            max_messages: 5,
            max_tokens: 1000,
        });

        // 添加消息
        for i in 0..3 {
            let msg = Message::user(format!("Message {}", i));
            let item = MemoryItem::from_message(msg, 0.5);
            memory.add(item);
        }

        assert_eq!(memory.len(), 3);
    }
}
