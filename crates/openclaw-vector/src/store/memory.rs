//! 内存向量存储 (用于测试和开发)

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::VectorStore;
use crate::types::{Filter, FilterOperator, SearchQuery, SearchResult, StoreStats, VectorItem};
use openclaw_core::Result;

/// 内存向量存储
pub struct MemoryStore {
    data: RwLock<HashMap<String, VectorItem>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }

    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot / (norm_a * norm_b)
        }
    }

    fn matches_filter(item: &VectorItem, filter: &Filter) -> bool {
        if filter.conditions.is_empty() {
            return true;
        }

        for condition in &filter.conditions {
            let value = item.payload.get(&condition.field);
            let matches = match (&condition.operator, value) {
                (FilterOperator::Eq, Some(v)) => v == &condition.value,
                (FilterOperator::Ne, Some(v)) => v != &condition.value,
                (FilterOperator::Gt, Some(v)) => {
                    v.as_f64().unwrap_or(0.0) > condition.value.as_f64().unwrap_or(0.0)
                }
                (FilterOperator::Gte, Some(v)) => {
                    v.as_f64().unwrap_or(0.0) >= condition.value.as_f64().unwrap_or(0.0)
                }
                (FilterOperator::Lt, Some(v)) => {
                    v.as_f64().unwrap_or(0.0) < condition.value.as_f64().unwrap_or(0.0)
                }
                (FilterOperator::Lte, Some(v)) => {
                    v.as_f64().unwrap_or(0.0) <= condition.value.as_f64().unwrap_or(0.0)
                }
                (FilterOperator::In, Some(v)) => {
                    if let Some(arr) = condition.value.as_array() {
                        arr.contains(v)
                    } else {
                        false
                    }
                }
                (FilterOperator::Contains, Some(v)) => {
                    if let Some(arr) = v.as_array() {
                        arr.contains(&condition.value)
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !matches {
                return false;
            }
        }
        true
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for MemoryStore {
    async fn upsert(&self, item: VectorItem) -> Result<()> {
        let mut data = self.data.write()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        data.insert(item.id.clone(), item);
        Ok(())
    }

    async fn upsert_batch(&self, items: Vec<VectorItem>) -> Result<usize> {
        let mut data = self.data.write()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        let count = items.len();
        for item in items {
            data.insert(item.id.clone(), item);
        }
        Ok(count)
    }

    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        let data = self.data.read()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;

        let mut results: Vec<SearchResult> = data
            .values()
            .filter(|item| {
                if let Some(filter) = &query.filter {
                    Self::matches_filter(item, filter)
                } else {
                    true
                }
            })
            .map(|item| {
                let score = Self::cosine_similarity(&query.vector, &item.vector);
                SearchResult {
                    id: item.id.clone(),
                    score,
                    payload: item.payload.clone(),
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(min_score) = query.min_score {
            results.retain(|r| r.score >= min_score);
        }

        results.truncate(query.limit);

        Ok(results)
    }

    async fn get(&self, id: &str) -> Result<Option<VectorItem>> {
        let data = self.data.read()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        Ok(data.get(id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut data = self.data.write()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        data.remove(id);
        Ok(())
    }

    async fn delete_by_filter(&self, filter: Filter) -> Result<usize> {
        let mut data = self.data.write()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        let ids_to_remove: Vec<String> = data
            .values()
            .filter(|item| Self::matches_filter(item, &filter))
            .map(|item| item.id.clone())
            .collect();

        let count = ids_to_remove.len();
        for id in ids_to_remove {
            data.remove(&id);
        }
        Ok(count)
    }

    async fn stats(&self) -> Result<StoreStats> {
        let data = self.data.read()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        Ok(StoreStats {
            total_vectors: data.len(),
            total_size_bytes: data.values().map(|v| v.vector.len() * 4).sum(),
            last_updated: Utc::now(),
        })
    }

    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write()
            .map_err(|_| openclaw_core::OpenClawError::VectorStore("Lock poisoned".to_string()))?;
        data.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_memory_store() {
        let store = MemoryStore::new();

        // 插入向量
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"label": "A"}));
        let id = item.id.clone();
        store.upsert(item).await.unwrap();

        // 搜索
        let query = SearchQuery::new(vec![1.0, 0.0, 0.0]).with_limit(1);
        let results = store.search(query).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert!(results[0].score > 0.99);
    }

    #[tokio::test]
    async fn test_upsert_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        let result = store.upsert(item).await;
        assert!(result.is_ok(), "upsert should succeed with valid lock");
    }

    #[tokio::test]
    async fn test_search_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        store.upsert(item).await.unwrap();
        
        let query = SearchQuery::new(vec![1.0, 0.0, 0.0]);
        let result = store.search(query).await;
        assert!(result.is_ok(), "search should succeed with valid lock");
    }

    #[tokio::test]
    async fn test_get_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        store.upsert(item).await.unwrap();
        
        let result = store.get("test").await;
        assert!(result.is_ok(), "get should succeed with valid lock");
    }

    #[tokio::test]
    async fn test_delete_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        store.upsert(item).await.unwrap();
        
        let result = store.delete("test").await;
        assert!(result.is_ok(), "delete should succeed with valid lock");
    }

    #[tokio::test]
    async fn test_stats_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        store.upsert(item).await.unwrap();
        
        let result = store.stats().await;
        assert!(result.is_ok(), "stats should succeed with valid lock");
        let stats = result.unwrap();
        assert_eq!(stats.total_vectors, 1);
    }

    #[tokio::test]
    async fn test_clear_lock_poison_handling() {
        let store = MemoryStore::new();
        
        let item = VectorItem::new(vec![1.0, 0.0, 0.0], json!({"test": "data"}));
        store.upsert(item).await.unwrap();
        
        let result = store.clear().await;
        assert!(result.is_ok(), "clear should succeed with valid lock");
        
        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_vectors, 0);
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        use tokio::task;
        
        let store = Arc::new(MemoryStore::new());
        let store_clone = store.clone();
        
        let handle = task::spawn(async move {
            for i in 0..100 {
                let item = VectorItem::new(
                    vec![i as f32, 0.0, 0.0],
                    json!({"index": i})
                );
                store_clone.upsert(item).await.unwrap();
            }
        });
        
        for i in 0..100 {
            let item = VectorItem::new(
                vec![i as f32, 0.0, 0.0],
                json!({"index": i})
            );
            store.upsert(item).await.unwrap();
        }
        
        handle.await.unwrap();
        
        let stats = store.stats().await.unwrap();
        assert_eq!(stats.total_vectors, 200);
    }
}
