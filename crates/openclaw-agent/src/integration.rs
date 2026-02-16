//! OpenClaw Agent 集成示例
//!
//! 展示如何创建和配置 Agent 系统，支持多种 AI 提供商

use std::sync::Arc;

use crate::{Agent, BaseAgent, TaskInput, TaskRequest, TaskType};
use openclaw_ai::{
    AIProvider,
    providers::{
        OpenAIProvider, AnthropicProvider, GeminiProvider,
        DeepSeekProvider, QwenProvider, GlmProvider,
        MinimaxProvider, KimiProvider, ProviderConfig,
        ProviderFactory, ProviderType,
    },
    models::get_all_models,
};

// ============== 提供商创建函数 ==============

/// 创建 OpenAI 提供商
pub fn create_openai_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("openai", api_key)
        .with_default_model("gpt-4o");
    Arc::new(OpenAIProvider::new(config))
}

/// 创建 Anthropic 提供商
pub fn create_anthropic_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("anthropic", api_key)
        .with_default_model("claude-3-7-sonnet");
    Arc::new(AnthropicProvider::new(config))
}

/// 创建 Google Gemini 提供商
pub fn create_gemini_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("google", api_key)
        .with_default_model("gemini-2.0-flash");
    Arc::new(GeminiProvider::new(config))
}

/// 创建 DeepSeek 提供商
pub fn create_deepseek_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("deepseek", api_key)
        .with_default_model("deepseek-chat");
    Arc::new(DeepSeekProvider::new(config))
}

/// 创建 Qwen 通义千问提供商
pub fn create_qwen_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("qwen", api_key)
        .with_default_model("qwen-plus");
    Arc::new(QwenProvider::new(config))
}

/// 创建 GLM 智谱提供商
pub fn create_glm_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("glm", api_key)
        .with_default_model("glm-4-flash");
    Arc::new(GlmProvider::new(config))
}

/// 创建 Minimax 提供商
pub fn create_minimax_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("minimax", api_key)
        .with_default_model("abab6.5s-chat");
    Arc::new(MinimaxProvider::new(config))
}

/// 创建 Kimi 月之暗面提供商
pub fn create_kimi_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("kimi", api_key)
        .with_default_model("moonshot-v1-128k");
    Arc::new(KimiProvider::new(config))
}

/// 创建 OpenRouter 提供商
pub fn create_openrouter_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new("openrouter", api_key)
        .with_default_model("openai/gpt-4o");
    match ProviderFactory::create(ProviderType::OpenRouter, config) {
        Ok(provider) => provider,
        Err(e) => panic!("Failed to create OpenRouter provider: {}", e),
    }
}

/// 创建 Ollama 本地模型提供商
pub fn create_ollama_provider(base_url: Option<&str>) -> Arc<dyn AIProvider> {
    let url = base_url.unwrap_or("http://localhost:11434");
    let config = ProviderConfig::new("ollama", "dummy")
        .with_base_url(url)
        .with_default_model("llama3.1");
    match ProviderFactory::create(ProviderType::Ollama, config) {
        Ok(provider) => provider,
        Err(e) => panic!("Failed to create Ollama provider: {}", e),
    }
}

/// 使用工厂模式创建提供商 (推荐)
pub fn create_provider(provider_type: ProviderType, api_key: &str, base_url: Option<&str>) -> Arc<dyn AIProvider> {
    let config = ProviderConfig::new(provider_type.to_string(), api_key);
    let config = if let Some(url) = base_url {
        config.with_base_url(url)
    } else {
        config
    };
    
    match ProviderFactory::create(provider_type, config) {
        Ok(provider) => provider,
        Err(e) => panic!("Failed to create provider: {}", e),
    }
}

/// 从名称创建提供商 (支持动态配置)
pub fn create_provider_by_name(name: &str, api_key: Option<String>, base_url: Option<String>) -> Result<Arc<dyn AIProvider>, String> {
    ProviderFactory::create_from_name(name, api_key, base_url)
}

// ============== Agent 创建函数 ==============

/// 为 Agent 配置 AI 提供商
pub fn configure_agent_with_ai(agent: &mut BaseAgent, provider: Arc<dyn AIProvider>) {
    agent.set_ai_provider(provider);
}

/// 创建配置好 AI 的 Coder Agent
pub fn create_coder_agent(provider: Arc<dyn AIProvider>) -> BaseAgent {
    let mut agent = BaseAgent::coder();
    agent.set_ai_provider(provider);
    agent
}

