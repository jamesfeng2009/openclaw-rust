use crate::types::{AgentType, Capability};
use crate::task::TaskInput;
use openclaw_core::Content;

pub struct TaskAnalysis {
    pub complexity: TaskComplexity,
    pub required_capabilities: Vec<Capability>,
    pub suggested_agent: AgentType,
    pub needs_decomposition: bool,
    pub suggested_tools: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskComplexity {
    Simple,
    Medium,
    Complex,
}

impl TaskComplexity {
    pub fn from_input(input: &TaskInput) -> Self {
        match input {
            TaskInput::Text { content } => {
                let len = content.len();
                let word_count = content.split_whitespace().count();
                
                if len < 100 && word_count < 20 {
                    TaskComplexity::Simple
                } else if len < 500 && word_count < 100 {
                    TaskComplexity::Medium
                } else {
                    TaskComplexity::Complex
                }
            }
            TaskInput::Code { code, .. } => {
                let lines = code.lines().count();
                if lines < 50 {
                    TaskComplexity::Simple
                } else if lines < 200 {
                    TaskComplexity::Medium
                } else {
                    TaskComplexity::Complex
                }
            }
            TaskInput::SearchQuery { .. } => TaskComplexity::Simple,
            TaskInput::ToolCall { .. } => TaskComplexity::Simple,
            TaskInput::Message { .. } => TaskComplexity::Medium,
            TaskInput::Data { data } => {
                if let Some(arr) = data.as_array() {
                    if arr.len() > 100 {
                        return TaskComplexity::Complex;
                    }
                }
                TaskComplexity::Medium
            }
            TaskInput::File { content, .. } => {
                let lines = content.lines().count();
                if lines < 100 {
                    TaskComplexity::Medium
                } else {
                    TaskComplexity::Complex
                }
            }
        }
    }
}

pub struct TaskAnalyzer;

impl TaskAnalyzer {
    pub fn analyze(input: &TaskInput) -> TaskAnalysis {
        let complexity = TaskComplexity::from_input(input);
        let (required_capabilities, suggested_agent, needs_decomposition, suggested_tools) = 
            Self::analyze_content(input, &complexity);

        TaskAnalysis {
            complexity,
            required_capabilities,
            suggested_agent,
            needs_decomposition,
            suggested_tools,
        }
    }

