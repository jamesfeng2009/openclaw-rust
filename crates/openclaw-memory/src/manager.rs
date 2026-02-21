//! 记忆管理器

use std::sync::Arc;

use openclaw_core::{Message, OpenClawError, Result};
use openclaw_vector::VectorStore;

use crate::compressor::MemoryCompressor;
use crate::embedding::EmbeddingProvider;
use crate::hybrid_search::{HybridSearchConfig, HybridSearchManager};
use crate::recall::{MemoryRecall, RecallResult, SimpleMemoryRecall};
use crate::scorer::ImportanceScorer;
use crate::types::{MemoryConfig, MemoryContent, MemoryItem, MemoryLevel, MemoryRetrieval};
use crate::working::WorkingMemory;

/// 记忆管理器 - 统一管理三层记忆
pub struct MemoryManager {
    working: WorkingMemory,
    short_term: Vec<MemoryItem>,
    long_term: Option<Arc<dyn VectorStore>>,
    hybrid_search: Option<Arc<HybridSearchManager>>,
    config: MemoryConfig,
    scorer: ImportanceScorer,
    compressor: MemoryCompressor,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
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
            embedding_provider: None,
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

    /// 设置嵌入向量提供者
    pub fn with_embedding_provider<E: EmbeddingProvider + 'static>(mut self, provider: E) -> Self {
        self.embedding_provider = Some(Arc::new(provider));
        self
    }

    /// 自动召回相关记忆
    pub async fn recall(&self, query: &str) -> Result<RecallResult> {
        if let Some(provider) = &self.embedding_provider {
            let vector_store: Arc<dyn openclaw_vector::VectorStore> = Arc::new(
                openclaw_vector::store::MemoryStore::new()
            );
            let recall_tool = SimpleMemoryRecall::new(provider.clone(), vector_store);
            let result = recall_tool.recall(query, None).await?;
            Ok(result)
        } else {
            Err(OpenClawError::Memory(
                "Embedding provider not configured".to_string(),
            ))
        }
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
                    if self.config.long_term.enabled
                        && let Some(store) = &self.long_term
                    {
                        self.archive_to_long_term(store.as_ref(), old_summary)
                            .await?;
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
        if self.config.long_term.enabled
            && let Some(search) = &self.hybrid_search
        {
            let config = HybridSearchConfig::default();
            if let Ok(results) = search.search(_query, None, &config).await {
                for result in results {
                    let content_preview = result
                        .payload
                        .get("content")
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
        store: &dyn VectorStore,
        mut item: MemoryItem,
    ) -> Result<()> {
        let text = item.content.to_text();
        let vector_id = item.id.to_string();

        let embedding = if let Some(provider) = &self.embedding_provider {
            provider.embed(&text).await?
        } else {
            return Err(OpenClawError::Config("未配置嵌入向量提供者".to_string()));
        };

        let vector_item = openclaw_vector::VectorItem {
            id: vector_id.clone(),
            vector: embedding,
            payload: serde_json::json!({
                "memory_id": item.id.to_string(),
                "level": item.level,
                "importance": item.importance_score,
                "content": if text.len() > 200 { &text[..200] } else { &text },
            }),
            created_at: item.created_at,
        };

        store.upsert(vector_item).await?;

        item.content = crate::types::MemoryContent::VectorRef {
            vector_id,
            preview: if text.len() > 200 {
                format!("{}...", &text[..200])
            } else {
                text
            },
        };
        item.level = MemoryLevel::LongTerm;

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

    #[test]
    fn test_memory_content_to_text() {
        use crate::types::MemoryContent;

        let content = MemoryContent::Message {
            message: Message::user("Hello"),
        };
        assert_eq!(content.to_text(), "Hello");

        let summary = MemoryContent::Summary {
            text: "Summary text".to_string(),
            original_count: 5,
        };
        assert_eq!(summary.to_text(), "Summary text");

        let vector_ref = MemoryContent::VectorRef {
            vector_id: "123".to_string(),
            preview: "Preview text".to_string(),
        };
        assert_eq!(vector_ref.to_text(), "Preview text");
    }
}
