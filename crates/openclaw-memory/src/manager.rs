//! 记忆管理器

use std::sync::Arc;

use openclaw_core::{Message, OpenClawError, Result};
use openclaw_vector::{VectorStore, SearchQuery};

use crate::types::{MemoryConfig, MemoryItem, MemoryContent, MemoryLevel, MemoryRetrieval};
use crate::compressor::MemoryCompressor;
use crate::scorer::ImportanceScorer;
use crate::working::WorkingMemory;
use crate::hybrid_search::{HybridSearchManager, HybridSearchConfig};

/// 记忆管理器 - 统一管理三层记忆
pub struct MemoryManager {
    /// 工作记忆
    working: WorkingMemory,
    /// 短期记忆 (摘要)
    short_term: Vec<MemoryItem>,
    /// 长期记忆 (向量存储)
    long_term: Option<Arc<dyn VectorStore>>,
    /// 混合搜索管理器
    hybrid_search: Option<Arc<HybridSearchManager>>,
    /// 配置
    config: MemoryConfig,
    /// 重要性评分器
    scorer: ImportanceScorer,
    /// 压缩器
    compressor: MemoryCompressor,
}

impl MemoryManager {
    pub fn new(config: MemoryConfig) -> Self {
        Self {
            working: WorkingMemory::new(config.working.clone()),
            short_term: Vec::new(),
            long_term: None,
            hybrid_search: None,
            scorer: ImportanceScorer::new(),
            compressor: MemoryCompressor::new(config.short_term.clone()),
            config,
        }
    }

    /// 设置向量存储后端
    pub fn with_vector_store(mut self, store: Arc<dyn VectorStore>) -> Self {
        self.long_term = Some(store);
        self
    }

    /// 设置混合搜索管理器
    pub fn with_hybrid_search(mut self, search: Arc<HybridSearchManager>) -> Self {
        self.hybrid_search = Some(search);
        self
    }

    /// 添加消息到记忆
    pub async fn add(&mut self, message: Message) -> Result<()> {
        // 计算重要性分数
        let score = self.scorer.score(&message);
        let item = MemoryItem::from_message(message, score);

        // 添加到工作记忆
        if let Some(overflow) = self.working.add(item) {
            // 压缩溢出的消息到短期记忆
            let summary = self.compressor.compress(overflow).await?;
            self.short_term.push(summary);

            // 检查短期记忆是否需要清理
            if self.short_term.len() > self.config.short_term.max_summaries {
                // 将最旧的摘要移到长期记忆
                if let Some(old_summary) = self.short_term.first().cloned() {
                    if self.config.long_term.enabled {
                        if let Some(store) = &self.long_term {
                            self.archive_to_long_term(store.as_ref(), old_summary).await?;
                        }
                    }
                    self.short_term.remove(0);
                }
            }
        }

        Ok(())
    }

    /// 检索相关记忆
    pub async fn retrieve(&self, _query: &str, max_tokens: usize) -> Result<MemoryRetrieval> {
        let mut retrieval = MemoryRetrieval::new();
        let mut current_tokens = 0;

        // 1. 从工作记忆获取最近的完整消息
        let working_items = self.working.get_all();
        for item in working_items.iter().rev() {
            if current_tokens + item.token_count > max_tokens {
                break;
            }
            retrieval.add(item.clone());
            current_tokens += item.token_count;
        }

        // 2. 添加短期记忆摘要
        for item in self.short_term.iter().rev() {
            if current_tokens + item.token_count > max_tokens {
                break;
            }
            retrieval.add(item.clone());
            current_tokens += item.token_count;
        }

        // 3. 从长期记忆检索相关内容
        if self.config.long_term.enabled {
            if let Some(search) = &self.hybrid_search {
                let config = HybridSearchConfig::default();
                if let Ok(results) = search.search(_query, None, &config).await {
                     for result in results {
                         let content_preview = result.payload.get("content")
                             .and_then(|v| v.as_str())
                             .unwrap_or("")
                             .to_string();
                         
                         let token_count = content_preview.len() / 4;
                        let memory_item = MemoryItem {
                            id: uuid::Uuid::new_v4(),
                            level: MemoryLevel::LongTerm,
                            content: MemoryContent::VectorRef {
                                vector_id: result.id.clone(),
                                preview: content_preview,
                            },
                            created_at: chrono::Utc::now(),
                            last_accessed: chrono::Utc::now(),
                            access_count: 1,
                            importance_score: result.score,
                            token_count,
                            metadata: crate::types::MemoryMetadata::default(),
                        };
                        
                        if current_tokens + token_count <= max_tokens {
                            retrieval.add(memory_item);
                            current_tokens += token_count;
                        }
                    }
                }
            }
        }

        Ok(retrieval)
    }

    /// 获取当前上下文的消息列表
    pub fn get_context(&self) -> Vec<Message> {
        self.working.to_messages()
    }

    /// 获取统计信息
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            working_count: self.working.len(),
            working_tokens: self.working.total_tokens(),
            short_term_count: self.short_term.len(),
            short_term_tokens: self.short_term.iter().map(|i| i.token_count).sum(),
            long_term_enabled: self.long_term.is_some(),
        }
    }

    /// 清空所有记忆
    pub async fn clear(&mut self) -> Result<()> {
        self.working.clear();
        self.short_term.clear();
        
        if let Some(store) = &self.long_term {
            store.clear().await?;
        }

        Ok(())
    }

    /// 归档到长期记忆
    async fn archive_to_long_term(
        &self,
        _store: &dyn VectorStore,
        _item: MemoryItem,
    ) -> Result<()> {
        // TODO: 向量化并存储
        Ok(())
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new(MemoryConfig::default())
    }
}

/// 记忆统计信息
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub working_count: usize,
    pub working_tokens: usize,
    pub short_term_count: usize,
    pub short_term_tokens: usize,
    pub long_term_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_manager() {
        let mut manager = MemoryManager::default();

        // 添加消息
        manager.add(Message::user("你好")).await.unwrap();
        manager.add(Message::assistant("你好！")).await.unwrap();

        let stats = manager.stats();
        assert_eq!(stats.working_count, 2);
    }
}
