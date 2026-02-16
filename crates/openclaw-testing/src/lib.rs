#[cfg(test)]
pub mod ai {
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MockChatMessage {
        pub role: String,
        pub content: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MockChatChoice {
        pub message: MockChatMessage,
        pub index: u32,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MockChatResponse {
        pub id: String,
        pub choices: Vec<MockChatChoice>,
        pub usage: MockUsage,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct MockUsage {
        pub prompt_tokens: u32,
        pub completion_tokens: u32,
        pub total_tokens: u32,
    }

    #[derive(Clone)]
    pub struct MockAiProvider {
        responses: Arc<Mutex<Vec<MockChatResponse>>>,
        call_count: Arc<Mutex<u32>>,
    }

    impl Default for MockAiProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockAiProvider {
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(vec![MockChatResponse {
                    id: "mock-chat-1".to_string(),
                    choices: vec![MockChatChoice {
                        message: MockChatMessage {
                            role: "assistant".to_string(),
                            content: "Hello, I am a mock AI response!".to_string(),
                        },
                        index: 0,
                    }],
                    usage: MockUsage::default(),
                }])),
                call_count: Arc::new(Mutex::new(0)),
            }
        }

        pub fn with_response(mut self, response: MockChatResponse) -> Self {
            self.responses.lock().unwrap().push(response);
            self
        }

        pub fn call_count(&self) -> u32 {
            *self.call_count.lock().unwrap()
        }

        pub fn reset_count(&self) {
            *self.call_count.lock().unwrap() = 0;
        }
    }

    #[async_trait]
    pub trait MockAiClient: Send + Sync {
        async fn chat(&self, messages: Vec<MockChatMessage>) -> MockChatResponse;
        fn name(&self) -> &str;
    }

    #[derive(Clone)]
    pub struct MockOpenAiClient {
        provider: MockAiProvider,
    }

    impl Default for MockOpenAiClient {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockOpenAiClient {
        pub fn new() -> Self {
            Self {
                provider: MockAiProvider::new(),
            }
        }

        pub fn with_response(mut self, response: MockChatResponse) -> Self {
            self.provider = self.provider.with_response(response);
            self
        }
    }

    #[async_trait]
    impl MockAiClient for MockOpenAiClient {
        async fn chat(&self, _messages: Vec<MockChatMessage>) -> MockChatResponse {
            *self.provider.call_count.lock().unwrap() += 1;
            let responses = self.provider.responses.lock().unwrap();
            responses.first().cloned().unwrap_or_else(|| MockChatResponse {
                id: "mock-default".to_string(),
                choices: vec![],
                usage: MockUsage::default(),
            })
        }

        fn name(&self) -> &str {
            "mock-openai"
        }
    }
}

#[cfg(test)]
pub mod device {
    use openclaw_device::{DeviceCapabilities, Platform, DeviceStatus};
    use openclaw_device::DeviceHandle;
    use openclaw_device::DeviceRegistry;

    pub fn mock_device_handle() -> DeviceHandle {
        DeviceHandle {
            id: "mock-device-1".to_string(),
            name: "Mock Device".to_string(),
            platform: Platform::LinuxServer,
            capabilities: mock_device_capabilities(),
            status: DeviceStatus::Online,
        }
    }

    pub fn mock_device_capabilities() -> DeviceCapabilities {
        DeviceCapabilities::default()
    }

    pub fn create_mock_registry() -> DeviceRegistry {
        DeviceRegistry::new()
    }
}

#[cfg(test)]
pub mod config {
    use openclaw_core::config::{Config, ServerConfig, AiConfig, DevicesConfig, AgentsConfig};
    
    pub fn mock_config() -> Config {
        Config {
            server: ServerConfig::default(),
            ai: AiConfig::default(),
            memory: Default::default(),
            vector: Default::default(),
            channels: Default::default(),
            agents: AgentsConfig::default(),
            devices: DevicesConfig {
                enabled: false,
                ..Default::default()
            },
            workspaces: Default::default(),
        }
    }
}

#[cfg(test)]
pub mod channel {
    use serde::{Deserialize, Serialize};
    
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MockMessage {
        pub id: String,
        pub channel_id: String,
        pub user_id: String,
        pub content: String,
        pub timestamp: i64,
    }

    impl MockMessage {
        pub fn new(channel_id: impl Into<String>, content: impl Into<String>) -> Self {
            Self {
                id: "mock-msg-1".to_string(),
                channel_id: channel_id.into(),
                user_id: "mock-user".to_string(),
                content: content.into(),
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
}

#[cfg(test)]
pub mod agent {
    use openclaw_agent::{AgentConfig, AgentType, types::Capability};
    
    pub fn mock_agent_config() -> AgentConfig {
        AgentConfig {
            id: "mock-agent".to_string(),
            name: "Mock Agent".to_string(),
            agent_type: AgentType::Conversationalist,
            description: Some("A mock agent for testing".to_string()),
            system_prompt: Some("You are a helpful assistant.".to_string()),
            aieos: None,
            model: Some("gpt-4o".to_string()),
            capabilities: vec![Capability::Conversation],
            priority: 50,
            max_concurrent_tasks: 3,
            enabled: true,
        }
    }
}
