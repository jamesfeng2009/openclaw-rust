use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

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
    pub fn stdio(name: impl Into<String>, command: impl Into<String>) -> Self {
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

#[derive(Debug, Clone)]
pub struct McpServer {
    config: McpServerConfig,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    connected: bool,
}

impl McpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            tools: Vec::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
            connected: false,
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
                self.connected = true;
                Ok(())
            }
            McpTransport::Sse { url } => {
                info!("Connecting to MCP SSE server at: {}", url);
                self.connected = true;
                Ok(())
            }
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

    pub async fn call_tool(&self, name: &str, arguments: serde_json::Value) -> Result<serde_json::Value, McpError> {
        if !self.connected {
            return Err(McpError::NotConnected(self.config.name.clone()));
        }

        let tool = self.tools.iter().find(|t| t.name == name);
        if tool.is_none() {
            return Err(McpError::ToolNotFound(name.to_string()));
        }

        debug!("Calling MCP tool: {} on server: {}", name, self.config.name);
        
        Ok(serde_json::json!({
            "success": true,
            "content": []
        }))
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

    pub async fn get_prompt(&self, name: &str, arguments: HashMap<String, String>) -> Result<serde_json::Value, McpError> {
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

    pub async fn call_tool(&self, server_name: &str, tool_name: &str, arguments: serde_json::Value) -> Result<serde_json::Value, McpError> {
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
            if server.is_connected() {
                if let Ok(tools) = server.list_tools().await {
                    result.insert(name.clone(), tools);
                }
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
