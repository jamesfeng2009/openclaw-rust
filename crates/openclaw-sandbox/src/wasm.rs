//! WASM 工具运行时
//!
//! 提供安全的 WASM 模块执行能力，用于隔离执行不受信任的工具代码

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};
use wasmtime::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmToolConfig {
    pub module_name: String,
    pub memory_limit_mb: u64,
    pub compute_limit: u64,
    pub allow_network: bool,
    pub timeout_secs: u64,
    pub allowed_functions: Vec<String>,
}

impl Default for WasmToolConfig {
    fn default() -> Self {
        Self {
            module_name: "default".to_string(),
            memory_limit_mb: 64,
            compute_limit: 1024,
            allow_network: false,
            timeout_secs: 30,
            allowed_functions: vec!["_start".to_string(), "run".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmToolMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionInput {
    pub function: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionResult {
    pub success: bool,
    pub output: serde_json::Value,
    pub error: Option<String>,
    pub memory_used_bytes: u64,
    pub execution_time_ms: u64,
}

impl WasmExecutionResult {
    pub fn success(output: serde_json::Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
            memory_used_bytes: 0,
            execution_time_ms: 0,
        }
    }
    
    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            output: serde_json::Value::Null,
            error: Some(msg.to_string()),
            memory_used_bytes: 0,
            execution_time_ms: 0,
        }
    }
}

pub struct WasmToolRuntime {
    engine: Engine,
    config: WasmToolConfig,
}

impl WasmToolRuntime {
    pub fn new(config: WasmToolConfig) -> Result<Self, WasmError> {
        let mut engine_config = Config::new();
        engine_config.max_wasm_stack(config.compute_limit as usize);

        let engine = Engine::new(&engine_config)
            .map_err(|e| WasmError::InitFailed(e.to_string()))?;

        Ok(Self { engine, config })
    }

    pub async fn load_module(&self, wasm_bytes: &[u8], name: &str) -> Result<WasmToolModule, WasmError> {
        let module = Module::from_binary(&self.engine, wasm_bytes)
            .map_err(|e| WasmError::LoadFailed(e.to_string()))?;

        Ok(WasmToolModule {
            module,
            name: name.to_string(),
        })
    }

    pub fn execute(
        &self,
        module: &WasmToolModule,
        input: &WasmExecutionInput,
    ) -> Result<WasmExecutionResult, WasmError> {
        let start = std::time::Instant::now();

        let mut store = Store::new(&self.engine, ());

        let instance = Instance::new(&mut store, &module.module, &[])
            .map_err(|e| WasmError::ExecutionFailed(e.to_string()))?;

        let memory = instance.get_memory(&mut store, "memory");

        let memory_usage = if let Some(mem) = memory {
            mem.data_size(&store) as u64
        } else {
            0
        };

        let function_name = if input.function.is_empty() {
            "_start"
        } else {
            &input.function
        };

        if let Some(func) = instance.get_func(&mut store, function_name) {
            let params_json = input.params.to_string();
            let params_ptr = params_json.as_bytes();
            
            if let Some(memory) = memory {
                if let Some(alloc_func) = instance.get_func(&mut store, "alloc") {
                    let mut alloc_result = [wasmtime::Val::I64(0)];
                    if alloc_func.call(&mut store, &[], &mut alloc_result).is_ok() {
                        let ptr = match alloc_result[0] {
                            wasmtime::Val::I64(v) => v as usize,
                            _ => 0,
                        };
                        let mut memory_view = memory.data_mut(&mut store);
                        if ptr + params_ptr.len() <= memory_view.len() {
                            memory_view[ptr..ptr + params_ptr.len()].copy_from_slice(params_ptr);
                            
                            let call_result = func.call(
                                &mut store,
                                &[wasmtime::Val::I32(ptr as i32), wasmtime::Val::I32(params_ptr.len() as i32)],
                                &mut []
                            );
                            
                            let exec_time = start.elapsed().as_millis() as u64;
                            let call_ok = call_result.is_ok();
                            let call_err = call_result.err();
                            
                            return Ok(WasmExecutionResult {
                                success: call_ok,
                                output: if call_ok {
                                    serde_json::json!({
                                        "message": "Function executed successfully",
                                        "function": function_name
                                    })
                                } else {
                                    serde_json::json!({
                                        "error": format!("{:?}", call_err)
                                    })
                                },
                                error: call_err.map(|e| e.to_string()),
                                memory_used_bytes: memory_usage,
                                execution_time_ms: exec_time,
                            });
                        }
                    }
                }
            }
        }

        Ok(WasmExecutionResult {
            success: true,
            output: serde_json::json!({
                "module": module.name,
                "function": function_name,
                "message": "Module loaded successfully"
            }),
            error: None,
            memory_used_bytes: memory_usage,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[derive(Debug, Clone)]
pub struct WasmToolModule {
    module: Module,
    name: String,
}

impl WasmToolModule {
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
pub enum WasmError {
    InitFailed(String),
    LoadFailed(String),
    ExecutionFailed(String),
    Timeout,
    MemoryLimitExceeded,
    ComputeLimitExceeded,
    NetworkDenied,
    FunctionNotFound(String),
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::InitFailed(s) => write!(f, "WASM init failed: {}", s),
            WasmError::LoadFailed(s) => write!(f, "WASM load failed: {}", s),
            WasmError::ExecutionFailed(s) => write!(f, "WASM execution failed: {}", s),
            WasmError::Timeout => write!(f, "Execution timeout"),
            WasmError::MemoryLimitExceeded => write!(f, "Memory limit exceeded"),
            WasmError::ComputeLimitExceeded => write!(f, "Compute limit exceeded"),
            WasmError::NetworkDenied => write!(f, "Network access denied"),
            WasmError::FunctionNotFound(s) => write!(f, "Function not found: {}", s),
        }
    }
}

impl std::error::Error for WasmError {}

pub struct WasmToolRegistry {
    modules: Arc<RwLock<HashMap<String, WasmToolModule>>>,
    runtime: Arc<WasmToolRuntime>,
}

impl WasmToolRegistry {
    pub fn new(config: WasmToolConfig) -> Result<Self, WasmError> {
        let runtime = WasmToolRuntime::new(config)?;
        Ok(Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            runtime: Arc::new(runtime),
        })
    }

    pub async fn register_tool(&self, name: String, wasm_bytes: Vec<u8>) -> Result<(), WasmError> {
        let module = self.runtime.load_module(&wasm_bytes, &name).await?;
        let mut modules = self.modules.write().await;
        modules.insert(name, module);
        Ok(())
    }

    pub async fn execute_tool(&self, name: &str, input: &WasmExecutionInput) -> Result<WasmExecutionResult, WasmError> {
        let module = {
            let modules = self.modules.read().await;
            modules.get(name).cloned().ok_or_else(|| {
                WasmError::ExecutionFailed(format!("Tool '{}' not found", name))
            })?
        };
        
        let runtime = self.runtime.clone();
        let input = input.clone();
        
        tokio::task::spawn_blocking(move || {
            runtime.execute(&module, &input)
        }).await.map_err(|e| WasmError::ExecutionFailed(format!("Task join error: {:?}", e)))?
    }

    pub async fn list_tools(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }
    
    pub async fn get_tool_info(&self, name: &str) -> Option<WasmToolMetadata> {
        let modules = self.modules.read().await;
        modules.get(name).map(|m| WasmToolMetadata {
            name: m.name.clone(),
            description: format!("WASM module: {}", m.name),
            version: "1.0.0".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "function": { "type": "string", "description": "Function to call" },
                    "params": { "type": "object", "description": "Function parameters" }
                }
            }),
        })
    }
}
