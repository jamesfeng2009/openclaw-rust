use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicSkill {
    pub id: String,
    pub name: String,
    pub code: String,
    pub language: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub version: String,
}

impl DynamicSkill {
    pub fn new(
        id: String,
        name: String,
        code: String,
        language: String,
        created_by: String,
    ) -> Self {
        Self {
            id,
            name,
            code,
            language,
            created_by,
            created_at: Utc::now(),
            version: "1.0.0".to_string(),
        }
    }
}

pub struct SharedSkillRegistry {
    inner: Arc<RwLock<SkillRegistryInner>>,
}

#[derive(Debug, Clone)]
struct SkillRegistryInner {
    skills: Vec<DynamicSkill>,
}

impl Default for SkillRegistryInner {
    fn default() -> Self {
        Self {
            skills: Vec::new(),
        }
    }
}

impl SharedSkillRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(SkillRegistryInner::default())),
        }
    }

    pub async fn register_skill(&self, skill: DynamicSkill) {
        let mut registry = self.inner.write().await;
        registry.skills.push(skill);
    }

    pub async fn get_skill(&self, id: &str) -> Option<DynamicSkill> {
        let registry = self.inner.read().await;
        registry.skills.iter().find(|s| s.id == id).cloned()
    }

    pub async fn get_all_skills(&self) -> Vec<DynamicSkill> {
        let registry = self.inner.read().await;
        registry.skills.clone()
    }

    pub async fn skill_exists(&self, name: &str) -> bool {
        let registry = self.inner.read().await;
        registry.skills.iter().any(|s| s.name == name)
    }

    pub fn clone_arc(&self) -> Arc<RwLock<SkillRegistryInner>> {
        Arc::clone(&self.inner)
    }
}

impl Default for SharedSkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}
