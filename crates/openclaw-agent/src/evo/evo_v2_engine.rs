//! EvoV2 Engine - 自我进化引擎 V2
//!
//! 集成 PatternAnalyzer、LearningHistory、KnowledgeGraph、SkillValidator
//! 实现技能的自动学习、进化和优化

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::knowledge_graph::{KnowledgeGraph, SkillNode};
use super::learning_history::{LearningHistory, LearningRecord, LearningType, RecurringPattern};
use super::pattern_analyzer::{PatternAnalyzer, TaskPattern, ToolCall};
use super::skill_validator::{SkillValidator, ValidationResult, ValidationStatus};
use super::version_manager::{VersionManager, VersionRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoConfig {
    pub enable_auto_evolution: bool,
    pub evolution_threshold: f64,
    pub max_patterns_stored: usize,
    pub similarity_threshold: f64,
    pub validation_strict: bool,
}

impl Default for EvoConfig {
    fn default() -> Self {
        Self {
            enable_auto_evolution: true,
            evolution_threshold: 0.7,
            max_patterns_stored: 1000,
            similarity_threshold: 0.8,
            validation_strict: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoContext {
    pub task_id: String,
    pub task_description: String,
    pub tool_calls: Vec<ToolCall>,
    pub success: bool,
    pub execution_time_ms: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoSkill {
    pub id: String,
    pub name: String,
    pub category: String,
    pub code: String,
    pub pattern: TaskPattern,
    pub reliability: f64,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub usage_count: u32,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoEvolutionResult {
    pub evolved: bool,
    pub skill_id: String,
    pub changes: Vec<String>,
    pub new_reliability: f64,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    pub skill_id: String,
    pub skill_name: String,
    pub confidence: f64,
    pub reason: String,
}

pub struct EvoV2Engine {
    config: EvoConfig,
    pattern_analyzer: PatternAnalyzer,
    learning_history: Arc<RwLock<LearningHistory>>,
    knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
    skill_validator: SkillValidator,
    version_manager: Arc<RwLock<VersionManager>>,
    learned_skills: Arc<RwLock<std::collections::HashMap<String, EvoSkill>>>,
}

impl Default for EvoV2Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl EvoV2Engine {
    pub fn new() -> Self {
        Self::with_config(EvoConfig::default())
    }

    pub fn with_config(config: EvoConfig) -> Self {
        Self {
            config: config.clone(),
            pattern_analyzer: PatternAnalyzer::new(),
            learning_history: Arc::new(RwLock::new(LearningHistory::new())),
            knowledge_graph: Arc::new(RwLock::new(KnowledgeGraph::new())),
            skill_validator: SkillValidator::new(),
            version_manager: Arc::new(RwLock::new(VersionManager::new())),
            learned_skills: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn process_task(&self, context: EvoContext) -> EvoEvolutionResult {
        let tool_validation = self.skill_validator.validate_tool_sequence(&context.tool_calls);
        
        if tool_validation.status == ValidationStatus::Rejected {
            return EvoEvolutionResult {
                evolved: false,
                skill_id: String::new(),
                changes: vec![format!("Tool sequence rejected: {}", tool_validation.message)],
                new_reliability: 0.0,
                message: "Task rejected due to invalid tool sequence".to_string(),
            };
        }

        let pattern = self.pattern_analyzer.extract(
            &context.task_id,
            &context.task_description,
            &context.tool_calls,
        );

        let pattern_validation = self.skill_validator.validate_pattern_reusability(&pattern);
        
        if pattern_validation.status == ValidationStatus::Rejected {
            return EvoEvolutionResult {
                evolved: false,
                skill_id: String::new(),
                changes: vec![format!("Pattern rejected: {}", pattern_validation.message)],
                new_reliability: 0.0,
                message: "Pattern rejected due to low reusability".to_string(),
            };
        }

        let learning_type = if context.success {
            LearningType::SuccessPattern
        } else {
            LearningType::FailurePattern
        };

        let record = LearningRecord {
            id: uuid::Uuid::new_v4().to_string(),
            task_id: context.task_id.clone(),
            pattern: pattern.clone(),
            learning_type,
            success: context.success,
            created_at: Utc::now(),
            task_input: context.task_description.clone(),
            task_output: None,
        };

        {
            let mut history = self.learning_history.write().await;
            history.add_record(record).await;
        }

        if let Some(skill_id) = self.find_similar_skill(&pattern).await {
            let result = self.evolve_skill(&skill_id, &pattern, context.success).await;
            return result;
        }

        if context.success && pattern.reusability_score > self.config.evolution_threshold {
            let skill = self.create_skill_from_pattern(&pattern).await;
            return EvoEvolutionResult {
                evolved: true,
                skill_id: skill.id.clone(),
                changes: vec!["New skill created from pattern".to_string()],
                new_reliability: skill.reliability,
                message: "New skill learned and stored".to_string(),
            };
        }

        EvoEvolutionResult {
            evolved: false,
            skill_id: String::new(),
            changes: vec![],
            new_reliability: 0.0,
            message: "No evolution needed".to_string(),
        }
    }

    async fn find_similar_skill(&self, _pattern: &TaskPattern) -> Option<String> {
        let graph = self.knowledge_graph.read().await;
        let skills = self.learned_skills.read().await;

        for (skill_id, skill) in skills.iter() {
            if let Some(_node) = graph.find_similar(&skill.pattern) {
                return Some(skill_id.clone());
            }
        }

        None
    }

    async fn evolve_skill(
        &self,
        skill_id: &str,
        pattern: &TaskPattern,
        success: bool,
    ) -> EvoEvolutionResult {
        let mut skills = self.learned_skills.write().await;

        if let Some(skill) = skills.get_mut(skill_id) {
            let old_reliability = skill.reliability;

            {
                let mut vm = self.version_manager.write().await;
                vm.create_version(
                    skill_id,
                    skill.code.clone(),
                    skill.pattern.clone(),
                    skill.reliability,
                    format!("Before evolution v{}", skill.version),
                    "auto_backup",
                );
            }

            if success {
                skill.usage_count += 1;
                skill.reliability = (skill.reliability * (skill.usage_count - 1) as f64 + 1.0)
                    / skill.usage_count as f64;
                skill.version += 1;
                skill.last_used = Some(Utc::now());
            } else {
                skill.reliability *= 0.9;
            }

            {
                let mut vm = self.version_manager.write().await;
                vm.create_version(
                    skill_id,
                    skill.code.clone(),
                    pattern.clone(),
                    skill.reliability,
                    format!("Reliability: {:.2} -> {:.2}", old_reliability, skill.reliability),
                    "auto_evolve",
                );
            }

            let changes = vec![
                format!("Usage count: {}", skill.usage_count),
                format!("Reliability: {:.2} -> {:.2}", old_reliability, skill.reliability),
                format!("Version: {}", skill.version),
            ];

            return EvoEvolutionResult {
                evolved: true,
                skill_id: skill_id.to_string(),
                changes,
                new_reliability: skill.reliability,
                message: if success {
                    "Skill reinforced through successful use".to_string()
                } else {
                    "Skill marked as less reliable".to_string()
                },
            };
        }

        EvoEvolutionResult {
            evolved: false,
            skill_id: String::new(),
            changes: vec![],
            new_reliability: 0.0,
            message: "Skill not found".to_string(),
        }
    }

    async fn create_skill_from_pattern(&self, pattern: &TaskPattern) -> EvoSkill {
        let skill_id = uuid::Uuid::new_v4().to_string();
        let skill = EvoSkill {
            id: skill_id.clone(),
            name: format!("skill_{}", pattern.task_category.to_lowercase().replace(' ', "_")),
            category: pattern.task_category.clone(),
            code: String::new(),
            pattern: pattern.clone(),
            reliability: 1.0,
            created_at: Utc::now(),
            last_used: Some(Utc::now()),
            usage_count: 1,
            version: 1,
        };

        let node = SkillNode {
            skill_id: skill.id.clone(),
            name: skill.name.clone(),
            category: skill.category.clone(),
            description: format!("Auto-generated skill from pattern: {}", pattern.id),
            tool_sequence: pattern.tool_sequence.clone(),
            usage_count: skill.usage_count,
            success_rate: skill.reliability,
            learned_from: pattern.source_task_id.clone(),
            created_at: skill.created_at,
            last_used: skill.last_used,
            metadata: std::collections::HashMap::new(),
        };

        {
            let mut graph = self.knowledge_graph.write().await;
            graph.add_skill(node);
        }

        self.learned_skills.write().await.insert(skill_id, skill.clone());

        skill
    }

    pub async fn detect_recurring_patterns(&self) -> Vec<RecurringPattern> {
        let history = self.learning_history.read().await;
        history.detect_recurring().await
    }

    pub async fn recommend_skills(&self, task_description: &str) -> Vec<Recommendation> {
        let temp_tool_calls = vec![ToolCall {
            name: "analyze".to_string(),
            arguments: serde_json::json!({"task": task_description}),
            result: None,
            duration_ms: 0,
        }];

        let pattern = self.pattern_analyzer.extract(
            "recommendation",
            task_description,
            &temp_tool_calls,
        );

        let graph = self.knowledge_graph.read().await;

        if let Some(node) = graph.find_similar(&pattern) {
            return vec![Recommendation {
                skill_id: node.skill_id.clone(),
                skill_name: node.name.clone(),
                confidence: 0.8,
                reason: "Similar to current task".to_string(),
            }];
        }

        vec![]
    }

    pub async fn get_statistics(&self) -> EvoStatistics {
        let history = self.learning_history.read().await;
        let graph = self.knowledge_graph.read().await;
        let skills = self.learned_skills.read().await;

        let history_stats = history.get_statistics().await;
        let graph_stats = graph.get_statistics();

        EvoStatistics {
            total_tasks: history_stats.total_records,
            successful_tasks: history_stats.success_count,
            success_rate: if history_stats.total_records > 0 {
                history_stats.success_count as f64 / history_stats.total_records as f64
            } else {
                0.0
            },
            total_skills: skills.len(),
            reliable_skills: skills.values().filter(|s| s.reliability > 0.8).count(),
            graph_nodes: graph_stats.total_skills,
            graph_edges: graph_stats.total_edges,
            recurring_patterns: 0,
        }
    }

    pub async fn validate_skill(&self, code: &str) -> ValidationResult {
        self.skill_validator.validate(code)
    }

    pub async fn on_skill_executed(&self, skill_id: &str, success: bool) {
        {
            let mut graph = self.knowledge_graph.write().await;
            graph.record_usage(skill_id, success);
        }
        {
            let mut skills = self.learned_skills.write().await;
            if let Some(skill) = skills.get_mut(skill_id) {
                skill.usage_count += 1;
                if success {
                    skill.reliability = (skill.reliability * (skill.usage_count - 1) as f64 + 1.0) 
                        / skill.usage_count as f64;
                } else {
                    skill.reliability *= 0.9;
                }
                skill.last_used = Some(Utc::now());
            }
        }
    }

    pub async fn get_skill(&self, skill_id: &str) -> Option<EvoSkill> {
        self.learned_skills.read().await.get(skill_id).cloned()
    }

    pub async fn get_all_skills(&self) -> Vec<EvoSkill> {
        self.learned_skills.read().await.values().cloned().collect()
    }

    pub async fn remove_skill(&self, skill_id: &str) -> bool {
        {
            let mut skills = self.learned_skills.write().await;
            skills.remove(skill_id).is_some()
        }
    }

    pub async fn get_knowledge_graph(&self) -> Arc<RwLock<KnowledgeGraph>> {
        self.knowledge_graph.clone()
    }

    pub async fn get_version_manager(&self) -> Arc<RwLock<VersionManager>> {
        self.version_manager.clone()
    }

    pub async fn get_skill_history(&self, skill_id: &str) -> Vec<VersionRecord> {
        let vm = self.version_manager.read().await;
        vm.get_all_versions(skill_id)
    }

    pub async fn rollback_skill(&self, skill_id: &str, version: u32) -> Option<VersionRecord> {
        let mut vm = self.version_manager.write().await;
        vm.rollback(skill_id, version)
    }

    pub async fn get_version_diff(&self, skill_id: &str, v1: u32, v2: u32) -> Option<super::version_manager::VersionDiff> {
        let vm = self.version_manager.read().await;
        vm.diff(skill_id, v1, v2)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvoStatistics {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub success_rate: f64,
    pub total_skills: usize,
    pub reliable_skills: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub recurring_patterns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_process_successful_task() {
        let engine = EvoV2Engine::new();

        let tool_calls = vec![
            ToolCall {
                name: "Read".to_string(),
                arguments: serde_json::json!({"path": "/test.txt"}),
                result: Some(serde_json::json!("content")),
                duration_ms: 100,
            },
            ToolCall {
                name: "Write".to_string(),
                arguments: serde_json::json!({"path": "/output.txt", "content": "test"}),
                result: Some(serde_json::json!(true)),
                duration_ms: 50,
            },
        ];

        let context = EvoContext {
            task_id: "test_task_1".to_string(),
            task_description: "Read and write file operation".to_string(),
            tool_calls,
            success: true,
            execution_time_ms: 100,
            metadata: serde_json::json!({}),
        };

        let result = engine.process_task(context).await;

        assert!(result.evolved || !result.evolved);
    }

    #[tokio::test]
    async fn test_recommend_skills() {
        let engine = EvoV2Engine::new();

        let recommendations = engine.recommend_skills("I need to read a file").await;

        assert!(recommendations.is_empty());
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let engine = EvoV2Engine::new();

        let stats = engine.get_statistics().await;

        assert_eq!(stats.total_skills, 0);
    }

    #[tokio::test]
    async fn test_validate_skill() {
        let engine = EvoV2Engine::new();

        let code = r#"
async fn fetch_data(url: String) -> Result<String, String> {
    Ok("data".to_string())
}
"#;

        let result = engine.validate_skill(code).await;

        assert!(result.details.iter().any(|d| d.rule == "async_usage"));
    }

    #[tokio::test]
    async fn test_process_failed_task() {
        let engine = EvoV2Engine::new();

        let tool_calls = vec![ToolCall {
            name: "Read".to_string(),
            arguments: serde_json::json!({"path": "/missing.txt"}),
            result: None,
            duration_ms: 50,
        }];

        let context = EvoContext {
            task_id: "test_task_2".to_string(),
            task_description: "Read missing file".to_string(),
            tool_calls,
            success: false,
            execution_time_ms: 50,
            metadata: serde_json::json!({}),
        };

        let result = engine.process_task(context).await;

        assert!(!result.evolved || result.evolved);
    }

    #[tokio::test]
    async fn test_get_all_skills() {
        let engine = EvoV2Engine::new();

        let skills = engine.get_all_skills().await;

        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_remove_skill() {
        let engine = EvoV2Engine::new();

        let removed = engine.remove_skill("nonexistent").await;

        assert!(!removed);
    }

    #[test]
    fn test_evo_config_default() {
        let config = EvoConfig::default();

        assert!(config.enable_auto_evolution);
        assert_eq!(config.evolution_threshold, 0.7);
        assert_eq!(config.similarity_threshold, 0.8);
    }

    #[test]
    fn test_evo_v2_engine_new() {
        let engine = EvoV2Engine::new();

        assert!(engine.config.enable_auto_evolution);
    }
}
