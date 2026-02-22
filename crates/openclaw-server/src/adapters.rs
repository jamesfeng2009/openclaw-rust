use async_trait::async_trait;
use std::sync::Arc;
use openclaw_agent::ports::{AIPort, MemoryPort, SecurityPort, ToolPort, MemoryEntry, SecurityCheckResult, ToolInfo, RecallItem};
use openclaw_core::{Result as OpenClawResult, Message};
use openclaw_ai::AIProvider;
use openclaw_memory::MemoryManager;
use openclaw_security::SecurityPipeline;
use openclaw_tools::ToolRegistry;

pub struct AIProviderAdapter {
    provider: Arc<dyn AIProvider>,
}

impl AIProviderAdapter {
    pub fn new(provider: Arc<dyn AIProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl AIPort for AIProviderAdapter {
    async fn chat(&self, messages: Vec<Message>) -> OpenClawResult<String> {
        use openclaw_ai::types::ChatRequest;
        
        let request = ChatRequest::new("default", messages);
        
        let response = self.provider.chat(request).await?;
        Ok(response.message.content.first()
            .map(|c| match c {
                openclaw_core::Content::Text { text } => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default())
    }
    
    async fn chat_stream(
        &self, 
        _messages: Vec<Message>
    ) -> OpenClawResult<Box<dyn futures::Stream<Item = OpenClawResult<String>> + Send + Sync>> {
        Err(openclaw_core::OpenClawError::Execution("chat_stream not implemented".to_string()))
    }

    async fn embed(&self, texts: Vec<String>) -> OpenClawResult<Vec<Vec<f32>>> {
        use openclaw_ai::types::EmbeddingRequest;
        
        let request = EmbeddingRequest {
            model: String::new(),
            input: texts,
        };
        
        let response = self.provider.embed(request).await?;
        Ok(response.embeddings)
    }
}

pub struct MemoryManagerAdapter {
    manager: Arc<MemoryManager>,
}

impl MemoryManagerAdapter {
    pub fn new(manager: Arc<MemoryManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl MemoryPort for MemoryManagerAdapter {
    async fn add(&self, _entry: MemoryEntry) -> OpenClawResult<()> {
        Ok(())
    }
    
    async fn retrieve(&self, query: &str, limit: usize) -> OpenClawResult<Vec<MemoryEntry>> {
        let retrieval = self.manager.retrieve(query, limit).await?;
        
        Ok(retrieval.items.into_iter().map(|item| {
            MemoryEntry {
                id: item.id.to_string(),
                content: serde_json::to_string(&item.content).unwrap_or_default(),
                metadata: std::collections::HashMap::new(),
            }
        }).collect())
    }
    
    async fn recall(&self, context: &str, limit: usize) -> OpenClawResult<Vec<RecallItem>> {
        let result = self.manager.recall(context).await?;
        
        Ok(result.items.into_iter().take(limit).map(|item| {
            RecallItem {
                entry: MemoryEntry {
                    id: item.id.to_string(),
                    content: serde_json::to_string(&item.content).unwrap_or_default(),
                    metadata: std::collections::HashMap::new(),
                },
                score: item.similarity,
            }
        }).collect())
    }
    
    async fn get_context(&self) -> OpenClawResult<Vec<Message>> {
        let retrieval = self.manager.retrieve("", 4096).await?;
        
        Ok(retrieval.items.into_iter().map(|item| {
            Message {
                id: item.id,
                role: openclaw_core::Role::User,
                content: vec![openclaw_core::Content::Text { 
                    text: serde_json::to_string(&item.content).unwrap_or_default() 
                }],
                created_at: item.created_at,
                metadata: Default::default(),
            }
        }).collect())
    }
}

pub struct SecurityPipelineAdapter {
    pipeline: Arc<SecurityPipeline>,
}

impl SecurityPipelineAdapter {
    pub fn new(pipeline: Arc<SecurityPipeline>) -> Self {
        Self { pipeline }
    }
}

#[async_trait]
impl SecurityPort for SecurityPipelineAdapter {
    async fn check(&self, input: &str) -> OpenClawResult<SecurityCheckResult> {
        let (result, _) = self.pipeline.check_input("default", input).await;
        
        match result {
            openclaw_security::PipelineResult::Allow => {
                Ok(SecurityCheckResult {
                    allowed: true,
                    reason: None,
                })
            }
            openclaw_security::PipelineResult::Block(reason) => {
                Ok(SecurityCheckResult {
                    allowed: false,
                    reason: Some(reason),
                })
            }
            openclaw_security::PipelineResult::Warn(reason) => {
                Ok(SecurityCheckResult {
                    allowed: true,
                    reason: Some(reason),
                })
            }
        }
    }
}

pub struct ToolRegistryAdapter {
    registry: Arc<ToolRegistry>,
}

impl ToolRegistryAdapter {
    pub fn new(registry: Arc<ToolRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl ToolPort for ToolRegistryAdapter {
    async fn execute(
        &self, 
        tool_name: &str, 
        arguments: serde_json::Value
    ) -> OpenClawResult<serde_json::Value> {
        self.registry.execute(tool_name, arguments).await
    }
    
    async fn list_tools(&self) -> OpenClawResult<Vec<ToolInfo>> {
        let tools = self.registry.list_tools();
        
        Ok(tools.into_iter().map(|name| ToolInfo {
            name: name.clone(),
            description: String::new(),
            parameters: serde_json::json!({}),
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_security_pipeline_adapter_check() {
        let pipeline = Arc::new(SecurityPipeline::default());
        let adapter = SecurityPipelineAdapter::new(pipeline);
        
        let result = adapter.check("hello").await;
        assert!(result.is_ok());
        assert!(result.unwrap().allowed);
    }
    
    #[tokio::test]
    async fn test_tool_registry_adapter_execute() {
        let registry = Arc::new(ToolRegistry::new());
        let adapter = ToolRegistryAdapter::new(registry);
        
        let result = adapter.execute("mock_tool", serde_json::json!({"key": "value"})).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_tool_registry_adapter_list_tools() {
        let registry = Arc::new(ToolRegistry::new());
        let adapter = ToolRegistryAdapter::new(registry);
        
        let result = adapter.list_tools().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
