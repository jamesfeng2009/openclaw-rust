//! Knowledge Graph - 技能知识图谱
//!
//! 存储和管理学习的知识，建立技能之间的关联

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::pattern_analyzer::{TaskPattern, ToolCallPattern};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillNode {
    pub skill_id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub tool_sequence: Vec<ToolCallPattern>,
    pub usage_count: u32,
    pub success_rate: f64,
    pub learned_from: String,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEdge {
    pub from_skill_id: String,
    pub to_skill_id: String,
    pub edge_type: EdgeType,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeType {
    Similar,
    DependsOn,
    Composes,
    VariantOf,
}

#[derive(Serialize, Deserialize)]
pub struct KnowledgeGraph {
    nodes: HashMap<String, SkillNode>,
    edges: HashMap<String, Vec<SkillEdge>>,
    category_index: HashMap<String, HashSet<String>>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            category_index: HashMap::new(),
        }
    }

    pub fn add_skill(&mut self, skill: SkillNode) {
        let skill_id = skill.skill_id.clone();
        let category = skill.category.clone();

        self.nodes.insert(skill_id.clone(), skill);

        self.category_index
            .entry(category)
            .or_insert_with(HashSet::new)
            .insert(skill_id);
    }

    pub fn add_learned_skill(
        &mut self,
        skill_id: &str,
        name: &str,
        pattern: &TaskPattern,
        metadata: HashMap<String, String>,
    ) {
        let node = SkillNode {
            skill_id: skill_id.to_string(),
            name: name.to_string(),
            category: pattern.task_category.clone(),
            description: format!("Auto-learned skill from pattern: {}", pattern.task_category),
            tool_sequence: pattern.tool_sequence.clone(),
            usage_count: 0,
            success_rate: 1.0,
            learned_from: pattern.source_task_id.clone(),
            created_at: Utc::now(),
            last_used: None,
            metadata,
        };

        self.add_skill(node);

        self.link_similar_skills(skill_id, pattern);
    }

    fn link_similar_skills(&mut self, skill_id: &str, pattern: &TaskPattern) {
        if let Some(existing) = self.nodes.get(skill_id) {
            let category = &existing.category;

            if let Some(similar_ids) = self.category_index.get(category) {
                for other_id in similar_ids {
                    if other_id != skill_id {
                        if let Some(other) = self.nodes.get(other_id) {
                            let similarity = self.calculate_similarity(&pattern.tool_sequence, &other.tool_sequence);

                            if similarity > 0.5 {
                                let edge = SkillEdge {
                                    from_skill_id: skill_id.to_string(),
                                    to_skill_id: other_id.clone(),
                                    edge_type: EdgeType::Similar,
                                    weight: similarity,
                                };

                                self.edges
                                    .entry(skill_id.to_string())
                                    .or_insert_with(Vec::new)
                                    .push(edge);
                            }
                        }
                    }
                }
            }
        }
    }

    fn calculate_similarity(&self, seq1: &[ToolCallPattern], seq2: &[ToolCallPattern]) -> f64 {
        if seq1.is_empty() && seq2.is_empty() {
            return 1.0;
        }
        if seq1.is_empty() || seq2.is_empty() {
            return 0.0;
        }

        let max_len = seq1.len().max(seq2.len());
        let mut matches = 0usize;

        for item1 in seq1 {
            for item2 in seq2 {
                if item1.tool_name == item2.tool_name {
                    matches += 1;
                    break;
                }
            }
        }

        matches as f64 / max_len as f64
    }

    pub fn find_similar(&self, pattern: &TaskPattern) -> Option<&SkillNode> {
        let category = &pattern.task_category;

        if let Some(similar_ids) = self.category_index.get(category) {
            let mut best_match: Option<(&SkillNode, f64)> = None;

            for id in similar_ids {
                if let Some(node) = self.nodes.get(id) {
                    let similarity = self.calculate_similarity(&pattern.tool_sequence, &node.tool_sequence);

                    match best_match {
                        None => best_match = Some((node, similarity)),
                        Some((_, best_sim)) if similarity > best_sim => {
                            best_match = Some((node, similarity));
                        }
                        _ => {}
                    }
                }
            }

            if let Some((node, sim)) = best_match {
                if sim > 0.5 {
                    return Some(node);
                }
            }
        }

        None
    }

    pub fn find_by_category(&self, category: &str) -> Vec<&SkillNode> {
        self.category_index
            .get(category)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SkillNode> {
        self.nodes.values().find(|n| n.name == name)
    }

    pub fn record_usage(&mut self, skill_id: &str, success: bool) {
        if let Some(node) = self.nodes.get_mut(skill_id) {
            node.usage_count += 1;

            let total = node.usage_count as f64;
            let current_successes = node.success_rate * (total - 1.0);
            let new_successes = if success { current_successes + 1.0 } else { current_successes };
            node.success_rate = new_successes / total;

            node.last_used = Some(Utc::now());
        }
    }

    pub fn get_skill(&self, skill_id: &str) -> Option<&SkillNode> {
        self.nodes.get(skill_id)
    }

    pub fn get_all_skills(&self) -> Vec<&SkillNode> {
        self.nodes.values().collect()
    }

    pub fn get_skills_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_related_skills(&self, skill_id: &str) -> Vec<&SkillNode> {
        self.edges
            .get(skill_id)
            .map(|edges| {
                edges
                    .iter()
                    .filter_map(|e| self.nodes.get(&e.to_skill_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_popular_skills(&self, limit: usize) -> Vec<&SkillNode> {
        let mut skills: Vec<_> = self.nodes.values().collect();
        skills.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        skills.truncate(limit);
        skills
    }

    pub fn get_reliable_skills(&self, min_success_rate: f64) -> Vec<&SkillNode> {
        self.nodes
            .values()
            .filter(|n| n.success_rate >= min_success_rate)
            .collect()
    }

    pub fn remove_skill(&mut self, skill_id: &str) -> bool {
        if self.nodes.remove(skill_id).is_some() {
            self.edges.remove(skill_id);

            for edges in self.edges.values_mut() {
                edges.retain(|e| e.to_skill_id != skill_id);
            }

            for (_, set) in self.category_index.iter_mut() {
                set.remove(skill_id);
            }

            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.category_index.clear();
    }

    pub fn get_statistics(&self) -> GraphStatistics {
        let total_skills = self.nodes.len();
        let total_edges = self.edges.values().map(|e| e.len()).sum();

        let mut category_counts: HashMap<String, usize> = HashMap::new();
        for (_, set) in &self.category_index {
            *category_counts.entry("total".to_string()).or_insert(0) += set.len();
        }

        let avg_usage: f64 = if total_skills > 0 {
            self.nodes.values().map(|n| n.usage_count as f64).sum::<f64>() / total_skills as f64
        } else {
            0.0
        };

        let avg_success_rate: f64 = if total_skills > 0 {
            self.nodes.values().map(|n| n.success_rate).sum::<f64>() / total_skills as f64
        } else {
            0.0
        };

        GraphStatistics {
            total_skills,
            total_edges,
            category_counts,
            avg_usage,
            avg_success_rate,
        }
    }

    pub fn add_edge(&mut self, edge: SkillEdge) {
        self.edges
            .entry(edge.from_skill_id.clone())
            .or_insert_with(Vec::new)
            .push(edge);
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let graph: KnowledgeGraph = serde_json::from_str(&json).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(graph)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub total_skills: usize,
    pub total_edges: usize,
    pub category_counts: HashMap<String, usize>,
    pub avg_usage: f64,
    pub avg_success_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(skill_id: &str, category: &str) -> SkillNode {
        SkillNode {
            skill_id: skill_id.to_string(),
            name: skill_id.to_string(),
            category: category.to_string(),
            description: "Test skill".to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "search".to_string(),
                    param_schema: std::collections::HashMap::new(),
                    result_schema: std::collections::HashMap::new(),
                },
            ],
            usage_count: 0,
            success_rate: 1.0,
            learned_from: "test".to_string(),
            created_at: Utc::now(),
            last_used: None,
            metadata: HashMap::new(),
        }
    }

    fn create_test_pattern(category: &str) -> TaskPattern {
        TaskPattern {
            id: "pattern-1".to_string(),
            task_category: category.to_string(),
            tool_sequence: vec![
                ToolCallPattern {
                    tool_name: "search".to_string(),
                    param_schema: std::collections::HashMap::new(),
                    result_schema: std::collections::HashMap::new(),
                },
            ],
            param_patterns: vec![],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.8,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_add_skill() {
        let mut graph = KnowledgeGraph::new();

        let node = create_test_node("skill-1", "search");
        graph.add_skill(node);

        assert_eq!(graph.get_skills_count(), 1);
    }

    #[test]
    fn test_add_learned_skill() {
        let mut graph = KnowledgeGraph::new();

        let pattern = create_test_pattern("search");
        graph.add_learned_skill("skill-1", "search_skill", &pattern, HashMap::new());

        assert_eq!(graph.get_skills_count(), 1);

        let skill = graph.get_skill("skill-1").unwrap();
        assert_eq!(skill.name, "search_skill");
        assert_eq!(skill.category, "search");
    }

    #[test]
    fn test_find_by_category() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));
        graph.add_skill(create_test_node("skill-2", "search"));
        graph.add_skill(create_test_node("skill-3", "api_call"));

        let search_skills = graph.find_by_category("search");
        assert_eq!(search_skills.len(), 2);

        let api_skills = graph.find_by_category("api_call");
        assert_eq!(api_skills.len(), 1);
    }

    #[test]
    fn test_find_by_name() {
        let mut graph = KnowledgeGraph::new();

        let node = create_test_node("skill-1", "search");
        graph.add_skill(node);

        let found = graph.find_by_name("skill-1");
        assert!(found.is_some());

        let not_found = graph.find_by_name("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_record_usage() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));

        graph.record_usage("skill-1", true);
        graph.record_usage("skill-1", true);
        graph.record_usage("skill-1", false);

        let skill = graph.get_skill("skill-1").unwrap();
        assert_eq!(skill.usage_count, 3);
        assert!((skill.success_rate - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_find_similar() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));

        let pattern = create_test_pattern("search");
        let similar = graph.find_similar(&pattern);

        assert!(similar.is_some());
        assert_eq!(similar.unwrap().skill_id, "skill-1");
    }

    #[test]
    fn test_get_related_skills() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));
        graph.add_skill(create_test_node("skill-2", "search"));

        let pattern = create_test_pattern("search");
        graph.add_learned_skill("skill-3", "search_skill_3", &pattern, HashMap::new());

        let related = graph.get_related_skills("skill-3");
        assert!(!related.is_empty());
    }

    #[test]
    fn test_get_popular_skills() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));
        graph.add_skill(create_test_node("skill-2", "search"));
        graph.add_skill(create_test_node("skill-3", "search"));

        graph.record_usage("skill-1", true);
        graph.record_usage("skill-1", true);
        graph.record_usage("skill-2", true);

        let popular = graph.get_popular_skills(2);
        assert_eq!(popular.len(), 2);
        assert_eq!(popular[0].skill_id, "skill-1");
    }

    #[test]
    fn test_get_reliable_skills() {
        let mut graph = KnowledgeGraph::new();

        let mut node1 = create_test_node("skill-1", "search");
        node1.success_rate = 0.9;
        graph.add_skill(node1);

        let mut node2 = create_test_node("skill-2", "search");
        node2.success_rate = 0.5;
        graph.add_skill(node2);

        let reliable = graph.get_reliable_skills(0.8);
        assert_eq!(reliable.len(), 1);
        assert_eq!(reliable[0].skill_id, "skill-1");
    }

    #[test]
    fn test_remove_skill() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));

        assert_eq!(graph.get_skills_count(), 1);

        let removed = graph.remove_skill("skill-1");
        assert!(removed);
        assert_eq!(graph.get_skills_count(), 0);
    }

    #[test]
    fn test_clear() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));
        graph.add_skill(create_test_node("skill-2", "api_call"));

        graph.clear();

        assert_eq!(graph.get_skills_count(), 0);
    }

    #[test]
    fn test_get_statistics() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));

        let stats = graph.get_statistics();

        assert_eq!(stats.total_skills, 1);
        assert_eq!(stats.total_edges, 0);
    }

    #[test]
    fn test_calculate_similarity() {
        let graph = KnowledgeGraph::new();

        let seq1 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
            ToolCallPattern {
                tool_name: "fetch".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];

        let seq2 = vec![
            ToolCallPattern {
                tool_name: "search".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
            ToolCallPattern {
                tool_name: "fetch".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];

        let seq3 = vec![
            ToolCallPattern {
                tool_name: "different".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            },
        ];

        let sim_12 = graph.calculate_similarity(&seq1, &seq2);
        assert!(sim_12 > 0.9);

        let sim_13 = graph.calculate_similarity(&seq1, &seq3);
        assert!(sim_13 < 0.5);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = KnowledgeGraph::new();

        graph.add_skill(create_test_node("skill-1", "search"));
        graph.add_skill(create_test_node("skill-2", "search"));

        let edge = SkillEdge {
            from_skill_id: "skill-1".to_string(),
            to_skill_id: "skill-2".to_string(),
            edge_type: EdgeType::Similar,
            weight: 0.8,
        };
        graph.add_edge(edge);

        let stats = graph.get_statistics();
        assert_eq!(stats.total_edges, 1);
    }

    #[test]
    fn test_save_load() {
        let mut graph = KnowledgeGraph::new();
        graph.add_skill(create_test_node("skill-1", "search"));

        let path = "/tmp/test_knowledge_graph.json";
        graph.save_to_file(path).unwrap();

        let loaded = KnowledgeGraph::load_from_file(path).unwrap();
        assert_eq!(loaded.get_skills_count(), 1);

        std::fs::remove_file(path).ok();
    }
}
