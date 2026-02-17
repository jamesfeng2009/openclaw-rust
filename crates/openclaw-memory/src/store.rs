//! 统一记忆存储 Trait
//!
//! 定义三层记忆的统一接口，支持可插拔的存储后端

use async_trait::async_trait;
use openclaw_core::{Message, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{MemoryConfig, MemoryItem, MemoryRetrieval, MemorySearchQuery};

#[async_trait]
pub trait MemoryStore: Send + Sync {
    fn name(&self) -> &str;
    async fn initialize(&self, _config: &MemoryConfig) -> Result<()>;
    async fn add(&self, message: Message) -> Result<()>;
    async fn add_batch(&self, messages: Vec<Message>) -> Result<usize>;
    async fn search(&self, query: MemorySearchQuery) -> Result<MemoryRetrieval>;
    async fn get_recent(&self, count: usize) -> Result<Vec<MemoryItem>>;
    async fn get(&self, id: &str) -> Result<Option<MemoryItem>>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn clear(&self) -> Result<()>;
    async fn stats(&self) -> Result<MemoryStats>;
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub working_count: usize,
    pub short_term_count: usize,
    pub long_term_count: usize,
    pub total_tokens: usize,
}

pub struct InMemoryStore {
    working: Arc<RwLock<Vec<MemoryItem>>>,
    short_term: Arc<RwLock<Vec<MemoryItem>>>,
    long_term: Arc<RwLock<Vec<MemoryItem>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            working: Arc::new(RwLock::new(Vec::new())),
            short_term: Arc::new(RwLock::new(Vec::new())),
            long_term: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl MemoryStore for InMemoryStore {
    fn name(&self) -> &str {
        "in_memory"
    }

    async fn initialize(&self, _config: &MemoryConfig) -> Result<()> {
        Ok(())
    }

    async fn add(&self, message: Message) -> Result<()> {
        let item = MemoryItem::from_message(message, 0.5);
        self.working.write().await.push(item);
        Ok(())
    }

    async fn add_batch(&self, messages: Vec<Message>) -> Result<usize> {
        let count = messages.len();
        let mut working = self.working.write().await;
        for msg in messages {
            let item = MemoryItem::from_message(msg, 0.5);
            working.push(item);
        }
        Ok(count)
    }

    async fn search(&self, query: MemorySearchQuery) -> Result<MemoryRetrieval> {
        let mut retrieval = MemoryRetrieval::new();
        let working = self.working.read().await;

        for item in working.iter().rev() {
            if query.level.is_some() && query.level != Some(item.level) {
                continue;
            }
            retrieval.add(item.clone());
            if retrieval.items.len() >= query.limit {
                break;
            }
        }

        Ok(retrieval)
    }

    async fn get_recent(&self, count: usize) -> Result<Vec<MemoryItem>> {
        let working = self.working.read().await;
        Ok(working.iter().rev().take(count).cloned().collect())
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryItem>> {
        let working = self.working.read().await;
        for item in working.iter() {
            if item.id.to_string() == id {
                return Ok(Some(item.clone()));
            }
        }
        Ok(None)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        self.working
            .write()
            .await
            .retain(|i| i.id.to_string() != id);
        self.short_term
            .write()
            .await
            .retain(|i| i.id.to_string() != id);
        self.long_term
            .write()
            .await
            .retain(|i| i.id.to_string() != id);
        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        self.working.write().await.clear();
        self.short_term.write().await.clear();
        self.long_term.write().await.clear();
        Ok(())
    }

    async fn stats(&self) -> Result<MemoryStats> {
        let working = self.working.read().await;
        let short_term = self.short_term.read().await;
        let long_term = self.long_term.read().await;

        Ok(MemoryStats {
            working_count: working.len(),
            short_term_count: short_term.len(),
            long_term_count: long_term.len(),
            total_tokens: working
                .iter()
                .chain(short_term.iter())
                .chain(long_term.iter())
                .map(|i| i.token_count)
                .sum(),
        })
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}
