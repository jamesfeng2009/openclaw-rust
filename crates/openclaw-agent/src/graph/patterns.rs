//! Predefined Collaboration Patterns

use super::definition::{EdgeDef, GraphDef, NodeConfig, NodeDef, NodeType};

#[derive(Debug, Clone)]
pub enum CollaborationPattern {
    Sequential,
    Parallel,
    Broadcast,
    Tree,
    MapReduce,
    MixtureOfExperts,
}

pub struct GraphPatterns;

impl GraphPatterns {
    pub fn sequential(agent_ids: &[&str]) -> GraphDef {
        let mut graph = GraphDef::new("sequential", "Sequential Pattern");
        
        if agent_ids.is_empty() {
            return graph;
        }
        
        let mut prev_id = String::new();
        
        for (i, agent_id) in agent_ids.iter().enumerate() {
            let node_id = format!("node_{}", i);
            
            graph = graph.with_node(
                NodeDef::new(&node_id, NodeType::Executor)
                    .with_agent(*agent_id)
                    .with_name(format!("Step {}", i + 1))
            );
            
            if i == 0 {
                graph = graph.with_node(
                    NodeDef::new("start", NodeType::Router)
                        .with_name("Start")
                );
                graph = graph.with_edge(EdgeDef::new("start", &node_id));
                graph = graph.with_start("start".to_string());
            } else {
                graph = graph.with_edge(EdgeDef::new(&prev_id, &node_id));
            }
            
            prev_id = node_id;
        }
        
        let end_id = format!("end");
        graph = graph.with_node(
            NodeDef::new(&end_id, NodeType::Terminal)
                .with_name("End")
        );
        graph = graph.with_edge(EdgeDef::new(&prev_id, &end_id));
        graph = graph.with_end(end_id);
        
        graph
    }
    
    pub fn parallel(agent_ids: &[&str]) -> GraphDef {
        let mut graph = GraphDef::new("parallel", "Parallel Pattern");
        
        graph = graph.with_node(
            NodeDef::new("start", NodeType::Router).with_name("Start")
        );
        graph = graph.with_start("start".to_string());
        
        let workers: Vec<NodeDef> = agent_ids
            .iter()
            .enumerate()
            .map(|(i, agent_id)| {
                NodeDef::new(format!("worker_{}", i), NodeType::Executor)
                    .with_agent(*agent_id)
                    .with_name(format!("Worker {}", i + 1))
            })
            .collect();
        
        for worker in &workers {
            graph = graph.with_node(worker.clone());
            graph = graph.with_edge(EdgeDef::new("start", &worker.id));
        }
        
        let end_id = "end";
        graph = graph.with_node(
            NodeDef::new(end_id, NodeType::Aggregator)
                .with_name("End")
                .with_config(NodeConfig {
                    aggregation: Some("merge".to_string()),
                    ..Default::default()
                })
        );
        
        for worker in &workers {
            graph = graph.with_edge(EdgeDef::new(&worker.id, end_id));
        }
        
        graph = graph.with_end(end_id.to_string());
        
        graph
    }
    
    pub fn broadcast(agent_ids: &[&str]) -> GraphDef {
        let mut graph = GraphDef::new("broadcast", "Broadcast Pattern");
        
        graph = graph.with_node(
            NodeDef::new("start", NodeType::Router).with_name("Broadcaster")
        );
        graph = graph.with_start("start".to_string());
        
        let mut end_nodes = Vec::new();
        
        for (i, agent_id) in agent_ids.iter().enumerate() {
            let worker_id = format!("worker_{}", i);
            graph = graph.with_node(
                NodeDef::new(&worker_id, NodeType::Executor)
                    .with_agent(*agent_id)
                    .with_name(format!("Receiver {}", i + 1))
            );
            graph = graph.with_edge(EdgeDef::new("start", &worker_id));
            end_nodes.push(worker_id);
        }
        
        for end in end_nodes {
            graph = graph.with_end(end);
        }
        
        graph
    }
    
