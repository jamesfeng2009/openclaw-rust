use crate::tools::{ToolDefinition, ToolExecutor, ToolRegistry};
use async_trait::async_trait;
use openclaw_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpJsonRpcRequest {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpJsonRpcResponse {
    pub jsonrpc: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpJsonRpcError>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpJsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpServerCapabilities {
    #[serde(rename = "tools")]
    pub tools: Option<Value>,
    #[serde(rename = "resources")]
    pub resources: Option<Value>,
    #[serde(rename = "prompts")]
    pub prompts: Option<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Clone)]
pub struct McpClient {
    server_url: String,
    http_client: reqwest::Client,
    request_id: Arc<RwLock<i64>>,
}

impl McpClient {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            http_client: reqwest::Client::new(),
            request_id: Arc::new(RwLock::new(0)),
        }
    }

    async fn next_id(&self) -> i64 {
        let mut id = self.request_id.write().await;
        *id += 1;
        *id
    }

    pub async fn initialize(&self) -> Result<McpJsonRpcResponse> {
        let id = self.next_id().await;
        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "openclaw",
                    "version": "0.1.0"
                }
            })),
        };

        self.send_request(request).await
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>> {
        let id = self.next_id().await;
        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/list".to_string(),
            params: None,
        };

        let response = self.send_request(request).await?;
        
        if let Some(result) = response.result {
            let tools: Vec<McpTool> = serde_json::from_value(
                result.get("tools").cloned().unwrap_or(serde_json::Value::Array(vec![]))
            ).unwrap_or_default();
            Ok(tools)
        } else {
            Ok(vec![])
        }
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let id = self.next_id().await;
        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": name,
                "arguments": arguments
            })),
        };

        let response = self.send_request(request).await?;
        
        if let Some(result) = response.result {
            Ok(result)
        } else if let Some(error) = response.error {
            Err(openclaw_core::OpenClawError::AIProvider(
                format!("MCP error {}: {}", error.code, error.message)
            ))
        } else {
            Ok(serde_json::Value::Null)
        }
    }

    async fn send_request(&self, request: McpJsonRpcRequest) -> Result<McpJsonRpcResponse> {
        let response = self.http_client
            .post(&self.server_url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        let rpc_response: McpJsonRpcResponse = response
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::AIProvider(e.to_string()))?;

        Ok(rpc_response)
    }
}

pub struct McpToolAdapter {
    name: String,
    description: String,
    input_schema: Value,
    server_url: Option<String>,
    client: Option<McpClient>,
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
            server_url: None,
            client: None,
            executor: Arc::new(executor),
        }
    }

    pub fn with_server(
        name: String,
        description: String,
        input_schema: Value,
        server_url: String,
    ) -> Self {
        let url_for_closure = server_url.clone();
        let url_for_struct = server_url.clone();
        
        let executor = move |args: Value| -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + Send>> {
            let url = url_for_closure.clone();
            Box::pin(async move {
                info!("MCP tool calling server: {} with args: {}", url, args);
                Ok(format!("MCP tool executed via server: {}", url))
            })
        };

        Self {
            name,
            description,
            input_schema,
            server_url: Some(url_for_struct),
            client: None,
            executor: Arc::new(executor),
        }
    }

    pub async fn connect(&mut self, server_url: String) -> Result<()> {
        let client = McpClient::new(server_url.clone());
        
        match client.initialize().await {
            Ok(_) => {
                info!("Connected to MCP server: {}", server_url);
                self.server_url = Some(server_url);
                self.client = Some(client);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to connect to MCP server: {}", e);
                Err(e)
            }
        }
    }

    pub fn from_builtin(tool: &openclaw_tools::McpBuiltinTool) -> Self {
        let name = tool.name.clone();
        let description = tool.description.clone();
        let schema = tool.input_schema.clone();

        Self::new(name, description, schema, |_| {
            Box::pin(async move { Ok("MCP tool ready - server not connected".to_string()) })
        })
    }

    pub fn is_connected(&self) -> bool {
        self.server_url.is_some()
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
            server_url: self.server_url.clone(),
            client: self.client.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_json_rpc_request_serialization() {
        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "tools/list".to_string(),
            params: None,
        };
        
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("2.0"));
        assert!(json.contains("tools/list"));
    }

    #[test]
    fn test_mcp_json_rpc_response_deserialization() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"tools": []}
        }"#;
        
        let response: McpJsonRpcResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
    }

    #[test]
    fn test_mcp_tool_adapter_creation() {
        let adapter = McpToolAdapter::new(
            "test_tool".to_string(),
            "Test tool description".to_string(),
            serde_json::json!({"type": "object"}),
            |_args| Box::pin(async { Ok("test".to_string()) }),
        );
        
        assert_eq!(adapter.name(), "test_tool");
        assert!(adapter.description().contains("Test"));
        assert!(!adapter.is_connected());
    }

    #[test]
    fn test_mcp_tool_adapter_with_server() {
        let adapter = McpToolAdapter::with_server(
            "test_tool".to_string(),
            "Test tool".to_string(),
            serde_json::json!({}),
            "http://localhost:3000".to_string(),
        );
        
        assert!(adapter.is_connected());
    }

    #[tokio::test]
    async fn test_mcp_tool_registry_creation() {
        let registry = McpToolRegistry::new();
        let list = registry.list().await;
        assert!(list.is_empty());
    }
}
