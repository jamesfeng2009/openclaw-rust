//! 结果反思器

use std::sync::Arc;

use async_trait::async_trait;

use openclaw_core::{Message, Result};

use super::config::ReflectorConfig;
use super::executor::RetrievalResult;

#[derive(Debug, Clone)]
pub struct Reflection {
    pub is_sufficient: bool,
    pub confidence: f32,
    pub missing_info: Vec<String>,
    pub suggestions: Vec<String>,
}

#[async_trait]
pub trait ResultReflector: Send + Sync {
    async fn reflect(
        &self,
        query: &str,
        results: &[RetrievalResult],
        config: &ReflectorConfig,
    ) -> Result<Reflection>;

    async fn generate_answer(
        &self,
        query: &str,
        results: &[RetrievalResult],
        context: &[Message],
    ) -> Result<String>;
}

pub struct DefaultResultReflector {
    llm: Arc<dyn openclaw_ai::AIProvider>,
}

impl DefaultResultReflector {
    pub fn new(llm: Arc<dyn openclaw_ai::AIProvider>) -> Self {
        Self { llm }
    }
}

#[async_trait]
impl ResultReflector for DefaultResultReflector {
    async fn reflect(
        &self,
        query: &str,
        results: &[RetrievalResult],
        config: &ReflectorConfig,
    ) -> Result<Reflection> {
        if results.is_empty() {
            return Ok(Reflection {
                is_sufficient: false,
                confidence: 0.0,
                missing_info: vec!["No results retrieved".to_string()],
                suggestions: vec!["Try different search terms".to_string()],
            });
        }

        if config.enable_verification {
            self.verify_with_llm(query, results, config).await
        } else {
            self.verify_simple(results, config)
        }
    }

    async fn generate_answer(
        &self,
        query: &str,
        results: &[RetrievalResult],
        context: &[Message],
    ) -> Result<String> {
        if results.is_empty() {
            return Ok("I couldn't find relevant information to answer your question.".to_string());
        }

        let context_str = results
            .iter()
            .map(|r| format!("[{:?}] {}", r.source, r.content))
            .collect::<Vec<_>>()
            .join("\n\n");

        let history_str = if context.is_empty() {
            String::new()
        } else {
            context
                .iter()
                .filter_map(|m| m.text_content())
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join("\n")
        };

        let prompt_text = if history_str.is_empty() {
            format!(
                r#"Based on the following retrieved information, answer the user's question.

Retrieved information:
{}

Question: {}

Provide a clear, accurate answer based on the retrieved information. If the information is insufficient, state that clearly."#,
                context_str, query
            )
        } else {
            format!(
                r#"Based on the following retrieved information and conversation history, answer the user's question.

Conversation history:
{}

Retrieved information:
{}

Question: {}

Provide a clear, accurate answer based on the retrieved information. If the information is insufficient, state that clearly."#,
                history_str, context_str, query
            )
        };

        let prompt = Message::system(prompt_text);
        let request = openclaw_ai::ChatRequest::new("default", vec![prompt]);
        let response = self.llm.chat(request).await?;
        let content = response.message.text_content().unwrap_or("");
        Ok(content.to_string())
    }
}

impl DefaultResultReflector {
    async fn verify_with_llm(
        &self,
        query: &str,
        results: &[RetrievalResult],
        config: &ReflectorConfig,
    ) -> Result<Reflection> {
        let context_str = results
            .iter()
            .take(5)
            .map(|r| format!("- {}\n  Source: {:?}\n  Relevance: {:.2}", r.content, r.source, r.relevance_score))
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt_text = format!(
            r#"Analyze whether the retrieved information is sufficient to answer the user's question.

Question: {}

Retrieved information:
{}

Analyze and respond in JSON format:
{{
  "is_sufficient": true/false,
  "confidence": 0.0-1.0,
  "missing_info": ["what information is still needed"],
  "suggestions": ["how to improve the search"]
}}

Response:"#,
            query, context_str
        );

        let prompt = Message::system(prompt_text);
        let request = openclaw_ai::ChatRequest::new("default", vec![prompt]);
        let response = self.llm.chat(request).await?;
        let content = response.message.text_content().unwrap_or("");
        self.parse_reflection(content, config)
    }

