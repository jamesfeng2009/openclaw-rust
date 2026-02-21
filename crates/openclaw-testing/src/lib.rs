pub mod ai {
    use async_trait::async_trait;
    use openclaw_ai::{ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason, StreamChunk};
    use openclaw_core::{Message, Result, Role, Content};
    use serde::{Deserialize, Serialize};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use futures::stream::{Stream, self};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct MockChatMessage {
        pub role: String,
        pub content: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    pub struct MockUsage {
        pub prompt_tokens: u32,
        pub completion_tokens: u32,
        pub total_tokens: u32,
    }

    #[derive(Clone)]
    pub struct MockAiProvider {
        responses: Arc<Mutex<Vec<String>>>,
        call_count: Arc<Mutex<u32>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl Default for MockAiProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MockAiProvider {
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(vec!["Mock AI response".to_string()])),
                call_count: Arc::new(Mutex::new(0)),
                should_fail: Arc::new(Mutex::new(false)),
            }
        }

        pub fn with_response(self, response: String) -> Self {
            self.responses.lock().unwrap().push(response);
            self
        }

        pub fn with_responses(self, responses: Vec<String>) -> Self {
            *self.responses.lock().unwrap() = responses;
            self
        }

        pub fn call_count(&self) -> u32 {
            *self.call_count.lock().unwrap()
        }

        pub fn reset_count(&self) {
            *self.call_count.lock().unwrap() = 0;
        }

        pub fn set_should_fail(&self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }
    }

    #[async_trait]
    impl openclaw_ai::AIProvider for MockAiProvider {
        fn name(&self) -> &str {
            "mock-ai-provider"
        }

        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
            *self.call_count.lock().unwrap() += 1;

            if *self.should_fail.lock().unwrap() {
                return Err(openclaw_core::OpenClawError::AIProvider("Mock AI error".to_string()));
            }

            let responses = self.responses.lock().unwrap();
            let content = responses.first().cloned().unwrap_or_else(|| "Default mock response".to_string());

            Ok(ChatResponse {
                id: "mock-chat-1".to_string(),
                model: "mock-model".to_string(),
                message: Message {
                    id: uuid::Uuid::new_v4(),
                    role: Role::Assistant,
                    content: vec![Content::Text { text: content }],
                    created_at: chrono::Utc::now(),
                    metadata: Default::default(),
                },
                usage: openclaw_ai::TokenUsage {
                    prompt_tokens: 10,
                    completion_tokens: 20,
                    total_tokens: 30,
                },
                finish_reason: FinishReason::Stop,
            })
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            Ok(Box::pin(stream::empty()))
        }

        async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            Ok(EmbeddingResponse {
                embeddings: vec![],
                model: "mock-embedding".to_string(),
                usage: openclaw_ai::TokenUsage::new(0, 0),
            })
        }

        async fn models(&self) -> Result<Vec<String>> {
            Ok(vec!["mock-model-1".to_string(), "mock-model-2".to_string()])
        }

        async fn health_check(&self) -> Result<bool> {
            Ok(true)
        }
    }
}

#[cfg(test)]
pub mod device {
    use openclaw_device::DeviceHandle;
    use openclaw_device::DeviceRegistry;
    use openclaw_device::{DeviceCapabilities, DeviceStatus, Platform};

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
    use openclaw_core::config::{AgentsConfig, AiConfig, Config, DevicesConfig, ServerConfig};

    pub fn mock_config() -> Config {
        Config {
            server: ServerConfig::default(),
            ai: AiConfig::default(),
            memory: Default::default(),
            vector: Default::default(),
            channels: Default::default(),
            security: openclaw_core::config::SecurityConfig::default(),
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
