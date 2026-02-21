//! Graph Definition - Graph 数据结构定义

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub agent_id: Option<String>,
    pub config: NodeConfig,
    pub description: Option<String>,
}

impl NodeDef {
    pub fn new(id: impl Into<String>, node_type: NodeType) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            name: id_str,
            node_type,
            agent_id: None,
            config: NodeConfig::default(),
            description: None,
        }
    }
    
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }
    
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    
    pub fn with_config(mut self, config: NodeConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Router,
    Executor,
    Aggregator,
    Terminal,
}

impl Default for NodeType {
    fn default() -> Self {
        NodeType::Executor
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub timeout_ms: Option<u64>,
    pub retry_on_failure: Option<bool>,
    pub max_retries: Option<usize>,
    pub aggregation: Option<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            timeout_ms: Some(30000),
            retry_on_failure: Some(true),
            max_retries: Some(3),
            aggregation: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub from: String,
    pub to: String,
    pub condition: Option<ConditionDef>,
    pub weight: Option<f32>,
}

impl EdgeDef {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            condition: None,
            weight: None,
        }
    }
    
    pub fn with_condition(mut self, condition: ConditionDef) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionDef {
    pub condition_type: ConditionType,
    pub params: HashMap<String, serde_json::Value>,
}

