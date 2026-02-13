//! 向量存储抽象层实现

mod lancedb;
mod memory;

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use std::sync::Arc;

pub use lancedb::LanceDbStore;
pub use memory::MemoryStore;

use super::types::{Filter, SearchQuery, SearchResult, StoreStats, VectorItem};

/// 向量存储 Trait
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// 插入或更新向量
    async fn upsert(&self, item: VectorItem) -> Result<()>;

    /// 批量插入
    async fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize>;

    /// 向量搜索
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>;

    /// 根据 ID 获取
    async fn get(&self, id: &str) -> Result<Option<VectorItem>>;

    /// 根据 ID 删除
    async fn delete(&self, id: &str) -> Result<()>;

    /// 根据条件删除
    async fn delete_by_filter(&self, filter: Filter) -> Result<usize>;

    /// 获取统计信息
    async fn stats(&self) -> Result<StoreStats>;

    /// 清空所有数据
    async fn clear(&self) -> Result<()>;
}

/// 向量存储后端类型
#[derive(Debug, Clone)]
pub enum StoreBackend {
    /// 内存存储 (仅用于测试)
    Memory,
    /// LanceDB 嵌入式存储
    LanceDB { path: std::path::PathBuf },
    /// Qdrant 服务
    Qdrant { url: String, collection: String, api_key: Option<String> },
    /// pgvector
    PgVector { url: String, table: String },
    /// SQLite-vec
    SQLite { path: std::path::PathBuf, table: String },
}

/// 创建向量存储实例
pub fn create_store(backend: StoreBackend) -> Result<Arc<dyn VectorStore>> {
    match backend {
        StoreBackend::Memory => Ok(Arc::new(MemoryStore::new())),
        StoreBackend::LanceDB { path } => {
            // LanceDB 需要异步初始化，这里返回一个占位符
            // 实际使用时应该在 async 上下文中创建
            Err(OpenClawError::VectorStore(
                "LanceDB requires async initialization. Use create_store_async instead.".to_string()
            ))
        }
        StoreBackend::Qdrant { .. } => {
            // TODO: 实现 Qdrant 存储
            Err(OpenClawError::VectorStore(
                "Qdrant store not yet implemented".to_string()
            ))
        }
        StoreBackend::PgVector { .. } => {
            Err(OpenClawError::VectorStore(
                "pgvector store not yet implemented".to_string()
            ))
        }
        StoreBackend::SQLite { .. } => {
            Err(OpenClawError::VectorStore(
                "SQLite-vec store not yet implemented".to_string()
            ))
        }
    }
}

/// 异步创建向量存储实例
pub async fn create_store_async(backend: StoreBackend) -> Result<Arc<dyn VectorStore>> {
    match backend {
        StoreBackend::Memory => Ok(Arc::new(MemoryStore::new())),
        StoreBackend::LanceDB { path } => {
            let store = LanceDbStore::new(&path).await?;
            Ok(Arc::new(store))
        }
        StoreBackend::Qdrant { url, collection, api_key } => {
            // TODO: 实现 Qdrant 存储
            Err(OpenClawError::VectorStore(
                format!("Qdrant store not yet implemented: {} {}", url, collection)
            ))
        }
        StoreBackend::PgVector { .. } => {
            Err(OpenClawError::VectorStore(
                "pgvector store not yet implemented".to_string()
            ))
        }
        StoreBackend::SQLite { .. } => {
            Err(OpenClawError::VectorStore(
                "SQLite-vec store not yet implemented".to_string()
            ))
        }
    }
}
