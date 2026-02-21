use std::sync::Arc;
use async_trait::async_trait;
use serde_json::Value;
use openclaw_core::{Message, OpenClawError, Result};

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    async fn get_context(&self) -> Result<Vec<Message>>;
    async fn add_memory(&self, memory: Message) -> Result<()>;
    async fn recall(&self, query: &str) -> Result<Value>;
    async fn clear(&self) -> Result<()>;
}

#[async_trait]
pub trait SecurityProvider: Send + Sync {
    async fn check_input(&self, session_id: &str, input: &str) -> Result<()>;
    async fn validate_output(&self, session_id: &str, output: &str) -> Result<()>;
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn execute(&self, tool_name: &str, args: Value) -> Result<Value>;
    async fn list_tools(&self) -> Result<Vec<String>>;
}

pub struct MemoryManagerWrapper {
    inner: Arc<tokio::sync::Mutex<openclaw_memory::MemoryManager>>,
}

impl MemoryManagerWrapper {
    pub fn new(manager: openclaw_memory::MemoryManager) -> Self {
        Self {
            inner: Arc::new(tokio::sync::Mutex::new(manager)),
        }
    }
}

#[async_trait]
impl MemoryProvider for MemoryManagerWrapper {
    async fn get_context(&self) -> Result<Vec<Message>> {
        let manager = self.inner.lock().await;
        Ok(manager.get_context())
    }

    async fn add_memory(&self, memory: Message) -> Result<()> {
        let mut manager = self.inner.lock().await;
        manager.add(memory).await
    }

    async fn recall(&self, query: &str) -> Result<Value> {
        let manager = self.inner.lock().await;
        let result = manager.recall(query).await?;
        Ok(serde_json::to_value(result).unwrap_or(Value::Null))
    }

    async fn clear(&self) -> Result<()> {
        let mut manager = self.inner.lock().await;
        manager.clear().await
    }
}

pub struct SecurityPipelineWrapper {
    inner: Arc<openclaw_security::SecurityPipeline>,
}

impl SecurityPipelineWrapper {
    pub fn new(pipeline: openclaw_security::SecurityPipeline) -> Self {
        Self {
            inner: Arc::new(pipeline),
        }
    }
}

#[async_trait]
impl SecurityProvider for SecurityPipelineWrapper {
    async fn check_input(&self, session_id: &str, input: &str) -> Result<()> {
        let (result, _) = self.inner.check_input(session_id, input).await;
        match result {
            openclaw_security::PipelineResult::Allow => Ok(()),
            openclaw_security::PipelineResult::Block(reason) => {
                Err(OpenClawError::Execution(format!("Input blocked: {}", reason)))
            }
            openclaw_security::PipelineResult::Warn(reason) => {
                tracing::warn!("Security warning: {}", reason);
                Ok(())
            }
        }
    }

    async fn validate_output(&self, session_id: &str, output: &str) -> Result<()> {
        let (_validated_output, validation) = self.inner.validate_output(session_id, output).await;
        if validation.requires_action {
            Err(OpenClawError::Execution(format!(
                "Output validation failed: {} issues found",
                validation.matches.len()
            )))
        } else {
            Ok(())
        }
    }
}

#[async_trait]
impl ToolProvider for openclaw_tools::ToolRegistry {
    async fn execute(&self, tool_name: &str, args: Value) -> Result<Value> {
        self.execute(tool_name, args).await
    }

    async fn list_tools(&self) -> Result<Vec<String>> {
        Ok(self.list_tools())
    }
}

pub struct ProviderRegistry {
    memory: Option<Arc<dyn MemoryProvider>>,
    security: Option<Arc<dyn SecurityProvider>>,
    tools: Option<Arc<dyn ToolProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            memory: None,
            security: None,
            tools: None,
        }
    }

    pub fn with_memory(mut self, memory: Arc<dyn MemoryProvider>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_security(mut self, security: Arc<dyn SecurityProvider>) -> Self {
        self.security = Some(security);
        self
    }

    pub fn with_tools(mut self, tools: Arc<dyn ToolProvider>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn memory(&self) -> Option<&Arc<dyn MemoryProvider>> {
        self.memory.as_ref()
    }

    pub fn security(&self) -> Option<&Arc<dyn SecurityProvider>> {
        self.security.as_ref()
    }

    pub fn tools(&self) -> Option<&Arc<dyn ToolProvider>> {
        self.tools.as_ref()
    }

    pub fn take_memory(&mut self) -> Option<Arc<dyn MemoryProvider>> {
        self.memory.take()
    }

    pub fn take_security(&mut self) -> Option<Arc<dyn SecurityProvider>> {
        self.security.take()
    }

    pub fn take_tools(&mut self) -> Option<Arc<dyn ToolProvider>> {
        self.tools.take()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registry_new() {
        let registry = ProviderRegistry::new();
        assert!(registry.memory().is_none());
        assert!(registry.security().is_none());
        assert!(registry.tools().is_none());
    }

    #[test]
    fn test_provider_registry_builder_pattern() {
        let registry = ProviderRegistry::new()
            .with_memory(Arc::new(MemoryManagerWrapper::new(
                openclaw_memory::MemoryManager::new(openclaw_memory::MemoryConfig::default())
            )))
            .with_security(Arc::new(SecurityPipelineWrapper::new(
                openclaw_security::SecurityPipeline::new(openclaw_security::PipelineConfig::default())
            )))
            .with_tools(Arc::new(openclaw_tools::ToolRegistry::new()));
        
        assert!(registry.memory().is_some());
        assert!(registry.security().is_some());
        assert!(registry.tools().is_some());
    }

    #[test]
    fn test_provider_registry_take() {
        let mut registry = ProviderRegistry::new()
            .with_memory(Arc::new(MemoryManagerWrapper::new(
                openclaw_memory::MemoryManager::new(openclaw_memory::MemoryConfig::default())
            )));
        
        assert!(registry.memory().is_some());
        let taken = registry.take_memory();
        assert!(taken.is_some());
        assert!(registry.memory().is_none());
    }
}
