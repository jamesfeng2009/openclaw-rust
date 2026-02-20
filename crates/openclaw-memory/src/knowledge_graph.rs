//! 知识图谱模块
//!
//! 实现记忆的图结构化存储和查询：
//! - 实体节点
//! - 关系边
//! - 图查询和遍历

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub name: String,
    pub entity_type: EntityType,
    pub properties: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Person,
    Organization,
    Location,
    Project,
    Concept,
    Preference,
    Skill,
    Goal,
    Other,
}

impl Entity {
    pub fn new(name: String, entity_type: EntityType) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            entity_type,
            properties: HashMap::new(),
            created_at: now,
            updated_at: now,
            confidence: 1.0,
        }
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub relation_type: RelationType,
    pub properties: HashMap<String, String>,
    pub weight: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    Knows,
    WorksFor,
    ParticipatesIn,
    HasPreference,
    HasSkill,
    RelatedTo,
    Owns,
    Uses,
    GoalOf,
}

impl Relation {
    pub fn new(source_id: String, target_id: String, relation_type: RelationType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            source_id,
            target_id,
            relation_type,
            properties: HashMap::new(),
            weight: 1.0,
            created_at: Utc::now(),
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

pub struct KnowledgeGraph {
    entities: Arc<RwLock<HashMap<String, Entity>>>,
    relations: Arc<RwLock<HashMap<String, Relation>>>,
    entity_index: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    relation_index: Arc<RwLock<HashMap<RelationType, HashSet<String>>>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: Arc::new(RwLock::new(HashMap::new())),
            relations: Arc::new(RwLock::new(HashMap::new())),
            entity_index: Arc::new(RwLock::new(HashMap::new())),
            relation_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_entity(&self, entity: Entity) -> Result<(), String> {
        let mut entities = self.entities.write().await;
        let mut index = self.entity_index.write().await;

        let name_key = entity.name.to_lowercase();
        index.entry(name_key).or_default().insert(entity.id.clone());

        let type_key = format!("{:?}", entity.entity_type);
        index.entry(type_key).or_default().insert(entity.id.clone());

        entities.insert(entity.id.clone(), entity);
        Ok(())
    }

    pub async fn add_relation(&self, relation: Relation) -> Result<(), String> {
        let mut relations = self.relations.write().await;
        let mut index = self.relation_index.write().await;

        index
            .entry(relation.relation_type.clone())
            .or_default()
            .insert(relation.id.clone());

        relations.insert(relation.id.clone(), relation);
        Ok(())
    }

    pub async fn get_entity(&self, id: &str) -> Option<Entity> {
        self.entities.read().await.get(id).cloned()
    }

    pub async fn find_entities_by_name(&self, name: &str) -> Vec<Entity> {
        let index = self.entity_index.read().await;
        let entities = self.entities.read().await;

        let name_key = name.to_lowercase();
        if let Some(ids) = index.get(&name_key) {
            ids.iter()
                .filter_map(|id| entities.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn find_entities_by_type(&self, entity_type: &EntityType) -> Vec<Entity> {
        let index = self.entity_index.read().await;
        let entities = self.entities.read().await;

        let type_key = format!("{:?}", entity_type);
        if let Some(ids) = index.get(&type_key) {
            ids.iter()
                .filter_map(|id| entities.get(id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn get_relations_from(&self, source_id: &str) -> Vec<Relation> {
        self.relations
            .read()
            .await
            .values()
            .filter(|r| r.source_id == source_id)
            .cloned()
            .collect()
    }

    pub async fn get_relations_to(&self, target_id: &str) -> Vec<Relation> {
        self.relations
            .read()
            .await
            .values()
            .filter(|r| r.target_id == target_id)
            .cloned()
            .collect()
    }

    pub async fn find_path(&self, from: &str, to: &str) -> Option<Vec<Relation>> {
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: Vec<(String, Vec<Relation>)> = vec![(from.to_string(), Vec::new())];

        while let Some((current_id, path)) = queue.pop() {
            if current_id == to {
                return Some(path);
            }

            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id.clone());

            let relations = self.get_relations_from(&current_id).await;
            for relation in relations {
                let mut new_path = path.clone();
                new_path.push(relation.clone());
                queue.push((relation.target_id, new_path));
            }
        }

        None
    }

    pub async fn get_subgraph(&self, entity_id: &str, depth: usize) -> Subgraph {
        let mut visited: HashSet<String> = HashSet::new();
        let mut entities = Vec::new();
        let mut relations = Vec::new();

        self.bfs_collect(entity_id, depth, &mut visited, &mut entities, &mut relations).await;

        Subgraph { entities, relations }
    }

    async fn bfs_collect(
        &self,
        entity_id: &str,
        depth: usize,
        visited: &mut HashSet<String>,
        entities: &mut Vec<Entity>,
        relations: &mut Vec<Relation>,
    ) {
        if depth == 0 {
            return;
        }

        if visited.contains(entity_id) {
            return;
        }

        visited.insert(entity_id.to_string());

        if let Some(entity) = self.get_entity(entity_id).await {
            entities.push(entity);
        }

        let outgoing = self.get_relations_from(entity_id).await;
        for relation in outgoing {
            let already_added = relations.iter().any(|r| r.id == relation.id);
            if !already_added {
                relations.push(relation.clone());
                Box::pin(self.bfs_collect(
                    &relation.target_id,
                    depth - 1,
                    visited,
                    entities,
                    relations,
                )).await;
            }
        }

        let incoming = self.get_relations_to(entity_id).await;
        for relation in incoming {
            let already_added = relations.iter().any(|r| r.id == relation.id);
            if !already_added {
                relations.push(relation.clone());
                Box::pin(self.bfs_collect(
                    &relation.source_id,
                    depth - 1,
                    visited,
                    entities,
                    relations,
                )).await;
            }
        }
    }

    pub async fn get_preferences(&self) -> Vec<Entity> {
        self.find_entities_by_type(&EntityType::Preference).await
    }

    pub async fn get_skills(&self) -> Vec<Entity> {
        self.find_entities_by_type(&EntityType::Skill).await
    }

    pub async fn get_goals(&self) -> Vec<Entity> {
        self.find_entities_by_type(&EntityType::Goal).await
    }

    pub async fn stats(&self) -> KnowledgeGraphStats {
        let entities = self.entities.read().await;
        let relations = self.relations.read().await;

        KnowledgeGraphStats {
            entity_count: entities.len(),
            relation_count: relations.len(),
            entity_types: {
                let mut types: HashMap<String, usize> = HashMap::new();
                for entity in entities.values() {
                    *types.entry(format!("{:?}", entity.entity_type)).or_insert(0) += 1;
                }
                types
            },
        }
    }

    pub async fn clear(&self) {
        let mut entities = self.entities.write().await;
        let mut relations = self.relations.write().await;
        let mut entity_index = self.entity_index.write().await;
        let mut relation_index = self.relation_index.write().await;

        entities.clear();
        relations.clear();
        entity_index.clear();
        relation_index.clear();
    }

    pub fn search_entities(&self, query_terms: &[&str]) -> Vec<Entity> {
        let entities = self.entities.blocking_read();
        let mut results: Vec<(Entity, usize)> = Vec::new();

        for entity in entities.values() {
            let name_lower = entity.name.to_lowercase();
            let mut match_count = 0;

            for term in query_terms {
                if name_lower.contains(*term) {
                    match_count += 1;
                }
                for (_, value) in &entity.properties {
                    if value.to_lowercase().contains(*term) {
                        match_count += 1;
                    }
                }
            }

            if match_count > 0 {
                results.push((entity.clone(), match_count));
            }
        }

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(e, _)| e).collect()
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub entities: Vec<Entity>,
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphStats {
    pub entity_count: usize,
    pub relation_count: usize,
    pub entity_types: HashMap<String, usize>,
}

pub struct KnowledgeGraphBuilder {
    graph: KnowledgeGraph,
}

impl KnowledgeGraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: KnowledgeGraph::new(),
        }
    }

    pub async fn add_person(self, name: &str) -> Self {
        let entity = Entity::new(name.to_string(), EntityType::Person);
        self.graph.add_entity(entity).await.ok();
        self
    }

    pub async fn add_preference(self, name: &str) -> Self {
        let entity = Entity::new(name.to_string(), EntityType::Preference);
        self.graph.add_entity(entity).await.ok();
        self
    }

    pub async fn add_relation(
        self,
        source: &str,
        target: &str,
        relation_type: RelationType,
    ) -> Self {
        let source_id = self.graph.find_entities_by_name(source).await.first().map(|e| e.id.clone());
        let target_id = self.graph.find_entities_by_name(target).await.first().map(|e| e.id.clone());

        if let (Some(s), Some(t)) = (source_id, target_id) {
            let relation = Relation::new(s, t, relation_type);
            self.graph.add_relation(relation).await.ok();
        }
        self
    }

    pub fn build(self) -> KnowledgeGraph {
        self.graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_entity() {
        let graph = KnowledgeGraph::new();
        let entity = Entity::new("张三".to_string(), EntityType::Person);
        
        graph.add_entity(entity.clone()).await.unwrap();
        
        let retrieved = graph.get_entity(&entity.id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "张三");
    }

    #[tokio::test]
    async fn test_find_entities_by_name() {
        let graph = KnowledgeGraph::new();
        
        let entity = Entity::new("Python".to_string(), EntityType::Skill);
        graph.add_entity(entity).await.unwrap();
        
        let results = graph.find_entities_by_name("Python").await;
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_add_relation() {
        let graph = KnowledgeGraph::new();
        
        let person = Entity::new("李四".to_string(), EntityType::Person);
        let skill = Entity::new("Rust".to_string(), EntityType::Skill);
        
        graph.add_entity(person.clone()).await.unwrap();
        graph.add_entity(skill.clone()).await.unwrap();
        
        let relation = Relation::new(
            person.id.clone(),
            skill.id.clone(),
            RelationType::HasSkill,
        );
        graph.add_relation(relation).await.unwrap();
        
        let relations = graph.get_relations_from(&person.id).await;
        assert_eq!(relations.len(), 1);
    }

    #[tokio::test]
    async fn test_find_path() {
        let graph = KnowledgeGraph::new();
        
        let a = Entity::new("A".to_string(), EntityType::Person);
        let b = Entity::new("B".to_string(), EntityType::Person);
        let c = Entity::new("C".to_string(), EntityType::Person);
        
        graph.add_entity(a.clone()).await.unwrap();
        graph.add_entity(b.clone()).await.unwrap();
        graph.add_entity(c.clone()).await.unwrap();
        
        graph.add_relation(Relation::new(a.id.clone(), b.id.clone(), RelationType::Knows)).await.unwrap();
        graph.add_relation(Relation::new(b.id.clone(), c.id.clone(), RelationType::Knows)).await.unwrap();
        
        let path = graph.find_path(&a.id, &c.id).await;
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_get_subgraph() {
        let graph = KnowledgeGraph::new();
        
        let root = Entity::new("root".to_string(), EntityType::Person);
        let child1 = Entity::new("child1".to_string(), EntityType::Skill);
        
        graph.add_entity(root.clone()).await.unwrap();
        graph.add_entity(child1.clone()).await.unwrap();
        
        graph.add_relation(Relation::new(root.id.clone(), child1.id.clone(), RelationType::HasSkill)).await.unwrap();
        
        let outgoing = graph.get_relations_from(&root.id).await;
        assert_eq!(outgoing.len(), 1);
        
        let root_entity = graph.get_entity(&root.id).await;
        assert!(root_entity.is_some());
    }

    #[test]
    fn test_entity_builder() {
        let entity = Entity::new("测试用户".to_string(), EntityType::Person)
            .with_property("email", "test@example.com")
            .with_confidence(0.9);
        
        assert_eq!(entity.name, "测试用户");
        assert_eq!(entity.properties.get("email"), Some(&"test@example.com".to_string()));
        assert!((entity.confidence - 0.9).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_stats() {
        let graph = KnowledgeGraph::new();
        
        graph.add_entity(Entity::new("p1".to_string(), EntityType::Person)).await.unwrap();
        graph.add_entity(Entity::new("p2".to_string(), EntityType::Person)).await.unwrap();
        graph.add_entity(Entity::new("s1".to_string(), EntityType::Skill)).await.unwrap();
        
        let stats = graph.stats().await;
        assert_eq!(stats.entity_count, 3);
        assert_eq!(stats.entity_types.get("Person"), Some(&2));
        assert_eq!(stats.entity_types.get("Skill"), Some(&1));
    }

    #[tokio::test]
    async fn test_clear() {
        let graph = KnowledgeGraph::new();
        
        graph.add_entity(Entity::new("test".to_string(), EntityType::Person)).await.unwrap();
        
        let stats_before = graph.stats().await;
        assert_eq!(stats_before.entity_count, 1);
        
        graph.clear().await;
        
        let stats_after = graph.stats().await;
        assert_eq!(stats_after.entity_count, 0);
    }
}
