//! 记忆冲突解决器
//!
//! 检测并解决记忆中的矛盾信息

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::fact_extractor::{AtomicFact, FactCategory};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    pub id: String,
    pub fact_a: AtomicFact,
    pub fact_b: AtomicFact,
    pub conflict_type: ConflictType,
    pub resolution: Option<ConflictResolution>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    Direct,
    Implied,
    Temporal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub winner: String,
    pub loser: String,
    pub reason: String,
    pub resolved_at: DateTime<Utc>,
    pub method: ResolutionMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionMethod {
    Latest,
    HigherConfidence,
    UserConfirmed,
    LLMDecision,
}

pub struct ConflictResolver {
    category_weights: HashMap<FactCategory, f32>,
}

impl ConflictResolver {
    pub fn new() -> Self {
        let mut weights = HashMap::new();
        weights.insert(FactCategory::UserPreference, 1.0);
        weights.insert(FactCategory::UserGoal, 1.0);
        weights.insert(FactCategory::Decision, 1.2);
        weights.insert(FactCategory::PersonalInfo, 0.9);
        weights.insert(FactCategory::UserBackground, 0.8);
        weights.insert(FactCategory::WorkInfo, 0.9);
        weights.insert(FactCategory::ProjectInfo, 1.0);
        weights.insert(FactCategory::Note, 0.7);
        weights.insert(FactCategory::Other, 0.5);

        Self {
            category_weights: weights,
        }
    }

    pub fn with_category_weight(mut self, category: FactCategory, weight: f32) -> Self {
        self.category_weights.insert(category, weight);
        self
    }

    pub fn detect_conflicts(&self, facts: &[AtomicFact]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        for (i, fact_a) in facts.iter().enumerate() {
            for fact_b in facts.iter().skip(i + 1) {
                if fact_a.is_contradicting(fact_b) {
                    let conflict = Conflict {
                        id: uuid::Uuid::new_v4().to_string(),
                        fact_a: fact_a.clone(),
                        fact_b: fact_b.clone(),
                        conflict_type: ConflictType::Direct,
                        resolution: None,
                        detected_at: Utc::now(),
                    };
                    conflicts.push(conflict);
                }
            }
        }

        conflicts
    }

    pub fn resolve_conflict(&self, conflict: &Conflict, method: ResolutionMethod) -> ConflictResolution {
        match method {
            ResolutionMethod::Latest => self.resolve_by_time(conflict),
            ResolutionMethod::HigherConfidence => self.resolve_by_confidence(conflict),
            ResolutionMethod::LLMDecision => self.resolve_by_llm(conflict),
            ResolutionMethod::UserConfirmed => {
                ConflictResolution {
                    winner: String::new(),
                    loser: String::new(),
                    reason: "等待用户确认".to_string(),
                    resolved_at: Utc::now(),
                    method: ResolutionMethod::UserConfirmed,
                }
            }
        }
    }

    fn resolve_by_time(&self, conflict: &Conflict) -> ConflictResolution {
        let winner = if conflict.fact_a.created_at > conflict.fact_b.created_at {
            conflict.fact_a.id.clone()
        } else {
            conflict.fact_b.id.clone()
        };

        let loser = if winner == conflict.fact_a.id {
            conflict.fact_b.id.clone()
        } else {
            conflict.fact_a.id.clone()
        };

        ConflictResolution {
            winner,
            loser,
            reason: "根据时间戳选择最新事实".to_string(),
            resolved_at: Utc::now(),
            method: ResolutionMethod::Latest,
        }
    }

    fn resolve_by_confidence(&self, conflict: &Conflict) -> ConflictResolution {
        let winner = if conflict.fact_a.confidence >= conflict.fact_b.confidence {
            conflict.fact_a.id.clone()
        } else {
            conflict.fact_b.id.clone()
        };

        let loser = if winner == conflict.fact_a.id {
            conflict.fact_b.id.clone()
        } else {
            conflict.fact_a.id.clone()
        };

        ConflictResolution {
            winner,
            loser,
            reason: "根据置信度选择更高置信度的事实".to_string(),
            resolved_at: Utc::now(),
            method: ResolutionMethod::HigherConfidence,
        }
    }

    fn resolve_by_llm(&self, _conflict: &Conflict) -> ConflictResolution {
        ConflictResolution {
            winner: String::new(),
            loser: String::new(),
            reason: "需要 LLM 决策".to_string(),
            resolved_at: Utc::now(),
            method: ResolutionMethod::LLMDecision,
        }
    }

    pub fn resolve_facts(&self, facts: &[AtomicFact], method: ResolutionMethod) -> Vec<AtomicFact> {
        let conflicts = self.detect_conflicts(facts);
        
        if conflicts.is_empty() {
            return facts.to_vec();
        }

        let mut resolved: HashMap<String, &AtomicFact> = HashMap::new();
        
        for fact in facts {
            resolved.insert(fact.id.clone(), fact);
        }

        for conflict in &conflicts {
            if let Some(resolution) = conflict.resolution.clone() {
                resolved.remove(&resolution.loser);
            } else {
                let auto_resolution = self.resolve_conflict(conflict, method.clone());
                resolved.remove(&auto_resolution.loser);
            }
        }

        resolved.into_values().cloned().collect()
    }

