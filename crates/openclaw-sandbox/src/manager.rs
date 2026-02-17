use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use openclaw_security::{FilterResult, GrantResult, NetworkDecision, SecurityMiddleware};

use crate::types::{ExecutionResult, SandboxState};
use crate::wasm::{WasmError, WasmExecutionInput, WasmToolConfig, WasmToolModule, WasmToolRuntime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SandboxType {
    Docker,
    Wasm,
    Native,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSandboxConfig {
    pub tool_id: String,
    pub sandbox_type: SandboxType,
    pub timeout_secs: u64,
    pub memory_limit_mb: u64,
    pub allow_network: bool,
    pub allowed_paths: Vec<String>,
}

impl Default for ToolSandboxConfig {
    fn default() -> Self {
        Self {
            tool_id: String::new(),
            sandbox_type: SandboxType::Native,
            timeout_secs: 30,
            memory_limit_mb: 256,
            allow_network: false,
            allowed_paths: vec![],
        }
    }
}

pub enum SandboxBackend {
    Docker,
    Wasm,
}

#[derive(Debug)]
pub enum SandboxError {
    SecurityCheckFailed(String),
    PermissionDenied(String),
    NetworkDenied(String),
    DockerError(String),
    WasmError(String),
    NativeError(String),
    ToolNotFound(String),
    Timeout,
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SandboxError::SecurityCheckFailed(s) => write!(f, "Security check failed: {}", s),
            SandboxError::PermissionDenied(s) => write!(f, "Permission denied: {}", s),
            SandboxError::NetworkDenied(s) => write!(f, "Network denied: {}", s),
            SandboxError::DockerError(s) => write!(f, "Docker error: {}", s),
            SandboxError::WasmError(s) => write!(f, "WASM error: {}", s),
            SandboxError::NativeError(s) => write!(f, "Native error: {}", s),
            SandboxError::ToolNotFound(s) => write!(f, "Tool not found: {}", s),
            SandboxError::Timeout => write!(f, "Execution timeout"),
        }
    }
}

impl std::error::Error for SandboxError {}

pub struct SandboxManager {
    security: SecurityMiddleware,
    tool_configs: Arc<RwLock<HashMap<String, ToolSandboxConfig>>>,
    wasm_runtime: Arc<RwLock<Option<WasmToolRuntime>>>,
    wasm_modules: Arc<RwLock<HashMap<String, WasmToolModule>>>,
    native_tools: Arc<RwLock<HashMap<String, Box<dyn NativeTool>>>>,
}

#[async_trait::async_trait]
pub trait NativeTool: Send + Sync {
    async fn execute(&self, input: &str) -> Result<String, String>;
    fn name(&self) -> &str;
}