    pub fn tree(depth: usize, branching: usize, agent_ids: &[&str]) -> GraphDef {
        let mut graph = GraphDef::new("tree", "Tree Pattern");
        
        graph = graph.with_node(
            NodeDef::new("root", NodeType::Router).with_name("Root")
        );
        graph = graph.with_start("root".to_string());
        
        let mut current_level = vec!["root".to_string()];
        let mut agent_index = 0;
        
        for d in 0..depth {
            let mut next_level = Vec::new();
            
            for parent_id in &current_level {
                for b in 0..branching {
                    if agent_index >= agent_ids.len() {
                        break;
                    }
                    
                    let node_id = format!("node_{}_{}", d, b);
                    graph = graph.with_node(
                        NodeDef::new(&node_id, NodeType::Executor)
                            .with_agent(agent_ids[agent_index])
                            .with_name(format!("Level {} Node {}", d + 1, b + 1))
                    );
                    graph = graph.with_edge(EdgeDef::new(parent_id, &node_id));
                    next_level.push(node_id);
                    agent_index += 1;
                }
            }
            
            current_level = next_level;
        }
        
        for leaf in &current_level {
            graph = graph.with_end(leaf.clone());
        }
        
        graph
    }
    
    pub fn map_reduce(map_agents: &[&str], reduce_agent: &str) -> GraphDef {
        let mut graph = GraphDef::new("map_reduce", "MapReduce Pattern");
        
        graph = graph.with_node(
            NodeDef::new("start", NodeType::Router).with_name("Splitter")
        );
        graph = graph.with_start("start".to_string());
        
        let map_nodes: Vec<NodeDef> = map_agents
            .iter()
            .enumerate()
            .map(|(i, agent_id)| {
                NodeDef::new(format!("map_{}", i), NodeType::Executor)
                    .with_agent(*agent_id)
                    .with_name(format!("Mapper {}", i + 1))
            })
            .collect();
        
        for node in &map_nodes {
            graph = graph.with_node(node.clone());
            graph = graph.with_edge(EdgeDef::new("start", &node.id));
        }
        
        let reduce_id = "reduce";
        graph = graph.with_node(
            NodeDef::new(reduce_id, NodeType::Aggregator)
                .with_agent(reduce_agent)
                .with_name("Reducer")
                .with_config(NodeConfig {
                    aggregation: Some("merge".to_string()),
                    ..Default::default()
                })
        );
        
        for node in &map_nodes {
            graph = graph.with_edge(EdgeDef::new(&node.id, reduce_id));
        }
        
        let end_id = "end";
        graph = graph.with_node(
            NodeDef::new(end_id, NodeType::Terminal).with_name("End")
        );
        graph = graph.with_edge(EdgeDef::new(reduce_id, end_id));
        
        graph = graph.with_end(end_id.to_string());
        
        graph
    }
    
    pub fn mixture_of_experts(expert_agents: &[&str]) -> GraphDef {
        let mut graph = GraphDef::new("moe", "Mixture of Experts");
        
        graph = graph.with_node(
            NodeDef::new("router", NodeType::Router).with_name("Expert Router")
        );
        graph = graph.with_start("router".to_string());
        
        let expert_nodes: Vec<NodeDef> = expert_agents
            .iter()
            .enumerate()
            .map(|(i, agent_id)| {
                NodeDef::new(format!("expert_{}", i), NodeType::Executor)
                    .with_agent(*agent_id)
                    .with_name(format!("Expert {}", i + 1))
            })
            .collect();
        
        for node in &expert_nodes {
            graph = graph.with_node(node.clone());
            graph = graph.with_edge(EdgeDef::new("router", &node.id));
        }
        
        let aggregator_id = "aggregator";
        graph = graph.with_node(
            NodeDef::new(aggregator_id, NodeType::Aggregator)
                .with_name("Result Aggregator")
                .with_config(NodeConfig {
                    aggregation: Some("merge".to_string()),
                    ..Default::default()
                })
        );
        
        for node in &expert_nodes {
            graph = graph.with_edge(EdgeDef::new(&node.id, aggregator_id));
        }
        
        graph = graph.with_end(aggregator_id.to_string());
        
        graph
    }
    
