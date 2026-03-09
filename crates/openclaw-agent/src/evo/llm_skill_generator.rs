use std::sync::Arc;

use async_trait::async_trait;
use openclaw_ai::{AIProvider, ChatRequest};
use openclaw_core::Result;

use super::pattern_analyzer::{TaskPattern, ToolCallPattern};

#[derive(Debug, Clone)]
pub struct GenerationConfig {
    pub model: Option<String>,
    pub temperature: f32,
    pub max_tokens: Option<usize>,
    pub language: super::ProgrammingLanguage,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            model: None,
            temperature: 0.7,
            max_tokens: Some(4096),
            language: super::ProgrammingLanguage::Rust,
        }
    }
}

pub struct LlmSkillGenerator {
    ai_provider: Arc<dyn AIProvider>,
    config: GenerationConfig,
}

impl LlmSkillGenerator {
    pub fn new(ai_provider: Arc<dyn AIProvider>) -> Self {
        Self::with_config(ai_provider, GenerationConfig::default())
    }

    pub fn with_config(ai_provider: Arc<dyn AIProvider>, config: GenerationConfig) -> Self {
        Self {
            ai_provider,
            config,
        }
    }

    pub async fn generate_skill_code(&self, pattern: &TaskPattern) -> Result<String> {
        let prompt = self.build_prompt(pattern);

        let model = self
            .config
            .model
            .clone()
            .unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string());

        let mut request = openclaw_ai::ChatRequest::new(&model, vec![openclaw_core::Message::user(&prompt)]);
        request.temperature = Some(self.config.temperature);
        request.max_tokens = self.config.max_tokens;

        let response = self.ai_provider.chat(request).await?;

        let content = response
            .message
            .text_content()
            .map(|s| s.to_string())
            .unwrap_or_default();

        Ok(self.extract_code_from_response(&content))
    }

    fn build_prompt(&self, pattern: &TaskPattern) -> String {
        let tool_sequence_desc = pattern
            .tool_sequence
            .iter()
            .map(|t| {
                format!(
                    "- {}: input={:?}, output={:?}",
                    t.tool_name, t.param_schema, t.result_schema
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let steps_desc = pattern
            .steps
            .iter()
            .map(|s| {
                format!(
                    "{}. {}: {} -> {}",
                    s.step_number, s.tool_name, s.input_summary, s.output_summary
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let language_str = match self.config.language {
            super::ProgrammingLanguage::Rust => "Rust",
            super::ProgrammingLanguage::Python => "Python",
            super::ProgrammingLanguage::JavaScript => "JavaScript",
            super::ProgrammingLanguage::Wasm => "Rust (WASM target)",
        };

        let prompt = format!(
            "You are an expert {} developer. Generate a complete, production-ready skill function based on the following task pattern.\n\n\
## Task Pattern Information\n\
- **Category**: {}\n\
- **Reusability Score**: {:.2}\n\
- **Source Task ID**: {}\n\n\
## Tool Sequence (what tools were used)\n\
{}\n\n\
## Execution Steps\n\
{}\n\n\
## Requirements\n\
1. Generate ONLY the code, no explanations or comments outside the code\n\
2. The function should be async and handle errors gracefully\n\
3. Use proper error handling with Result<T, String> return type\n\
4. Include appropriate imports for the {} ecosystem\n\
5. The function signature should follow this pattern:\n\
   - Function name: describe the action (e.g., process_data, analyze_file)\n\
   - Parameters: based on the tool sequence inputs\n\
   - Return type: Result<T, String> where T is the appropriate output type\n\n\
## Output Format\n\
Provide only the executable code block, no markdown formatting markers.",
            language_str,
            pattern.task_category,
            pattern.reusability_score,
            pattern.source_task_id,
            tool_sequence_desc,
            steps_desc,
            language_str
        );

        prompt
    }

    fn extract_code_from_response(&self, response: &str) -> String {
        let trimmed = response.trim();

        if trimmed.starts_with("```") {
            if let Some(end_idx) = trimmed[3..].find("```") {
                trimmed[3..3 + end_idx].trim().to_string()
            } else {
                trimmed[3..].trim().to_string()
            }
        } else {
            trimmed.to_string()
        }
    }

    pub fn set_model(&mut self, model: String) {
        self.config.model = Some(model);
    }

    pub fn set_temperature(&mut self, temperature: f32) {
        self.config.temperature = temperature;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use openclaw_ai::providers::BaseProvider;
    use openclaw_ai::ProviderConfig;

    fn create_test_pattern() -> TaskPattern {
        TaskPattern {
            id: "test_001".to_string(),
            task_category: "File Processing".to_string(),
            tool_sequence: vec![ToolCallPattern {
                tool_name: "read_file".to_string(),
                param_schema: std::collections::HashMap::new(),
                result_schema: std::collections::HashMap::new(),
            }],
            param_patterns: vec![],
            success_indicators: vec!["file read successfully".to_string()],
            steps: vec![],
            reusability_score: 0.85,
            source_task_id: "task_123".to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_extract_code_from_response_with_fences() {
        let generator = LlmSkillGenerator::new(Arc::new(BaseProvider::new(
            ProviderConfig::new("test", "key"),
        )));

        let response = "```rust\npub async fn process_file(path: &str) -> Result<String, String> {\n    Ok(\"processed\".to_string())\n}\n```";

        let code = generator.extract_code_from_response(response);
        assert!(code.contains("pub async fn process_file"));
    }

    #[test]
    fn test_extract_code_from_response_without_fences() {
        let generator = LlmSkillGenerator::new(Arc::new(BaseProvider::new(
            ProviderConfig::new("test", "key"),
        )));

        let response = "pub async fn process_file(path: &str) -> Result<String, String> {\n    Ok(\"processed\".to_string())\n}";

        let code = generator.extract_code_from_response(response);
        assert!(code.contains("pub async fn process_file"));
    }
}
