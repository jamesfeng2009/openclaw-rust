use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum McpTransport {
    Stdio,
    Http { url: String },
    Sse { url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    pub enabled: bool,
    pub timeout_secs: u64,
    pub retry_attempts: u32,
    pub env_vars: HashMap<String, String>,
}

impl McpServerConfig {
    pub fn stdio(name: impl Into<String>, _command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::Stdio,
            enabled: true,
            timeout_secs: 30,
            retry_attempts: 3,
            env_vars: HashMap::new(),
        }
    }

    pub fn http(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::Http { url: url.into() },
            enabled: true,
            timeout_secs: 30,
            retry_attempts: 3,
            env_vars: HashMap::new(),
        }
    }

    pub fn sse(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            transport: McpTransport::Sse { url: url.into() },
            enabled: true,
            timeout_secs: 30,
            retry_attempts: 3,
            env_vars: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout_secs = timeout;
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub mime_type: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: String,
    pub arguments: Vec<McpPromptArgument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptArgument {
    pub name: String,
    pub description: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapability {
    pub tools: Option<McpToolsCapability>,
    pub resources: Option<McpResourcesCapability>,
    pub prompts: Option<McpPromptsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourcesCapability {
    pub subscribe: Option<bool>,
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptsCapability {
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub protocol_version: String,
    pub capabilities: McpCapability,
    pub server_info: McpServerInfoDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfoDetail {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpInitializeResult {
    pub protocol_version: String,
    pub capabilities: McpCapability,
    pub server_info: McpServerInfoDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolsListResult {
    pub tools: Vec<McpTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolsCallResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: McpResourceContent },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResourcesListResult {
    pub resources: Vec<McpResource>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptsListResult {
    pub prompts: Vec<McpPrompt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptsGetResult {
    pub messages: Vec<McpPromptMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPromptMessage {
    pub role: String,
    pub content: McpContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcRequest {
    pub jsonrpc: String,
    pub id: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcResponse {
    pub jsonrpc: String,
    pub id: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpJsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpJsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct McpServer {
    config: McpServerConfig,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    connected: bool,
    http_client: Option<Client>,
    server_info: Option<McpServerInfo>,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            connected: false,
            http_client: None,
            server_info: None,
        }
    }

    pub async fn connect(&mut self) -> Result<(), McpError> {
        if !self.config.enabled {
            return Err(McpError::Disabled(self.config.name.clone()));
        }

        info!("Connecting to MCP server: {}", self.config.name);

        match &self.config.transport {
            McpTransport::Stdio => {
                self.connected = true;
                Ok(())
            }
            McpTransport::Http { url } => {
                info!("Connecting to MCP HTTP server at: {}", url);
                self.http_client = Some(Client::new());
                self.connected = true;
                Ok(())
            }
            McpTransport::Sse { url } => {
                info!("Connecting to MCP SSE server at: {}", url);
                self.http_client = Some(Client::new());
                self.connected = true;
                Ok(())
            }
        }
    }

    pub async fn initialize(&mut self) -> Result<McpInitializeResult, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }

        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4().to_string(),
            method: "initialize".to_string(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "openclaw-rust",
                    "version": "0.1.0"
                }
            })),
        };

        let result = self.send_jsonrpc_request(request).await?;

        if let Ok(init_result) = serde_json::from_value::<McpInitializeResult>(result.clone()) {
            self.server_info = Some(McpServerInfo {
                protocol_version: init_result.protocol_version.clone(),
                capabilities: init_result.capabilities.clone(),
                server_info: init_result.server_info.clone(),
            });

            let _ = self
                .send_notification("notifications/initialized", serde_json::Value::Null)
                .await;
            if let Err(e) = self.list_tools_internal().await {
                tracing::warn!("Failed to list tools during initialization: {}", e);
            }

            Ok(init_result)
        } else {
            Err(McpError::TransportError(
                "Failed to parse initialize result".to_string(),
            ))
        }
    }

    async fn send_jsonrpc_request(
        &self,
        request: McpJsonRpcRequest,
    ) -> Result<serde_json::Value, McpError> {
        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| McpError::TransportError("HTTP client not initialized".to_string()))?;

        let url = match &self.config.transport {
            McpTransport::Http { url } => url.clone(),
            McpTransport::Sse { url } => url.clone(),
            _ => {
                return Err(McpError::TransportError(
                    "Not an HTTP transport".to_string(),
                ));
            }
        };

        let response = client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(self.config.timeout_secs))
            .send()
            .await
            .map_err(|e| McpError::TransportError(e.to_string()))?;

        let response_json: McpJsonRpcResponse = response
            .json()
            .await
            .map_err(|e| McpError::TransportError(e.to_string()))?;

        if let Some(error) = response_json.error {
            return Err(McpError::TransportError(error.message));
        }

        response_json
            .result
            .ok_or_else(|| McpError::TransportError("No result in response".to_string()))
    }

    async fn send_notification(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<(), McpError> {
        let client = self
            .http_client
            .as_ref()
            .ok_or_else(|| McpError::TransportError("HTTP client not initialized".to_string()))?;

        let url = match &self.config.transport {
            McpTransport::Http { url } => url.clone(),
            McpTransport::Sse { url } => url.clone(),
            _ => {
                return Err(McpError::TransportError(
                    "Not an HTTP transport".to_string(),
                ));
            }
        };

        let notification = McpJsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: Some(params),
        };

        let _ = client
            .post(&url)
            .json(&notification)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;

        Ok(())
    }

    async fn list_tools_internal(&mut self) -> Result<Vec<McpTool>, McpError> {
        let request = McpJsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Uuid::new_v4().to_string(),
            method: "tools/list".to_string(),
            params: None,
        };

        let result = self.send_jsonrpc_request(request).await?;

        if let Ok(list_result) = serde_json::from_value::<McpToolsListResult>(result) {
            self.tools = list_result.tools.clone();
            Ok(self.tools.clone())
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn disconnect(&mut self) {
        self.connected = false;
        info!("Disconnected from MCP server: {}", self.config.name);
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }
        Ok(self.tools.clone())
    }

    pub async fn call_tool(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }

        let tool = self.tools.iter().find(|t| t.name == name);
        if tool.is_none() {
            return Err(McpError::ToolNotFound(name.to_string()));
        }

        debug!("Calling MCP tool: {} on server: {}", name, self.config.name);

        if self.http_client.is_some() {
            let request = McpJsonRpcRequest {
                jsonrpc: "2.0".to_string(),
                id: Uuid::new_v4().to_string(),
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "name": name,
                    "arguments": arguments
                })),
            };

            let result = self.send_jsonrpc_request(request).await?;
            Ok(result)
        } else {
            Ok(serde_json::json!({
                "success": true,
                "content": []
            }))
        }
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }
        Ok(self.resources.clone())
    }

    pub async fn read_resource(&self, uri: &str) -> Result<serde_json::Value, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }

        let resource = self.resources.iter().find(|r| r.uri == uri);
        if resource.is_none() {
            return Err(McpError::ResourceNotFound(uri.to_string()));
        }

        Ok(serde_json::json!({
            "contents": [{
                "uri": uri,
                "mimeType": "application/json",
                "text": ""
            }]
        }))
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }
        Ok(self.prompts.clone())
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        _arguments: HashMap<String, String>,
    ) -> Result<serde_json::Value, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }

        let prompt = self.prompts.iter().find(|p| p.name == name);
        if prompt.is_none() {
            return Err(McpError::PromptNotFound(name.to_string()));
        }

        Ok(serde_json::json!({
            "messages": []
        }))
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn config(&self) -> &McpServerConfig {
        &self.config
    }
}