    pub fn weighted_resolve(&self, facts: &[AtomicFact]) -> Vec<AtomicFact> {
        let conflicts = self.detect_conflicts(facts);
        
        if conflicts.is_empty() {
            return facts.to_vec();
        }

        let mut fact_scores: HashMap<String, f32> = HashMap::new();
        
        for fact in facts {
            let category_weight = self.category_weights.get(&fact.category).copied().unwrap_or(0.5);
            let score = fact.confidence * category_weight;
            fact_scores.insert(fact.id.clone(), score);
        }

        for conflict in &conflicts {
            let score_a = fact_scores.get(&conflict.fact_a.id).copied().unwrap_or(0.0);
            let score_b = fact_scores.get(&conflict.fact_b.id).copied().unwrap_or(0.0);
            
            if score_a >= score_b {
                fact_scores.remove(&conflict.fact_b.id);
            } else {
                fact_scores.remove(&conflict.fact_a.id);
            }
        }

        facts.iter()
            .filter(|f| fact_scores.contains_key(&f.id))
            .cloned()
            .collect()
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fact(id: &str, content: &str, category: FactCategory, created_at: DateTime<Utc>, confidence: f32) -> AtomicFact {
        AtomicFact {
            id: id.to_string(),
            content: content.to_string(),
            category,
            source_message_id: None,
            created_at,
            confidence,
            is_negative: false,
        }
    }

    #[test]
    fn test_detect_direct_conflict() {
        let resolver = ConflictResolver::new();
        
        let fact1 = create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, Utc::now(), 1.0);
        let fact2 = create_test_fact("2", "用户不喜欢Python", FactCategory::UserPreference, Utc::now(), 1.0);
        
        let conflicts = resolver.detect_conflicts(&[fact1, fact2]);
        
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::Direct);
    }

    #[test]
    fn test_no_conflict_different_category() {
        let resolver = ConflictResolver::new();
        
        let fact1 = create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, Utc::now(), 1.0);
        let fact2 = create_test_fact("2", "用户在Google工作", FactCategory::WorkInfo, Utc::now(), 1.0);
        
        let conflicts = resolver.detect_conflicts(&[fact1, fact2]);
        
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_resolve_by_latest() {
        let resolver = ConflictResolver::new();
        
        let fact1 = create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, 
            Utc::now() - chrono::Duration::days(1), 0.8);
        let fact2 = create_test_fact("2", "用户不喜欢Python", FactCategory::UserPreference, 
            Utc::now(), 0.9);
        
        let conflict = Conflict {
            id: "c1".to_string(),
            fact_a: fact1.clone(),
            fact_b: fact2.clone(),
            conflict_type: ConflictType::Direct,
            resolution: None,
            detected_at: Utc::now(),
        };
        
        let resolution = resolver.resolve_conflict(&conflict, ResolutionMethod::Latest);
        
        assert_eq!(resolution.winner, "2");
        assert_eq!(resolution.method, ResolutionMethod::Latest);
    }

    #[test]
    fn test_resolve_by_confidence() {
        let resolver = ConflictResolver::new();
        
        let fact1 = create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, 
            Utc::now(), 0.8);
        let fact2 = create_test_fact("2", "用户不喜欢Python", FactCategory::UserPreference, 
            Utc::now(), 0.9);
        
        let conflict = Conflict {
            id: "c1".to_string(),
            fact_a: fact1.clone(),
            fact_b: fact2.clone(),
            conflict_type: ConflictType::Direct,
            resolution: None,
            detected_at: Utc::now(),
        };
        
        let resolution = resolver.resolve_conflict(&conflict, ResolutionMethod::HigherConfidence);
        
        assert_eq!(resolution.winner, "2");
        assert_eq!(resolution.method, ResolutionMethod::HigherConfidence);
    }

    #[test]
    fn test_resolve_facts() {
        let resolver = ConflictResolver::new();
        
        let facts = vec![
            create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, Utc::now(), 0.8),
            create_test_fact("2", "用户不喜欢Python", FactCategory::UserPreference, Utc::now(), 0.9),
            create_test_fact("3", "用户在Google工作", FactCategory::WorkInfo, Utc::now(), 1.0),
        ];
        
        let resolved = resolver.resolve_facts(&facts, ResolutionMethod::Latest);
        
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|f| f.id == "2"));
        assert!(resolved.iter().any(|f| f.id == "3"));
    }

    #[test]
    fn test_weighted_resolve() {
        let resolver = ConflictResolver::new();
        
        let facts = vec![
            create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, Utc::now(), 0.8),
            create_test_fact("2", "用户不喜欢Python", FactCategory::UserPreference, Utc::now(), 0.9),
            create_test_fact("3", "用户决定学习Rust", FactCategory::Decision, Utc::now(), 1.0),
        ];
        
        let resolved = resolver.weighted_resolve(&facts);
        
        assert_eq!(resolved.len(), 2);
        assert!(resolved.iter().any(|f| f.id == "3"));
    }

    #[test]
    fn test_no_conflicts_returns_original() {
        let resolver = ConflictResolver::new();
        
        let facts = vec![
            create_test_fact("1", "用户喜欢Python", FactCategory::UserPreference, Utc::now(), 0.8),
            create_test_fact("2", "用户在Google工作", FactCategory::WorkInfo, Utc::now(), 0.9),
        ];
        
        let resolved = resolver.resolve_facts(&facts, ResolutionMethod::Latest);
        
        assert_eq!(resolved.len(), 2);
    }

    #[test]
    fn test_category_weights() {
        let resolver = ConflictResolver::new();
        
        let decision_weight = resolver.category_weights.get(&FactCategory::Decision);
        assert!(decision_weight.is_some());
        assert!(*decision_weight.unwrap() > 1.0);
        
        let other_weight = resolver.category_weights.get(&FactCategory::Other);
        assert!(other_weight.is_some());
        assert!(*other_weight.unwrap() < 1.0);
    }
}
