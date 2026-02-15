use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::types::{ExecutionResult, ResourceUsage};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    pub module_name: String,
    pub memory_limit_mb: u64,
    pub compute_limit: u64,
    pub allow_network: bool,
    pub timeout_secs: u64,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            module_name: "default".to_string(),
            memory_limit_mb: 64,
            compute_limit: 1000,
            allow_network: false,
            timeout_secs: 30,
        }
    }
}

#[derive(Debug)]
pub struct WasmModule {
    config: WasmConfig,
    loaded: bool,
}

impl WasmModule {
    pub fn new(config: WasmConfig) -> Self {
        Self {
            config,
            loaded: false,
        }
    }

    pub async fn load(&mut self) -> Result<(), WasmError> {
        self.loaded = true;
        Ok(())
    }

    pub async fn execute(&self, input: &str) -> Result<ExecutionResult, WasmError> {
        if !self.loaded {
            return Err(WasmError::ModuleNotLoaded);
        }

        let start = std::time::Instant::now();

        let output = format!("WASM executed: {}", input);

        Ok(ExecutionResult {
            exit_code: 0,
            stdout: output,
            stderr: String::new(),
            timed_out: false,
            duration_secs: start.elapsed().as_secs_f64(),
            resource_usage: Some(ResourceUsage {
                cpu_time_nanos: 0,
                memory_bytes: self.config.memory_limit_mb * 1024 * 1024,
                network_rx_bytes: 0,
                network_tx_bytes: 0,
                disk_read_bytes: 0,
                disk_write_bytes: 0,
            }),
        })
    }
}

#[derive(Debug)]
pub enum WasmError {
    ModuleNotLoaded,
    ExecutionFailed(String),
    Timeout,
    MemoryLimitExceeded,
    ComputeLimitExceeded,
    NetworkDenied,
}

impl std::fmt::Display for WasmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmError::ModuleNotLoaded => write!(f, "WASM module not loaded"),
            WasmError::ExecutionFailed(s) => write!(f, "Execution failed: {}", s),
            WasmError::Timeout => write!(f, "Execution timeout"),
            WasmError::MemoryLimitExceeded => write!(f, "Memory limit exceeded"),
            WasmError::ComputeLimitExceeded => write!(f, "Compute limit exceeded"),
            WasmError::NetworkDenied => write!(f, "Network access denied"),
        }
    }
}

impl std::error::Error for WasmError {}

pub struct WasmRuntime {
    modules: Arc<RwLock<HashMap<String, WasmModule>>>,
    allow_network: bool,
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            allow_network: false,
        }
    }

    pub async fn load_module(&self, name: String, config: WasmConfig) -> Result<(), WasmError> {
        let mut module = WasmModule::new(config);
        module.load().await?;

        let mut modules = self.modules.write().await;
        modules.insert(name, module);

        info!("WASM module loaded");
        Ok(())
    }

    pub async fn execute(&self, module_name: &str, input: &str) -> Result<ExecutionResult, WasmError> {
        let modules = self.modules.read().await;
        
        if let Some(module) = modules.get(module_name) {
            module.execute(input).await
        } else {
            Err(WasmError::ModuleNotLoaded)
        }
    }

    pub async fn list_modules(&self) -> Vec<String> {
        let modules = self.modules.read().await;
        modules.keys().cloned().collect()
    }

    pub fn set_network_allowed(&mut self, allowed: bool) {
        self.allow_network = allowed;
    }
}

pub struct WasmSandbox {
    runtime: WasmRuntime,
    config: WasmConfig,
}

impl Default for WasmSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmSandbox {
    pub fn new() -> Self {
        Self {
            runtime: WasmRuntime::new(),
            config: WasmConfig::default(),
        }
    }

    pub async fn execute(&self, input: &str) -> Result<ExecutionResult, WasmError> {
        self.runtime
            .execute(&self.config.module_name, input)
            .await
    }

    pub async fn load_tool(&self, name: &str, wasm_bytes: &[u8]) -> Result<(), WasmError> {
        let config = WasmConfig {
            module_name: name.to_string(),
            ..Default::default()
        };
        
        self.runtime.load_module(name.to_string(), config).await
    }
}
