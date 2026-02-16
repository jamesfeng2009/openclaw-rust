//! 记忆类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use openclaw_core::Message;

/// 记忆层级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLevel {
    /// 工作记忆 - 最近消息
    Working,
    /// 短期记忆 - 摘要
    ShortTerm,
    /// 长期记忆 - 向量存储
    LongTerm,
}

/// 记忆项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: Uuid,
    pub level: MemoryLevel,
    pub content: MemoryContent,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
    pub access_count: usize,
    pub importance_score: f32,
    pub token_count: usize,
    pub metadata: MemoryMetadata,
}

/// 记忆内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryContent {
    /// 原始消息
    Message { message: Message },
    /// 摘要
    Summary { text: String, original_count: usize },
    /// 向量引用
    VectorRef { vector_id: String, preview: String },
}

/// 记忆元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryMetadata {
    pub session_id: Option<Uuid>,
    pub channel: Option<String>,
    pub peer_id: Option<String>,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
}

impl MemoryItem {
    pub fn from_message(message: Message, importance: f32) -> Self {
        let token_count = message.estimate_tokens();
        Self {
            id: Uuid::new_v4(),
            level: MemoryLevel::Working,
            content: MemoryContent::Message { message },
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            importance_score: importance,
            token_count,
            metadata: MemoryMetadata::default(),
        }
    }

    pub fn summary(text: impl Into<String>, original_count: usize, token_count: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            level: MemoryLevel::ShortTerm,
            content: MemoryContent::Summary {
                text: text.into(),
                original_count,
            },
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            importance_score: 0.5,
            token_count,
            metadata: MemoryMetadata::default(),
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed = Utc::now();
        self.access_count += 1;
    }
}

/// 记忆配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// 工作记忆配置
    pub working: WorkingMemoryConfig,
    /// 短期记忆配置
    pub short_term: ShortTermMemoryConfig,
    /// 长期记忆配置
    pub long_term: LongTermMemoryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemoryConfig {
    /// 最大消息数
    pub max_messages: usize,
    /// 最大 token 数
    pub max_tokens: usize,
}

impl Default for WorkingMemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 20,
            max_tokens: 8000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermMemoryConfig {
    /// 压缩阈值 (消息数)
    pub compress_after: usize,
    /// 最大摘要数
    pub max_summaries: usize,
}

impl Default for ShortTermMemoryConfig {
    fn default() -> Self {
        Self {
            compress_after: 10,
            max_summaries: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermMemoryConfig {
    /// 是否启用
    pub enabled: bool,
    /// 向量存储后端 (memory, sqlite, lancedb, qdrant, pgvector)
    pub backend: String,
    /// 向量存储集合名
    pub collection: String,
    /// 嵌入模型
    pub embedding_model: String,
}

impl Default for LongTermMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: "lancedb".to_string(),
            collection: "memories".to_string(),
            embedding_model: "text-embedding-3-small".to_string(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            working: WorkingMemoryConfig::default(),
            short_term: ShortTermMemoryConfig::default(),
            long_term: LongTermMemoryConfig::default(),
        }
    }
}

/// 记忆检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRetrieval {
    pub items: Vec<MemoryItem>,
    pub total_tokens: usize,
    pub from_levels: Vec<MemoryLevel>,
}

impl MemoryRetrieval {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            total_tokens: 0,
            from_levels: Vec::new(),
        }
    }

    pub fn add(&mut self, item: MemoryItem) {
        self.total_tokens += item.token_count;
        if !self.from_levels.contains(&item.level) {
            self.from_levels.push(item.level);
        }
        self.items.push(item);
    }
}

/// 记忆搜索查询
#[derive(Debug, Clone)]
pub struct MemorySearchQuery {
    pub query: String,
    pub limit: usize,
    pub max_tokens: usize,
    pub level: Option<MemoryLevel>,
    pub min_importance: Option<f32>,
    pub session_id: Option<uuid::Uuid>,
}

impl MemorySearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            limit: 10,
            max_tokens: 4000,
            level: None,
            min_importance: None,
            session_id: None,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn with_max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = tokens;
        self
    }

    pub fn with_level(mut self, level: MemoryLevel) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_session(mut self, session_id: uuid::Uuid) -> Self {
        self.session_id = Some(session_id);
        self
    }
}
