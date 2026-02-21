//! Agentic RAG 核心引擎

use std::sync::Arc;

use async_trait::async_trait;
use openclaw_core::{Message, Result};

use super::config::{AgenticRAGConfig, SourceType};
use super::executor::{MultiSourceRetrievalExecutor, RetrievalResult};
use super::loop_control::{AgentLoopState, DefaultLoopController, LoopController};
use super::planner::QueryPlanner;
use super::reflector::{Reflection, ResultReflector};

pub struct AgenticRAGEngine {
    config: AgenticRAGConfig,
    llm: Arc<dyn openclaw_ai::AIProvider>,
    planner: Arc<dyn QueryPlanner>,
    executor: Arc<MultiSourceRetrievalExecutor>,
    reflector: Arc<dyn ResultReflector>,
    loop_controller: Arc<dyn LoopController>,
}

impl AgenticRAGEngine {
    pub async fn new(
        config: AgenticRAGConfig,
        llm: Arc<dyn openclaw_ai::AIProvider>,
        memory_manager: Option<Arc<openclaw_memory::MemoryManager>>,
        vector_store: Option<Arc<dyn openclaw_vector::VectorStore>>,
        embedding_provider: Option<Arc<dyn openclaw_memory::embedding::EmbeddingProvider>>,
    ) -> Result<Self> {
        let planner: Arc<dyn QueryPlanner> = Arc::new(super::planner::DefaultQueryPlanner::new(llm.clone()));
        
        let reflector: Arc<dyn ResultReflector> = Arc::new(super::reflector::DefaultResultReflector::new(llm.clone()));
        
        let loop_controller: Arc<dyn LoopController> = Arc::new(DefaultLoopController);

        let mut executor = MultiSourceRetrievalExecutor::new();

        if let Some(memory) = memory_manager {
            let memory_executor = super::executor::MemoryRetrievalExecutor::new(memory);
            executor = executor.add_executor(Box::new(memory_executor));
        }

        if let Some(vs) = vector_store {
            if let Some(ep) = embedding_provider {
                let vector_executor = super::executor::VectorDBRetrievalExecutor::new(vs, ep);
                executor = executor.add_executor(Box::new(vector_executor));
            }
        }

        Ok(Self {
            config,
            llm,
            planner,
            executor: Arc::new(executor),
            reflector,
            loop_controller,
        })
    }

