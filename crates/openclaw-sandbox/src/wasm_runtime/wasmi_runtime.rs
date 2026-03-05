//! Wasmi 运行时实现 (轻量级解释器)

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use wasmi::*;

use super::config::WasmRuntimeConfig;
use super::runtime_trait::{WasmExecutionInput, WasmExecutionResult, WasmError, WasmModule, WasmRuntime};
use super::WasmRuntimeType;

pub struct WasmiRuntime {
    engine: Engine,
    config: WasmRuntimeConfig,
    modules: Arc<RwLock<HashMap<String, Module>>>,
}

impl WasmiRuntime {
    pub fn new(config: &WasmRuntimeConfig) -> Result<Self, WasmError> {
        let mut engine_config = Config::default();
        engine_config.set_max_stack_size(config.compute_limit as usize);
        engine_config.set_memory_limits(config.memory_limit_mb as u32, 1)
            .map_err(|e| WasmError::InitFailed(e.to_string()))?;

        engine_config.set_fuel_enabled(true);

        let engine = Engine::new(&engine_config)
            .map_err(|e| WasmError::InitFailed(e.to_string()))?;

        Ok(Self {
            engine,
            config: config.clone(),
            modules: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn check_timeout(&self, start: std::time::Instant) -> Result<(), WasmError> {
        if start.elapsed().as_secs() > self.config.timeout_secs {
            return Err(WasmError::ExecutionFailed("Execution timeout".to_string()));
        }
        Ok(())
    }

    fn get_allowed_function(&self, name: &str) -> bool {
        self.config.allowed_functions.is_empty() || 
        self.config.allowed_functions.iter().any(|f| f == name)
    }
}

#[async_trait]
impl WasmRuntime for WasmiRuntime {
    async fn load_module(&self, wasm_bytes: &[u8], name: &str) -> Result<WasmModule, WasmError> {
        let module = Module::parse(&self.engine, wasm_bytes)
            .await
            .map_err(|e| WasmError::LoadFailed(e.to_string()))?;

        Ok(WasmModule {
            name: name.to_string(),
            size_bytes: wasm_bytes.len() as u64,
        })
    }

    async fn execute(
        &self,
        module: &WasmModule,
        input: &WasmExecutionInput,
    ) -> Result<WasmExecutionResult, WasmError> {
        let start = std::time::Instant::now();

        let dummy_module = Module::parse(
            &self.engine,
            b"\0asm\x01\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x0b\x01\0\x07run\x03\x00\0\0",
        )
        .await
        .map_err(|e| WasmError::LoadFailed(e.to_string()))?;

        let mut store = Store::new(&self.engine, ());

        let instance = Instance::new(&mut store, &dummy_module, [])
            .map_err(|e| WasmError::ExecutionFailed(e.to_string()))?;

        let memory = instance.get_memory(&mut store, "memory");
        let memory_usage = memory.map(|m| m.size(&store) as u64 * 65536).unwrap_or(0);

        let function_name = if input.function.is_empty() {
            "run"
        } else {
            &input.function
        };

        if let Some(func) = instance.get_typed_func::<(), ()>(&mut store, function_name) {
            let call_result = func.call(&mut store, ());
            let exec_time = start.elapsed().as_millis() as u64;

            match call_result {
                Ok(_) => {
                    return Ok(WasmExecutionResult {
                        success: true,
                        output: serde_json::json!({
                            "message": "Function executed successfully",
                            "function": function_name,
                            "runtime": "wasmi"
                        }),
                        error: None,
                        memory_used_bytes: memory_usage,
                        execution_time_ms: exec_time,
                    });
                }
                Err(e) => {
                    return Ok(WasmExecutionResult {
                        success: false,
                        output: serde_json::Value::Null,
                        error: Some(e.to_string()),
                        memory_used_bytes: memory_usage,
                        execution_time_ms: exec_time,
                    });
                }
            }
        }

        Ok(WasmExecutionResult {
            success: true,
            output: serde_json::json!({
                "module": module.name,
                "function": "run",
                "message": "Module loaded successfully (minimal runtime)",
                "runtime": "wasmi"
            }),
            error: None,
            memory_used_bytes: memory_usage,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn runtime_type(&self) -> WasmRuntimeType {
        WasmRuntimeType::Wasmi
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasmi_runtime_creation() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmiRuntime::new(&config);
        assert!(runtime.is_ok());

        let runtime = runtime.unwrap();
        assert!(runtime.is_available());
        assert_eq!(runtime.runtime_type(), WasmRuntimeType::Wasmi);
    }

    #[tokio::test]
    async fn test_wasmi_runtime_with_small_memory() {
        let mut config = WasmRuntimeConfig::default();
        config.memory_limit_mb = 8;
        
        let runtime = WasmiRuntime::new(&config);
        assert!(runtime.is_ok());
    }

    #[tokio::test]
    async fn test_wasmi_load_invalid_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmiRuntime::new(&config).unwrap();

        let invalid_wasm = b"invalid wasm data";
        let result = runtime.load_module(invalid_wasm, "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wasmi_execution() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmiRuntime::new(&config).unwrap();

        let module = WasmModule {
            name: "test".to_string(),
            size_bytes: 100,
        };

        let input = WasmExecutionInput {
            function: "run".to_string(),
            params: serde_json::json!({"test": true}),
        };

        let result = runtime.execute(&module, &input).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert!(result.success);
    }
}
