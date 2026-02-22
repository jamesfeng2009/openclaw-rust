#[cfg(feature = "testing")]
pub mod mock {
    use async_trait::async_trait;
    use openclaw_ai::{
        ChatRequest, ChatResponse, EmbeddingRequest, EmbeddingResponse, FinishReason,
        StreamChunk, TokenUsage, AIProvider,
    };
    use openclaw_core::{Message, Result, Role, Content};
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use futures::stream::{Stream, self};

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
    impl AIProvider for MockAiProvider {
        fn name(&self) -> &str {
            "mock-ai-provider"
        }

        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse> {
            *self.call_count.lock().unwrap() += 1;

            if *self.should_fail.lock().unwrap() {
                return Err(openclaw_core::OpenClawError::AIProvider(
                    "Mock AI error".to_string(),
                ));
            }

            let responses = self.responses.lock().unwrap();
            let content = responses
                .first()
                .cloned()
                .unwrap_or_else(|| "Default mock response".to_string());

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
                usage: TokenUsage::new(10, 20),
                finish_reason: FinishReason::Stop,
            })
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
            use futures::stream;
            
            let responses = self.responses.lock().unwrap();
            let content = responses.first().cloned().unwrap_or_else(|| "Default mock response".to_string());
            
            let chunks: Vec<Result<StreamChunk>> = content
                .chars()
                .collect::<Vec<_>>()
                .chunks(3)
                .map(|chunk| {
                    Ok(StreamChunk {
                        id: "mock-chunk".to_string(),
                        model: "mock-model".to_string(),
                        delta: openclaw_ai::types::StreamDelta {
                            role: Some("assistant".to_string()),
                            content: Some(chunk.iter().collect()),
                            tool_calls: vec![],
                        },
                        finished: false,
                        finish_reason: None,
                    })
                })
                .collect();
            
            let last_chunk = Ok(StreamChunk {
                id: "mock-chunk-end".to_string(),
                model: "mock-model".to_string(),
                delta: openclaw_ai::types::StreamDelta {
                    role: Some("assistant".to_string()),
                    content: None,
                    tool_calls: vec![],
                },
                finished: true,
                finish_reason: Some(FinishReason::Stop),
            });
            
            let mut all_chunks = chunks;
            all_chunks.push(last_chunk);
            
            Ok(Box::pin(stream::iter(all_chunks)))
        }

        async fn embed(&self, _request: EmbeddingRequest) -> Result<EmbeddingResponse> {
            Ok(EmbeddingResponse {
                embeddings: vec![],
                model: "mock-embedding".to_string(),
                usage: TokenUsage::new(0, 0),
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