    pub fn from_pattern(pattern: CollaborationPattern, agents: &[&str]) -> GraphDef {
        match pattern {
            CollaborationPattern::Sequential => Self::sequential(agents),
            CollaborationPattern::Parallel => Self::parallel(agents),
            CollaborationPattern::Broadcast => Self::broadcast(agents),
            CollaborationPattern::Tree => Self::tree(2, 2, agents),
            CollaborationPattern::MapReduce => {
                let mid = agents.len() / 2;
                Self::map_reduce(&agents[..mid], agents[mid])
            }
            CollaborationPattern::MixtureOfExperts => Self::mixture_of_experts(agents),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sequential_pattern() {
        let agents = &["agent_a", "agent_b", "agent_c"];
        let graph = GraphPatterns::sequential(agents);
        
        assert_eq!(graph.nodes.len(), 5); // start + 3 workers + end
        assert_eq!(graph.edges.len(), 4); // start→node_0 + 3 worker edges + worker→end
    }
    
    #[test]
    fn test_parallel_pattern() {
        let agents = &["agent_a", "agent_b", "agent_c"];
        let graph = GraphPatterns::parallel(agents);
        
        assert_eq!(graph.nodes.len(), 5); // start + 3 workers + end
        assert_eq!(graph.edges.len(), 6); // start→3workers(3) + 3workers→end(3)
    }
    
    #[test]
    fn test_map_reduce_pattern() {
        let map_agents = &["agent_a", "agent_b"];
        let graph = GraphPatterns::map_reduce(map_agents, "reducer");
        
        assert!(graph.nodes.iter().any(|n| n.id == "reduce"));
    }
    
    #[test]
    fn test_moe_pattern() {
        let experts = &["expert_1", "expert_2", "expert_3"];
        let graph = GraphPatterns::mixture_of_experts(experts);
        
        assert!(graph.nodes.iter().any(|n| n.id == "router"));
        assert!(graph.nodes.iter().any(|n| n.id == "aggregator"));
    }
    
    #[test]
    fn test_broadcast_pattern() {
        let agents = &["agent_a", "agent_b", "agent_c"];
        let graph = GraphPatterns::broadcast(agents);
        
        assert!(graph.nodes.iter().any(|n| n.id == "start"));
        for i in 0..3 {
            assert!(graph.nodes.iter().any(|n| n.id == format!("worker_{}", i)));
        }
    }
    
    #[test]
    fn test_tree_pattern() {
        let agents = &["agent_1", "agent_2", "agent_3", "agent_4"];
        let graph = GraphPatterns::tree(2, 2, agents);
        
        assert!(graph.nodes.iter().any(|n| n.id == "root"));
        assert!(!graph.end.is_empty());
    }
    
    #[test]
    fn test_from_pattern_sequential() {
        let agents = &["agent_a", "agent_b"];
        let graph = GraphPatterns::from_pattern(CollaborationPattern::Sequential, agents);
        
        assert!(!graph.nodes.is_empty());
    }
    
    #[test]
    fn test_from_pattern_parallel() {
        let agents = &["agent_a", "agent_b"];
        let graph = GraphPatterns::from_pattern(CollaborationPattern::Parallel, agents);
        
        assert!(!graph.nodes.is_empty());
    }
    
    #[test]
    fn test_from_pattern_map_reduce() {
        let agents = &["map_1", "map_2", "reduce"];
        let graph = GraphPatterns::from_pattern(CollaborationPattern::MapReduce, agents);
        
        assert!(!graph.nodes.is_empty());
    }
    
    #[test]
    fn test_from_pattern_moe() {
        let agents = &["expert_1", "expert_2", "expert_3"];
        let graph = GraphPatterns::from_pattern(CollaborationPattern::MixtureOfExperts, agents);
        
        assert!(graph.nodes.iter().any(|n| n.id == "router"));
    }
    
    #[test]
    fn test_empty_sequential() {
        let agents: &[&str] = &[];
        let graph = GraphPatterns::sequential(agents);
        
        assert_eq!(graph.nodes.len(), 0);
    }
    
    #[test]
    fn test_single_agent_parallel() {
        let agents = &["single_agent"];
        let graph = GraphPatterns::parallel(agents);
        
        assert!(graph.nodes.len() >= 2);
    }
    
    #[test]
    fn test_pattern_has_start_node() {
        let agents = &["a", "b", "c"];
        
        let seq_graph = GraphPatterns::sequential(agents);
        assert!(!seq_graph.start.is_empty());
        
        let par_graph = GraphPatterns::parallel(agents);
        assert!(!par_graph.start.is_empty());
    }
    
    #[test]
    fn test_pattern_has_end_nodes() {
        let agents = &["a", "b", "c"];
        
        let seq_graph = GraphPatterns::sequential(agents);
        assert!(!seq_graph.end.is_empty());
        
        let par_graph = GraphPatterns::parallel(agents);
        assert!(!par_graph.end.is_empty());
    }
}
