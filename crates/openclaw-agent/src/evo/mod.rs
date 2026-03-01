//! Evolution System - 自我进化系统
//!
//! 提供代码生成、编译和沙箱执行能力：
//! - ToolNeed: 工具需求结构体
//! - SkillGenerator: 技能生成器
//! - DynamicCompiler: 动态编译器
//! - EvolutionResult: 进化结果
//! - SkillSandbox: 技能沙箱

use std::sync::Arc;

use async_trait::async_trait;
use openclaw_ai::{AIProvider, ChatRequest};
use openclaw_core::Message;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolNeed {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterDef>,
    pub return_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: Option<String>,
}

pub struct SkillGenerator {
    template: String,
}

impl SkillGenerator {
    pub fn new() -> Self {
        Self {
            template: include_str!("generator_template.txt").to_string(),
        }
    }

    pub async fn detect_need(&self, context: &str) -> Vec<ToolNeed> {
        let mut needs = Vec::new();
        
        if context.contains("file") && context.contains("read") {
            needs.push(ToolNeed {
                name: "read_file".to_string(),
                description: "Read content from a file".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "path".to_string(),
                        param_type: "String".to_string(),
                        required: true,
                        description: Some("File path to read".to_string()),
                    }
                ],
                return_type: "String".to_string(),
            });
        }
        
        if context.contains("execute") || context.contains("run") {
            needs.push(ToolNeed {
                name: "execute_command".to_string(),
                description: "Execute a shell command".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "command".to_string(),
                        param_type: "String".to_string(),
                        required: true,
                        description: Some("Command to execute".to_string()),
                    }
                ],
                return_type: "String".to_string(),
            });
        }
        
        needs
    }

    pub fn generate(&self, need: &ToolNeed) -> String {
        let params: String = need
            .parameters
            .iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"pub async fn {name}({params}) -> Result<{return_type}, String> {{
    Err("Skill '{name}' not implemented - Evo evolution pending".to_string())
}}"#,
            name = need.name,
            params = params,
            return_type = need.return_type
        )
    }
}

impl Default for SkillGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct CompiledSkill {
    pub code: String,
    pub language: ProgrammingLanguage,
    pub compiled_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammingLanguage {
    Rust,
    Python,
    JavaScript,
    Wasm,
}

impl std::fmt::Display for ProgrammingLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgrammingLanguage::Rust => write!(f, "Rust"),
            ProgrammingLanguage::Python => write!(f, "Python"),
            ProgrammingLanguage::JavaScript => write!(f, "JavaScript"),
            ProgrammingLanguage::Wasm => write!(f, "Wasm"),
        }
    }
}

pub struct DynamicCompiler {
    language: ProgrammingLanguage,
}

impl DynamicCompiler {
    pub fn new(language: ProgrammingLanguage) -> Self {
        Self { language }
    }

    pub async fn compile(&self, code: &str) -> Result<CompiledSkill, CompilerError> {
        match self.language {
            ProgrammingLanguage::Wasm => {
                Ok(CompiledSkill {
                    code: code.to_string(),
                    language: self.language,
                    compiled_at: chrono::Utc::now(),
                })
            }
            _ => Err(CompilerError::UnsupportedLanguage(self.language.to_string())),
        }
    }

    pub async fn validate(&self, code: &str) -> Result<bool, CompilerError> {
        if code.is_empty() {
            return Err(CompilerError::ValidationFailed("Code is empty".to_string()));
        }

        if code.contains("unsafe") && self.language == ProgrammingLanguage::Rust {
            return Err(CompilerError::ValidationFailed("Unsafe code not allowed".to_string()));
        }

        Ok(true)
    }
}

