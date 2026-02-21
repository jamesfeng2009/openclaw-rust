//! Agent Loop 循环控制

use std::sync::Arc;

use openclaw_core::{Message, Result};

use super::config::ReflectorConfig;
use super::executor::RetrievalResult;
use super::planner::{QueryPlanner, RetrievalPlan, SubQuery};
use super::reflector::Reflection;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentAction {
    Think,
    Plan,
    Retrieve,
    Execute,
    Observe,
    Reflect,
    Answer,
    Done,
}

impl std::fmt::Display for AgentAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentAction::Think => write!(f, "Think"),
            AgentAction::Plan => write!(f, "Plan"),
            AgentAction::Retrieve => write!(f, "Retrieve"),
            AgentAction::Execute => write!(f, "Execute"),
            AgentAction::Observe => write!(f, "Observe"),
            AgentAction::Reflect => write!(f, "Reflect"),
            AgentAction::Answer => write!(f, "Answer"),
            AgentAction::Done => write!(f, "Done"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Thought {
    pub action: AgentAction,
    pub reasoning: String,
    pub timestamp: std::time::SystemTime,
}

impl Thought {
    pub fn new(action: AgentAction, reasoning: String) -> Self {
        Self {
            action,
            reasoning,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<String>,
}

pub struct AgentLoopState {
    pub current_action: AgentAction,
    pub iteration: usize,
    pub max_iterations: usize,
    pub retrieved_context: Vec<RetrievalResult>,
    pub thought_history: Vec<Thought>,
    pub tool_calls: Vec<ToolCall>,
    pub plan: Option<RetrievalPlan>,
    pub current_sub_query: Option<usize>,
}

impl AgentLoopState {
    pub fn new(max_iterations: usize) -> Self {
        Self {
            current_action: AgentAction::Think,
            iteration: 0,
            max_iterations,
            retrieved_context: Vec::new(),
            thought_history: Vec::new(),
            tool_calls: Vec::new(),
            plan: None,
            current_sub_query: None,
        }
    }

    pub fn next_action(&mut self, reflection: &Reflection) -> AgentAction {
        self.iteration += 1;

        if self.iteration >= self.max_iterations {
            return AgentAction::Answer;
        }

        if reflection.is_sufficient {
            AgentAction::Answer
        } else if !reflection.suggestions.is_empty() && self.plan.is_some() {
            AgentAction::Plan
        } else {
            AgentAction::Answer
        }
    }

    pub fn add_thought(&mut self, action: AgentAction, reasoning: String) {
        self.thought_history.push(Thought::new(action, reasoning));
    }

    pub fn add_result(&mut self, result: RetrievalResult) {
        self.retrieved_context.push(result);
    }

    pub fn add_results(&mut self, results: Vec<RetrievalResult>) {
        self.retrieved_context.extend(results);
    }

    pub fn has_sufficient_results(&self, min_confidence: f32) -> bool {
        if self.retrieved_context.is_empty() {
            return false;
        }
        
        let avg_score: f32 = self.retrieved_context.iter()
            .map(|r| r.relevance_score)
            .sum::<f32>() 
            / self.retrieved_context.len() as f32;
        
        avg_score >= min_confidence
    }
}

pub struct AgentLoop<C: LoopController> {
    controller: C,
}

impl<C: LoopController> AgentLoop<C> {
    pub fn new(controller: C) -> Self {
        Self { controller }
    }

    pub async fn run(
        &self,
        query: &str,
        context: &[Message],
        planner: &dyn QueryPlanner,
        executor: &dyn super::executor::RetrievalExecutor,
        reflector: &dyn super::reflector::ResultReflector,
        config: &super::config::ReflectorConfig,
    ) -> Result<LoopResult> {
        let mut state = AgentLoopState::new(config.max_iterations);

        state.add_thought(AgentAction::Think, format!("Analyzing user query: {}", query));

        let plan = planner
            .plan(
                query,
                context,
                &super::config::PlannerConfig::default(),
                &[],
            )
            .await?;

        state.plan = Some(plan.clone());
        state.add_thought(AgentAction::Plan, format!("Created plan with {} sub-queries", plan.sub_queries.len()));

        for (idx, sub_query) in plan.sub_queries.iter().enumerate() {
            state.current_sub_query = Some(idx);
            state.add_thought(
                AgentAction::Retrieve,
                format!("Executing sub-query {}: {}", idx + 1, sub_query.query),
            );

            let results = executor.execute(&sub_query.query, &super::config::ExecutorConfig::default()).await?;
            state.add_results(results);

            state.add_thought(AgentAction::Observe, format!("Retrieved {} results", state.retrieved_context.len()));
        }

        let reflection = reflector.reflect(query, &state.retrieved_context, config).await?;

        if !reflection.is_sufficient && self.controller.should_continue(&state, &reflection) {
            if let Some(ref plan) = state.plan {
                let new_sub_queries = self.generate_refinement_queries(&reflection, plan);
                for sub_query in new_sub_queries {
                    let results = executor.execute(&sub_query.query, &super::config::ExecutorConfig::default()).await?;
                    state.add_results(results);
                }
            }
        }

        state.add_thought(AgentAction::Reflect, format!("Confidence: {:.2}", reflection.confidence));

        let answer = reflector.generate_answer(query, &state.retrieved_context, context).await?;

        state.add_thought(AgentAction::Answer, "Generated final answer".to_string());
        state.add_thought(AgentAction::Done, "Agent loop completed".to_string());

        Ok(LoopResult {
            answer,
            iterations: state.iteration,
            retrieved_results: state.retrieved_context,
            thought_trace: state.thought_history,
            reflection,
        })
    }
}

impl<C: LoopController> AgentLoop<C> {
    fn generate_refinement_queries(&self, reflection: &Reflection, plan: &RetrievalPlan) -> Vec<SubQuery> {
        let mut new_queries = Vec::new();

        for suggestion in &reflection.suggestions {
            new_queries.push(SubQuery {
                query: format!("{} {}", plan.query_rewrite, suggestion),
                source: plan.sources.first().cloned().unwrap_or(super::config::SourceType::Memory),
                description: format!("Refined based on: {}", suggestion),
            });
        }

        new_queries
    }
}

pub trait LoopController: Send + Sync {
    fn should_continue(&self, state: &AgentLoopState, reflection: &Reflection) -> bool;
}

pub struct DefaultLoopController;

impl LoopController for DefaultLoopController {
    fn should_continue(&self, state: &AgentLoopState, reflection: &Reflection) -> bool {
        !reflection.is_sufficient && state.iteration < state.max_iterations
    }
}

pub struct LoopResult {
    pub answer: String,
    pub iterations: usize,
    pub retrieved_results: Vec<RetrievalResult>,
    pub thought_trace: Vec<Thought>,
    pub reflection: Reflection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_action_display() {
        assert_eq!(AgentAction::Think.to_string(), "Think");
        assert_eq!(AgentAction::Plan.to_string(), "Plan");
        assert_eq!(AgentAction::Answer.to_string(), "Answer");
    }

    #[test]
    fn test_agent_loop_state_new() {
        let state = AgentLoopState::new(5);
        assert_eq!(state.current_action, AgentAction::Think);
        assert_eq!(state.iteration, 0);
        assert_eq!(state.max_iterations, 5);
        assert!(state.retrieved_context.is_empty());
    }

    #[test]
    fn test_agent_loop_state_add_thought() {
        let mut state = AgentLoopState::new(3);
        state.add_thought(AgentAction::Think, "Initial reasoning".to_string());
        
        assert_eq!(state.thought_history.len(), 1);
        assert_eq!(state.thought_history[0].action, AgentAction::Think);
    }

    #[test]
    fn test_agent_loop_state_add_results() {
        let mut state = AgentLoopState::new(3);
        
        let results = vec![
            RetrievalResult {
                id: "1".to_string(),
                content: "Content 1".to_string(),
                source: super::super::config::SourceType::Memory,
                relevance_score: 0.9,
                metadata: std::collections::HashMap::new(),
            },
            RetrievalResult {
                id: "2".to_string(),
                content: "Content 2".to_string(),
                source: super::super::config::SourceType::VectorDB,
                relevance_score: 0.8,
                metadata: std::collections::HashMap::new(),
            },
        ];
        
        state.add_results(results);
        
        assert_eq!(state.retrieved_context.len(), 2);
    }

    #[test]
    fn test_has_sufficient_results() {
        let mut state = AgentLoopState::new(3);
        
        let results = vec![
            RetrievalResult {
                id: "1".to_string(),
                content: "Content".to_string(),
                source: super::super::config::SourceType::Memory,
                relevance_score: 0.9,
                metadata: std::collections::HashMap::new(),
            },
        ];
        
        state.add_results(results);
        
        assert!(state.has_sufficient_results(0.7));
        assert!(!state.has_sufficient_results(0.95));
    }

    #[test]
    fn test_next_action_when_sufficient() {
        let mut state = AgentLoopState::new(3);
        
        let reflection = Reflection {
            is_sufficient: true,
            confidence: 0.9,
            missing_info: vec![],
            suggestions: vec![],
        };
        
        let next = state.next_action(&reflection);
        assert_eq!(next, AgentAction::Answer);
    }

    #[test]
    fn test_next_action_when_not_sufficient_with_suggestions() {
        let mut state = AgentLoopState::new(3);
        
        let reflection = Reflection {
            is_sufficient: false,
            confidence: 0.5,
            missing_info: vec!["Need more info".to_string()],
            suggestions: vec!["Try broader terms".to_string()],
        };
        
        state.plan = Some(RetrievalPlan {
            query_rewrite: "test".to_string(),
            sub_queries: vec![],
            sources: vec![],
            max_iterations: 3,
        });
        
        let next = state.next_action(&reflection);
        assert_eq!(next, AgentAction::Plan);
    }

    #[test]
    fn test_next_action_max_iterations_reached() {
        let mut state = AgentLoopState::new(3);
        state.iteration = 3;
        
        let reflection = Reflection {
            is_sufficient: false,
            confidence: 0.5,
            missing_info: vec![],
            suggestions: vec![],
        };
        
        let next = state.next_action(&reflection);
        assert_eq!(next, AgentAction::Answer);
    }

    #[test]
    fn test_default_loop_controller() {
        let controller = DefaultLoopController;
        
        let state = AgentLoopState::new(3);
        
        let insufficient_reflection = Reflection {
            is_sufficient: false,
            confidence: 0.5,
            missing_info: vec![],
            suggestions: vec![],
        };
        
        assert!(controller.should_continue(&state, &insufficient_reflection));
        
        let sufficient_reflection = Reflection {
            is_sufficient: true,
            confidence: 0.9,
            missing_info: vec![],
            suggestions: vec![],
        };
        
        assert!(!controller.should_continue(&state, &sufficient_reflection));
    }
}
