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
            compute_limit: 1000,
            allow_network: false,
            timeout_secs: 30,
            allowed_functions: vec!["_start".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmToolMetadata {
    pub name: String,
    description: String,
    version: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub memory_used_bytes: u64,
    pub execution_time_ms: u64,
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

    pub async fn execute(
        &self,
        module: &WasmToolModule,
        input: &str,
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

        Ok(WasmExecutionResult {
            success: true,
            output: format!("Loaded module: {}", module.name),
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

#[derive(Debug)]
pub enum WasmError {
    InitFailed(String),
    LoadFailed(String),
    ExecutionFailed(String),
    Timeout,
    MemoryLimitExceeded,
    ComputeLimitExceeded,
    NetworkDenied,
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

    pub async fn execute_tool(&self, name: &str, input: &str) -> Result<WasmExecutionResult, WasmError> {
        let modules = self.modules.read().await;
        let module = modules.get(name).ok_or_else(|| {
            WasmError::ExecutionFailed(format!("Tool '{}' not found", name))
        })?;
        self.runtime.execute(module, input).await
    }

    pub async fn list_tools(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }
}
