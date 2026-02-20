use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchConfig {
    pub vector_weight: f32,
    pub bm25_weight: f32,
    pub knowledge_graph_weight: f32,
    pub min_score: f32,
    pub max_results: usize,
    pub enable_vector: bool,
    pub enable_bm25: bool,
    pub enable_knowledge_graph: bool,
}

impl Default for UnifiedSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.5,
            bm25_weight: 0.3,
            knowledge_graph_weight: 0.2,
            min_score: 0.1,
            max_results: 10,
            enable_vector: true,
            enable_bm25: true,
            enable_knowledge_graph: true,
        }
    }
}

impl UnifiedSearchConfig {
    pub fn new(
        vector_weight: f32,
        bm25_weight: f32,
        knowledge_graph_weight: f32,
    ) -> Self {
        Self {
            vector_weight,
            bm25_weight,
            knowledge_graph_weight,
            ..Default::default()
        }
    }

    pub fn validate(&self) -> bool {
        let total = self.vector_weight + self.bm25_weight + self.knowledge_graph_weight;
        (total - 1.0).abs() < 0.001
            && self.vector_weight >= 0.0
            && self.bm25_weight >= 0.0
            && self.knowledge_graph_weight >= 0.0
            && self.min_score >= 0.0
            && self.max_results > 0
    }
}
