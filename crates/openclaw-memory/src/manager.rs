//! 记忆管理器

use std::path::{Path, PathBuf};
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
use crate::workspace::AgentWorkspace;

/// 记忆管理器 - 统一管理三层记忆
#[derive(Clone)]
pub struct MemoryManager {
    working: WorkingMemory,
    short_term: Vec<MemoryItem>,
    long_term: Option<Arc<dyn VectorStore>>,
    hybrid_search: Option<Arc<HybridSearchManager>>,
    config: MemoryConfig,
    scorer: ImportanceScorer,
    compressor: MemoryCompressor,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    recall_strategy: Option<Arc<dyn MemoryRecall>>,
    workspace: Option<Arc<AgentWorkspace>>,
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
            recall_strategy: None,
            workspace: None,
        }
    }

    /// 设置 workspace，用于 Markdown 文件的默认存储路径
    pub fn with_workspace(mut self, workspace: Arc<AgentWorkspace>) -> Self {
        self.workspace = Some(workspace);
        self
    }

    /// 设置自定义召回策略
    pub fn with_recall_strategy(mut self, strategy: Arc<dyn MemoryRecall>) -> Self {
        self.recall_strategy = Some(strategy);
        self
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
        if let Some(strategy) = &self.recall_strategy {
            return strategy.recall(query, None, None).await;
        }
        
        if let Some(provider) = &self.embedding_provider {
            if let Some(vector_store) = &self.long_term {
                let recall_tool = SimpleMemoryRecall::new(provider.clone(), vector_store.clone());
                return recall_tool.recall(query, None, None).await;
            }
            return Err(OpenClawError::Memory(
                "Vector store not configured".to_string(),
            ));
        }
        
        Err(OpenClawError::Memory(
            "Embedding provider or recall strategy not configured".to_string(),
        ))
    }

    /// 添加消息到记忆
    pub async fn add(&mut self, message: Message) -> Result<()> {
        self.add_with_namespace(message, None).await
    }

    /// 添加消息到记忆（带命名空间）
    pub async fn add_with_namespace(&mut self, message: Message, namespace: Option<crate::types::MemoryNamespace>) -> Result<()> {
        // 计算重要性分数
        let score = self.scorer.score(&message);
        let item = MemoryItem::from_message(message, score).with_optional_namespace(namespace);

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
                        namespace: None,
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

        let mut payload = serde_json::json!({
            "memory_id": item.id.to_string(),
            "level": item.level,
            "importance": item.importance_score,
            "content": if text.len() > 200 { &text[..200] } else { &text },
        });

        if let Some(ref namespace) = item.namespace {
            payload["user_id"] = serde_json::Value::String(namespace.user_id.clone());
            payload["persona_id"] = serde_json::Value::String(namespace.persona_id.clone());
        }

        let vector_item = openclaw_vector::VectorItem {
            id: vector_id.clone(),
            vector: embedding,
            payload,
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

impl MemoryManager {
    pub async fn export_to_markdown(&self, path: &Path) -> Result<usize> {
        let mut md = String::from("# AI 记忆\n\n");
        let mut count = 0;

        let working_items = self.working.get_all();
        if !working_items.is_empty() {
            md.push_str("## 最近对话\n\n");
            for item in working_items.iter().rev().take(50) {
                let content = item.content.to_text();
                if !content.is_empty() {
                    md.push_str(&format!("- {}\n", content));
                    count += 1;
                }
            }
            md.push_str("\n---\n\n");
        }

        if !self.short_term.is_empty() {
            md.push_str("## 摘要\n\n");
            for item in &self.short_term {
                let content = item.content.to_text();
                if !content.is_empty() {
                    md.push_str(&format!(
                        "### {}\n\n{}\n\n---\n\n",
                        item.created_at.format("%Y-%m-%d %H:%M"),
                        content
                    ));
                    count += 1;
                }
            }
        }

        if let Some(store) = &self.long_term {
            let empty_vector = vec![0.0; 384];
            let query = openclaw_vector::SearchQuery::new(empty_vector);
            if let Ok(items) = store.search(query).await {
                md.push_str("## 长期记忆\n\n");
                for item in items {
                    let content = item
                        .payload
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !content.is_empty() {
                        md.push_str(&format!("- {}\n", content));
                        count += 1;
                    }
                }
            }
        }

        tokio::fs::write(path, md).await?;
        Ok(count)
    }

    /// 导出到默认路径 (需要先设置 workspace)
    pub async fn export_to_markdown_default(&self) -> Result<usize> {
        let path = self.get_default_markdown_path()?;
        self.export_to_markdown(&path).await
    }

    /// 获取默认 Markdown 文件路径
    fn get_default_markdown_path(&self) -> Result<PathBuf> {
        if let Some(ws) = &self.workspace {
            Ok(ws.memory_path())
        } else {
            Err(OpenClawError::Config(
                "未设置 workspace，请使用 export_to_markdown(path) 指定路径".to_string(),
            ))
        }
    }

    pub async fn export_related_to_markdown(&self, query: &str, path: &Path) -> Result<usize> {
        let result = self.recall(query).await?;
        let mut md = String::from("# AI 记忆 - 相关记忆\n\n");
        md.push_str(&format!("## 查询: {}\n\n---\n\n", query));
        let mut count = 0;

        for item in result.items {
            let content = &item.content;
            if !content.is_empty() {
                md.push_str(&format!(
                    "### [相关性: {:.2}] {}\n\n{}\n\n---\n\n",
                    item.similarity,
                    item.memory_level,
                    content
                ));
                count += 1;
            }
        }

        tokio::fs::write(path, md).await?;
        Ok(count)
    }

    pub async fn import_from_markdown(&self, path: &Path) -> Result<usize> {
        let content = tokio::fs::read_to_string(path).await?;
        let items = self.parse_markdown_entries(&content)?;
        let mut count = 0;

        for item in items {
            if let Some(store) = &self.long_term {
                store.upsert(item).await?;
                count += 1;
            }
        }

        Ok(count)
    }

    /// 从默认路径导入 (需要先设置 workspace)
    pub async fn import_from_markdown_default(&self) -> Result<usize> {
        let path = self.get_default_markdown_path()?;
        self.import_from_markdown(&path).await
    }

    fn parse_markdown_entries(&self, content: &str) -> Result<Vec<openclaw_vector::VectorItem>> {
        let mut items = Vec::new();
        let mut current_section = String::new();

        for line in content.lines() {
            if line.starts_with('#') {
                if !current_section.trim().is_empty() {
                    if let Some(item) = self.create_vector_item(&current_section) {
                        items.push(item);
                    }
                }
                current_section = String::new();
                continue;
            }

            if line.starts_with("---") || line.starts_with('-') {
                continue;
            }

            current_section.push_str(line);
            current_section.push('\n');
        }

        if !current_section.trim().is_empty() {
            if let Some(item) = self.create_vector_item(&current_section) {
                items.push(item);
            }
        }

        Ok(items)
    }

    fn create_vector_item(&self, content: &str) -> Option<openclaw_vector::VectorItem> {
        let text = content.trim().to_string();
        if text.is_empty() {
            return None;
        }

        Some(openclaw_vector::VectorItem {
            id: uuid::Uuid::new_v4().to_string(),
            vector: vec![0.0; 384],
            payload: serde_json::json!({
                "content": text,
                "source": "markdown_import",
                "created_at": chrono::Utc::now().to_rfc3339()
            }),
            created_at: chrono::Utc::now(),
        })
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
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_memory_manager() {
        let mut manager = MemoryManager::default();

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

    #[tokio::test]
    async fn test_export_to_markdown_empty() {
        let manager = MemoryManager::default();
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("empty.md");

        let count = manager.export_to_markdown(&path).await.unwrap();

        assert_eq!(count, 0);
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("# AI 记忆"));
    }

    #[tokio::test]
    async fn test_export_to_markdown_with_working_memory() {
        let mut manager = MemoryManager::default();
        manager.add(Message::user("测试消息1")).await.unwrap();
        manager.add(Message::assistant("测试回复1")).await.unwrap();

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.md");

        let count = manager.export_to_markdown(&path).await.unwrap();

        assert!(count > 0);
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("测试消息1"));
        assert!(content.contains("测试回复1"));
    }

    #[test]
    fn test_parse_markdown_entries() {
        let manager = MemoryManager::default();
        let content = r#"# AI 记忆

## 用户偏好

用户喜欢简洁的回答风格

---

## 项目信息

这是一个测试项目
"#;

        let items = manager.parse_markdown_entries(content).unwrap();
        assert!(!items.is_empty());
        
        let first_content = items[0].payload.get("content").unwrap().as_str().unwrap();
        assert!(first_content.contains("用户喜欢简洁的回答风格"));
    }

    #[test]
    fn test_parse_markdown_entries_empty_lines() {
        let manager = MemoryManager::default();
        let content = r#"# 标题



内容1



---
内容2"#;

        let items = manager.parse_markdown_entries(content).unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_create_vector_item() {
        let manager = MemoryManager::default();
        
        let item = manager.create_vector_item("测试内容").unwrap();
        assert!(!item.id.is_empty());
        assert_eq!(item.vector.len(), 384);
        assert_eq!(item.payload.get("content").unwrap().as_str().unwrap(), "测试内容");
        assert_eq!(item.payload.get("source").unwrap().as_str().unwrap(), "markdown_import");
    }

    #[test]
    fn test_create_vector_item_empty() {
        let manager = MemoryManager::default();
        
        let item = manager.create_vector_item("");
        assert!(item.is_none());
        
        let item = manager.create_vector_item("   ");
        assert!(item.is_none());
    }

    #[tokio::test]
    async fn test_export_to_markdown_format() {
        let mut manager = MemoryManager::default();
        manager.add(Message::user("今天天气真好")).await.unwrap();

        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("format_test.md");

        manager.export_to_markdown(&path).await.unwrap();

        let content = tokio::fs::read_to_string(&path).await.unwrap();
        
        assert!(content.starts_with("# AI 记忆"));
        assert!(content.contains("## 最近对话"));
        assert!(content.contains("今天天气真好"));
    }
}
