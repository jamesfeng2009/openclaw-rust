//! Graph Execution Context

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::definition::GraphDef;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub node_id: String,
    pub status: NodeStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub metadata: HashMap<String, String>,
}

impl NodeResult {
    pub fn success(node_id: impl Into<String>, output: serde_json::Value, time_ms: u64) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeStatus::Completed,
            output: Some(output),
            error: None,
            execution_time_ms: time_ms,
            metadata: HashMap::new(),
        }
    }
    
    pub fn failure(node_id: impl Into<String>, error: impl Into<String>, time_ms: u64) -> Self {
        Self {
            node_id: node_id.into(),
            status: NodeStatus::Failed,
            output: None,
            error: Some(error.into()),
            execution_time_ms: time_ms,
            metadata: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Waiting,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_nodes_executed: usize,
    pub failed_nodes: usize,
    pub total_retries: usize,
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
}

impl ExecutionStats {
    pub fn new() -> Self {
        Self {
            total_nodes_executed: 0,
            failed_nodes: 0,
            total_retries: 0,
            start_time_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            end_time_ms: None,
        }
    }
    
    pub fn total_time_ms(&self) -> u64 {
        let end = self.end_time_ms.unwrap_or(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        );
        end.saturating_sub(self.start_time_ms)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub timestamp_ms: u64,
    pub event_type: ExecutionEventType,
    pub node_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    NodeStarted,
    NodeCompleted,
    NodeFailed,
    WaitingForDependencies,
    AllCompleted,
    GraphFailed,
}

#[derive(Debug, Clone)]
pub struct GraphContext {
    pub request_id: String,
    pub input: serde_json::Value,
    pub results: HashMap<String, NodeResult>,
    pub completed: HashSet<String>,
    pub running: HashSet<String>,
    pub pending: Vec<String>,
    pub failed: HashSet<String>,
    pub status: ExecutionStatus,
    pub stats: ExecutionStats,
    pub events: Vec<ExecutionEvent>,
    pub shared_data: HashMap<String, serde_json::Value>,
}

impl GraphContext {
    pub fn new(request_id: impl Into<String>, input: serde_json::Value, graph: &GraphDef) -> Self {
        Self {
            request_id: request_id.into(),
            input,
            results: HashMap::new(),
            completed: HashSet::new(),
            running: HashSet::new(),
            pending: vec![graph.start.clone()],
            failed: HashSet::new(),
            status: ExecutionStatus::Pending,
            stats: ExecutionStats::new(),
            events: Vec::new(),
            shared_data: HashMap::new(),
        }
    }
    
    pub fn add_event(&mut self, event: ExecutionEventType, node_id: Option<String>, message: impl Into<String>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.events.push(ExecutionEvent {
            timestamp_ms: timestamp,
            event_type: event,
            node_id,
            message: message.into(),
        });
    }
    
    pub fn is_complete(&self, end_nodes: &[String]) -> bool {
        end_nodes.iter().all(|n| self.completed.contains(n))
    }
    
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }
    
    pub fn can_continue(&self, continue_on_error: bool) -> bool {
        if continue_on_error {
            true
        } else {
            !self.has_failures()
        }
    }
    
    pub fn set_shared_value(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.shared_data.insert(key.into(), value);
    }
    
    pub fn get_shared_value(&self, key: &str) -> Option<&serde_json::Value> {
        self.shared_data.get(key)
    }
    
    pub fn get_shared_value_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.shared_data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
    
    pub fn update_shared_value<F>(&mut self, key: &str, updater: F)
    where
        F: FnOnce(&mut serde_json::Value),
    {
        if let Some(value) = self.shared_data.get_mut(key) {
            updater(value);
        }
    }
    
    pub fn remove_shared(&mut self, key: &str) -> Option<serde_json::Value> {
        self.shared_data.remove(key)
    }
    
    pub fn clear_shared(&mut self) {
        self.shared_data.clear();
    }
}

#[derive(Debug, Clone)]
pub struct GraphResponse {
    pub request_id: String,
    pub status: ExecutionStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub stats: ExecutionStats,
    pub events: Vec<ExecutionEvent>,
    pub node_results: HashMap<String, NodeResult>,
}

impl GraphResponse {
    pub fn from_context(context: GraphContext, end_nodes: &[String]) -> Self {
        let status = if context.has_failures() {
            ExecutionStatus::Failed
        } else if context.is_complete(end_nodes) {
            ExecutionStatus::Completed
        } else {
            ExecutionStatus::Failed
        };
        
        let final_node = end_nodes.last();
        let result = final_node.and_then(|n| context.results.get(n))
            .and_then(|r| r.output.clone());
        
        let error = if context.has_failures() {
            let errors: Vec<String> = context.results.values()
                .filter_map(|r| r.error.clone())
                .collect();
            Some(errors.join("; "))
        } else {
            None
        };
        
        let mut stats = context.stats;
        stats.end_time_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        );
        
        Self {
            request_id: context.request_id,
            status,
            result,
            error,
            stats,
            events: context.events,
            node_results: context.results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::definition::{GraphDef, NodeDef, NodeType, EdgeDef};
    
    fn create_test_graph() -> GraphDef {
        GraphDef::new("test", "Test")
            .with_node(NodeDef::new("start", NodeType::Router))
            .with_node(NodeDef::new("a", NodeType::Executor).with_agent("agent_a"))
            .with_node(NodeDef::new("b", NodeType::Executor).with_agent("agent_b"))
            .with_node(NodeDef::new("end", NodeType::Aggregator))
            .with_edge(EdgeDef::new("start", "a"))
            .with_edge(EdgeDef::new("start", "b"))
            .with_edge(EdgeDef::new("a", "end"))
            .with_edge(EdgeDef::new("b", "end"))
            .with_end("end")
    }
    
    #[test]
    fn test_context_creation() {
        let graph = create_test_graph();
        let context = GraphContext::new("req_1", serde_json::json!({"query": "test"}), &graph);
        
        assert_eq!(context.pending, vec!["start"]);
        assert_eq!(context.status, ExecutionStatus::Pending);
        assert_eq!(context.request_id, "req_1");
    }
    
    #[test]
    fn test_context_add_event() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.add_event(ExecutionEventType::NodeStarted, Some("node_a".to_string()), "Starting node");
        
        assert_eq!(context.events.len(), 1);
        assert_eq!(context.events[0].node_id, Some("node_a".to_string()));
    }
    
    #[test]
    fn test_context_is_complete() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        assert!(!context.is_complete(&["end".to_string()]));
        
        context.completed.insert("end".to_string());
        
        assert!(context.is_complete(&["end".to_string()]));
    }
    
    #[test]
    fn test_context_has_failures() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        assert!(!context.has_failures());
        
        context.failed.insert("node_a".to_string());
        
        assert!(context.has_failures());
    }
    
    #[test]
    fn test_context_can_continue() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        assert!(context.can_continue(true));
        assert!(context.can_continue(false));
        
        context.failed.insert("node_a".to_string());
        
        assert!(context.can_continue(true));
        assert!(!context.can_continue(false));
    }
    
    #[test]
    fn test_node_result_success() {
        let result = NodeResult::success("node_1", serde_json::json!({"data": "test"}), 100);
        
        assert_eq!(result.node_id, "node_1");
        assert_eq!(result.status, NodeStatus::Completed);
        assert!(result.output.is_some());
        assert!(result.error.is_none());
        assert_eq!(result.execution_time_ms, 100);
    }
    
    #[test]
    fn test_node_result_failure() {
        let result = NodeResult::failure("node_1", "Something went wrong", 50);
        
        assert_eq!(result.node_id, "node_1");
        assert_eq!(result.status, NodeStatus::Failed);
        assert!(result.output.is_none());
        assert!(result.error.is_some());
        assert_eq!(result.execution_time_ms, 50);
    }
    
    #[test]
    fn test_execution_stats_new() {
        let stats = ExecutionStats::new();
        
        assert_eq!(stats.total_nodes_executed, 0);
        assert_eq!(stats.failed_nodes, 0);
        assert_eq!(stats.total_retries, 0);
        assert!(stats.end_time_ms.is_none());
    }
    
    #[test]
    fn test_execution_stats_total_time() {
        let mut stats = ExecutionStats::new();
        
        stats.start_time_ms = 1000;
        stats.end_time_ms = Some(1500);
        
        assert_eq!(stats.total_time_ms(), 500);
    }
    
    #[test]
    fn test_execution_status_variants() {
        let statuses = vec![
            ExecutionStatus::Pending,
            ExecutionStatus::Running,
            ExecutionStatus::Waiting,
            ExecutionStatus::Completed,
            ExecutionStatus::Failed,
            ExecutionStatus::Cancelled,
        ];
        
        for status in statuses {
            let _ = format!("{:?}", status);
        }
    }
    
    #[test]
    fn test_node_status_variants() {
        let statuses = vec![
            NodeStatus::Pending,
            NodeStatus::Running,
            NodeStatus::Completed,
            NodeStatus::Failed,
            NodeStatus::Cancelled,
        ];
        
        for status in statuses {
            let _ = format!("{:?}", status);
        }
    }
    
    #[test]
    fn test_execution_event_variants() {
        let events = vec![
            ExecutionEvent {
                timestamp_ms: 1000,
                event_type: ExecutionEventType::NodeStarted,
                node_id: Some("node_1".to_string()),
                message: "Started".to_string(),
            },
            ExecutionEvent {
                timestamp_ms: 2000,
                event_type: ExecutionEventType::NodeCompleted,
                node_id: Some("node_1".to_string()),
                message: "Completed".to_string(),
            },
            ExecutionEvent {
                timestamp_ms: 3000,
                event_type: ExecutionEventType::AllCompleted,
                node_id: None,
                message: "All done".to_string(),
            },
        ];
        
        assert_eq!(events.len(), 3);
    }
    
    #[test]
    fn test_graph_response_from_context() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.completed.insert("start".to_string());
        context.completed.insert("a".to_string());
        context.completed.insert("b".to_string());
        context.completed.insert("end".to_string());
        
        context.results.insert("end".to_string(), NodeResult::success(
            "end",
            serde_json::json!({"result": "done"}),
            100
        ));
        
        let response = GraphResponse::from_context(context, &["end".to_string()]);
        
        assert_eq!(response.status, ExecutionStatus::Completed);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }
    
    #[test]
    fn test_graph_response_from_failed_context() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.completed.insert("start".to_string());
        context.failed.insert("a".to_string());
        
        let response = GraphResponse::from_context(context, &["end".to_string()]);
        
        assert_eq!(response.status, ExecutionStatus::Failed);
        assert!(response.error.is_some());
    }
    
    #[test]
    fn test_graph_response_with_multiple_end_nodes() {
        let graph = GraphDef::new("test", "Test")
            .with_node(NodeDef::new("a", NodeType::Executor))
            .with_node(NodeDef::new("b", NodeType::Terminal))
            .with_node(NodeDef::new("c", NodeType::Terminal))
            .with_edge(EdgeDef::new("a", "b"))
            .with_edge(EdgeDef::new("a", "c"))
            .with_end("b")
            .with_end("c");
        
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.completed.insert("a".to_string());
        context.completed.insert("b".to_string());
        
        let response = GraphResponse::from_context(context, &["b".to_string(), "c".to_string()]);
        
        assert_eq!(response.status, ExecutionStatus::Failed);
    }
    
    #[test]
    fn test_context_shared_data() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.shared_data.insert("key1".to_string(), serde_json::json!("value1"));
        context.shared_data.insert("key2".to_string(), serde_json::json!(42));
        
        assert_eq!(context.shared_data.get("key1").unwrap(), &serde_json::json!("value1"));
        assert_eq!(context.shared_data.get("key2").unwrap(), &serde_json::json!(42));
    }
    
    #[test]
    fn test_set_and_get_shared_value() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.set_shared_value("test_key", serde_json::json!("test_value"));
        
        assert_eq!(context.get_shared_value("test_key"), Some(&serde_json::json!("test_value")));
        assert_eq!(context.get_shared_value("nonexistent"), None);
    }
    
    #[test]
    fn test_get_shared_value_as() {
        #[derive(serde::Deserialize, serde::Serialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            age: u32,
        }
        
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        let test_data = TestData { name: "Alice".to_string(), age: 30 };
        context.set_shared_value("person", serde_json::to_value(&test_data).unwrap());
        
        let retrieved: Option<TestData> = context.get_shared_value_as("person");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Alice");
        
        let none: Option<TestData> = context.get_shared_value_as("nonexistent");
        assert!(none.is_none());
    }
    
    #[test]
    fn test_update_shared_value() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.set_shared_value("counter", serde_json::json!(0));
        context.update_shared_value("counter", |v| {
            if let Some(n) = v.as_i64() {
                *v = serde_json::json!(n + 1);
            }
        });
        
        assert_eq!(context.get_shared_value("counter"), Some(&serde_json::json!(1)));
    }
    
    #[test]
    fn test_remove_shared() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.set_shared_value("to_remove", serde_json::json!("value"));
        let removed = context.remove_shared("to_remove");
        
        assert!(removed.is_some());
        assert_eq!(removed.unwrap(), serde_json::json!("value"));
        assert_eq!(context.get_shared_value("to_remove"), None);
    }
    
    #[test]
    fn test_clear_shared() {
        let graph = create_test_graph();
        let mut context = GraphContext::new("req_1", serde_json::json!({}), &graph);
        
        context.set_shared_value("key1", serde_json::json!("value1"));
        context.set_shared_value("key2", serde_json::json!("value2"));
        
        context.clear_shared();
        
        assert!(context.shared_data.is_empty());
    }
}
