use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SearchSource {
    Vector,
    Bm25,
    KnowledgeGraph,
    Fusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: SearchSource,
    pub source_scores: HashMap<SearchSource, f32>,
    pub metadata: HashMap<String, String>,
}

impl UnifiedSearchResult {
    pub fn new(id: String, content: String, score: f32, source: SearchSource) -> Self {
        let mut source_scores = HashMap::new();
        source_scores.insert(source.clone(), score);
        
        Self {
            id,
            content,
            score,
            source,
            source_scores,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn add_source_score(&mut self, source: SearchSource, score: f32) {
        self.source_scores.insert(source, score);
    }

    pub fn get_source_score(&self, source: &SearchSource) -> Option<f32> {
        self.source_scores.get(source).copied()
    }

    pub fn merge(&mut self, other: UnifiedSearchResult) {
        self.score = self.score.max(other.score);
        
        for (source, score) in other.source_scores {
            let existing = self.source_scores.get(&source).copied().unwrap_or(0.0);
            self.source_scores.insert(source, existing.max(score));
        }
        
        for (key, value) in other.metadata {
            self.metadata.entry(key).or_insert(value);
        }
    }
}

impl From<(String, String, f32, SearchSource)> for UnifiedSearchResult {
    fn from((id, content, score, source): (String, String, f32, SearchSource)) -> Self {
        UnifiedSearchResult::new(id, content, score, source)
    }
}
