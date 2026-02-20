//! 混合搜索管理器 - 结合向量搜索、BM25 全文搜索和知识图谱
//!
//! 支持三种搜索模式的灵活组合：
//! - 向量搜索：语义相似度匹配
//! - BM25 搜索：词频相关性匹配
//! - 知识图谱搜索：实体关系推理

use openclaw_core::Result;
use openclaw_vector::{SearchQuery, SearchResult, VectorItem, VectorStore};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::bm25::Bm25Index;
use crate::knowledge_graph::KnowledgeGraph;
use crate::unified_search::config::UnifiedSearchConfig;
use crate::unified_search::fusion::{FusionStrategy, ResultFusion};
use crate::unified_search::result::{SearchSource, UnifiedSearchResult};

pub struct HybridSearchManager {
    vector_store: Arc<dyn VectorStore>,
    vector_weight: f32,
    keyword_weight: f32,
    embedding_dimension: usize,
    bm25_index: Option<Arc<Bm25Index>>,
    knowledge_graph: Option<Arc<RwLock<KnowledgeGraph>>>,
}

#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub bm25_weight: f32,
    pub knowledge_graph_weight: f32,
    pub min_score: Option<f32>,
    pub limit: usize,
    pub embedding_dimension: Option<usize>,
    pub enable_vector: bool,
    pub enable_bm25: bool,
    pub enable_knowledge_graph: bool,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.5,
            keyword_weight: 0.3,
            bm25_weight: 0.3,
            knowledge_graph_weight: 0.2,
            min_score: Some(0.0),
            limit: 10,
            embedding_dimension: None,
            enable_vector: true,
            enable_bm25: true,
            enable_knowledge_graph: true,
        }
    }
}

impl HybridSearchManager {
    pub fn new(vector_store: Arc<dyn VectorStore>, config: HybridSearchConfig) -> Self {
        let embedding_dimension = config.embedding_dimension.unwrap_or(1536);
        Self {
            vector_store,
            vector_weight: config.vector_weight,
            keyword_weight: config.keyword_weight,
            embedding_dimension,
            bm25_index: None,
            knowledge_graph: None,
        }
    }

    pub fn with_bm25(mut self, bm25_index: Arc<Bm25Index>) -> Self {
        self.bm25_index = Some(bm25_index);
        self
    }

    pub fn with_knowledge_graph(mut self, kg: Arc<RwLock<KnowledgeGraph>>) -> Self {
        self.knowledge_graph = Some(kg);
        self
    }

    pub async fn search(
        &self,
        query_text: &str,
        query_vector: Option<Vec<f32>>,
        config: &HybridSearchConfig,
    ) -> Result<Vec<SearchResult>> {
        let mut all_results: Vec<SearchResult> = Vec::new();

        if let Some(vector) = query_vector
            && config.vector_weight > 0.0
        {
            let mut query = SearchQuery::new(vector);
            query.limit = config.limit;
            query.min_score = config.min_score;

            let vector_results = self.vector_store.search(query).await?;
            all_results.extend(vector_results);
        }

        if config.keyword_weight > 0.0
            && !query_text.is_empty()
            && let Ok(fts_results) = self.fts_search(query_text, config.limit).await
        {
            all_results.extend(fts_results);
        }

        Ok(self.merge_results(all_results, config))
    }

    async fn fts_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let all_items = self.get_all_items().await?;

        let mut results = Vec::new();
        for item in all_items {
            if let Some(content) = item.payload.get("content").and_then(|v| v.as_str())
                && content.to_lowercase().contains(&query.to_lowercase())
            {
                results.push(SearchResult {
                    id: item.id,
                    score: 1.0,
                    payload: item.payload,
                });
            }
        }

        results.truncate(limit);
        Ok(results)
    }

    async fn get_all_items(&self) -> Result<Vec<VectorItem>> {
        let stats = self.vector_store.stats().await?;
        let limit = stats.total_vectors.min(1000);

        let dummy_vector = vec![0.0; self.embedding_dimension];
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
                result.score /= total_weight;
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

    pub async fn unified_search(
        &self,
        query_text: &str,
        query_vector: Option<Vec<f32>>,
        config: &HybridSearchConfig,
    ) -> Result<Vec<UnifiedSearchResult>> {
        let mut vector_results: Vec<UnifiedSearchResult> = Vec::new();
        let mut bm25_results: Vec<UnifiedSearchResult> = Vec::new();
        let mut kg_results: Vec<UnifiedSearchResult> = Vec::new();

        if config.enable_vector && config.vector_weight > 0.0 {
            if let Some(vector) = query_vector {
                let mut query = SearchQuery::new(vector);
                query.limit = config.limit;
                query.min_score = config.min_score;

                let results = self.vector_store.search(query).await?;
                vector_results = results
                    .into_iter()
                    .map(|r| {
                        let content = r
                            .payload
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        UnifiedSearchResult::new(r.id, content, r.score, SearchSource::Vector)
                    })
                    .collect();
            }
        }

        if config.enable_bm25 && config.bm25_weight > 0.0 {
            if let Some(ref bm25) = self.bm25_index {
                let results = bm25.search(query_text, config.limit);
                if let Ok(results) = results {
                    bm25_results = results
                        .into_iter()
                        .map(|r| {
                            UnifiedSearchResult::new(r.id, r.content, r.score, SearchSource::Bm25)
                        })
                        .collect();
                }
            }
        }

        if config.enable_knowledge_graph && config.knowledge_graph_weight > 0.0 {
            if let Some(ref kg) = self.knowledge_graph {
                let kg_guard = kg.read().await;
                let query_lower = query_text.to_lowercase();
                let query_terms: Vec<&str> = query_lower.split_whitespace().collect();
                kg_results = self.knowledge_graph_search(&kg_guard, &query_terms);
            }
        }

        let unified_config = UnifiedSearchConfig {
            vector_weight: config.vector_weight,
            bm25_weight: config.bm25_weight,
            knowledge_graph_weight: config.knowledge_graph_weight,
            min_score: config.min_score.unwrap_or(0.0),
            max_results: config.limit,
            enable_vector: config.enable_vector,
            enable_bm25: config.enable_bm25,
            enable_knowledge_graph: config.enable_knowledge_graph,
        };

        let fusion = ResultFusion::new(unified_config, FusionStrategy::Weighted);
        let fused = fusion.fuse(&[vector_results, bm25_results, kg_results]);

        Ok(fused)
    }

    fn knowledge_graph_search(
        &self,
        kg: &KnowledgeGraph,
        query_terms: &[&str],
    ) -> Vec<UnifiedSearchResult> {
        let entities = kg.search_entities(query_terms);
        let mut results: Vec<UnifiedSearchResult> = Vec::new();

        for entity in entities {
            let content = format!("{}: {:?}", entity.name, entity.properties);
            results.push(UnifiedSearchResult::new(
                entity.id,
                content,
                entity.confidence,
                SearchSource::KnowledgeGraph,
            ));
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}