/// 创建配置好 AI 的 Conversationalist Agent
pub fn create_chat_agent(provider: Arc<dyn AIProvider>) -> BaseAgent {
    let mut agent = BaseAgent::conversationalist();
    agent.set_ai_provider(provider);
    agent
}

/// 创建配置好 AI 的 Researcher Agent
pub fn create_researcher_agent(provider: Arc<dyn AIProvider>) -> BaseAgent {
    let mut agent = BaseAgent::researcher();
    agent.set_ai_provider(provider);
    agent
}

/// 创建配置好 AI 的 Writer Agent
pub fn create_writer_agent(provider: Arc<dyn AIProvider>) -> BaseAgent {
    let mut agent = BaseAgent::writer();
    agent.set_ai_provider(provider);
    agent
}

// ============== 任务创建函数 ==============

/// 示例：创建一个简单的对话任务
pub fn create_conversation_task(message: &str) -> TaskRequest {
    TaskRequest::new(
        TaskType::Conversation,
        TaskInput::Text { content: message.to_string() },
    )
}

/// 示例：创建一个代码生成任务
pub fn create_code_task(description: &str) -> TaskRequest {
    TaskRequest::new(
        TaskType::CodeGeneration,
        TaskInput::Text { content: description.to_string() },
    )
}

/// 示例：创建一个问答任务
pub fn create_qa_task(question: &str) -> TaskRequest {
    TaskRequest::new(
        TaskType::QuestionAnswer,
        TaskInput::Text { content: question.to_string() },
    )
}

/// 示例：创建一个搜索任务
pub fn create_search_task(query: &str) -> TaskRequest {
    TaskRequest::new(
        TaskType::WebSearch,
        TaskInput::Text { content: query.to_string() },
    )
}

// ============== 工具函数 ==============

/// 获取所有支持的模型
pub fn list_all_models() -> Vec<String> {
    get_all_models().iter().map(|m| format!("{} ({})", m.id, m.name)).collect()
}

/// 获取指定提供商的模型列表
pub fn list_provider_models(provider: &str) -> Vec<String> {
    get_all_models()
        .iter()
        .filter(|m| m.provider == provider)
        .map(|m| m.id.clone())
        .collect()
}

// ============== 测试 ==============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_all_providers() {
        // 测试创建所有提供商
        let openai = create_openai_provider("test-key");
        assert_eq!(openai.name(), "openai");

        let anthropic = create_anthropic_provider("test-key");
        assert_eq!(anthropic.name(), "anthropic");

        let gemini = create_gemini_provider("test-key");
        assert_eq!(gemini.name(), "google");

        let deepseek = create_deepseek_provider("test-key");
        assert_eq!(deepseek.name(), "deepseek");

        let qwen = create_qwen_provider("test-key");
        assert_eq!(qwen.name(), "qwen");

        let glm = create_glm_provider("test-key");
        assert_eq!(glm.name(), "glm");

        let minimax = create_minimax_provider("test-key");
        assert_eq!(minimax.name(), "minimax");

        let kimi = create_kimi_provider("test-key");
        assert_eq!(kimi.name(), "kimi");
    }

    #[test]
    fn test_agent_creation() {
        let provider = create_openai_provider("test-key");
        
        let coder = create_coder_agent(provider.clone());
        assert_eq!(coder.id(), "coder");

        let chat = create_chat_agent(provider.clone());
        assert_eq!(chat.id(), "chat");

        let researcher = create_researcher_agent(provider.clone());
        assert_eq!(researcher.id(), "researcher");

        let writer = create_writer_agent(provider);
        assert_eq!(writer.id(), "writer");
    }

    #[test]
    fn test_task_request_creation() {
        let task = create_conversation_task("Hello");
        assert!(task.preferred_agent.is_none());

        let code_task = create_code_task("Write a Python hello world");
        assert_eq!(code_task.task_type, TaskType::CodeGeneration);

        let qa_task = create_qa_task("What is Rust?");
        assert_eq!(qa_task.task_type, TaskType::QuestionAnswer);
    }

    #[test]
    fn test_model_listing() {
        let all_models = list_all_models();
        assert!(!all_models.is_empty());

        let openai_models = list_provider_models("openai");
        assert!(!openai_models.is_empty());

        let qwen_models = list_provider_models("qwen");
        assert!(!qwen_models.is_empty());
    }
}