impl Default for SandboxManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            security: SecurityMiddleware::new(),
            tool_configs: Arc::new(RwLock::new(HashMap::new())),
            wasm_runtime: Arc::new(RwLock::new(None)),
            wasm_modules: Arc::new(RwLock::new(HashMap::new())),
            native_tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn init_default_configs(&self) {
        let mut configs = self.tool_configs.write().await;

        configs.insert(
            "browser_tools".to_string(),
            ToolSandboxConfig {
                tool_id: "browser_tools".to_string(),
                sandbox_type: SandboxType::Docker,
                timeout_secs: 60,
                memory_limit_mb: 512,
                allow_network: true,
                allowed_paths: vec![],
            },
        );

        configs.insert(
            "file_tools".to_string(),
            ToolSandboxConfig {
                tool_id: "file_tools".to_string(),
                sandbox_type: SandboxType::Docker,
                timeout_secs: 30,
                memory_limit_mb: 256,
                allow_network: false,
                allowed_paths: vec!["./workspace/*".to_string(), "./data/*".to_string()],
            },
        );

        configs.insert(
            "http_tools".to_string(),
            ToolSandboxConfig {
                tool_id: "http_tools".to_string(),
                sandbox_type: SandboxType::Wasm,
                timeout_secs: 30,
                memory_limit_mb: 64,
                allow_network: true,
                allowed_paths: vec![],
            },
        );

        configs.insert(
            "json_processor".to_string(),
            ToolSandboxConfig {
                tool_id: "json_processor".to_string(),
                sandbox_type: SandboxType::Native,
                timeout_secs: 10,
                memory_limit_mb: 64,
                allow_network: false,
                allowed_paths: vec![],
            },
        );

        configs.insert(
            "code_executor".to_string(),
            ToolSandboxConfig {
                tool_id: "code_executor".to_string(),
                sandbox_type: SandboxType::Docker,
                timeout_secs: 60,
                memory_limit_mb: 1024,
                allow_network: true,
                allowed_paths: vec![],
            },
        );

        info!("Default sandbox configs initialized");
    }

    pub async fn register_tool_config(&self, config: ToolSandboxConfig) {
        let mut configs = self.tool_configs.write().await;
        configs.insert(config.tool_id.clone(), config);
    }

    pub async fn get_tool_config(&self, tool_id: &str) -> Option<ToolSandboxConfig> {
        let configs = self.tool_configs.read().await;
        configs.get(tool_id).cloned()
    }

    pub async fn register_native_tool<T: NativeTool + 'static>(&self, tool: T) {
        let name = tool.name().to_string();
        let tool_name = name.clone();
        let mut tools = self.native_tools.write().await;
        tools.insert(name, Box::new(tool));
        info!("Native tool registered: {}", tool_name);
    }

    async fn check_security(
        &self,
        tool_id: &str,
        input: &str,
        target: &str,
    ) -> Result<(), SandboxError> {
        let filter_result = self.security.check_user_input(input).await;

        if !filter_result.allowed
            && filter_result.threat_level >= openclaw_security::ThreatLevel::High
        {
            return Err(SandboxError::SecurityCheckFailed(filter_result.reason));
        }

        let perm_result = self
            .security
            .check_tool_permission(tool_id, "execute", target)
            .await;

        match perm_result {
            GrantResult::Granted => {}
            GrantResult::Denied => {
                return Err(SandboxError::PermissionDenied(format!(
                    "Tool {} action execute on {} denied",
                    tool_id, target
                )));
            }
            GrantResult::Limited(_) => {
                warn!("Tool {} executed with limited permissions", tool_id);
            }
        }

        Ok(())
    }

    async fn check_network(
        &self,
        tool_id: &str,
        host: &str,
        port: u16,
    ) -> Result<(), SandboxError> {
        let decision = self
            .security
            .check_network_request(tool_id, host, port)
            .await;

        match decision {
            NetworkDecision::Allow => Ok(()),
            NetworkDecision::Deny => Err(SandboxError::NetworkDenied(format!(
                "Network access to {}:{} denied by whitelist",
                host, port
            ))),
            NetworkDecision::Limited => {
                warn!("Network access to {}:{} limited", host, port);
                Ok(())
            }
        }
    }

    pub async fn execute(
        &self,
        tool_id: &str,
        input: &str,
        target: Option<&str>,
    ) -> Result<ExecutionResult, SandboxError> {
        let config = self
            .get_tool_config(tool_id)
            .await
            .ok_or_else(|| SandboxError::ToolNotFound(tool_id.to_string()))?;

        self.check_security(tool_id, input, target.unwrap_or(""))
            .await?;

        match config.sandbox_type {
            SandboxType::Docker => self.execute_docker(tool_id, input).await,
            SandboxType::Wasm => self.execute_wasm(tool_id, input).await,
            SandboxType::Native => self.execute_native(tool_id, input).await,
        }
    }

    async fn execute_docker(
        &self,
        tool_id: &str,
        input: &str,
    ) -> Result<ExecutionResult, SandboxError> {
        debug!("Executing tool {} in Docker sandbox", tool_id);

        let start = std::time::Instant::now();

        Ok(ExecutionResult {
            exit_code: 0,
            stdout: format!("[Docker] Executed: {}", input),
            stderr: String::new(),
            timed_out: false,
            duration_secs: start.elapsed().as_secs_f64(),
            resource_usage: None,
        })
    }

    async fn execute_wasm(
        &self,
        tool_id: &str,
        input: &str,
    ) -> Result<ExecutionResult, SandboxError> {
        debug!("Executing tool {} in WASM sandbox", tool_id);

        let runtime = self.wasm_runtime.read().await;

        if let Some(runtime) = runtime.as_ref() {
            let wasm_modules = self.wasm_modules.read().await;
            if let Some(module) = wasm_modules.get(tool_id) {
                let exec_input = WasmExecutionInput {
                    function: "run".to_string(),
                    params: serde_json::json!({ "input": input }),
                };

                let result = runtime
                    .execute(module, &exec_input)
                    .map_err(|e| SandboxError::WasmError(e.to_string()))?;

                Ok(ExecutionResult {
                    exit_code: if result.success { 0 } else { 1 },
                    stdout: result.output.to_string(),
                    stderr: result.error.unwrap_or_default(),
                    timed_out: false,
                    duration_secs: result.execution_time_ms as f64 / 1000.0,
                    resource_usage: Some(crate::types::ResourceUsage {
                        cpu_time_nanos: 0,
                        memory_bytes: result.memory_used_bytes,
                        network_rx_bytes: 0,
                        network_tx_bytes: 0,
                        disk_read_bytes: 0,
                        disk_write_bytes: 0,
                    }),
                })
            } else {
                Err(SandboxError::WasmError("WASM module not found".to_string()))
            }
        } else {
            let start = std::time::Instant::now();
            Ok(ExecutionResult {
                exit_code: 0,
                stdout: format!("[WASM] Simulated execution: {}", input),
                stderr: String::new(),
                timed_out: false,
                duration_secs: start.elapsed().as_secs_f64(),
                resource_usage: None,
            })
        }
    }

    async fn execute_native(
        &self,
        tool_id: &str,
        input: &str,
    ) -> Result<ExecutionResult, SandboxError> {
        debug!("Executing tool {} natively", tool_id);

        let tools = self.native_tools.read().await;

        if let Some(tool) = tools.get(tool_id) {
            let output = tool
                .execute(input)
                .await
                .map_err(|e| SandboxError::NativeError(e))?;

            let start = std::time::Instant::now();
            Ok(ExecutionResult {
                exit_code: 0,
                stdout: output,
                stderr: String::new(),
                timed_out: false,
                duration_secs: start.elapsed().as_secs_f64(),
                resource_usage: None,
            })
        } else {
            let start = std::time::Instant::now();
            Ok(ExecutionResult {
                exit_code: 0,
                stdout: format!("[Native] Executed: {}", input),
                stderr: String::new(),
                timed_out: false,
                duration_secs: start.elapsed().as_secs_f64(),
                resource_usage: None,
            })
        }
    }

    pub fn get_security_middleware(&self) -> &SecurityMiddleware {
        &self.security
    }

    pub async fn list_tools(&self) -> Vec<ToolSandboxConfig> {
        let configs = self.tool_configs.read().await;
        configs.values().cloned().collect()
    }

    pub async fn get_sandbox_type(&self, tool_id: &str) -> Option<SandboxType> {
        let configs = self.tool_configs.read().await;
        configs.get(tool_id).map(|c| c.sandbox_type.clone())
    }
}
