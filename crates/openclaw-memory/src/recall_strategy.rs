use serde::{Deserialize, Serialize};
use chrono::{Utc, Duration};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallItem {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: String,
    pub timestamp: i64,
    pub importance: f32,
    pub access_count: u32,
    pub last_access: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct RecallConfig {
    pub vector_weight: f32,
    pub bm25_weight: f32,
    pub recency_weight: f32,
    pub importance_weight: f32,
    pub decay_half_life_days: f32,
    pub min_score: f32,
    pub max_results: usize,
}

impl Default for RecallConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.4,
            bm25_weight: 0.3,
            recency_weight: 0.15,
            importance_weight: 0.15,
            decay_half_life_days: 7.0,
            min_score: 0.1,
            max_results: 10,
        }
    }
}

pub struct RecallStrategy {
    config: RecallConfig,
    access_history: HashMap<String, AccessRecord>,
}

#[derive(Debug, Clone)]
struct AccessRecord {
    count: u32,
    last_access: i64,
}

impl RecallStrategy {
    pub fn new(config: RecallConfig) -> Self {
        Self {
            config,
            access_history: HashMap::new(),
        }
    }

    pub fn with_default() -> Self {
        Self::new(RecallConfig::default())
    }

    pub fn rerank(&self, items: Vec<RecallItem>, _query: &str) -> Vec<RecallItem> {
        let now = Utc::now().timestamp();
        
        let mut scored_items: Vec<(RecallItem, f32)> = items
            .into_iter()
            .map(|mut item| {
                let recency_score = self.calculate_recency_score(item.timestamp, now);
                let access_score = self.calculate_access_score(&item.id);
                let final_score = self.calculate_final_score(
                    item.score,
                    recency_score,
                    access_score,
                    item.importance,
                );
                
                item.last_access = Some(now);
                
                (item, final_score)
            })
            .collect();
        
        scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        scored_items
            .into_iter()
            .filter(|(_, score)| *score >= self.config.min_score)
            .take(self.config.max_results)
            .map(|(item, _)| item)
            .collect()
    }

    fn calculate_recency_score(&self, timestamp: i64, now: i64) -> f32 {
        let age_days = (now - timestamp) as f32 / (24.0 * 60.0 * 60.0);
        
        let half_life = self.config.decay_half_life_days;
        let score = 0.5f32.powf(age_days / half_life);
        
        score.max(0.0).min(1.0)
    }

    fn calculate_access_score(&self, id: &str) -> f32 {
        if let Some(record) = self.access_history.get(id) {
            let count_factor = (record.count as f32).sqrt() / 10.0;
            count_factor.min(1.0)
        } else {
            0.0
        }
    }

    fn calculate_final_score(
        &self,
        relevance: f32,
        recency: f32,
        access: f32,
        importance: f32,
    ) -> f32 {
        let base = relevance * (self.config.vector_weight + self.config.bm25_weight);
        let recency_contrib = recency * self.config.recency_weight;
        let access_contrib = access * self.config.importance_weight * 0.5;
        let importance_contrib = importance * self.config.importance_weight;
        
        base + recency_contrib + access_contrib + importance_contrib
    }

    pub fn record_access(&mut self, id: &str) {
        let now = Utc::now().timestamp();
        
        if let Some(record) = self.access_history.get_mut(id) {
            record.count += 1;
            record.last_access = now;
        } else {
            self.access_history.insert(
                id.to_string(),
                AccessRecord {
                    count: 1,
                    last_access: now,
                },
            );
        }
    }

    pub fn get_access_count(&self, id: &str) -> u32 {
        self.access_history
            .get(id)
            .map(|r| r.count)
            .unwrap_or(0)
    }
}

pub struct HybridRecall {
    vector_weight: f32,
    bm25_weight: f32,
}

impl HybridRecall {
    pub fn new(vector_weight: f32, bm25_weight: f32) -> Self {
        let total = vector_weight + bm25_weight;
        Self {
            vector_weight: vector_weight / total,
            bm25_weight: bm25_weight / total,
        }
    }

    pub fn combine_scores(
        &self,
        vector_results: Vec<RecallItem>,
        bm25_results: Vec<RecallItem>,
    ) -> Vec<RecallItem> {
        let mut combined: HashMap<String, RecallItem> = HashMap::new();
        
        for item in vector_results {
            let score = item.score * self.vector_weight;
            combined.insert(
                item.id.clone(),
                RecallItem {
                    score,
                    ..item
                },
            );
        }
        
        for item in bm25_results {
            let id = item.id.clone();
            if let Some(existing) = combined.get_mut(&id) {
                existing.score += item.score * self.bm25_weight;
            } else {
                combined.insert(
                    id,
                    RecallItem {
                        score: item.score * self.bm25_weight,
                        ..item
                    },
                );
            }
        }
        
        let mut results: Vec<RecallItem> = combined.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        results
    }
}

pub struct TimeWindowRecall {
    window_days: i64,
}

impl TimeWindowRecall {
    pub fn new(window_days: i64) -> Self {
        Self { window_days }
    }

    pub fn filter_by_time(&self, items: Vec<RecallItem>) -> Vec<RecallItem> {
        let cutoff = Utc::now() - Duration::days(self.window_days);
        let cutoff_timestamp = cutoff.timestamp();
        
        items
            .into_iter()
            .filter(|item| item.timestamp >= cutoff_timestamp)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_recency_decay() {
        let strategy = RecallStrategy::with_default();
        
        let now = Utc::now().timestamp();
        let recent = strategy.calculate_recency_score(now, now);
        let old = strategy.calculate_recency_score(now - 7 * 24 * 60 * 60, now);
        
        assert!(recent > old);
    }
    
    #[test]
    fn test_hybrid_combine() {
        let hybrid = HybridRecall::new(0.6, 0.4);
        
        let vector_results = vec![
            RecallItem {
                id: "doc1".to_string(),
                content: "Rust programming".to_string(),
                score: 0.9,
                source: "memory".to_string(),
                timestamp: 1000,
                importance: 0.5,
                access_count: 1,
                last_access: None,
            },
        ];
        
        let bm25_results = vec![
            RecallItem {
                id: "doc2".to_string(),
                content: "Python programming".to_string(),
                score: 0.8,
                source: "memory".to_string(),
                timestamp: 1000,
                importance: 0.5,
                access_count: 1,
                last_access: None,
            },
        ];
        
        let combined = hybrid.combine_scores(vector_results, bm25_results);
        
        assert_eq!(combined.len(), 2);
    }
}
