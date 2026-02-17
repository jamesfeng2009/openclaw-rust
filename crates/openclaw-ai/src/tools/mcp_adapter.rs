use crate::tools::{FunctionDefinition, ToolDefinition, ToolExecutor, ToolRegistry};
use async_trait::async_trait;
use openclaw_core::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct McpToolAdapter {
    name: String,
    description: String,
    input_schema: Value,
    executor: Arc<
        dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            + Send
            + Sync,
    >,
}

impl McpToolAdapter {
    pub fn new(
        name: String,
        description: String,
        input_schema: Value,
        executor: impl Fn(
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        Self {
            name,
            description,
            input_schema,
            executor: Arc::new(executor),
        }
    }

    pub fn from_builtin(tool: &openclaw_tools::McpBuiltinTool) -> Self {
        let name = tool.name.clone();
        let description = tool.description.clone();
        let schema = tool.input_schema.clone();

        Self::new(name, description, schema, |_| {
            Box::pin(async move { Ok("MCP tool execution not implemented".to_string()) })
        })
    }
}

#[async_trait]
impl ToolExecutor for McpToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let executor = self.executor.clone();
        executor(arguments).await
    }
}

pub struct McpToolRegistry {
    tools: Arc<RwLock<Vec<McpToolAdapter>>>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, adapter: McpToolAdapter) {
        let mut tools = self.tools.write().await;
        tools.push(adapter);
    }

    pub async fn register_all(&self, mcp_tools: &[openclaw_tools::McpBuiltinTool]) {
        let mut tools = self.tools.write().await;
        for tool in mcp_tools {
            tools.push(McpToolAdapter::from_builtin(tool));
        }
    }

    pub async fn register_builtin_tools(&self) {
        let builtin_tools = openclaw_tools::McpBuiltinTools::all();
        self.register_all(&builtin_tools).await;
    }

    pub async fn get(&self, name: &str) -> Option<McpToolAdapter> {
        let tools = self.tools.read().await;
        tools.iter().find(|t| t.name() == name).cloned()
    }

    pub async fn list(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools
            .iter()
            .map(|t| ToolDefinition::new(t.name(), t.description()).with_parameters(t.parameters()))
            .collect()
    }

    pub async fn to_ai_registry(&self) -> ToolRegistry {
        let mut registry = ToolRegistry::new();
        let tools = self.tools.read().await;

        for tool in tools.iter() {
            let adapter = tool.clone();
            registry.register(Box::new(adapter));
        }

        registry
    }
}

impl Default for McpToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for McpToolAdapter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
            executor: self.executor.clone(),
        }
    }
}

impl Clone for McpToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}
