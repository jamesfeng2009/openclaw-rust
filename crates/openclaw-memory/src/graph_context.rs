
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingContext {
    pub current_agent: Option<String>,
    pub current_task: Option<String>,
    pub recent_outputs: Vec<String>,
}

impl Default for WorkingContext {
    fn default() -> Self {
        Self {
            current_agent: None,
            current_task: None,
            recent_outputs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub user_preferences: Vec<String>,
    pub history_summary: String,
    pub active_agents: Vec<String>,
}

impl SessionContext {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            user_preferences: Vec::new(),
            history_summary: String::new(),
            active_agents: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: String,
    pub content: String,
    pub source_path: String,
    pub memory_type: KnowledgeType,
    pub similarity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeType {
    AgentDefinition,
    Skill,
    UserProfile,
    ProjectKnowledge,
    SessionMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalStep {
    pub step_type: RetrievalStepType,
    pub path: String,
    pub reasoning: String,
    pub results_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalStepType {
    DirectoryLocate,
    SemanticSearch,
    KeywordMatch,
    PathTraverse,
    FileRead,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextBundle {
    pub working: WorkingContext,
    pub session: Option<SessionContext>,
    pub knowledge: Vec<KnowledgeItem>,
    pub retrieval_trace: Vec<RetrievalStep>,
}

impl ContextBundle {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_knowledge(mut self, items: Vec<KnowledgeItem>) -> Self {
        self.knowledge = items;
        self
    }
    
    pub fn with_working(mut self, working: WorkingContext) -> Self {
        self.working = working;
        self
    }
    
    pub fn with_session(mut self, session: SessionContext) -> Self {
        self.session = Some(session);
        self
    }
    
    pub fn add_retrieval_step(&mut self, step: RetrievalStep) {
        self.retrieval_trace.push(step);
    }
    
    pub fn to_system_prompt(&self) -> String {
        let mut prompt = String::new();
        
        if !self.knowledge.is_empty() {
            prompt.push_str("## Relevant Knowledge\n\n");
            for item in &self.knowledge {
                prompt.push_str(&format!("- [{}] {}\n", item.source_path, item.content));
            }
            prompt.push('\n');
        }
        
        if let Some(ref session) = self.session {
            if !session.history_summary.is_empty() {
                prompt.push_str("## Session Context\n\n");
                prompt.push_str(&session.history_summary);
                prompt.push_str("\n\n");
            }
        }
        
        prompt
    }
}

#[async_trait]
pub trait ContextProvider: Send + Sync {
    async fn prepare_context(&self, query: &str) -> Result<ContextBundle, Box<dyn std::error::Error + Send + Sync>>;
}

pub struct SimpleContextProvider;

impl SimpleContextProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SimpleContextProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContextProvider for SimpleContextProvider {
    async fn prepare_context(&self, query: &str) -> Result<ContextBundle, Box<dyn std::error::Error + Send + Sync>> {
        let mut bundle = ContextBundle::new();
        
        bundle.working = WorkingContext {
            current_agent: None,
            current_task: Some(query.to_string()),
            recent_outputs: Vec::new(),
        };
        
        bundle.retrieval_trace.push(RetrievalStep {
            step_type: RetrievalStepType::KeywordMatch,
            path: "working".to_string(),
            reasoning: "Initialized working context with query".to_string(),
            results_count: 1,
        });
        
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_working_context_default() {
        let ctx = WorkingContext::default();
        assert!(ctx.current_agent.is_none());
        assert!(ctx.current_task.is_none());
        assert!(ctx.recent_outputs.is_empty());
    }
    
    #[test]
    fn test_session_context_new() {
        let ctx = SessionContext::new("session-123");
        assert_eq!(ctx.session_id, "session-123");
        assert!(ctx.user_preferences.is_empty());
    }
    
    #[test]
    fn test_context_bundle_new() {
        let bundle = ContextBundle::new();
        assert!(bundle.knowledge.is_empty());
        assert!(bundle.retrieval_trace.is_empty());
    }
    
    #[test]
    fn test_context_bundle_with_knowledge() {
        let items = vec![
            KnowledgeItem {
                id: "1".to_string(),
                content: "Test knowledge".to_string(),
                source_path: "test.md".to_string(),
                memory_type: KnowledgeType::ProjectKnowledge,
                similarity: 0.9,
            }
        ];
        let bundle = ContextBundle::new().with_knowledge(items.clone());
        assert_eq!(bundle.knowledge.len(), 1);
        assert_eq!(bundle.knowledge[0].content, "Test knowledge");
    }
    
    #[test]
    fn test_context_bundle_to_system_prompt_with_knowledge() {
        let items = vec![
            KnowledgeItem {
                id: "1".to_string(),
                content: "Important info".to_string(),
                source_path: "docs/guide.md".to_string(),
                memory_type: KnowledgeType::ProjectKnowledge,
                similarity: 0.8,
            }
        ];
        let bundle = ContextBundle::new().with_knowledge(items);
        let prompt = bundle.to_system_prompt();
        
        assert!(prompt.contains("Relevant Knowledge"));
        assert!(prompt.contains("docs/guide.md"));
        assert!(prompt.contains("Important info"));
    }
    
    #[test]
    fn test_context_bundle_to_system_prompt_with_session() {
        let mut session = SessionContext::new("test-session");
        session.history_summary = "Previous conversation about Rust programming".to_string();
        let bundle = ContextBundle::new()
            .with_session(session);
        let prompt = bundle.to_system_prompt();
        
        assert!(prompt.contains("Session Context"));
        assert!(prompt.contains("Previous conversation"));
    }
    
    #[test]
    fn test_context_bundle_add_retrieval_step() {
        let mut bundle = ContextBundle::new();
        bundle.add_retrieval_step(RetrievalStep {
            step_type: RetrievalStepType::SemanticSearch,
            path: "knowledge".to_string(),
            reasoning: "Searching for relevant documents".to_string(),
            results_count: 5,
        });
        
        assert_eq!(bundle.retrieval_trace.len(), 1);
        assert_eq!(bundle.retrieval_trace[0].results_count, 5);
    }
    
    #[tokio::test]
    async fn test_simple_context_provider_prepare_context() {
        let provider = SimpleContextProvider::new();
        let result = provider.prepare_context("test query").await;
        
        assert!(result.is_ok());
        let bundle = result.unwrap();
        assert!(bundle.working.current_task.is_some());
    }
    
    #[test]
    fn test_knowledge_type_variants() {
        let types = vec![
            KnowledgeType::AgentDefinition,
            KnowledgeType::Skill,
            KnowledgeType::UserProfile,
            KnowledgeType::ProjectKnowledge,
            KnowledgeType::SessionMemory,
        ];
        
        for t in types {
            let _ = format!("{:?}", t);
        }
    }
    
    #[test]
    fn test_retrieval_step_type_variants() {
        let types = vec![
            RetrievalStepType::DirectoryLocate,
            RetrievalStepType::SemanticSearch,
            RetrievalStepType::KeywordMatch,
            RetrievalStepType::PathTraverse,
            RetrievalStepType::FileRead,
        ];
        
        for t in types {
            let _ = format!("{:?}", t);
        }
    }
}
