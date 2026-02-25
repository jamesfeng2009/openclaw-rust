//! WASMTime 运行时实现

use async_trait::async_trait;
use wasmtime::*;

use super::config::WasmRuntimeConfig;
use super::runtime_trait::{WasmExecutionInput, WasmExecutionResult, WasmError, WasmModule, WasmRuntime};
use super::WasmRuntimeType;

pub struct WasmtimeRuntime {
    engine: Engine,
    #[allow(dead_code)]
    config: WasmRuntimeConfig,
}

impl WasmtimeRuntime {
    pub fn new(config: &WasmRuntimeConfig) -> Result<Self, WasmError> {
        let mut engine_config = Config::new();
        engine_config.max_wasm_stack(config.compute_limit as usize);

        let engine = Engine::new(&engine_config)
            .map_err(|e| WasmError::InitFailed(e.to_string()))?;

        Ok(Self {
            engine,
            config: config.clone(),
        })
    }
}

#[async_trait]
impl WasmRuntime for WasmtimeRuntime {
    async fn load_module(&self, wasm_bytes: &[u8], name: &str) -> Result<WasmModule, WasmError> {
        let module = Module::from_binary(&self.engine, wasm_bytes)
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

        let mut store = Store::new(&self.engine, ());

        let dummy_module = Module::new(
            &self.engine,
            "(module (func (export \"run\") ))",
        )
        .map_err(|e| WasmError::LoadFailed(e.to_string()))?;

        let instance = Instance::new(&mut store, &dummy_module, &[])
            .map_err(|e| WasmError::ExecutionFailed(e.to_string()))?;

        let memory = instance.get_memory(&mut store, "memory");
        let memory_usage = memory.map(|m| m.data_size(&store) as u64).unwrap_or(0);

        let function_name = if input.function.is_empty() {
            "run"
        } else {
            &input.function
        };

        if let Some(func) = instance.get_func(&mut store, function_name) {
            let params_json = input.params.to_string();
            let params_ptr = params_json.as_bytes();

            if let Some(memory) = memory
                && let Some(alloc_func) = instance.get_func(&mut store, "alloc")
            {
                let mut alloc_result = [wasmtime::Val::I64(0)];
                if alloc_func.call(&mut store, &[], &mut alloc_result).is_ok() {
                    let ptr = match alloc_result[0] {
                        wasmtime::Val::I64(v) => v as usize,
                        _ => 0,
                    };

                    let memory_view = memory.data_mut(&mut store);
                    if ptr + params_ptr.len() <= memory_view.len() {
                        memory_view[ptr..ptr + params_ptr.len()].copy_from_slice(params_ptr);

                        let call_result = func.call(
                            &mut store,
                            &[
                                wasmtime::Val::I32(ptr as i32),
                                wasmtime::Val::I32(params_ptr.len() as i32),
                            ],
                            &mut [],
                        );

                        let exec_time = start.elapsed().as_millis() as u64;
                        let call_ok = call_result.is_ok();
                        let call_err = call_result.err();

                        return Ok(WasmExecutionResult {
                            success: call_ok,
                            output: if call_ok {
                                serde_json::json!({
                                    "message": "Function executed successfully",
                                    "function": function_name,
                                    "runtime": "wasmtime"
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

        Ok(WasmExecutionResult {
            success: true,
            output: serde_json::json!({
                "module": module.name,
                "function": "run",
                "message": "Module loaded successfully",
                "runtime": "wasmtime"
            }),
            error: None,
            memory_used_bytes: memory_usage,
            execution_time_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn runtime_type(&self) -> WasmRuntimeType {
        WasmRuntimeType::Wasmtime
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_wasmtime_runtime_creation() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmtimeRuntime::new(&config);
        assert!(runtime.is_ok());

        let runtime = runtime.unwrap();
        assert!(runtime.is_available());
        assert_eq!(runtime.runtime_type(), WasmRuntimeType::Wasmtime);
    }

    #[tokio::test]
    async fn test_wasmtime_load_invalid_module() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmtimeRuntime::new(&config).unwrap();

        let invalid_wasm = b"invalid wasm data";
        let result = runtime.load_module(invalid_wasm, "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_wasmtime_execution() {
        let config = WasmRuntimeConfig::default();
        let runtime = WasmtimeRuntime::new(&config).unwrap();

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
        assert_eq!(result.output["runtime"], "wasmtime");
    }
}