    fn analyze_content(
        input: &TaskInput,
        complexity: &TaskComplexity,
    ) -> (Vec<Capability>, AgentType, bool, Vec<String>) {
        match input {
            TaskInput::Text { content } => {
                let content_lower = content.to_lowercase();
                
                if content_lower.contains("write") || content_lower.contains("create") || content_lower.contains("generate") {
                    if content_lower.contains("code") || content_lower.contains("function") || content_lower.contains("class") {
                        (
                            vec![Capability::CodeGeneration],
                            AgentType::Coder,
                            *complexity == TaskComplexity::Complex,
                            vec!["code_generator".to_string()],
                        )
                    } else if content_lower.contains("article") || content_lower.contains("blog") || content_lower.contains("post") {
                        (
                            vec![Capability::ContentGeneration],
                            AgentType::Writer,
                            *complexity == TaskComplexity::Complex,
                            vec!["content_writer".to_string()],
                        )
                    } else {
                        (
                            vec![Capability::General],
                            AgentType::Conversationalist,
                            false,
                            vec![],
                        )
                    }
                } else if content_lower.contains("search") || content_lower.contains("find") || content_lower.contains("research") {
                    (
                        vec![Capability::WebSearch, Capability::InformationAnalysis],
                        AgentType::Researcher,
                        false,
                        vec!["web_search".to_string()],
                    )
                } else if content_lower.contains("analyze") || content_lower.contains("data") || content_lower.contains("statistics") {
                    (
                        vec![Capability::DataAnalysis, Capability::Visualization],
                        AgentType::DataAnalyst,
                        *complexity == TaskComplexity::Complex,
                        vec!["data_analyzer".to_string()],
                    )
                } else if content_lower.contains("debug") || content_lower.contains("fix") || content_lower.contains("error") {
                    (
                        vec![Capability::Debugging, Capability::CodeReview],
                        AgentType::Coder,
                        false,
                        vec!["debugger".to_string()],
                    )
                } else {
                    (
                        vec![Capability::Conversation, Capability::QAndA],
                        AgentType::Conversationalist,
                        false,
                        vec![],
                    )
                }
            }
            TaskInput::Code { .. } => (
                vec![Capability::CodeGeneration, Capability::CodeReview],
                AgentType::Coder,
                *complexity == TaskComplexity::Complex,
                vec!["code_generator".to_string(), "code_executor".to_string()],
            ),
            TaskInput::SearchQuery { query } => {
                let query_lower = query.to_lowercase();
                if query_lower.contains("how") || query_lower.contains("what") || query_lower.contains("why") {
                    (
                        vec![Capability::WebSearch, Capability::InformationAnalysis],
                        AgentType::Researcher,
                        false,
                        vec!["web_search".to_string()],
                    )
                } else {
                    (
                        vec![Capability::WebSearch],
                        AgentType::Researcher,
                        false,
                        vec!["web_search".to_string()],
                    )
                }
            }
            TaskInput::ToolCall { name, .. } => (
                vec![Capability::ToolExecution],
                AgentType::ToolUser,
                false,
                vec![name.clone()],
            ),
            TaskInput::Data { .. } => (
                vec![Capability::DataAnalysis],
                AgentType::DataAnalyst,
                *complexity == TaskComplexity::Complex,
                vec!["data_analyzer".to_string()],
            ),
            TaskInput::File { content, .. } => {
                let content_lower = content.to_lowercase();
                if content_lower.contains("function") || content_lower.contains("fn ") || content_lower.contains("class ") {
                    (
                        vec![Capability::CodeGeneration, Capability::CodeReview],
                        AgentType::Coder,
                        *complexity == TaskComplexity::Complex,
                        vec!["code_analyzer".to_string()],
                    )
                } else {
                    (
                        vec![Capability::InformationAnalysis, Capability::Summarization],
                        AgentType::Researcher,
                        *complexity == TaskComplexity::Complex,
                        vec!["file_reader".to_string()],
                    )
                }
            }
            TaskInput::Message { message } => {
                let content = message.content.iter()
                    .map(|c| match c {
                        openclaw_core::Content::Text { text } => text.clone(),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                
                let inner_complexity = TaskComplexity::from_input(&TaskInput::Text { content: content.clone() });
                Self::analyze_content(&TaskInput::Text { content }, &inner_complexity)
            }
        }
    }
}

pub struct ToolSelector;

impl ToolSelector {
    pub fn select_for_agent(agent_type: &AgentType) -> Vec<String> {
        match agent_type {
            AgentType::Orchestrator => vec![
                "task_router".to_string(),
                "agent_coordinator".to_string(),
            ],
            AgentType::Researcher => vec![
                "web_search".to_string(),
                "file_reader".to_string(),
                "summarizer".to_string(),
            ],
            AgentType::Coder => vec![
                "code_generator".to_string(),
                "code_executor".to_string(),
                "file_writer".to_string(),
            ],
            AgentType::Writer => vec![
                "content_writer".to_string(),
                "editor".to_string(),
            ],
            AgentType::DataAnalyst => vec![
                "data_analyzer".to_string(),
                "visualizer".to_string(),
            ],
            AgentType::Conversationalist => vec![],
            AgentType::ToolUser => vec![],
            AgentType::Custom(_) => vec![],
        }
    }

    pub fn select_for_task(analysis: &TaskAnalysis) -> Vec<String> {
        let mut tools = analysis.suggested_tools.clone();
        
        if analysis.needs_decomposition {
            tools.push("task_decomposer".to_string());
        }
        
        tools
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_complexity_simple() {
        let input = TaskInput::Text { content: "Hello, how are you?".to_string() };
        assert_eq!(TaskComplexity::from_input(&input), TaskComplexity::Simple);
    }

    #[test]
    fn test_task_complexity_medium() {
        let input = TaskInput::Text { content: "This is a medium length text that contains some information about various topics and needs to be analyzed properly.".to_string() };
        assert_eq!(TaskComplexity::from_input(&input), TaskComplexity::Medium);
    }

    #[test]
    fn test_task_analysis_code() {
        let input = TaskInput::Code { 
            language: "rust".to_string(), 
            code: "fn main() { println!(\"Hello\"); }".to_string() 
        };
        let analysis = TaskAnalyzer::analyze(&input);
        assert_eq!(analysis.suggested_agent, AgentType::Coder);
    }

    #[test]
    fn test_task_analysis_search() {
        let input = TaskInput::SearchQuery { query: "What is Rust?".to_string() };
        let analysis = TaskAnalyzer::analyze(&input);
        assert_eq!(analysis.suggested_agent, AgentType::Researcher);
    }
}