impl ConditionDef {
    pub fn new(condition_type: ConditionType) -> Self {
        Self {
            condition_type,
            params: HashMap::new(),
        }
    }
    
    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    Always,
    ConfidenceAbove { threshold: f32 },
    HasResult { key: String },
    ResultCount { min: Option<usize>, max: Option<usize> },
    LlmJudge { prompt: String },
    RetryNeeded { max_retries: usize },
    Failed,
    Succeeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDef {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<EdgeDef>,
    pub start: String,
    pub end: Vec<String>,
}

impl GraphDef {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: None,
            nodes: Vec::new(),
            edges: Vec::new(),
            start: String::new(),
            end: Vec::new(),
        }
    }
    
    pub fn with_node(mut self, node: NodeDef) -> Self {
        if self.start.is_empty() {
            self.start = node.id.clone();
        }
        self.nodes.push(node);
        self
    }
    
    pub fn with_edge(mut self, edge: EdgeDef) -> Self {
        self.edges.push(edge);
        self
    }
    
    pub fn with_start(mut self, start_id: impl Into<String>) -> Self {
        self.start = start_id.into();
        self
    }
    
    pub fn with_end(mut self, end_id: impl Into<String>) -> Self {
        self.end.push(end_id.into());
        self
    }
    
    pub fn find_node(&self, id: &str) -> Option<&NodeDef> {
        self.nodes.iter().find(|n| n.id == id)
    }
    
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&EdgeDef> {
        self.edges.iter().filter(|e| e.from == node_id).collect()
    }
    
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&EdgeDef> {
        self.edges.iter().filter(|e| e.to == node_id).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    pub max_parallel_nodes: usize,
    pub timeout_ms: u64,
    pub retry_on_failure: bool,
    pub max_retries: usize,
    pub continue_on_error: bool,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            max_parallel_nodes: 10,
            timeout_ms: 300000,
            retry_on_failure: true,
            max_retries: 2,
            continue_on_error: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_node_def() {
        let node = NodeDef::new("test_node", NodeType::Executor)
            .with_agent("agent_1");
        
        assert_eq!(node.id, "test_node");
        assert_eq!(node.agent_id, Some("agent_1".to_string()));
    }
    
    #[test]
    fn test_node_def_with_name() {
        let node = NodeDef::new("node_1", NodeType::Executor)
            .with_name("Custom Name");
        
        assert_eq!(node.name, "Custom Name");
    }
    
    #[test]
    fn test_node_config_default() {
        let config = NodeConfig::default();
        
        assert_eq!(config.timeout_ms, Some(30000));
        assert_eq!(config.retry_on_failure, Some(true));
        assert_eq!(config.max_retries, Some(3));
    }
    
    #[test]
    fn test_node_config_custom() {
        let config = NodeConfig {
            timeout_ms: Some(60000),
            retry_on_failure: Some(false),
            max_retries: Some(5),
            aggregation: Some("sum".to_string()),
        };
        
        assert_eq!(config.timeout_ms, Some(60000));
        assert_eq!(config.retry_on_failure, Some(false));
        assert_eq!(config.max_retries, Some(5));
        assert_eq!(config.aggregation, Some("sum".to_string()));
    }
    
    #[test]
    fn test_edge_def() {
        let edge = EdgeDef::new("from_a", "to_b");
        
        assert_eq!(edge.from, "from_a");
        assert_eq!(edge.to, "to_b");
    }
    
    #[test]
    fn test_edge_def_with_condition() {
        let condition = ConditionDef::new(ConditionType::Always);
        let edge = EdgeDef::new("from_a", "to_b")
            .with_condition(condition);
        
        assert!(edge.condition.is_some());
    }
    
    #[test]
    fn test_condition_def() {
        let condition = ConditionDef::new(ConditionType::ConfidenceAbove { threshold: 0.8 })
            .with_param("key".to_string(), serde_json::json!("value"));
        
        assert!(matches!(condition.condition_type, ConditionType::ConfidenceAbove { threshold: 0.8 }));
        assert!(condition.params.contains_key("key"));
    }
    
    #[test]
    fn test_condition_type_variants() {
        let always = ConditionType::Always;
        assert!(matches!(always, ConditionType::Always));
        
        let confidence = ConditionType::ConfidenceAbove { threshold: 0.9 };
        assert!(matches!(confidence, ConditionType::ConfidenceAbove { threshold: 0.9 }));
        
        let has_result = ConditionType::HasResult { key: "result".to_string() };
        assert!(matches!(has_result, ConditionType::HasResult { key: _ }));
        
        let failed = ConditionType::Failed;
        assert!(matches!(failed, ConditionType::Failed));
        
        let succeeded = ConditionType::Succeeded;
        assert!(matches!(succeeded, ConditionType::Succeeded));
    }
    
    #[test]
    fn test_graph_def() {
        let graph = GraphDef::new("test_graph", "Test Graph")
            .with_node(NodeDef::new("node_a", NodeType::Router))
            .with_node(NodeDef::new("node_b", NodeType::Executor).with_agent("agent_1"))
            .with_edge(EdgeDef::new("node_a", "node_b"))
            .with_end("node_b");
        
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.start, "node_a");
    }
    
    #[test]
    fn test_graph_def_find_node() {
        let graph = GraphDef::new("test", "Test")
            .with_node(NodeDef::new("node_a", NodeType::Executor))
            .with_node(NodeDef::new("node_b", NodeType::Executor));
        
        let found = graph.find_node("node_a");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "node_a");
        
        let not_found = graph.find_node("nonexistent");
        assert!(not_found.is_none());
    }
    
    #[test]
    fn test_graph_def_get_edges() {
        let graph = GraphDef::new("test", "Test")
            .with_node(NodeDef::new("a", NodeType::Router))
            .with_node(NodeDef::new("b", NodeType::Executor))
            .with_node(NodeDef::new("c", NodeType::Executor))
            .with_edge(EdgeDef::new("a", "b"))
            .with_edge(EdgeDef::new("a", "c"))
            .with_edge(EdgeDef::new("b", "c"));
        
        let outgoing = graph.get_outgoing_edges("a");
        assert_eq!(outgoing.len(), 2);
        
        let incoming = graph.get_incoming_edges("c");
        assert_eq!(incoming.len(), 2);
    }
    
    #[test]
    fn test_graph_def_with_start() {
        let mut graph = GraphDef::new("test", "Test")
            .with_node(NodeDef::new("a", NodeType::Router))
            .with_node(NodeDef::new("b", NodeType::Executor))
            .with_edge(EdgeDef::new("a", "b"))
            .with_end("b");
        
        graph = graph.with_start("custom_start".to_string());
        assert_eq!(graph.start, "custom_start");
    }
    
    #[test]
    fn test_graph_config_default() {
        let config = GraphConfig::default();
        
        assert_eq!(config.max_parallel_nodes, 10);
        assert_eq!(config.timeout_ms, 300000);
        assert_eq!(config.retry_on_failure, true);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.continue_on_error, false);
    }
    
    #[test]
    fn test_graph_config_custom() {
        let config = GraphConfig {
            max_parallel_nodes: 5,
            timeout_ms: 600000,
            retry_on_failure: false,
            max_retries: 5,
            continue_on_error: true,
        };
        
        assert_eq!(config.max_parallel_nodes, 5);
        assert_eq!(config.timeout_ms, 600000);
        assert_eq!(config.retry_on_failure, false);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.continue_on_error, true);
    }
    
    #[test]
    fn test_node_type_default() {
        let node_type = NodeType::default();
        assert!(matches!(node_type, NodeType::Executor));
    }
}