    fn verify_simple(
        &self,
        results: &[RetrievalResult],
        config: &ReflectorConfig,
    ) -> Result<Reflection> {
        let total: f32 = results.iter().map(|r| r.relevance_score).sum();
        let avg_score = total / results.len() as f32;
        let count = results.len();

        let is_sufficient = avg_score >= config.min_confidence && count >= 2;

        Ok(Reflection {
            is_sufficient,
            confidence: avg_score,
            missing_info: if is_sufficient {
                vec![]
            } else {
                vec!["Results below confidence threshold".to_string()]
            },
            suggestions: if is_sufficient {
                vec![]
            } else {
                vec!["Try broader search terms".to_string()]
            },
        })
    }

    fn parse_reflection(&self, response: &str, config: &ReflectorConfig) -> Result<Reflection> {
        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_end_matches("```")
            .trim();

        if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
            let is_sufficient = value
                .get("is_sufficient")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let confidence = value
                .get("confidence")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(0.0);

            let missing_info: Vec<String> = value
                .get("missing_info")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let suggestions: Vec<String> = value
                .get("suggestions")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            Ok(Reflection {
                is_sufficient: is_sufficient && confidence >= config.min_confidence,
                confidence,
                missing_info,
                suggestions,
            })
        } else {
            Ok(Reflection {
                is_sufficient: false,
                confidence: 0.0,
                missing_info: vec!["Failed to parse reflection".to_string()],
                suggestions: vec![],
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agentic_rag::config::SourceType;
    use std::collections::HashMap;

    #[test]
    fn test_reflection_creation() {
        let reflection = Reflection {
            is_sufficient: true,
            confidence: 0.85,
            missing_info: vec![],
            suggestions: vec![],
        };

        assert!(reflection.is_sufficient);
        assert_eq!(reflection.confidence, 0.85);
    }

    #[test]
    fn test_reflection_not_sufficient() {
        let reflection = Reflection {
            is_sufficient: false,
            confidence: 0.3,
            missing_info: vec!["Need more context".to_string()],
            suggestions: vec!["Try broader terms".to_string()],
        };

        assert!(!reflection.is_sufficient);
        assert_eq!(reflection.missing_info.len(), 1);
    }

    #[test]
    fn test_reflector_config_default() {
        let config = ReflectorConfig::default();
        assert_eq!(config.min_confidence, 0.7);
        assert_eq!(config.max_iterations, 3);
        assert!(config.enable_verification);
    }

    #[test]
    fn test_verify_simple_with_empty_results() {
        let results: Vec<RetrievalResult> = vec![];
        
        let config = ReflectorConfig::default();
        
        // Test with empty results - should return insufficient
        let total: f32 = results.iter().map(|r| r.relevance_score).sum();
        let avg_score = if results.is_empty() { 0.0 } else { total / results.len() as f32 };
        assert!(avg_score < config.min_confidence);
    }
    
    #[test]
    fn test_reflection_with_single_result() {
        let results = vec![
            RetrievalResult {
                id: "1".to_string(),
                content: "Content 1".to_string(),
                source: SourceType::Memory,
                relevance_score: 0.9,
                metadata: HashMap::new(),
            },
        ];

        let config = ReflectorConfig::default();
        
        // Single result should not be sufficient (need at least 2)
        let total: f32 = results.iter().map(|r| r.relevance_score).sum();
        let avg_score = total / results.len() as f32;
        let count = results.len();
        let is_sufficient = avg_score >= config.min_confidence && count >= 2;
        
        assert!(!is_sufficient);
        assert!(avg_score >= config.min_confidence);
    }
}
