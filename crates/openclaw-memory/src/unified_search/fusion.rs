use super::config::UnifiedSearchConfig;
use super::result::{SearchSource, UnifiedSearchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum FusionStrategy {
    Weighted,
    RRF,
    MaxScore,
    Average,
}

impl Default for FusionStrategy {
    fn default() -> Self {
        FusionStrategy::Weighted
    }
}

pub struct ResultFusion {
    config: UnifiedSearchConfig,
    strategy: FusionStrategy,
}

impl ResultFusion {
    pub fn new(config: UnifiedSearchConfig, strategy: FusionStrategy) -> Self {
        Self { config, strategy }
    }

    pub fn with_default_strategy(config: UnifiedSearchConfig) -> Self {
        Self {
            config,
            strategy: FusionStrategy::Weighted,
        }
    }

    pub fn fuse(&self, results: &[Vec<UnifiedSearchResult>]) -> Vec<UnifiedSearchResult> {
        if results.is_empty() || results.iter().all(|r| r.is_empty()) {
            return Vec::new();
        }

        let enabled_results: Vec<&Vec<UnifiedSearchResult>> = results
            .iter()
            .filter(|r| !r.is_empty())
            .collect();

        if enabled_results.is_empty() {
            return Vec::new();
        }

        if enabled_results.len() == 1 {
            return enabled_results[0][..self.config.max_results.min(enabled_results[0].len())]
                .to_vec();
        }

        match self.strategy {
            FusionStrategy::Weighted => self.fuse_weighted(&enabled_results),
            FusionStrategy::RRF => self.fuse_rrf(&enabled_results),
            FusionStrategy::MaxScore => self.fuse_max_score(&enabled_results),
            FusionStrategy::Average => self.fuse_average(&enabled_results),
        }
    }

    fn fuse_weighted(&self, results: &Vec<&Vec<UnifiedSearchResult>>) -> Vec<UnifiedSearchResult> {
        let mut merged: HashMap<String, UnifiedSearchResult> = HashMap::new();

        for (idx, result_set) in results.iter().enumerate() {
            let weight = match idx {
                0 => self.config.vector_weight,
                1 => self.config.bm25_weight,
                2 => self.config.knowledge_graph_weight,
                _ => 0.0,
            };

            if weight <= 0.0 {
                continue;
            }

            for item in result_set.iter() {
                let weighted_score = item.score * weight;
                
                if let Some(existing) = merged.get_mut(&item.id) {
                    if weighted_score > existing.score {
                        existing.score = weighted_score;
                    }
                    existing.add_source_score(item.source.clone(), item.score);
                } else {
                    let mut new_item = UnifiedSearchResult::new(
                        item.id.clone(),
                        item.content.clone(),
                        weighted_score,
                        SearchSource::Fusion,
                    );
                    new_item.add_source_score(item.source.clone(), item.score);
                    merged.insert(item.id.clone(), new_item);
                }
            }
        }

        let mut fused: Vec<UnifiedSearchResult> = merged.into_values().collect();
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(self.config.max_results);
        fused
    }

    fn fuse_rrf(&self, results: &Vec<&Vec<UnifiedSearchResult>>) -> Vec<UnifiedSearchResult> {
        let k = 60;
        let mut rrf_scores: HashMap<String, (f32, UnifiedSearchResult)> = HashMap::new();

        for result_set in results.iter() {
            for (rank, item) in result_set.iter().enumerate() {
                let rrf_score = 1.0 / (k + rank + 1) as f32;
                
                if let Some((score, existing)) = rrf_scores.get_mut(&item.id) {
                    *score += rrf_score;
                    if item.score > existing.score {
                        existing.score = item.score;
                    }
                    existing.add_source_score(item.source.clone(), item.score);
                } else {
                    let mut new_item = UnifiedSearchResult::new(
                        item.id.clone(),
                        item.content.clone(),
                        rrf_score,
                        SearchSource::Fusion,
                    );
                    new_item.add_source_score(item.source.clone(), item.score);
                    rrf_scores.insert(item.id.clone(), (rrf_score, new_item));
                }
            }
        }

        let mut fused: Vec<UnifiedSearchResult> = rrf_scores
            .into_values()
            .map(|(_, mut item)| {
                item.source = SearchSource::Fusion;
                item
            })
            .collect();
        
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(self.config.max_results);
        fused
    }

    fn fuse_max_score(&self, results: &Vec<&Vec<UnifiedSearchResult>>) -> Vec<UnifiedSearchResult> {
        let mut merged: HashMap<String, UnifiedSearchResult> = HashMap::new();

        for result_set in results.iter() {
            for item in result_set.iter() {
                if let Some(existing) = merged.get_mut(&item.id) {
                    if item.score > existing.score {
                        existing.score = item.score;
                        existing.content = item.content.clone();
                    }
                    existing.add_source_score(item.source.clone(), item.score);
                } else {
                    let mut new_item = UnifiedSearchResult::new(
                        item.id.clone(),
                        item.content.clone(),
                        item.score,
                        SearchSource::Fusion,
                    );
                    new_item.add_source_score(item.source.clone(), item.score);
                    merged.insert(item.id.clone(), new_item);
                }
            }
        }

        let mut fused: Vec<UnifiedSearchResult> = merged.into_values().collect();
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(self.config.max_results);
        fused
    }

    fn fuse_average(&self, results: &Vec<&Vec<UnifiedSearchResult>>) -> Vec<UnifiedSearchResult> {
        let mut merged: HashMap<String, (f32, usize, UnifiedSearchResult)> = HashMap::new();

        for result_set in results.iter() {
            for item in result_set.iter() {
                if let Some((sum, count, existing)) = merged.get_mut(&item.id) {
                    *sum += item.score;
                    *count += 1;
                    existing.add_source_score(item.source.clone(), item.score);
                } else {
                    let mut new_item = UnifiedSearchResult::new(
                        item.id.clone(),
                        item.content.clone(),
                        item.score,
                        SearchSource::Fusion,
                    );
                    new_item.add_source_score(item.source.clone(), item.score);
                    merged.insert(item.id.clone(), (item.score, 1, new_item));
                }
            }
        }

        let fused: Vec<UnifiedSearchResult> = merged
            .into_values()
            .map(|(sum, count, mut item)| {
                item.score = sum / count as f32;
                item.source = SearchSource::Fusion;
                item
            })
            .collect();
        
        let mut fused = fused;
        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        fused.truncate(self.config.max_results);
        fused
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_result(id: &str, content: &str, score: f32, source: SearchSource) -> UnifiedSearchResult {
        UnifiedSearchResult::new(id.to_string(), content.to_string(), score, source)
    }

    #[test]
    fn test_fusion_config_default() {
        let config = UnifiedSearchConfig::default();
        assert!(config.validate());
        assert_eq!(config.vector_weight, 0.5);
        assert_eq!(config.bm25_weight, 0.3);
        assert_eq!(config.knowledge_graph_weight, 0.2);
    }

    #[test]
    fn test_fusion_config_custom() {
        let config = UnifiedSearchConfig::new(0.6, 0.25, 0.15);
        assert!(config.validate());
        assert_eq!(config.vector_weight, 0.6);
    }

    #[test]
    fn test_fusion_config_invalid() {
        let config = UnifiedSearchConfig {
            vector_weight: 0.5,
            bm25_weight: 0.3,
            knowledge_graph_weight: 0.5,
            ..Default::default()
        };
        assert!(!config.validate());
    }

    #[test]
    fn test_fusion_weighted_empty() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::Weighted);
        let result = fusion.fuse(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_fusion_weighted_single_source() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::Weighted);
        
        let vector_results = vec![
            create_test_result("1", "content 1", 0.9, SearchSource::Vector),
            create_test_result("2", "content 2", 0.8, SearchSource::Vector),
        ];
        
        let result = fusion.fuse(&[vector_results]);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "1");
    }

    #[test]
    fn test_fusion_weighted_multiple_sources() {
        let config = UnifiedSearchConfig {
            vector_weight: 0.5,
            bm25_weight: 0.3,
            knowledge_graph_weight: 0.2,
            max_results: 10,
            ..Default::default()
        };
        let fusion = ResultFusion::new(config, FusionStrategy::Weighted);
        
        let vector_results = vec![
            create_test_result("1", "content 1", 0.9, SearchSource::Vector),
            create_test_result("2", "content 2", 0.7, SearchSource::Vector),
        ];
        
        let bm25_results = vec![
            create_test_result("1", "content 1", 0.8, SearchSource::Bm25),
            create_test_result("3", "content 3", 0.6, SearchSource::Bm25),
        ];
        
        let result = fusion.fuse(&[vector_results, bm25_results]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_fusion_rrf() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::RRF);
        
        let results1 = vec![
            create_test_result("1", "content 1", 0.9, SearchSource::Vector),
            create_test_result("2", "content 2", 0.8, SearchSource::Vector),
        ];
        
        let results2 = vec![
            create_test_result("1", "content 1", 0.7, SearchSource::Bm25),
            create_test_result("3", "content 3", 0.6, SearchSource::Bm25),
        ];
        
        let result = fusion.fuse(&[results1, results2]);
        assert!(result.len() <= 10);
    }

    #[test]
    fn test_fusion_max_score() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::MaxScore);
        
        let results1 = vec![
            create_test_result("1", "content 1", 0.5, SearchSource::Vector),
        ];
        
        let results2 = vec![
            create_test_result("1", "content 1", 0.9, SearchSource::Bm25),
        ];
        
        let result = fusion.fuse(&[results1, results2]);
        assert_eq!(result.len(), 1);
        assert!((result[0].score - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_fusion_average() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::Average);
        
        let results1 = vec![
            create_test_result("1", "content 1", 0.8, SearchSource::Vector),
        ];
        
        let results2 = vec![
            create_test_result("1", "content 1", 0.4, SearchSource::Bm25),
        ];
        
        let result = fusion.fuse(&[results1, results2]);
        assert_eq!(result.len(), 1);
        assert!((result[0].score - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_search_result_creation() {
        let result = create_test_result("1", "test content", 0.5, SearchSource::Vector);
        assert_eq!(result.id, "1");
        assert_eq!(result.content, "test content");
        assert_eq!(result.score, 0.5);
        assert_eq!(result.source, SearchSource::Vector);
    }

    #[test]
    fn test_search_result_metadata() {
        let result = create_test_result("1", "test", 0.5, SearchSource::Vector)
            .with_metadata("key1", "value1")
            .with_metadata("key2", "value2");
        
        assert_eq!(result.metadata.get("key1"), Some(&"value1".to_string()));
        assert_eq!(result.metadata.get("key2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_search_result_source_score() {
        let mut result = create_test_result("1", "test", 0.5, SearchSource::Vector);
        result.add_source_score(SearchSource::Bm25, 0.8);
        
        assert_eq!(result.get_source_score(&SearchSource::Vector), Some(0.5));
        assert_eq!(result.get_source_score(&SearchSource::Bm25), Some(0.8));
    }

    #[test]
    fn test_search_source_equality() {
        assert_eq!(SearchSource::Vector, SearchSource::Vector);
        assert_ne!(SearchSource::Vector, SearchSource::Bm25);
        assert_eq!(SearchSource::Fusion, SearchSource::Fusion);
    }

    #[test]
    fn test_fusion_strategy_default() {
        assert_eq!(FusionStrategy::default(), FusionStrategy::Weighted);
    }

    #[test]
    fn test_config_enable_flags() {
        let config = UnifiedSearchConfig {
            enable_vector: true,
            enable_bm25: false,
            enable_knowledge_graph: true,
            ..Default::default()
        };
        
        assert!(config.enable_vector);
        assert!(!config.enable_bm25);
        assert!(config.enable_knowledge_graph);
    }

    #[test]
    fn test_max_results_limit() {
        let config = UnifiedSearchConfig {
            max_results: 3,
            ..Default::default()
        };
        let fusion = ResultFusion::new(config, FusionStrategy::Weighted);
        
        let results = vec![
            create_test_result("1", "content 1", 0.9, SearchSource::Vector),
            create_test_result("2", "content 2", 0.8, SearchSource::Vector),
            create_test_result("3", "content 3", 0.7, SearchSource::Vector),
            create_test_result("4", "content 4", 0.6, SearchSource::Vector),
            create_test_result("5", "content 5", 0.5, SearchSource::Vector),
        ];
        
        let result = fusion.fuse(&[results]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_empty_result_sets() {
        let config = UnifiedSearchConfig::default();
        let fusion = ResultFusion::new(config, FusionStrategy::Weighted);
        
        let empty1: Vec<UnifiedSearchResult> = Vec::new();
        let empty2: Vec<UnifiedSearchResult> = Vec::new();
        
        let result = fusion.fuse(&[empty1, empty2]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_result_merge() {
        let mut result1 = create_test_result("1", "content", 0.5, SearchSource::Vector);
        let result2 = create_test_result("1", "content", 0.8, SearchSource::Bm25);
        
        result1.merge(result2);
        
        assert_eq!(result1.score, 0.8);
    }
}
