use crate::tools::{FunctionDefinition, ToolDefinition, ToolExecutor, ToolRegistry};
use async_trait::async_trait;
use openclaw_core::Result;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct WasmToolAdapter {
    name: String,
    description: String,
    input_schema: Value,
    executor: Arc<
        dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>>
            + Send
            + Sync,
    >,
}

impl WasmToolAdapter {
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

    pub fn from_config(name: &str, description: &str, input_schema: Value) -> Self {
        let name_arc = Arc::new(name.to_string());
        let desc = description.to_string();
        let name_for_closure = name_arc.clone();

        let executor = move |_args: Value| -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<String>> + Send>,
        > {
            let name = name_for_closure.clone();
            Box::pin(async move { Ok(format!("WASM tool '{}' execution placeholder", name)) })
        };

        Self::new(name.to_string(), desc, input_schema, executor)
    }
}

#[async_trait]
impl ToolExecutor for WasmToolAdapter {
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

pub struct WasmToolRegistry {
    tools: Arc<RwLock<Vec<WasmToolAdapter>>>,
}

impl WasmToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register(&self, adapter: WasmToolAdapter) {
        let mut tools = self.tools.write().await;
        tools.push(adapter);
    }

    pub async fn register_tool(
        &self,
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
    ) {
        let adapter = WasmToolAdapter::new(name, description, input_schema, executor);
        self.register(adapter).await;
    }

    pub async fn get(&self, name: &str) -> Option<WasmToolAdapter> {
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

impl Default for WasmToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for WasmToolAdapter {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
            executor: self.executor.clone(),
        }
    }
}

impl Clone for WasmToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
        }
    }
}
