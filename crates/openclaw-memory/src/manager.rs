//! 记忆管理器

use std::sync::Arc;

use chrono::Utc;
use openclaw_core::{Message, OpenClawError, Result};
use openclaw_vector::VectorStore;

use crate::compressor::MemoryCompressor;
use crate::conflict_resolver::{ConflictResolver, ResolutionMethod};
use crate::embedding::EmbeddingProvider;
use crate::fact_extractor::AtomicFact;
use crate::hybrid_search::{HybridSearchConfig, HybridSearchManager};
use crate::recall::{MemoryRecall, RecallResult, SimpleMemoryRecall};
use crate::scorer::ImportanceScorer;
use crate::types::{MemoryConfig, MemoryContent, MemoryItem, MemoryLevel, MemoryRetrieval};
use crate::working::WorkingMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveLearningTrigger {
    OnError,
    OnSuccess,
    OnUncertainty,
    OnUserFeedback,
    OnTimeInterval,
}

#[derive(Debug, Clone)]
pub struct ActiveLearningConfig {
    pub enabled_triggers: Vec<ActiveLearningTrigger>,
    pub auto_interval_minutes: u32,
    pub min_importance_for_learning: f32,
}

impl Default for ActiveLearningConfig {
    fn default() -> Self {
        Self {
            enabled_triggers: vec![ActiveLearningTrigger::OnError, ActiveLearningTrigger::OnTimeInterval],
            auto_interval_minutes: 30,
            min_importance_for_learning: 0.7,
        }
    }
}

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
    conflict_resolver: ConflictResolver,
    extracted_facts: Vec<AtomicFact>,
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
            conflict_resolver: ConflictResolver::new(),
            extracted_facts: Vec::new(),
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
        if let (Some(provider), Some(store)) = (&self.embedding_provider, &self.long_term) {
            let recall_tool = SimpleMemoryRecall::new(provider.clone(), store.clone());
            let result = recall_tool.recall(query, None).await?;
            Ok(result)
        } else {
            Err(OpenClawError::Memory(
                "Embedding provider or VectorStore not configured".to_string(),
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

    /// 从消息中提取原子事实并检测冲突
    pub async fn extract_and_resolve_facts(&mut self, message: &Message) -> Result<Vec<AtomicFact>> {
        let text = message.text_content().unwrap_or_default().to_string();
        let category = crate::fact_extractor::FactCategory::from_text(&text);
        
        let new_fact = AtomicFact::new(text, category)
            .with_source(message.id.to_string());

        let mut all_facts = self.extracted_facts.clone();
        all_facts.push(new_fact);

        let conflicts = self.conflict_resolver.detect_conflicts(&all_facts);
        
        if !conflicts.is_empty() {
            let resolved = self.conflict_resolver.resolve_facts(&all_facts, ResolutionMethod::Latest);
            self.extracted_facts = resolved;
        } else {
            self.extracted_facts = all_facts;
        }

        Ok(self.extracted_facts.clone())
    }

    /// 检测记忆中的冲突
    pub fn detect_conflicts(&self) -> Vec<crate::conflict_resolver::Conflict> {
        self.conflict_resolver.detect_conflicts(&self.extracted_facts)
    }

    /// 解决所有冲突
    pub fn resolve_all_conflicts(&mut self, method: ResolutionMethod) -> Vec<AtomicFact> {
        let resolved = self.conflict_resolver.resolve_facts(&self.extracted_facts, method);
        self.extracted_facts = resolved.clone();
        resolved
    }

    /// 获取已提取的事实
    pub fn get_facts(&self) -> &[AtomicFact] {
        &self.extracted_facts
    }

    /// 清除所有提取的事实
    pub fn clear_facts(&mut self) {
        self.extracted_facts.clear();
    }

    /// 主动记录错误信息
    pub fn record_error(&mut self, error: &str, solution: &str) -> Result<()> {
        if !self.extracted_facts.is_empty() {
            let fact = AtomicFact::new(
                format!("错误: {} -> 解决方案: {}", error, solution),
                crate::fact_extractor::FactCategory::Error,
            );
            self.extracted_facts.push(fact);
        }
        Ok(())
    }

    /// 主动记录成功经验
    pub fn record_success(&mut self, action: &str, outcome: &str) -> Result<()> {
        let fact = AtomicFact::new(
            format!("成功: {} -> {}", action, outcome),
            crate::fact_extractor::FactCategory::Action,
        );
        self.extracted_facts.push(fact);
        Ok(())
    }

    /// 主动记录用户反馈
    pub fn record_feedback(&mut self, feedback: &str, adjustment: &str) -> Result<()> {
        let fact = AtomicFact::new(
            format!("反馈: {} -> 调整: {}", feedback, adjustment),
            crate::fact_extractor::FactCategory::Feedback,
        );
        self.extracted_facts.push(fact);
        Ok(())
    }

    /// 检查是否应该触发主动学习
    pub fn should_trigger_learning(&self, trigger: ActiveLearningTrigger, config: &ActiveLearningConfig) -> bool {
        config.enabled_triggers.contains(&trigger)
    }

    /// 周期性主动总结
    pub async fn periodic_summary(&mut self) -> Result<String> {
        let mut summary_parts = Vec::new();
        
        summary_parts.push(format!("=== {} 记忆总结 ===", chrono::Utc::now().format("%Y-%m-%d %H:%M")));
        
        if !self.extracted_facts.is_empty() {
            summary_parts.push("\n## 关键事实:".to_string());
            for fact in &self.extracted_facts {
                summary_parts.push(format!("- {}", fact.content));
            }
        }

        if !self.short_term.is_empty() {
            summary_parts.push(format!("\n## 短期记忆 ({}) 项:", self.short_term.len()));
        }

        let result = summary_parts.join("\n");
        
        if !result.is_empty() {
            let fact = AtomicFact::new(
                result.clone(),
                crate::fact_extractor::FactCategory::Summary,
            );
            self.extracted_facts.push(fact);
        }

        Ok(result)
    }

    /// 导出所有记忆为 Markdown 格式
    pub fn export_to_markdown(&self) -> String {
        let mut md = String::new();
        
        md.push_str("# 记忆导出\n\n");
        md.push_str(&format!("导出时间: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        md.push_str("## 工作记忆\n\n");
        let working_items = self.working.get_all();
        if working_items.is_empty() {
            md.push_str("*空*\n\n");
        } else {
            for item in &working_items {
                md.push_str(&format!("### {}\n", item.id));
                md.push_str(&format!("- 层级: {:?}\n", item.level));
                md.push_str(&format!("- 访问次数: {}\n", item.access_count));
                md.push_str(&format!("- 重要性: {:.2}\n", item.importance_score));
                md.push_str(&format!("- 创建时间: {}\n", item.created_at.format("%Y-%m-%d %H:%M")));
                md.push_str(&format!("- 内容: {}\n\n", item.content.to_text()));
            }
        }
        
        md.push_str("## 短期记忆\n\n");
        if self.short_term.is_empty() {
            md.push_str("*空*\n\n");
        } else {
            for item in &self.short_term {
                md.push_str(&format!("### {}\n", item.id));
                md.push_str(&format!("- 访问次数: {}\n", item.access_count));
                md.push_str(&format!("- 重要性: {:.2}\n", item.importance_score));
                md.push_str(&format!("- 内容: {}\n\n", item.content.to_text()));
            }
        }
        
        md.push_str("## 提取的事实\n\n");
        if self.extracted_facts.is_empty() {
            md.push_str("*空*\n\n");
        } else {
            for fact in &self.extracted_facts {
                md.push_str(&format!("- **[{:?}]** {}\n", fact.category, fact.content));
            }
            md.push('\n');
        }
        
        md
    }

    /// 导出记忆为带元数据的 Markdown 文件
    pub fn export_with_metadata(&self, include_stats: bool) -> String {
        let mut md = self.export_to_markdown();
        
        if include_stats {
            md.push_str("## 统计信息\n\n");
            let stats = self.stats();
            md.push_str(&format!("- 工作记忆项: {}\n", stats.working_count));
            md.push_str(&format!("- 工作记忆Token: {}\n", stats.working_tokens));
            md.push_str(&format!("- 短期记忆项: {}\n", stats.short_term_count));
            md.push_str(&format!("- 短期记忆Token: {}\n", stats.short_term_tokens));
            md.push_str(&format!("- 长期记忆启用: {}\n", stats.long_term_enabled));
            md.push_str(&format!("- 已提取事实: {}\n", self.extracted_facts.len()));
        }
        
        md
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

    #[test]
    fn test_active_learning_config_default() {
        let config = ActiveLearningConfig::default();
        assert!(config.enabled_triggers.contains(&ActiveLearningTrigger::OnError));
        assert!(config.enabled_triggers.contains(&ActiveLearningTrigger::OnTimeInterval));
        assert_eq!(config.auto_interval_minutes, 30);
    }

    #[test]
    fn test_active_learning_trigger_equality() {
        let trigger1 = ActiveLearningTrigger::OnError;
        let trigger2 = ActiveLearningTrigger::OnError;
        let trigger3 = ActiveLearningTrigger::OnSuccess;
        
        assert_eq!(trigger1, trigger2);
        assert_ne!(trigger1, trigger3);
    }

    #[test]
    fn test_active_learning_config_custom() {
        let config = ActiveLearningConfig {
            enabled_triggers: vec![ActiveLearningTrigger::OnSuccess, ActiveLearningTrigger::OnUserFeedback],
            auto_interval_minutes: 60,
            min_importance_for_learning: 0.8,
        };
        
        assert!(config.enabled_triggers.contains(&ActiveLearningTrigger::OnSuccess));
        assert!(!config.enabled_triggers.contains(&ActiveLearningTrigger::OnError));
        assert_eq!(config.auto_interval_minutes, 60);
    }

    #[tokio::test]
    async fn test_record_error() {
        let mut manager = MemoryManager::default();
        manager.extract_and_resolve_facts(&Message::user("test")).await.unwrap();
        
        let result = manager.record_error("test error", "fix solution");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_success() {
        let mut manager = MemoryManager::default();
        
        let result = manager.record_success("did something", "got good result");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_feedback() {
        let mut manager = MemoryManager::default();
        
        let result = manager.record_feedback("user feedback", "adjustment");
        assert!(result.is_ok());
    }

    #[test]
    fn test_should_trigger_learning() {
        let manager = MemoryManager::default();
        let config = ActiveLearningConfig::default();
        
        assert!(manager.should_trigger_learning(ActiveLearningTrigger::OnError, &config));
        assert!(manager.should_trigger_learning(ActiveLearningTrigger::OnTimeInterval, &config));
        assert!(!manager.should_trigger_learning(ActiveLearningTrigger::OnSuccess, &config));
    }

    #[test]
    fn test_export_to_markdown() {
        let manager = MemoryManager::default();
        let md = manager.export_to_markdown();
        
        assert!(md.contains("# 记忆导出"));
        assert!(md.contains("## 工作记忆"));
        assert!(md.contains("## 短期记忆"));
        assert!(md.contains("## 提取的事实"));
    }

    #[test]
    fn test_export_with_metadata() {
        let manager = MemoryManager::default();
        let md = manager.export_with_metadata(true);
        
        assert!(md.contains("## 统计信息"));
        assert!(md.contains("工作记忆项:"));
        assert!(md.contains("短期记忆项:"));
    }

    #[test]
    fn test_export_empty_memory() {
        let manager = MemoryManager::default();
        let md = manager.export_to_markdown();
        
        assert!(md.contains("*空*"));
    }
}
