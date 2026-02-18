//! WASM 工具执行器
//!
//! 提供 WASM 模块的加载和执行能力

use crate::types::*;
use openclaw_sandbox::wasm::{WasmError, WasmExecutionInput, WasmToolConfig, WasmToolRegistry};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct WasmToolExecutor {
    registry: Arc<WasmToolRegistry>,
}

impl WasmToolExecutor {
    pub fn new() -> Result<Self, WasmError> {
        let config = WasmToolConfig::default();
        let registry = WasmToolRegistry::new(config)?;
        Ok(Self {
            registry: Arc::new(registry),
        })
    }

    pub async fn register_tool(&self, name: String, wasm_bytes: Vec<u8>) -> Result<(), WasmError> {
        info!("Registering WASM tool: {}", name);
        self.registry.register_tool(name, wasm_bytes).await
    }

    pub async fn execute(&self, name: &str, params: serde_json::Value) -> ToolResult {
        let input = WasmExecutionInput {
            function: "run".to_string(),
            params,
        };

        match self.registry.execute_tool(name, &input).await {
            Ok(result) => {
                if result.success {
                    ToolResult {
                        success: true,
                        output: result.output,
                        error: result.error,
                        metadata: {
                            let mut m = std::collections::HashMap::new();
                            m.insert(
                                "memory_used".to_string(),
                                format!("{} bytes", result.memory_used_bytes),
                            );
                            m.insert(
                                "execution_time".to_string(),
                                format!("{} ms", result.execution_time_ms),
                            );
                            m
                        },
                    }
                } else {
                    ToolResult {
                        success: false,
                        output: result.output,
                        error: result.error,
                        metadata: std::collections::HashMap::new(),
                    }
                }
            }
            Err(e) => {
                error!("WASM execution error: {}", e);
                ToolResult {
                    success: false,
                    output: serde_json::Value::Null,
                    error: Some(e.to_string()),
                    metadata: std::collections::HashMap::new(),
                }
            }
        }
    }

    pub async fn list_tools(&self) -> Vec<String> {
        self.registry.list_tools().await
    }

    pub async fn get_tool_info(&self, name: &str) -> Option<ToolDefinition> {
        let info = self.registry.get_tool_info(name).await?;
        Some(ToolDefinition {
            id: info.name.clone(),
            name: info.name,
            description: info.description,
            parameters: ToolParameters {
                properties: {
                    let mut props = std::collections::HashMap::new();
                    props.insert(
                        "function".to_string(),
                        ParameterProperty {
                            param_type: "string".to_string(),
                            description: "Function to execute".to_string(),
                            enum_values: vec![],
                            default: Some(serde_json::Value::String("run".to_string())),
                        },
                    );
                    props.insert(
                        "params".to_string(),
                        ParameterProperty {
                            param_type: "object".to_string(),
                            description: "Function parameters".to_string(),
                            enum_values: vec![],
                            default: None,
                        },
                    );
                    props
                },
                required: vec!["function".to_string()],
            },
            category: ToolCategory::Custom,
            enabled: true,
        })
    }
}

pub type WasmExecutor = Arc<RwLock<WasmToolExecutor>>;

pub async fn create_wasm_executor() -> Result<WasmExecutor, WasmError> {
    let executor = WasmToolExecutor::new()?;
    Ok(Arc::new(RwLock::new(executor)))
}
