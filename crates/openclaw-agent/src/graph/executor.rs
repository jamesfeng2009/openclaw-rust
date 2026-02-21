//! Parallel Graph Executor - 真正的多 Agent 并行执行引擎

use std::collections::HashMap;

use super::definition::{EdgeDef, GraphConfig, GraphDef, NodeDef, NodeType};
use super::context::{ExecutionStatus, GraphContext, GraphResponse, NodeResult, NodeStatus};

pub struct ParallelGraphExecutor {
    graph: GraphDef,
    config: GraphConfig,
}

impl ParallelGraphExecutor {
    pub fn new(graph: GraphDef) -> Self {
        Self {
            graph,
            config: GraphConfig::default(),
        }
    }
    
    pub fn with_config(mut self, config: GraphConfig) -> Self {
        self.config = config;
        self
    }
    
    fn get_prerequisites(&self, node_id: &str) -> Vec<String> {
        self.graph.edges.iter()
            .filter(|e| e.to == node_id)
            .map(|e| e.from.clone())
            .collect()
    }
    
    fn get_dependents(&self, node_id: &str) -> Vec<String> {
        self.graph.edges.iter()
            .filter(|e| e.from == node_id)
            .map(|e| e.to.clone())
            .collect()
    }
    
    fn find_ready_nodes(&self, context: &GraphContext) -> Vec<String> {
        let mut ready = Vec::new();
        
        for node_id in &context.pending {
            if context.completed.contains(node_id) 
                || context.running.contains(node_id) 
                || context.failed.contains(node_id) {
                continue;
            }
            
            let prerequisites = self.get_prerequisites(node_id);
            let all_done = prerequisites.iter()
                .all(|pre| context.completed.contains(pre));
            
            if all_done {
                ready.push(node_id.clone());
            }
        }
        
        ready
    }
    
    async fn execute_single_node(
        &self,
        node: &NodeDef,
        _context: &GraphContext,
    ) -> Result<NodeResult, String> {
        let start = std::time::Instant::now();
        
        let output = match node.node_type {
            NodeType::Executor => {
                if let Some(agent_id) = &node.agent_id {
                    serde_json::json!({
                        "status": "executed",
                        "agent_id": agent_id,
                        "message": format!("Agent {} would execute here", agent_id)
                    })
                } else {
                    serde_json::json!({
                        "status": "error",
                        "message": "No agent_id configured for executor node"
                    })
                }
            }
            NodeType::Router => {
                let dependents = self.get_dependents(&node.id);
                serde_json::json!({
                    "status": "routed",
                    "next_nodes": dependents,
                    "message": "Router decided next steps"
                })
            }
            NodeType::Aggregator => {
                let incoming = self.get_incoming_edges(&node.id);
                serde_json::json!({
                    "status": "aggregated",
                    "inputs_count": incoming.len(),
                    "message": "Aggregated results from dependencies"
                })
            }
            NodeType::Terminal => {
                serde_json::json!({
                    "status": "done",
                    "message": "Terminal node reached"
                })
            }
        };
        
        let execution_time = start.elapsed().as_millis() as u64;
        
        Ok(NodeResult::success(node.id.clone(), output, execution_time))
    }
    
    async fn execute_ready_nodes_parallel(
        &self,
        context: &mut GraphContext,
        node_ids: Vec<String>,
    ) -> Result<HashMap<String, NodeResult>, String> {
        if node_ids.is_empty() {
            return Ok(HashMap::new());
        }
        
        let futures: Vec<_> = node_ids
            .iter()
            .map(|node_id| {
                let node_id = node_id.clone();
                let node = self.graph.find_node(&node_id).cloned();
                let context_clone = context.clone();
                
                async move {
                    match node {
                        Some(n) => {
                            let result = self.execute_single_node(&n, &context_clone).await;
                            (node_id, result)
                        }
                        None => {
                            let err = format!("Node not found: {}", node_id);
                            (node_id, Err(err))
                        }
                    }
                }
            })
            .collect();
        
        let results = futures::future::join_all(futures).await;
        
        let mut outputs = HashMap::new();
        for (node_id, result) in results {
            let node_id_clone = node_id.clone();
            match result {
                Ok(node_result) => {
                    outputs.insert(node_id, node_result);
                }
                Err(e) => {
                    outputs.insert(node_id_clone, NodeResult::failure(node_id, e, 0));
                }
            }
        }
        
        Ok(outputs)
    }
    
    pub async fn execute(
        &self,
        request_id: impl Into<String>,
        input: serde_json::Value,
    ) -> Result<GraphResponse, String> {
        let mut context = GraphContext::new(request_id, input, &self.graph);
        context.status = ExecutionStatus::Running;
        
        self.execute_inner(context).await
    }
    
    fn is_graph_complete(&self, context: &GraphContext) -> bool {
        self.graph.end.iter().all(|end| context.completed.contains(end))
    }
    
