//! Version Manager - 技能版本管理
//!
//! 管理技能的版本历史、备份和回滚

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub version: u32,
    pub code: String,
    pub pattern: super::pattern_analyzer::TaskPattern,
    pub reliability: f64,
    pub changes: String,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub from_version: u32,
    pub to_version: u32,
    pub added_lines: u32,
    pub removed_lines: u32,
    pub changed_params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionManager {
    versions: HashMap<String, Vec<VersionRecord>>,
    config: VersionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConfig {
    pub max_versions_per_skill: usize,
    pub auto_backup_on_evolve: bool,
    pub keep_failed_versions: bool,
}

impl Default for VersionConfig {
    fn default() -> Self {
        Self {
            max_versions_per_skill: 10,
            auto_backup_on_evolve: true,
            keep_failed_versions: false,
        }
    }
}

impl Default for VersionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            versions: HashMap::new(),
            config: VersionConfig::default(),
        }
    }

    pub fn with_config(config: VersionConfig) -> Self {
        Self {
            versions: HashMap::new(),
            config,
        }
    }

    pub fn create_version(
        &mut self,
        skill_id: &str,
        code: String,
        pattern: super::pattern_analyzer::TaskPattern,
        reliability: f64,
        changes: String,
        created_by: &str,
    ) -> u32 {
        let versions = self.versions.entry(skill_id.to_string()).or_insert_with(Vec::new);
        let new_version = versions.len() as u32 + 1;

        let record = VersionRecord {
            version: new_version,
            code,
            pattern,
            reliability,
            changes,
            created_at: Utc::now(),
            created_by: created_by.to_string(),
        };

        versions.push(record);

        if versions.len() > self.config.max_versions_per_skill {
            versions.remove(0);
        }

        new_version
    }

    pub fn get_version(&self, skill_id: &str, version: u32) -> Option<VersionRecord> {
        self.versions
            .get(skill_id)
            .and_then(|v| v.iter().find(|r| r.version == version))
            .cloned()
    }

    pub fn get_latest(&self, skill_id: &str) -> Option<VersionRecord> {
        self.versions
            .get(skill_id)
            .and_then(|v| v.last())
            .cloned()
    }

    pub fn get_all_versions(&self, skill_id: &str) -> Vec<VersionRecord> {
        self.versions.get(skill_id).cloned().unwrap_or_default()
    }

    pub fn get_version_count(&self, skill_id: &str) -> usize {
        self.versions.get(skill_id).map(|v| v.len()).unwrap_or(0)
    }

    pub fn has_versions(&self, skill_id: &str) -> bool {
        self.versions.contains_key(skill_id) && !self.versions.get(skill_id).unwrap().is_empty()
    }

    pub fn rollback(&mut self, skill_id: &str, target_version: u32) -> Option<VersionRecord> {
        let versions = self.versions.get_mut(skill_id)?;
        let target = versions.iter().find(|r| r.version == target_version)?.clone();

        let new_version = versions.len() as u32 + 1;
        let rollback_record = VersionRecord {
            version: new_version,
            code: target.code.clone(),
            pattern: target.pattern.clone(),
            reliability: target.reliability,
            changes: format!("Rollback to version {}", target_version),
            created_at: Utc::now(),
            created_by: "rollback".to_string(),
        };

        versions.push(rollback_record.clone());
        Some(rollback_record)
    }

    pub fn diff(&self, skill_id: &str, v1: u32, v2: u32) -> Option<VersionDiff> {
        let versions = self.versions.get(skill_id)?;
        let from = versions.iter().find(|r| r.version == v1)?;
        let to = versions.iter().find(|r| r.version == v2)?;

        let from_lines: HashSet<_> = from.code.lines().collect();
        let to_lines: HashSet<_> = to.code.lines().collect();

        let added = to_lines.difference(&from_lines).count() as u32;
        let removed = from_lines.difference(&to_lines).count() as u32;

        let from_params: HashSet<_> = from.pattern.param_patterns.iter().map(|p| p.name.clone()).collect();
        let to_params: HashSet<_> = to.pattern.param_patterns.iter().map(|p| p.name.clone()).collect();
        let changed_params: Vec<String> = from_params
            .symmetric_difference(&to_params)
            .cloned()
            .collect();

        Some(VersionDiff {
            from_version: v1,
            to_version: v2,
            added_lines: added,
            removed_lines: removed,
            changed_params,
        })
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let vm: VersionManager = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(vm)
    }

    pub fn clear(&mut self) {
        self.versions.clear();
    }

    pub fn remove_skill_versions(&mut self, skill_id: &str) -> bool {
        self.versions.remove(skill_id).is_some()
    }

    pub fn get_all_skill_ids(&self) -> Vec<String> {
        self.versions.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evo::pattern_analyzer::{ParamPattern, ParamType, TaskPattern, ToolCallPattern};
    use std::collections::HashMap;

    fn create_test_pattern(id: &str) -> TaskPattern {
        TaskPattern {
            id: id.to_string(),
            task_category: "test".to_string(),
            tool_sequence: vec![ToolCallPattern {
                tool_name: "test".to_string(),
                param_schema: HashMap::new(),
                result_schema: HashMap::new(),
            }],
            param_patterns: vec![ParamPattern {
                name: "query".to_string(),
                param_type: ParamType::String,
                is_generic: true,
                examples: vec!["<QUERY>".to_string()],
            }],
            success_indicators: vec![],
            steps: vec![],
            reusability_score: 0.8,
            source_task_id: "task-1".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_create_version() {
        let mut vm = VersionManager::new();
        let pattern = create_test_pattern("test-1");

        let version = vm.create_version(
            "skill-1",
            "fn test() {}".to_string(),
            pattern.clone(),
            0.8,
            "Initial version".to_string(),
            "test",
        );

        assert_eq!(version, 1);
        assert!(vm.has_versions("skill-1"));
    }

    #[test]
    fn test_get_latest() {
        let mut vm = VersionManager::new();
        let pattern = create_test_pattern("test-1");

        vm.create_version("skill-1", "v1".to_string(), pattern.clone(), 0.8, "v1".to_string(), "test");
        vm.create_version("skill-1", "v2".to_string(), pattern.clone(), 0.85, "v2".to_string(), "test");

        let latest = vm.get_latest("skill-1").unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.reliability, 0.85);
    }

    #[test]
    fn test_rollback() {
        let mut vm = VersionManager::new();
        let pattern = create_test_pattern("test-1");

        vm.create_version("skill-1", "v1".to_string(), pattern.clone(), 0.8, "v1".to_string(), "test");
        vm.create_version("skill-1", "v2".to_string(), pattern.clone(), 0.85, "v2".to_string(), "test");

        let rollback = vm.rollback("skill-1", 1).unwrap();
        assert_eq!(rollback.version, 3);
        assert!(rollback.changes.contains("Rollback"));
    }

    #[test]
    fn test_diff() {
        let mut vm = VersionManager::new();
        let pattern = create_test_pattern("test-1");

        vm.create_version("skill-1", "fn v1() {}\nfn other() {}".to_string(), pattern.clone(), 0.8, "v1".to_string(), "test");
        vm.create_version("skill-1", "fn v1() {}\nfn new_fn() {}".to_string(), pattern.clone(), 0.85, "v2".to_string(), "test");

        let diff = vm.diff("skill-1", 1, 2).unwrap();
        assert!(diff.added_lines > 0 || diff.removed_lines > 0);
    }

    #[test]
    fn test_save_load() {
        let mut vm = VersionManager::new();
        let pattern = create_test_pattern("test-1");
        vm.create_version("skill-1", "code".to_string(), pattern, 0.8, "init".to_string(), "test");

        let path = "/tmp/test_version_manager.json";
        vm.save_to_file(path).unwrap();

        let loaded = VersionManager::load_from_file(path).unwrap();
        assert!(loaded.has_versions("skill-1"));

        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_max_versions() {
        let mut vm = VersionManager::with_config(VersionConfig {
            max_versions_per_skill: 3,
            auto_backup_on_evolve: true,
            keep_failed_versions: false,
        });
        let pattern = create_test_pattern("test-1");

        for i in 1..=5 {
            vm.create_version("skill-1", format!("v{}", i), pattern.clone(), 0.8, format!("v{}", i), "test");
        }

        assert_eq!(vm.get_version_count("skill-1"), 3);
    }
}