#[derive(Debug, Clone)]
pub enum McpError {
    NotConnected(String),
    Disabled(String),
    ToolNotFound(String),
    ResourceNotFound(String),
    PromptNotFound(String),
    TransportError(String),
    Timeout(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::NotConnected(name) => write!(f, "MCP server '{}' is not connected", name),
            McpError::Disabled(name) => write!(f, "MCP server '{}' is disabled", name),
            McpError::ToolNotFound(name) => write!(f, "MCP tool '{}' not found", name),
            McpError::ResourceNotFound(uri) => write!(f, "MCP resource '{}' not found", uri),
            McpError::PromptNotFound(name) => write!(f, "MCP prompt '{}' not found", name),
            McpError::TransportError(msg) => write!(f, "MCP transport error: {}", msg),
            McpError::Timeout(msg) => write!(f, "MCP timeout: {}", msg),
        }
    }
}

impl std::error::Error for McpError {}

pub struct McpClient {
    servers: Arc<RwLock<HashMap<String, McpServer>>>,
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_server(&self, config: McpServerConfig) -> Result<(), McpError> {
        let mut server = McpServer::new(config.clone());
        server.connect().await?;

        if matches!(
            config.transport,
            McpTransport::Http { .. } | McpTransport::Sse { .. }
        ) {
            server.initialize().await?;
        }

        let mut servers = self.servers.write().await;
        servers.insert(config.name.clone(), server);

        info!("Added MCP server: {}", config.name);
        Ok(())
    }

    pub async fn remove_server(&self, name: &str) -> Option<McpServer> {
        let mut servers = self.servers.write().await;
        servers.remove(name)
    }

    pub async fn get_server(&self, name: &str) -> Option<McpServer> {
        let servers = self.servers.read().await;
        servers.get(name).cloned()
    }

    pub async fn list_servers(&self) -> Vec<String> {
        let servers = self.servers.read().await;
        servers.keys().cloned().collect()
    }

    pub async fn connect_server(&self, name: &str) -> Result<(), McpError> {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(name) {
            server.connect().await
        } else {
            Err(McpError::NotConnected(name.to_string()))
        }
    }

    pub async fn disconnect_server(&self, name: &str) {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(name) {
            server.disconnect().await;
        }
    }

    pub async fn call_tool(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let servers = self.servers.read().await;
        if let Some(server) = servers.get(server_name) {
            server.call_tool(tool_name, arguments).await
        } else {
            Err(McpError::NotConnected(server_name.to_string()))
        }
    }

    pub async fn get_all_tools(&self) -> HashMap<String, Vec<McpTool>> {
        let servers = self.servers.read().await;
        let mut result = HashMap::new();

        for (name, server) in servers.iter() {
            if server.is_connected()
                && let Ok(tools) = server.list_tools().await
            {
                result.insert(name.clone(), tools);
            }
        }

        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    pub servers: Vec<McpServerConfig>,
    pub enabled: bool,
    pub auto_connect: bool,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            servers: Vec::new(),
            enabled: true,
            auto_connect: true,
        }
    }
}

impl McpConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