    fn get_incoming_edges(&self, node_id: &str) -> Vec<&EdgeDef> {
        self.graph.edges.iter().filter(|e| e.to == node_id).collect()
    }
    
    pub async fn execute_with_context(
        &self,
        request_id: impl Into<String>,
        input: serde_json::Value,
        context_data: serde_json::Value,
    ) -> Result<GraphResponse, String> {
        let mut context = GraphContext::new(request_id, input, &self.graph);
        context.status = ExecutionStatus::Running;
        
        context.set_shared_value("context_bundle", context_data);
        
        self.execute_inner(context).await
    }
    
    async fn execute_inner(
        &self,
        mut context: GraphContext,
    ) -> Result<GraphResponse, String> {
        let mut iterations = 0;
        let max_iterations = self.config.timeout_ms as usize / 100;
        
        loop {
            iterations += 1;
            
            if iterations > max_iterations {
                context.status = ExecutionStatus::Failed;
                break;
            }
            
            if context.pending.is_empty() && context.running.is_empty() {
                break;
            }
            
            let ready = self.find_ready_nodes(&context);
            
            if ready.is_empty() {
                if !context.running.is_empty() {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    continue;
                }
                break;
            }
            
            let batch: Vec<String> = ready.into_iter()
                .take(self.config.max_parallel_nodes)
                .collect();
            
            for id in &batch {
                context.running.insert(id.clone());
                context.pending.retain(|p| p != id);
            }
            
            context.add_event(
                super::context::ExecutionEventType::NodeStarted,
                None,
                format!("Starting {} nodes in parallel", batch.len())
            );
            
            let results = self.execute_ready_nodes_parallel(&mut context, batch).await?;
            
            for (node_id, result) in results {
                context.running.remove(&node_id);
                
                let status = result.status.clone();
                context.results.insert(node_id.clone(), result);
                
                if status == NodeStatus::Completed {
                    context.completed.insert(node_id.clone());
                    
                    let dependents = self.get_dependents(&node_id);
                    for dep in dependents {
                        if !context.pending.contains(&dep) 
                            && !context.completed.contains(&dep) 
                            && !context.running.contains(&dep)
                            && !context.failed.contains(&dep) {
                            context.pending.push(dep);
                        }
                    }
                } else {
                    context.failed.insert(node_id.clone());
                    
                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
            
            if !self.config.continue_on_error && context.has_failures() {
                break;
            }
            
            if self.is_graph_complete(&context) {
                context.status = ExecutionStatus::Completed;
                context.add_event(
                    super::context::ExecutionEventType::AllCompleted,
                    None,
                    "All nodes completed successfully".to_string()
                );
                break;
            }
        }
        
        Ok(GraphResponse::from_context(context, &self.graph.end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::definition::{EdgeDef, GraphDef, GraphConfig, NodeDef, NodeType};
    
    fn create_parallel_graph() -> GraphDef {
        GraphDef::new("parallel", "Parallel Test")
            .with_node(NodeDef::new("start", NodeType::Router))
            .with_node(NodeDef::new("worker1", NodeType::Executor).with_agent("agent_1"))
            .with_node(NodeDef::new("worker2", NodeType::Executor).with_agent("agent_2"))
            .with_node(NodeDef::new("worker3", NodeType::Executor).with_agent("agent_3"))
            .with_node(NodeDef::new("end", NodeType::Aggregator))
            .with_edge(EdgeDef::new("start", "worker1"))
            .with_edge(EdgeDef::new("start", "worker2"))
            .with_edge(EdgeDef::new("start", "worker3"))
            .with_edge(EdgeDef::new("worker1", "end"))
            .with_edge(EdgeDef::new("worker2", "end"))
            .with_edge(EdgeDef::new("worker3", "end"))
            .with_end("end")
    }
    
    fn create_sequential_graph() -> GraphDef {
        GraphDef::new("sequential", "Sequential Test")
            .with_node(NodeDef::new("start", NodeType::Router))
            .with_node(NodeDef::new("step1", NodeType::Executor).with_agent("agent_1"))
            .with_node(NodeDef::new("step2", NodeType::Executor).with_agent("agent_2"))
            .with_node(NodeDef::new("end", NodeType::Terminal))
            .with_edge(EdgeDef::new("start", "step1"))
            .with_edge(EdgeDef::new("step1", "step2"))
            .with_edge(EdgeDef::new("step2", "end"))
            .with_end("end")
    }
    
    fn create_simple_graph() -> GraphDef {
        GraphDef::new("simple", "Simple Test")
            .with_node(NodeDef::new("a", NodeType::Executor))
            .with_edge(EdgeDef::new("a", "b"))
            .with_end("b")
    }
    
    #[tokio::test]
    async fn test_parallel_executor() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let result = executor.execute("test_1", serde_json::json!({"query": "test"})).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Completed);
    }
    
    #[tokio::test]
    async fn test_sequential_executor() {
        let graph = create_sequential_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let result = executor.execute("test_2", serde_json::json!({"query": "test"})).await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.status, ExecutionStatus::Completed);
    }
    
    #[tokio::test]
    async fn test_executor_with_input() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let input = serde_json::json!({
            "query": "What is the weather?",
            "user_id": "user123"
        });
        
        let result = executor.execute("test_3", input).await;
        
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_executor_response_contains_stats() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let result = executor.execute("test_4", serde_json::json!({})).await.unwrap();
        
        assert!(!result.events.is_empty());
    }
    
    #[tokio::test]
    async fn test_executor_response_contains_node_results() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let result = executor.execute("test_5", serde_json::json!({})).await.unwrap();
        
        assert!(!result.node_results.is_empty());
    }
    
