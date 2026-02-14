//! OpenClaw Agent 集成示例
//!
//! 展示如何创建和配置 Agent 系统

use std::sync::Arc;

use crate::{Agent, BaseAgent, Orchestrator, TaskInput, TaskRequest, TaskType};
use openclaw_ai::{AIProvider, providers::{OpenAIProvider, AnthropicProvider, ProviderConfig}};

/// 创建 OpenAI 提供商
pub fn create_openai_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig {
        name: "openai".to_string(),
        api_key: Some(api_key.to_string()),
        base_url: None,
        default_model: "gpt-4o".to_string(),
    };
    Arc::new(OpenAIProvider::new(config))
}

/// 创建 Anthropic 提供商
pub fn create_anthropic_provider(api_key: &str) -> Arc<dyn AIProvider> {
    let config = ProviderConfig {
        name: "anthropic".to_string(),
        api_key: Some(api_key.to_string()),
        base_url: None,
        default_model: "claude-3-5-sonnet-20241022".to_string(),
    };
    Arc::new(AnthropicProvider::new(config))
}

/// 为 Agent 配置 AI 提供商
pub fn configure_agent_with_ai(agent: &mut BaseAgent, provider: Arc<dyn AIProvider>) {
    agent.set_ai_provider(provider);
}

/// 创建配置好 AI 的 Coder Agent
pub fn create_coder_agent(api_key: &str, use_anthropic: bool) -> BaseAgent {
    let mut agent = BaseAgent::coder();
    let provider = if use_anthropic {
        create_anthropic_provider(api_key)
    } else {
        create_openai_provider(api_key)
    };
    agent.set_ai_provider(provider);
    agent
}

/// 创建配置好 AI 的 Conversationalist Agent
pub fn create_chat_agent(api_key: &str, use_anthropic: bool) -> BaseAgent {
    let mut agent = BaseAgent::conversationalist();
    let provider = if use_anthropic {
        create_anthropic_provider(api_key)
    } else {
        create_openai_provider(api_key)
    };
    agent.set_ai_provider(provider);
    agent
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_providers() {
        // 测试创建 OpenAI 提供商
        let openai = create_openai_provider("test-key");
        assert_eq!(openai.name(), "openai");

        // 测试创建 Anthropic 提供商
        let anthropic = create_anthropic_provider("test-key");
        assert_eq!(anthropic.name(), "anthropic");
    }

    #[test]
    fn test_agent_creation() {
        // 测试创建各种类型的 Agent
        let orchestrator = BaseAgent::orchestrator();
        assert_eq!(orchestrator.id(), "orchestrator");

        let coder = BaseAgent::coder();
        assert_eq!(coder.id(), "coder");

        let researcher = BaseAgent::researcher();
        assert_eq!(researcher.id(), "researcher");
    }

    #[test]
    fn test_task_request_creation() {
        // 测试创建任务请求
        let task = create_conversation_task("Hello");
        assert!(task.preferred_agent.is_none());
        assert!(!task.required_capabilities.is_empty());

        let code_task = create_code_task("Write a Python hello world");
        assert_eq!(code_task.task_type, TaskType::CodeGeneration);
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        // 测试创建 Orchestrator
        let orchestrator = Orchestrator::with_default_team();
        
        // 测试获取 agent 列表
        let agent_ids = orchestrator.team().agent_ids();
        assert!(!agent_ids.is_empty());
        
        // 测试获取特定 agent
        let agent = orchestrator.team().get_agent("chat");
        assert!(agent.is_some());
    }
}