    pub async fn process(&self, request: &RAGRequest) -> Result<RAGResponse> {
        if !self.config.enabled {
            return Err(openclaw_core::OpenClawError::Config(
                "Agentic RAG is not enabled".to_string(),
            ));
        }

        let plan = self
            .planner
            .plan(
                &request.query,
                &request.history,
                &self.config.planner,
                &self.config.sources,
            )
            .await?;

        let all_results = self
            .executor
            .execute_all(
                &plan.query_rewrite,
                &plan.sources,
                &self.config.executor,
                self.config.executor.enable_parallel,
            )
            .await?;

        let reflection = self
            .reflector
            .reflect(&request.query, &all_results, &self.config.reflector)
            .await?;

        let mut state = AgentLoopState::new(self.config.reflector.max_iterations);
        state.plan = Some(plan);
        state.add_results(all_results);

        if !reflection.is_sufficient && self.loop_controller.should_continue(&state, &reflection) {
            let refined_results = self.perform_refinement(&request.query, &reflection).await?;
            state.add_results(refined_results);
        }

        let answer = self
            .reflector
            .generate_answer(&request.query, &state.retrieved_context, &request.history)
            .await?;

        let confidence = if state.retrieved_context.is_empty() {
            0.0
        } else {
            state.retrieved_context.iter().map(|r| r.relevance_score).sum::<f32>()
                / state.retrieved_context.len() as f32
        };

        Ok(RAGResponse {
            answer,
            sources: state.retrieved_context,
            iterations: state.iteration,
            confidence,
            trace: state
                .thought_history
                .into_iter()
                .map(|t| ActionTrace {
                    action: t.action.to_string(),
                    reasoning: t.reasoning,
                    timestamp: t.timestamp
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0),
                })
                .collect(),
        })
    }

    async fn perform_refinement(
        &self,
        query: &str,
        reflection: &Reflection,
    ) -> Result<Vec<RetrievalResult>> {
        let mut refined_results = Vec::new();

        for suggestion in &reflection.suggestions {
            let refined_query = format!("{} {}", query, suggestion);
            
            let sources: Vec<SourceType> = self.config.sources.iter()
                .filter(|s| s.enabled)
                .map(|s| s.source_type.clone())
                .collect();
            
            let results = self
                .executor
                .execute_all(
                    &refined_query,
                    &sources,
                    &self.config.executor,
                    self.config.executor.enable_parallel,
                )
                .await?;
            
            refined_results.extend(results);
        }

        Ok(refined_results)
    }

    pub async fn retrieve(&self, query: &str, sources: &[SourceType]) -> Result<Vec<RetrievalResult>> {
        self.executor
            .execute_all(query, sources, &self.config.executor, self.config.executor.enable_parallel)
            .await
    }

    pub async fn verify(
        &self,
        query: &str,
        results: &[RetrievalResult],
    ) -> Result<Reflection> {
        self.reflector
            .reflect(query, results, &self.config.reflector)
            .await
    }

    pub fn config(&self) -> &AgenticRAGConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: AgenticRAGConfig) {
        self.config = config;
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RAGRequest {
    pub query: String,
    pub history: Vec<Message>,
    pub options: RAGOptions,
}

impl RAGRequest {
    pub fn new(query: String) -> Self {
        Self {
            query,
            history: Vec::new(),
            options: RAGOptions::default(),
        }
    }

    pub fn with_history(mut self, history: Vec<Message>) -> Self {
        self.history = history;
        self
    }

    pub fn with_options(mut self, options: RAGOptions) -> Self {
        self.options = options;
        self
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RAGOptions {
    pub max_iterations: Option<usize>,
    pub min_confidence: Option<f32>,
    pub sources: Option<Vec<SourceType>>,
}

impl Default for RAGOptions {
    fn default() -> Self {
        Self {
            max_iterations: None,
            min_confidence: None,
            sources: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RAGResponse {
    pub answer: String,
    pub sources: Vec<RetrievalResult>,
    pub iterations: usize,
    pub confidence: f32,
    pub trace: Vec<ActionTrace>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionTrace {
    pub action: String,
    pub reasoning: String,
    pub timestamp: u64,
}

#[async_trait]
pub trait AgenticRAGService: Send + Sync {
    async fn process(&self, request: &RAGRequest) -> Result<RAGResponse>;
    async fn retrieve(&self, query: &str, sources: &[SourceType]) -> Result<Vec<RetrievalResult>>;
    async fn verify(&self, query: &str, results: &[RetrievalResult]) -> Result<Reflection>;
}

#[async_trait]
impl AgenticRAGService for AgenticRAGEngine {
    async fn process(&self, request: &RAGRequest) -> Result<RAGResponse> {
        self.process(request).await
    }

    async fn retrieve(&self, query: &str, sources: &[SourceType]) -> Result<Vec<RetrievalResult>> {
        self.retrieve(query, sources).await
    }

    async fn verify(&self, query: &str, results: &[RetrievalResult]) -> Result<Reflection> {
        self.verify(query, results).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rag_request_new() {
        let request = RAGRequest::new("test query".to_string());
        assert_eq!(request.query, "test query");
        assert!(request.history.is_empty());
    }

    #[test]
    fn test_rag_request_with_history() {
        let history = vec![Message::user("hello".to_string())];
        let request = RAGRequest::new("test".to_string()).with_history(history.clone());
        assert_eq!(request.history.len(), 1);
    }

    #[test]
    fn test_rag_options_default() {
        let options = RAGOptions::default();
        assert!(options.max_iterations.is_none());
        assert!(options.min_confidence.is_none());
    }

    #[test]
    fn test_rag_response_creation() {
        let response = RAGResponse {
            answer: "Test answer".to_string(),
            sources: vec![],
            iterations: 1,
            confidence: 0.8,
            trace: vec![],
        };

        assert_eq!(response.answer, "Test answer");
        assert_eq!(response.iterations, 1);
    }

    #[test]
    fn test_action_trace_creation() {
        let trace = ActionTrace {
            action: "Think".to_string(),
            reasoning: "Analyzing query".to_string(),
            timestamp: 1234567890,
        };

        assert_eq!(trace.action, "Think");
    }

    #[test]
    fn test_agentic_rag_config_default() {
        let config = AgenticRAGConfig::default();
        assert!(config.enabled);
        assert_eq!(config.planner.max_sub_queries, 5);
        assert_eq!(config.executor.timeout_ms, 30000);
        assert_eq!(config.reflector.min_confidence, 0.7);
        assert_eq!(config.reflector.max_iterations, 3);
    }
}