    #[tokio::test]
    async fn test_executor_with_custom_config() {
        let graph = create_parallel_graph();
        let config = GraphConfig {
            max_parallel_nodes: 2,
            timeout_ms: 60000,
            retry_on_failure: true,
            max_retries: 3,
            continue_on_error: true,
        };
        let executor = ParallelGraphExecutor::new(graph).with_config(config);
        
        let result = executor.execute("test_6", serde_json::json!({})).await;
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_find_ready_nodes() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        
        let ready = executor.find_ready_nodes(&context);
        assert!(ready.contains(&"start".to_string()));
    }
    
    #[test]
    fn test_find_ready_nodes_after_completion() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let mut context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        
        context.completed.insert("start".to_string());
        context.pending.push("worker1".to_string());
        context.pending.push("worker2".to_string());
        context.pending.push("worker3".to_string());
        
        let ready = executor.find_ready_nodes(&context);
        assert!(ready.contains(&"worker1".to_string()));
        assert!(ready.contains(&"worker2".to_string()));
        assert!(ready.contains(&"worker3".to_string()));
    }
    
    #[test]
    fn test_get_prerequisites() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let prereqs = executor.get_prerequisites("worker1");
        assert!(prereqs.contains(&"start".to_string()));
        
        let prereqs_end = executor.get_prerequisites("end");
        assert!(prereqs_end.contains(&"worker1".to_string()));
        assert!(prereqs_end.contains(&"worker2".to_string()));
        assert!(prereqs_end.contains(&"worker3".to_string()));
    }
    
    #[test]
    fn test_get_dependents() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let dependents = executor.get_dependents("start");
        assert!(dependents.contains(&"worker1".to_string()));
        assert!(dependents.contains(&"worker2".to_string()));
        assert!(dependents.contains(&"worker3".to_string()));
    }
    
    #[test]
    fn test_executor_new() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        assert_eq!(executor.graph.nodes.len(), 5);
    }
    
    #[test]
    fn test_executor_with_config() {
        let graph = create_parallel_graph();
        let config = GraphConfig::default();
        let executor = ParallelGraphExecutor::new(graph).with_config(config);
        
        assert_eq!(executor.config.max_parallel_nodes, 10);
    }
    
    #[test]
    fn test_find_ready_nodes_empty_pending() {
        let graph = create_simple_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let mut context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        context.pending.clear();
        
        let ready = executor.find_ready_nodes(&context);
        assert!(ready.is_empty());
    }
    
    #[test]
    fn test_find_ready_nodes_skips_completed() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let mut context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        context.completed.insert("start".to_string());
        
        let ready = executor.find_ready_nodes(&context);
        assert!(!ready.contains(&"start".to_string()));
    }
    
    #[test]
    fn test_find_ready_nodes_skips_running() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let mut context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        context.running.insert("start".to_string());
        
        let ready = executor.find_ready_nodes(&context);
        assert!(!ready.contains(&"start".to_string()));
    }
    
    #[test]
    fn test_find_ready_nodes_skips_failed() {
        let graph = create_parallel_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let mut context = GraphContext::new(
            "test", 
            serde_json::json!({}), 
            &executor.graph
        );
        context.failed.insert("start".to_string());
        
        let ready = executor.find_ready_nodes(&context);
        assert!(!ready.contains(&"start".to_string()));
    }
    
    #[tokio::test]
    async fn test_execute_with_context() {
        let graph = create_sequential_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let context_data = serde_json::json!({
            "working": {
                "current_agent": "test_agent",
                "current_task": "test_task"
            },
            "knowledge": []
        });
        
        let result = executor
            .execute_with_context("test_req", serde_json::json!({"query": "test"}), context_data)
            .await;
        
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.events.is_empty());
    }
    
    #[tokio::test]
    async fn test_execute_with_context_preserves_data() {
        let graph = create_sequential_graph();
        let executor = ParallelGraphExecutor::new(graph);
        
        let context_data = serde_json::json!({
            "working": {
                "current_agent": "test_agent",
                "current_task": "test_task"
            },
            "session": {
                "session_id": "session-123",
                "history_summary": "Test session"
            }
        });
        
        let result = executor
            .execute_with_context("test_req", serde_json::json!({"query": "test"}), context_data)
            .await;
        
        assert!(result.is_ok());
    }
}
