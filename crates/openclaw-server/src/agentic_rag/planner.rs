//! 查询规划器

use std::sync::Arc;

use async_trait::async_trait;
use openclaw_core::{Message, Result};

use super::config::{PlannerConfig, SourceConfig, SourceType};

#[derive(Debug, Clone)]
pub struct RetrievalPlan {
    pub query_rewrite: String,
    pub sub_queries: Vec<SubQuery>,
    pub sources: Vec<SourceType>,
    pub max_iterations: usize,
}

#[derive(Debug, Clone)]
pub struct SubQuery {
    pub query: String,
    pub source: SourceType,
    pub description: String,
}

#[async_trait]
pub trait QueryPlanner: Send + Sync {
    async fn plan(
        &self,
        query: &str,
        context: &[Message],
        config: &PlannerConfig,
        available_sources: &[SourceConfig],
    ) -> Result<RetrievalPlan>;
}

pub struct DefaultQueryPlanner {
    llm: Arc<dyn openclaw_ai::AIProvider>,
}

impl DefaultQueryPlanner {
    pub fn new(llm: Arc<dyn openclaw_ai::AIProvider>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl QueryPlanner for DefaultQueryPlanner {
    async fn plan(
        &self,
        query: &str,
        context: &[Message],
        config: &PlannerConfig,
        available_sources: &[SourceConfig],
    ) -> Result<RetrievalPlan> {
        let enabled_sources: Vec<SourceType> = available_sources
            .iter()
            .filter(|s| s.enabled)
            .map(|s| s.source_type.clone())
            .collect();

        if config.enable_query_rewrite {
            let rewritten = self.rewrite_query(query, context).await?;
            
            if config.enable_hypothesis {
                let sub_queries = self.generate_sub_queries(&rewritten, &enabled_sources, config).await?;
                Ok(RetrievalPlan {
                    query_rewrite: rewritten,
                    sub_queries,
                    sources: enabled_sources,
                    max_iterations: config.max_sub_queries,
                })
            } else {
                let rewritten_clone = rewritten.clone();
                Ok(RetrievalPlan {
                    query_rewrite: rewritten,
                    sub_queries: vec![SubQuery {
                        query: rewritten_clone,
                        source: enabled_sources.first().cloned().unwrap_or(SourceType::Memory),
                        description: "Primary query".to_string(),
                    }],
                    sources: enabled_sources,
                    max_iterations: config.max_sub_queries,
                })
            }
        } else {
            Ok(RetrievalPlan {
                query_rewrite: query.to_string(),
                sub_queries: vec![SubQuery {
                    query: query.to_string(),
                    source: enabled_sources.first().cloned().unwrap_or(SourceType::Memory),
                    description: "Direct query".to_string(),
                }],
                sources: enabled_sources,
                max_iterations: config.max_sub_queries,
            })
        }
    }
}

impl DefaultQueryPlanner {
    async fn rewrite_query(&self, query: &str, context: &[Message]) -> Result<String> {
        let context_str = if context.is_empty() {
            "No previous context".to_string()
        } else {
            context
                .iter()
                .filter_map(|m| m.text_content())
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt = Message::system(format!(
            r#"Rewrite the following query to be more effective for knowledge retrieval.

Previous conversation:
{}

User query: {}

Rewritten query (just return the rewritten query, nothing else):"#,
            context_str, query
        ));

        let request = openclaw_ai::ChatRequest::new("default", vec![prompt]);
        let response = self.llm.chat(request).await?;

        let content = response.message.text_content().unwrap_or("");
        Ok(content.trim().to_string())
    }

    async fn generate_sub_queries(
        &self,
        query: &str,
        sources: &[SourceType],
        config: &PlannerConfig,
    ) -> Result<Vec<SubQuery>> {
        let sources_str = sources
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = Message::system(format!(
            r#"Given the main query, generate up to {} sub-queries that would help answer it.
Each sub-query should focus on a specific aspect or can be answered from a different knowledge source.

Main query: {}

Available sources: {}

Output format (JSON array):
[
  {{"query": "sub-query text", "source": "source type", "description": "what this query aims to find"}}
]

Generate sub-queries:"#,
            config.max_sub_queries, query, sources_str
        ));

        let request = openclaw_ai::ChatRequest::new("default", vec![prompt]);
        let response = self.llm.chat(request).await?;

        let content = response.message.text_content().unwrap_or("");
        self.parse_sub_queries(content, sources)
    }

    fn parse_sub_queries(
        &self,
        response: &str,
        sources: &[SourceType],
    ) -> Result<Vec<SubQuery>> {
        let json_str = response.trim().trim_start_matches("```json").trim_end_matches("```").trim();
        
        if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
            Ok(arr
                .into_iter()
                .filter_map(|v| {
                    Some(SubQuery {
                        query: v.get("query")?.as_str()?.to_string(),
                        source: match v.get("source")?.as_str() {
                            Some("memory") => SourceType::Memory,
                            Some("vector_db") => SourceType::VectorDB,
                            Some("web") => SourceType::Web,
                            Some("file") => SourceType::File,
                            Some("api") => SourceType::API,
                            _ => sources.first().cloned().unwrap_or(SourceType::Memory),
                        },
                        description: v
                            .get("description")
                            .and_then(|d| d.as_str())
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect())
        } else {
            Ok(vec![SubQuery {
                query: response.trim().to_string(),
                source: sources.first().cloned().unwrap_or(SourceType::Memory),
                description: "Parsed query".to_string(),
            }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_rag::config::SourceType;

    #[test]
    fn test_retrieval_plan_default() {
        let plan = RetrievalPlan {
            query_rewrite: "test query".to_string(),
            sub_queries: vec![],
            sources: vec![SourceType::Memory],
            max_iterations: 3,
        };
        
        assert_eq!(plan.query_rewrite, "test query");
        assert_eq!(plan.max_iterations, 3);
    }

    #[test]
    fn test_source_type_default() {
        assert_eq!(SourceType::default(), SourceType::Memory);
    }

    #[test]
    fn test_planner_config_default() {
        let config = PlannerConfig::default();
        assert_eq!(config.max_sub_queries, 5);
        assert!(config.enable_query_rewrite);
        assert!(config.enable_hypothesis);
    }
}
