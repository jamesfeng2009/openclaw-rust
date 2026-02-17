//! 混合搜索管理器 - 结合向量搜索和 FTS5 全文搜索

use openclaw_core::Result;
use openclaw_vector::{SearchQuery, SearchResult, VectorItem, VectorStore};
use std::sync::Arc;

pub struct HybridSearchManager {
    vector_store: Arc<dyn VectorStore>,
    #[allow(dead_code)]
    vector_weight: f32,
    #[allow(dead_code)]
    keyword_weight: f32,
}

#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub min_score: Option<f32>,
    pub limit: usize,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.7,
            keyword_weight: 0.3,
            min_score: Some(0.0),
            limit: 10,
        }
    }
}

impl HybridSearchManager {
    pub fn new(vector_store: Arc<dyn VectorStore>, config: HybridSearchConfig) -> Self {
        Self {
            vector_store,
            vector_weight: config.vector_weight,
            keyword_weight: config.keyword_weight,
        }
    }

    pub async fn search(
        &self,
        query_text: &str,
        query_vector: Option<Vec<f32>>,
        config: &HybridSearchConfig,
    ) -> Result<Vec<SearchResult>> {
        let mut all_results: Vec<SearchResult> = Vec::new();

        if let Some(vector) = query_vector {
            if config.vector_weight > 0.0 {
                let mut query = SearchQuery::new(vector);
                query.limit = config.limit;
                query.min_score = config.min_score;

                let vector_results = self.vector_store.search(query).await?;
                all_results.extend(vector_results);
            }
        }

        if config.keyword_weight > 0.0 && !query_text.is_empty() {
            if let Ok(fts_results) = self.fts_search(query_text, config.limit).await {
                all_results.extend(fts_results);
            }
        }

        Ok(self.merge_results(all_results, config))
    }

    async fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let all_items = self.get_all_items().await?;

        let mut results = Vec::new();
        for item in all_items {
            if let Some(content) = item.payload.get("content").and_then(|v| v.as_str()) {
                if content.to_lowercase().contains(&query.to_lowercase()) {
                    results.push(SearchResult {
                        id: item.id,
                        score: 1.0,
                        payload: item.payload,
                    });
                }
            }
        }

        results.truncate(limit);
        Ok(results)
    }

    async fn get_all_items(&self) -> Result<Vec<VectorItem>> {
        let stats = self.vector_store.stats().await?;
        let limit = stats.total_vectors.min(1000);

        let dummy_vector = vec![0.0; 128];
        let query = SearchQuery::new(dummy_vector).with_limit(limit);

        let results = self.vector_store.search(query).await?;

        let mut items = Vec::new();
        for result in results {
            if let Some(item) = self.vector_store.get(&result.id).await? {
                items.push(item);
            }
        }

        Ok(items)
    }

    fn merge_results(
        &self,
        results: Vec<SearchResult>,
        config: &HybridSearchConfig,
    ) -> Vec<SearchResult> {
        use std::collections::HashMap;

        let mut combined: HashMap<String, SearchResult> = HashMap::new();

        for result in results {
            if let Some(existing) = combined.get_mut(&result.id) {
                existing.score += result.score;
            } else {
                combined.insert(result.id.clone(), result);
            }
        }

        let total_weight = config.vector_weight + config.keyword_weight;
        if total_weight > 0.0 {
            for result in combined.values_mut() {
                result.score = result.score / total_weight;
            }
        }

        let mut sorted: Vec<SearchResult> = combined.into_values().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(config.limit);

        if let Some(min_score) = config.min_score {
            sorted.retain(|r| r.score >= min_score);
        }

        sorted
    }

    pub async fn add_memory(
        &self,
        id: String,
        content: String,
        vector: Vec<f32>,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let mut payload = metadata;
        payload["content"] = serde_json::json!(content);

        let item = VectorItem::new(vector, payload).with_id(id);
        self.vector_store.upsert(item).await?;
        Ok(())
    }

    pub async fn remove_memory(&self, id: &str) -> Result<()> {
        self.vector_store.delete(id).await?;
        Ok(())
    }

    pub async fn stats(&self) -> Result<openclaw_vector::StoreStats> {
        self.vector_store.stats().await
    }
}
