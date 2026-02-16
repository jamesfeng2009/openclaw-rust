//! 向量存储抽象层实现

mod lancedb;
mod memory;
mod sqlite;

use async_trait::async_trait;
use openclaw_core::{OpenClawError, Result};
use std::sync::Arc;

pub use lancedb::LanceDbStore;
pub use memory::MemoryStore;
pub use sqlite::SqliteStore;

use super::types::{Filter, SearchQuery, SearchResult, StoreStats, VectorItem};

/// 向量存储 Trait
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, item: VectorItem) -> Result<()>;
    async fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize>;
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>>;
    async fn get(&self, id: &str) -> Result<Option<VectorItem>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn delete_by_filter(&self, filter: Filter) -> Result<usize>;
    async fn stats(&self) -> Result<StoreStats>;
    async fn clear(&self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum StoreBackend {
    Memory,
    LanceDB { path: std::path::PathBuf },
    Qdrant { url: String, collection: String, api_key: Option<String> },
    PgVector { url: String, table: String },
    SQLite { path: std::path::PathBuf, table: String },
}

pub fn create_store(backend: StoreBackend) -> Result<Arc<dyn VectorStore>> {
    match backend {
        StoreBackend::Memory => Ok(Arc::new(MemoryStore::new())),
        StoreBackend::LanceDB { path: _ } => {
            Err(OpenClawError::VectorStore(
                "LanceDB requires async initialization. Use create_store_async instead.".to_string()
            ))
        }
        StoreBackend::Qdrant { url: _, collection: _, api_key: _ } => {
            Err(OpenClawError::VectorStore(
                "Qdrant store not yet implemented".to_string()
            ))
        }
        StoreBackend::PgVector { .. } => {
            Err(OpenClawError::VectorStore(
                "pgvector store not yet implemented".to_string()
            ))
        }
        StoreBackend::SQLite { path, table } => {
            let store = SqliteStore::new(path, &table)?;
            Ok(Arc::new(store))
        }
    }
}

pub async fn create_store_async(backend: StoreBackend) -> Result<Arc<dyn VectorStore>> {
    match backend {
        StoreBackend::Memory => Ok(Arc::new(MemoryStore::new())),
        StoreBackend::LanceDB { path } => {
            let store = LanceDbStore::new(&path, "vectors").await?;
            Ok(Arc::new(store))
        }
        StoreBackend::Qdrant { url, collection, api_key: _ } => {
            Err(OpenClawError::VectorStore(
                format!("Qdrant store not yet implemented: {} {}", url, collection)
            ))
        }
        StoreBackend::PgVector { .. } => {
            Err(OpenClawError::VectorStore(
                "pgvector store not yet implemented".to_string()
            ))
        }
        StoreBackend::SQLite { path, table } => {
            let store = SqliteStore::new(path, &table)?;
            Ok(Arc::new(store))
        }
    }
}
