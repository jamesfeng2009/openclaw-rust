//! Agentic RAG 配置

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgenticRAGConfig {
    pub enabled: bool,
    pub planner: PlannerConfig,
    pub executor: ExecutorConfig,
    pub reflector: ReflectorConfig,
    pub sources: Vec<SourceConfig>,
}

impl Default for AgenticRAGConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            planner: PlannerConfig::default(),
            executor: ExecutorConfig::default(),
            reflector: ReflectorConfig::default(),
            sources: vec![
                SourceConfig {
                    source_type: SourceType::Memory,
                    enabled: true,
                    priority: 1,
                    config: HashMap::new(),
                },
                SourceConfig {
                    source_type: SourceType::VectorDB,
                    enabled: true,
                    priority: 2,
                    config: HashMap::new(),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    pub max_sub_queries: usize,
    pub enable_query_rewrite: bool,
    pub enable_hypothesis: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_sub_queries: 5,
            enable_query_rewrite: true,
            enable_hypothesis: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    pub timeout_ms: u64,
    pub max_results_per_source: usize,
    pub enable_parallel: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30000,
            max_results_per_source: 10,
            enable_parallel: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectorConfig {
    pub min_confidence: f32,
    pub max_iterations: usize,
    pub enable_verification: bool,
}

impl Default for ReflectorConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.7,
            max_iterations: 3,
            enable_verification: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub source_type: SourceType,
    pub enabled: bool,
    pub priority: usize,
    pub config: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Memory,
    VectorDB,
    Web,
    File,
    API,
}

impl Default for SourceType {
    fn default() -> Self {
        SourceType::Memory
    }
}