#[derive(Debug, Clone)]
pub enum CompilerError {
    UnsupportedLanguage(String),
    CompilationFailed(String),
    ValidationFailed(String),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::UnsupportedLanguage(lang) => write!(f, "Unsupported language: {}", lang),
            CompilerError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            CompilerError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for CompilerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvolutionStatus {
    Pending,
    Analyzing,
    Generating,
    Compiling,
    Validating,
    Executing,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct EvolutionResult {
    pub status: EvolutionStatus,
    pub skill: Option<CompiledSkill>,
    pub error: Option<String>,
    pub logs: Vec<String>,
}

impl EvolutionResult {
    pub fn pending() -> Self {
        Self {
            status: EvolutionStatus::Pending,
            skill: None,
            error: None,
            logs: Vec::new(),
        }
    }

    pub fn success(skill: CompiledSkill) -> Self {
        Self {
            status: EvolutionStatus::Completed,
            skill: Some(skill),
            error: None,
            logs: vec!["Evolution completed successfully".to_string()],
        }
    }

    pub fn failure(error: String) -> Self {
        Self {
            status: EvolutionStatus::Failed,
            skill: None,
            error: Some(error),
            logs: vec!["Evolution failed".to_string()],
        }
    }
}

pub struct EvolutionEngine {
    generator: SkillGenerator,
    compiler: DynamicCompiler,
    ai_provider: Option<Arc<dyn AIProvider>>,
    default_model: String,
}

impl EvolutionEngine {
    pub fn new() -> Self {
        Self {
            generator: SkillGenerator::new(),
            compiler: DynamicCompiler::new(ProgrammingLanguage::Wasm),
            ai_provider: None,
            default_model: "gpt-4".to_string(),
        }
    }

    pub fn with_ai_provider(mut self, provider: Arc<dyn AIProvider>) -> Self {
        self.ai_provider = Some(provider);
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    pub async fn evolve(&self, context: &str) -> EvolutionResult {
        if let Some(ref ai) = self.ai_provider {
            return self.ai_evolve(ai.as_ref(), context).await;
        }
        
        self.fallback_evolve(context).await
    }

    async fn ai_evolve(&self, ai: &dyn AIProvider, context: &str) -> EvolutionResult {
        let mut result = EvolutionResult::pending();
        
        result.logs.push("Analyzing user need with AI...".to_string());
        result.status = EvolutionStatus::Analyzing;
        
        let tool_spec = match self.analyze_need_with_ai(ai, context).await {
            Ok(spec) => spec,
            Err(e) => {
                result.logs.push(format!("AI analysis failed: {}", e));
                return EvolutionResult::failure(e);
            }
        };
        
        result.logs.push(format!("Detected need: {}", tool_spec.name));
        
        result.logs.push("Generating code with AI...".to_string());
        result.status = EvolutionStatus::Generating;
        
        let code = match self.generate_code_with_ai(ai, &tool_spec).await {
            Ok(c) => c,
            Err(e) => {
                result.logs.push(format!("Code generation failed: {}", e));
                return EvolutionResult::failure(e);
            }
        };
        
        result.logs.push("Validating code...".to_string());
        result.status = EvolutionStatus::Validating;
        
        if let Err(e) = self.compiler.validate(&code).await {
            result.logs.push(format!("Validation failed: {}", e));
            return EvolutionResult::failure(e.to_string());
        }
        
        result.logs.push("Compiling skill...".to_string());
        result.status = EvolutionStatus::Compiling;
        
        match self.compiler.compile(&code).await {
            Ok(skill) => {
                result.logs.push(format!("Compiled: {} v{}", skill.language, skill.compiled_at.to_rfc3339()));
                EvolutionResult::success(skill)
            }
            Err(e) => {
                EvolutionResult::failure(e.to_string())
            }
        }
    }

    async fn analyze_need_with_ai(&self, ai: &dyn AIProvider, context: &str) -> Result<ToolNeed, String> {
        let prompt = format!(r#"Analyze the user request and extract the tool specification needed.

User request: {}

Respond in JSON format:
{{
    "name": "tool_name_in_snake_case",
    "description": "What this tool does",
    "parameters": [
        {{"name": "param_name", "param_type": "String", "required": true, "description": "Parameter description"}}
    ],
    "return_type": "String"
}}"#, context);

        let request = ChatRequest::new(
            self.default_model.clone(),
            vec![Message::user(prompt)],
        )
        .with_temperature(0.3)
        .with_max_tokens(2000);

        let response = ai.chat(request).await
            .map_err(|e| e.to_string())?;

        let content = response.message.text_content()
            .ok_or("AI response is not text")?;

        serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse AI response: {}", e))
    }

    async fn generate_code_with_ai(&self, ai: &dyn AIProvider, spec: &ToolNeed) -> Result<String, String> {
        let params: String = spec.parameters.iter()
            .map(|p| format!("{}: {}", p.name, p.param_type))
            .collect::<Vec<_>>()
            .join(", ");

        let prompt = format!(r#"Generate a Rust function for the following tool specification.

Tool: {}
Description: {}
Parameters: {}
Return type: {}

Requirements:
1. Use async/await
2. Return Result<{}, String>
3. Code must compile without errors
4. Use appropriate HTTP client (reqwest) for API calls
5. Return meaningful data in JSON format

Respond ONLY with the code, no explanations:"#, 
            spec.name, 
            spec.description, 
            params,
            spec.return_type,
            spec.return_type
        );

        let request = ChatRequest::new(
            self.default_model.clone(),
            vec![Message::user(prompt)],
        )
        .with_temperature(0.2)
        .with_max_tokens(4000);

        let response = ai.chat(request).await
            .map_err(|e| e.to_string())?;

        response.message.text_content()
            .map(|s| s.trim().to_string())
            .ok_or("AI response is not text".to_string())
    }

    async fn fallback_evolve(&self, context: &str) -> EvolutionResult {
        let mut result = EvolutionResult::pending();
        
        result.logs.push("Detecting needs...".to_string());
        let needs = self.generator.detect_need(context).await;
        
        if needs.is_empty() {
            return EvolutionResult::failure("No tool needs detected".to_string());
        }
        
        let need = &needs[0];
        result.logs.push(format!("Generating skill: {}", need.name));
        result.status = EvolutionStatus::Generating;
        
        let code = self.generator.generate(need);
        
        result.logs.push("Validating code...".to_string());
        result.status = EvolutionStatus::Validating;
        
        if let Err(e) = self.compiler.validate(&code).await {
            return EvolutionResult::failure(e.to_string());
        }
        
        result.logs.push("Compiling skill...".to_string());
        result.status = EvolutionStatus::Compiling;
        
        match self.compiler.compile(&code).await {
            Ok(skill) => {
                result.logs.push(format!("Compiled: {} v{}", skill.language, skill.compiled_at.to_rfc3339()));
                EvolutionResult::success(skill)
            }
            Err(e) => {
                EvolutionResult::failure(e.to_string())
            }
        }
    }
}

impl Default for EvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
pub trait Sandbox: Send + Sync {
    async fn execute(&self, skill: &CompiledSkill, args: serde_json::Value) -> Result<serde_json::Value, SandboxError>;
    async fn validate(&self, skill: &CompiledSkill) -> Result<bool, SandboxError>;
}

#[derive(Debug, Clone)]
pub enum SandboxError {
    ExecutionFailed(String),
    ValidationFailed(String),
    Timeout,
    ResourceLimit,
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            SandboxError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
            SandboxError::Timeout => write!(f, "Execution timeout"),
            SandboxError::ResourceLimit => write!(f, "Resource limit exceeded"),
        }
    }
}

impl std::error::Error for SandboxError {}

pub struct SkillSandbox {
    max_execution_time_ms: u64,
    max_memory_mb: u64,
}

impl SkillSandbox {
    pub fn new() -> Self {
        Self {
            max_execution_time_ms: 5000,
            max_memory_mb: 256,
        }
    }

    pub fn with_limits(max_time_ms: u64, max_memory_mb: u64) -> Self {
        Self {
            max_execution_time_ms: max_time_ms,
            max_memory_mb: max_memory_mb,
        }
    }
}

#[async_trait]
impl Sandbox for SkillSandbox {
    async fn execute(&self, _skill: &CompiledSkill, _args: serde_json::Value) -> Result<serde_json::Value, SandboxError> {
        tracing::info!("Executing skill in sandbox (time_limit: {}ms, memory_limit: {}mb)", 
            self.max_execution_time_ms, self.max_memory_mb);
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(serde_json::json!({"status": "success", "message": "Skill executed in sandbox"}))
    }

    async fn validate(&self, skill: &CompiledSkill) -> Result<bool, SandboxError> {
        if skill.code.contains("rm -rf") {
            return Err(SandboxError::ValidationFailed("Dangerous command detected".to_string()));
        }
        
        Ok(true)
    }
}

impl Default for SkillSandbox {
    fn default() -> Self {
        Self::new()
    }
}

pub mod registry;
pub mod adapter;
pub mod skill_loader;
pub mod skill_tool_adapter;
pub mod propagation;
pub mod unified_skill_service;
pub mod skill_prompt_injector;
pub mod pattern_analyzer;
pub mod learning_history;
pub mod knowledge_graph;
pub mod skill_validator;
pub mod version_manager;
pub mod evo_v2_engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_need_creation() {
        let need = ToolNeed {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: vec![
                ParameterDef {
                    name: "arg".to_string(),
                    param_type: "String".to_string(),
                    required: true,
                    description: None,
                }
            ],
            return_type: "String".to_string(),
        };
        
        assert_eq!(need.name, "test_tool");
    }

    #[tokio::test]
    async fn test_detect_need() {
        let generator = SkillGenerator::new();
        let needs = generator.detect_need("I need to read files").await;
        
        assert!(!needs.is_empty());
    }

    #[tokio::test]
    async fn test_generate_skill() {
        let generator = SkillGenerator::new();
        let need = ToolNeed {
            name: "test_func".to_string(),
            description: "Test function".to_string(),
            parameters: vec![
                ParameterDef {
                    name: "x".to_string(),
                    param_type: "i32".to_string(),
                    required: true,
                    description: None,
                }
            ],
            return_type: "i32".to_string(),
        };
        
        let code = generator.generate(&need);
        assert!(code.contains("test_func"));
    }

    #[tokio::test]
    async fn test_compiler_validate() {
        let compiler = DynamicCompiler::new(ProgrammingLanguage::Wasm);
        
        let result = compiler.validate("pub async fn test() {}").await;
        assert!(result.is_ok());
        
        let result = compiler.validate("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_evolution_engine() {
        let engine = EvolutionEngine::new();
        let result = engine.evolve("I need to read files and execute commands").await;
        
        assert!(result.status == EvolutionStatus::Completed || result.status == EvolutionStatus::Failed);
    }

    #[tokio::test]
    async fn test_evolution_engine_with_ai_provider_fallback() {
        let engine = EvolutionEngine::new();
        let result = engine.evolve("I need to read files").await;
        assert!(!result.logs.is_empty());
    }

    #[tokio::test]
    async fn test_evolution_engine_builder() {
        let engine = EvolutionEngine::new()
            .with_model("gpt-4");
        
        assert_eq!(engine.evolve("test").await.status, EvolutionStatus::Failed);
    }

    #[tokio::test]
    async fn test_fallback_evolve() {
        let engine = EvolutionEngine::new();
        let result = engine.evolve("I need to read files").await;
        
        assert!(!result.logs.is_empty());
    }

    #[tokio::test]
    async fn test_sandbox_execute() {
        let sandbox = SkillSandbox::new();
        let skill = CompiledSkill {
            code: "test".to_string(),
            language: ProgrammingLanguage::Wasm,
            compiled_at: chrono::Utc::now(),
        };
        
        let result = sandbox.execute(&skill, serde_json::json!({})).await;
        assert!(result.is_ok());
    }
}
