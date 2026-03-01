//! Evo Runner - 进化系统运行器

use std::sync::Arc;

use openclaw_agent::{
    EvoV2Engine, EvoContext, EvoSkill, EvoStatistics, 
    Recommendation, ValidationResult, RecurringPattern, ToolCall,
    GraphStatistics, VersionRecord, VersionDiff,
};

pub struct EvoRunner {
    engine: Arc<EvoV2Engine>,
}

impl EvoRunner {
    pub fn new() -> Self {
        Self {
            engine: Arc::new(EvoV2Engine::new()),
        }
    }

    pub async fn get_statistics(&self) -> EvoStatistics {
        self.engine.get_statistics().await
    }

    pub async fn validate_skill(&self, code: &str) -> ValidationResult {
        self.engine.validate_skill(code).await
    }

    pub async fn recommend_skills(&self, task: &str) -> Vec<Recommendation> {
        self.engine.recommend_skills(task).await
    }

    pub async fn get_all_skills(&self) -> Vec<EvoSkill> {
        self.engine.get_all_skills().await
    }

    pub async fn get_skill(&self, skill_id: &str) -> Option<EvoSkill> {
        self.engine.get_skill(skill_id).await
    }

    pub async fn remove_skill(&self, skill_id: &str) -> bool {
        self.engine.remove_skill(skill_id).await
    }

    pub async fn detect_recurring_patterns(&self) -> Vec<RecurringPattern> {
        self.engine.detect_recurring_patterns().await
    }

    pub async fn get_graph_statistics(&self) -> GraphStatistics {
        let graph = self.engine.get_knowledge_graph().await;
        graph.read().await.get_statistics()
    }

    pub async fn get_skill_history(&self, skill_id: &str) -> Vec<VersionRecord> {
        self.engine.get_skill_history(skill_id).await
    }

    pub async fn rollback_skill(&self, skill_id: &str, version: u32) -> Option<VersionRecord> {
        self.engine.rollback_skill(skill_id, version).await
    }

    pub async fn get_version_diff(&self, skill_id: &str, from: u32, to: u32) -> Option<VersionDiff> {
        self.engine.get_version_diff(skill_id, from, to).await
    }

    pub async fn process_task(
        &self,
        task_id: String,
        task_description: String,
        tool_calls: Vec<ToolCall>,
        success: bool,
        execution_time_ms: u64,
    ) {
        let context = EvoContext {
            task_id,
            task_description,
            tool_calls,
            success,
            execution_time_ms,
            metadata: serde_json::json!({}),
        };

        self.engine.process_task(context).await;
    }
}

impl Default for EvoRunner {
    fn default() -> Self {
        Self::new()
    }
}
